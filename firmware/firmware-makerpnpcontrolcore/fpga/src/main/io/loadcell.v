`timescale 1ns / 1ps
`include "src/main/logging.svh"

// loadcell.v
//
// HX717 dual-channel 24-bit load-cell ADC peripheral.
//
// The HX717 has no register file - every control is expressed through
// the PD_SCK pin. This module owns that pin timing so the MCU never has
// to: the MCU writes a channel/gain selection and an enable bit, and
// reads back a 24-bit two's-complement sample plus a ready flag.
//
// PROTOCOL SUMMARY (datasheet Fig.2, Tables 3 and 4)
// ---------------------------------------------------------------------
// DOUT is high while no result is pending. When it falls, a result is
// waiting. The host then issues PD_SCK pulses:
//
//   pulses 1..24  shift out the 24-bit result, MSB first. The bit for
//                 pulse N appears on DOUT within T2 (0.1us) of that
//                 pulse's *rising* edge, so we sample at the end of the
//                 high phase - ~1us of settling, 10x the T2 maximum.
//   pulse 25      returns DOUT high and selects CH A / gain 128 for the
//                 next conversion.
//   pulses 26/27/28  select CH B g64 / CH A g64 / CH B g8 instead.
//
// The *total* pulse count is the channel/gain command. There is no
// separate opcode - clocking 27 times instead of 25 is the entire
// mechanism. That is why this module counts pulses explicitly rather
// than treating the trailing pulses as an afterthought: an off-by-one
// silently reconfigures the analog front end.
//
// Power-down is likewise a pin gesture: hold PD_SCK high for more than
// 80us. Where in the pulse train you stop selects the depth (see
// LC_PD_* in loadcell_shared.svh). The datasheet is explicit that the
// power-down must not interrupt a conversion in progress, or the
// channel/gain setup is not saved - so the PD request bit is honoured
// only at the end of a conversion, never mid-train.
//
// CLOCKING
// ---------------------------------------------------------------------
// PD_SCK is generated from sys_clk by a phase counter, not by a derived
// clock net. A genuinely divided clock would need either a global
// buffer (iCE40 has 8, all spoken for) or a gated local net, and would
// put the DOUT sampling in a second clock domain for no benefit. The
// FSM below runs entirely on sys_clk and treats PD_SCK as ordinary
// registered output data, which is also what lets the same counter
// serve the T1 guard and the 80us power-down hold.
//
// BUS READS HAVE NO SIDE EFFECTS
// ---------------------------------------------------------------------
// The MCU reaches this peripheral through the STM32H735 OctoSPI in
// memory-mapped mode, which prefetches a 16-byte FIFO line aligned to
// 16 bytes. All three registers sit inside one line, so reading any of
// them causes the controller to read all of them. Anything that changed
// state on read would therefore fire spuriously - polling REG_LC_STATUS
// would clear a flag in REG_LC_VALUE, and a second access inside the
// same line would be served from the FIFO without reaching the FPGA at
// all. So: no register here is modified by a read, ever. New-sample
// detection uses the sequence counter in REG_LC_VALUE[31:24] instead of
// a read-to-consume handshake.
//
// SIGN
// ---------------------------------------------------------------------
// REG_LC_VALUE returns the raw 24 bits in [23:0] - i.e. exactly
// what came off the wire, in two's complement, 0x800000 (most negative)
// through 0x7FFFFF (most positive). The MCU firmware is Rust, and
// recovers a signed sample with
//
//     let sample = ((v << 8) as i32) >> 8;
//
// which shifts the sequence byte out of the top and lets the arithmetic
// shift do the sign extension, so no mask is needed. Sign-extending
// here in hardware instead would mean the register no longer reports
// what the chip actually said, which makes saturation (0x800000 /
// 0x7FFFFF exactly) harder to detect.

module loadcell (
    input  wire        reset,
    input  wire        sys_clk,

    input  wire        bus_stb,
    input  wire        bus_we,
    input  wire [7:0]  bus_addr,
    input  wire [31:0] bus_din,
    output reg  [31:0] bus_dout,
    output reg         bus_ack,

    output reg         s0,
    output reg         s1,
    output reg         pd_sck,
    input  wire        dout
);

    `include "src/main/io/loadcell_regs.svh"
    `include "src/main/io/loadcell_shared.svh"

    // ------------------------------------------------------------------
    // Counter widths
    // ------------------------------------------------------------------
    localparam US_CNT_W    = $clog2(LC_TIMEOUT_US + 1);
    localparam PHASE_CNT_W = $clog2(LC_SCK_HALF_CYCLES + 1);

    // Sized copies of the shared constants. The shared file states them
    // as plain integers so it stays readable; comparisons against
    // unsigned counters need them explicitly sized, or the comparison
    // silently becomes signed.
    localparam [US_CNT_W-1:0]    TIMEOUT_LAST  = LC_TIMEOUT_US - 1;
    localparam [US_CNT_W-1:0]    PD_HOLD_LAST  = LC_PD_HOLD_US - 1;
    localparam [PHASE_CNT_W-1:0] SCK_HALF_LAST = LC_SCK_HALF_CYCLES - 1;
    localparam [PHASE_CNT_W-1:0] T1_LAST       = LC_T1_CYCLES - 1;
    localparam [4:0]             DATA_PULSES   = LC_DATA_PULSES;
    localparam [4:0]             PULSES_BASE   = LC_PULSES_BASE;
    localparam [4:0]             PULSES_PD_REG = LC_PULSES_PD_REG;
    localparam [4:0]             PULSES_PD_ALL = LC_PULSES_PD_ALL;

    // ------------------------------------------------------------------
    // FSM states
    // ------------------------------------------------------------------
    localparam ST_IDLE      = 3'd0;  // PD_SCK low, nothing requested
    localparam ST_WAIT_RDY  = 3'd1;  // PD_SCK low, waiting for DOUT to fall
    localparam ST_T1_GUARD  = 3'd2;  // PD_SCK low, satisfying T1
    localparam ST_SCK_HI    = 3'd3;  // PD_SCK high phase of one pulse
    localparam ST_SCK_LO    = 3'd4;  // PD_SCK low phase of one pulse
    localparam ST_PD_HOLD   = 3'd5;  // PD_SCK high, counting out 80us+
    localparam ST_PD_IDLE   = 3'd6;  // PD_SCK high, device asleep

    reg [2:0] state;

    // ------------------------------------------------------------------
    // Register file
    // ------------------------------------------------------------------

    // enable, ODR control (uses S0, S1), start continuous conversion,
    // power-down mode, etc. See loadcell_shared.svh for the bit layout.
    reg [LC_CTRL_W-1:0] loadcell_ctrl;

    // enabled / ready / busy / powered-down / timeout
    reg [LC_STATUS_W-1:0] loadcell_status;

    // copy of the last buffer, for use by the bus
    reg [23:0] loadcell_value;

    // buffer for reading into
    reg [23:0] loadcell_buffer;

    // Incremented once per completed conversion and returned in the top
    // byte of REG_LC_VALUE. This is what replaces clear-on-read: the
    // host compares against the last value it saw rather than consuming
    // a flag, which is the only scheme that survives a prefetching
    // master.
    reg [LC_SEQ_W-1:0] sample_seq;

    wire       ctrl_enable = loadcell_ctrl[LC_CTRL_ENABLE_BIT];
    wire       ctrl_pd     = loadcell_ctrl[LC_CTRL_PD_BIT];
    wire [1:0] ctrl_rate   = loadcell_ctrl[LC_CTRL_RATE_LSB   +: 2];
    wire [1:0] ctrl_mode   = loadcell_ctrl[LC_CTRL_MODE_LSB   +: 2];
    wire [1:0] ctrl_pdmode = loadcell_ctrl[LC_CTRL_PDMODE_LSB +: 2];

    // ------------------------------------------------------------------
    // DOUT input synchronizer. DOUT is asynchronous to sys_clk and
    // changes within 0.1us of a PD_SCK rising edge we ourselves drive,
    // so it is genuinely metastability-prone. Three stages; the extra
    // 60ns of latency is irrelevant next to the 1us half-period.
    // ------------------------------------------------------------------
    reg [2:0] dout_sync_q;
    wire      dout_sync = dout_sync_q[2];

    // ------------------------------------------------------------------
    // Timing counters
    // ------------------------------------------------------------------
    reg [5:0]            us_div;     // sys_clk -> 1us tick
    reg [US_CNT_W-1:0]   us_cnt;     // watchdog / power-down hold
    reg [PHASE_CNT_W-1:0] phase_cnt; // within one PD_SCK half period
    reg [4:0]            pulse_index; // 1..30, the pulse being issued

    wire us_tick = (us_div == LC_CLK_MHZ - 1);

    // ------------------------------------------------------------------
    // Pulse target.
    //
    // pd_armed/pdmode_armed are latched at the end of pulse 24, which is
    // the last moment the count can still be changed and the first
    // moment the answer is needed. Latching there (rather than at
    // conversion start) means a PD request issued while a conversion is
    // already in flight is honoured by *this* conversion, not the next.
    // ------------------------------------------------------------------
    reg        pd_armed;
    reg [1:0]  pdmode_armed;
    reg [1:0]  mode_armed;

    wire [4:0] total_pulses =
        (pd_armed && (pdmode_armed == LC_PD_ADC_REG)) ? PULSES_PD_REG :
        (pd_armed && (pdmode_armed == LC_PD_ALL))     ? PULSES_PD_ALL :
                                          PULSES_BASE + {3'd0, mode_armed};

    // One-shot request. Set by a bus write of LC_CTRL_SINGLE_BIT,
    // consumed when the FSM leaves ST_IDLE.
    reg single_pending;

    wire run_request = ctrl_enable | single_pending;

    // ------------------------------------------------------------------
    // New-request detection, same shape as steppers.v: a request is new
    // whenever (addr, we) differs from the one last serviced, or bus_stb
    // was low last cycle. Gating on !bus_ack alone would miss a master
    // that drops and re-raises stb between two posedges.
    // ------------------------------------------------------------------
    reg [7:0] last_bus_addr;
    reg       last_bus_we;
    reg       last_bus_valid;
    wire is_new_request = bus_stb && (!last_bus_valid ||
                                      (bus_addr != last_bus_addr) ||
                                      (bus_we   != last_bus_we));

    always @(posedge sys_clk) begin
        if (reset) begin
            loadcell_ctrl   <= {LC_CTRL_W{1'b0}};
            loadcell_status <= {LC_STATUS_W{1'b0}};
            loadcell_value  <= 24'd0;
            loadcell_buffer <= 24'd0;
            sample_seq      <= {LC_SEQ_W{1'b0}};
            s0              <= 1'b0;
            s1              <= 1'b0;
            pd_sck          <= 1'b0;
            bus_dout        <= 32'h00000000;
            bus_ack         <= 1'b0;

            dout_sync_q     <= 3'b111;   // DOUT idles high
            us_div          <= 6'd0;
            us_cnt          <= {US_CNT_W{1'b0}};
            phase_cnt       <= {PHASE_CNT_W{1'b0}};
            pulse_index     <= 5'd0;
            pd_armed        <= 1'b0;
            pdmode_armed    <= 2'd0;
            mode_armed      <= 2'd0;
            single_pending  <= 1'b0;
            state           <= ST_IDLE;

            last_bus_addr   <= 8'd0;
            last_bus_we     <= 1'b0;
            last_bus_valid  <= 1'b0;
        end else begin
            // Defaults, overridden below where needed.
            bus_ack     <= 1'b0;
            dout_sync_q <= {dout_sync_q[1:0], dout};

            // Rate pins track the control register continuously. The
            // HX717 samples S1/S0 asynchronously, so a rate change takes
            // effect on whichever conversion the chip happens to be in -
            // change it while disabled if you care about the boundary.
            s1 <= ctrl_rate[1];
            s0 <= ctrl_rate[0];

            loadcell_status[LC_STATUS_ENABLED_BIT] <= ctrl_enable;

            // 1us timebase
            if (us_tick) us_div <= 6'd0;
            else         us_div <= us_div + 6'd1;

            // ==========================================================
            // Bus slave interface. Single-cycle ack; every register is a
            // plain flop, so there is nothing to wait for.
            // ==========================================================
            if (!bus_stb) begin
                last_bus_valid <= 1'b0;
            end else if (is_new_request) begin
                last_bus_addr  <= bus_addr;
                last_bus_we    <= bus_we;
                last_bus_valid <= 1'b1;

                case (bus_addr)
                    REG_LC_CTRL: begin
                        if (bus_we) begin
                            loadcell_ctrl <= bus_din[LC_CTRL_W-1:0];
                            // SINGLE is a self-clearing trigger, so it
                            // is peeled off rather than stored.
                            loadcell_ctrl[LC_CTRL_SINGLE_BIT] <= 1'b0;
                            if (bus_din[LC_CTRL_SINGLE_BIT])
                                single_pending <= 1'b1;
                            // Any control write clears the sticky
                            // timeout flag - writing control is how you
                            // acknowledge and retry - and clears READY,
                            // which is what gives READY its meaning:
                            // "a sample has landed under the current
                            // configuration". Both are write side
                            // effects, which the prefetching master
                            // cannot trigger accidentally.
                            loadcell_status[LC_STATUS_TIMEOUT_BIT] <= 1'b0;
                            loadcell_status[LC_STATUS_READY_BIT]   <= 1'b0;
                            `DBG_LOG(("[LC ] %0t CTRL write %03h en=%0d pd=%0d rate=%0d mode=%0d pdmode=%0d single=%0d",
                                      $time, bus_din[LC_CTRL_W-1:0],
                                      bus_din[LC_CTRL_ENABLE_BIT],
                                      bus_din[LC_CTRL_PD_BIT],
                                      bus_din[LC_CTRL_RATE_LSB   +: 2],
                                      bus_din[LC_CTRL_MODE_LSB   +: 2],
                                      bus_din[LC_CTRL_PDMODE_LSB +: 2],
                                      bus_din[LC_CTRL_SINGLE_BIT]));
                            bus_dout <= 32'h00000000;
                        end else begin
                            bus_dout <= {{(32-LC_CTRL_W){1'b0}}, loadcell_ctrl};
                        end
                        bus_ack <= 1'b1;
                    end

                    REG_LC_STATUS: begin
                        // Read-only. Writes are accepted and discarded
                        // so a careless host cannot hang the bus.
                        bus_dout <= {{(32-LC_STATUS_W){1'b0}}, loadcell_status};
                        bus_ack  <= 1'b1;
                    end

                    REG_LC_VALUE: begin
                        // Value and sequence counter in one word so a
                        // single 32-bit read gets a consistent pair.
                        // NO SIDE EFFECT - see the header. This read is
                        // issued by the OctoSPI prefetch whenever the
                        // host touches any register in this 16-byte
                        // line, including a bare poll of REG_LC_STATUS.
                        bus_dout <= {sample_seq, loadcell_value};
                        bus_ack  <= 1'b1;
                    end

                    default: begin
                        bus_dout <= 32'hDEADBEEF;
                        bus_ack  <= 1'b1;
                    end
                endcase
            end

            // ==========================================================
            // PD_SCK state machine.
            //
            // Runs after the bus decode in program order, so on the rare
            // cycle where both touch loadcell_status the FSM wins - the
            // hardware's view of READY is the authoritative one.
            // ==========================================================
            case (state)

                // ------------------------------------------------------
                ST_IDLE: begin
                    pd_sck      <= 1'b0;
                    pulse_index <= 5'd0;
                    phase_cnt   <= {PHASE_CNT_W{1'b0}};
                    us_cnt      <= {US_CNT_W{1'b0}};
                    loadcell_status[LC_STATUS_BUSY_BIT] <= 1'b0;
                    loadcell_status[LC_STATUS_PD_BIT]   <= 1'b0;

                    if (ctrl_pd && !run_request) begin
                        // Power-down with no conversion pending: the
                        // plain Fig.3 gesture, low -> high -> hold.
                        pd_armed     <= 1'b1;
                        pdmode_armed <= ctrl_pdmode;
                        pd_sck       <= 1'b1;
                        us_cnt       <= {US_CNT_W{1'b0}};
                        state        <= ST_PD_HOLD;
                        `DBG_LOG(("[LC ] %0t idle power-down, mode=%0d", $time, ctrl_pdmode));
                    end else if (run_request) begin
                        // single_pending is deliberately NOT cleared
                        // here. It is half of run_request, and
                        // ST_WAIT_RDY aborts on !run_request - clearing
                        // it on the way in would make a one-shot cancel
                        // itself on the very next cycle. It is retired
                        // once the pulse train actually starts, which is
                        // the point of no return.
                        pd_armed       <= 1'b0;
                        mode_armed     <= ctrl_mode;
                        loadcell_buffer <= 24'd0;
                        loadcell_status[LC_STATUS_BUSY_BIT] <= 1'b1;
                        state <= ST_WAIT_RDY;
                    end
                end

                // ------------------------------------------------------
                ST_WAIT_RDY: begin
                    pd_sck <= 1'b0;

                    if (!run_request) begin
                        // Enable withdrawn before we committed to a
                        // pulse train - safe to abandon, no partial
                        // clocking has happened.
                        state <= ST_IDLE;
                    end else if (!dout_sync) begin
                        phase_cnt <= {PHASE_CNT_W{1'b0}};
                        us_cnt    <= {US_CNT_W{1'b0}};
                        state     <= ST_T1_GUARD;
                    end else if (us_tick) begin
                        if (us_cnt >= TIMEOUT_LAST) begin
                            us_cnt <= {US_CNT_W{1'b0}};
                            loadcell_status[LC_STATUS_TIMEOUT_BIT] <= 1'b1;
                            loadcell_status[LC_STATUS_BUSY_BIT]    <= 1'b0;
                            single_pending <= 1'b0;  // one-shot gives up
                            state  <= ST_IDLE;
                            `DBG_LOG(("[LC ] %0t TIMEOUT waiting for DOUT low", $time));
                        end else begin
                            us_cnt <= us_cnt + 1'b1;
                        end
                    end
                end

                // ------------------------------------------------------
                // T1: DOUT falling edge to first PD_SCK rising edge,
                // 0.1us minimum. The synchronizer has already spent 60ns
                // of that; this adds 200ns more.
                // ------------------------------------------------------
                ST_T1_GUARD: begin
                    pd_sck <= 1'b0;
                    if (phase_cnt >= T1_LAST) begin
                        phase_cnt      <= {PHASE_CNT_W{1'b0}};
                        pulse_index    <= 5'd1;
                        pd_sck         <= 1'b1;
                        single_pending <= 1'b0;   // committed to a train
                        state          <= ST_SCK_HI;
                        // Both are printed because they legitimately
                        // differ: mode_armed was latched when this
                        // conversion was armed, while ctrl_mode may
                        // already hold a change the host made during the
                        // wait for DOUT. The pulse-24 re-latch below is
                        // what decides which one the train actually
                        // uses, so seeing them diverge here is normal.
                        `DBG_LOG(("[LC ] %0t conversion start, mode=%0d (ctrl_mode now %0d)",
                                  $time, mode_armed, ctrl_mode));
                    end else begin
                        phase_cnt <= phase_cnt + 1'b1;
                    end
                end

                // ------------------------------------------------------
                // High phase. The data bit for this pulse is sampled at
                // the very end of it - ~1us after the rising edge that
                // produced it, against a T2 maximum of 0.1us.
                // ------------------------------------------------------
                ST_SCK_HI: begin
                    pd_sck <= 1'b1;

                    if (phase_cnt >= SCK_HALF_LAST) begin
                        phase_cnt <= {PHASE_CNT_W{1'b0}};

                        if (pulse_index <= DATA_PULSES) begin
                            loadcell_buffer <= {loadcell_buffer[22:0], dout_sync};

                            if (pulse_index == DATA_PULSES) begin
                                // All 24 bits are in. Publish
                                // immediately - the trailing
                                // channel-select pulses carry no data,
                                // so there is no reason to make the host
                                // wait for them.
                                loadcell_value <= {loadcell_buffer[22:0], dout_sync};
                                sample_seq     <= sample_seq + 1'b1;
                                loadcell_status[LC_STATUS_READY_BIT] <= 1'b1;
                                // Last chance to fix the pulse target.
                                // Latching rather than using the live
                                // control bits matters: the pulse count
                                // IS the channel command, so a host
                                // write landing between pulse 25 and the
                                // end of the train would otherwise
                                // truncate it and select a channel
                                // nobody asked for.
                                pd_armed     <= ctrl_pd;
                                pdmode_armed <= ctrl_pdmode;
                                mode_armed   <= ctrl_mode;
                                `DBG_LOG(("[LC ] %0t sample = %06h", $time,
                                          {loadcell_buffer[22:0], dout_sync}));
                            end
                        end

                        if (pulse_index == total_pulses) begin
                            if (pd_armed) begin
                                // Do NOT complete the low phase: the
                                // power-down condition is PD_SCK staying
                                // high after this pulse's rising edge.
                                us_cnt <= {US_CNT_W{1'b0}};
                                state  <= ST_PD_HOLD;
                                `DBG_LOG(("[LC ] %0t power-down after %0d pulses, mode=%0d",
                                          $time, pulse_index, pdmode_armed));
                            end else begin
                                pd_sck <= 1'b0;
                                state  <= ST_SCK_LO;
                            end
                        end else begin
                            pd_sck <= 1'b0;
                            state  <= ST_SCK_LO;
                        end
                    end else begin
                        phase_cnt <= phase_cnt + 1'b1;
                    end
                end

                // ------------------------------------------------------
                ST_SCK_LO: begin
                    pd_sck <= 1'b0;

                    if (phase_cnt >= SCK_HALF_LAST) begin
                        phase_cnt <= {PHASE_CNT_W{1'b0}};

                        if (pulse_index >= total_pulses) begin
                            // The delivered pulse count is the
                            // channel/gain command the part actually
                            // received - log it unconditionally, since
                            // it is the one number worth diffing when a
                            // conversion comes back on the wrong
                            // channel.
                            `DBG_LOG(("[LC ] %0t train complete, %0d pulses, mode=%0d",
                                      $time, pulse_index, mode_armed));
                            // Train complete. If the host has since
                            // dropped ENABLE we stop here rather than
                            // mid-train - the pulse count is the channel
                            // command, so an aborted train would leave
                            // the analog front end configured for
                            // something nobody asked for.
                            pulse_index <= 5'd0;
                            loadcell_status[LC_STATUS_BUSY_BIT] <= 1'b0;
                            state <= ST_IDLE;
                        end else begin
                            pulse_index <= pulse_index + 5'd1;
                            pd_sck      <= 1'b1;
                            state       <= ST_SCK_HI;
                        end
                    end else begin
                        phase_cnt <= phase_cnt + 1'b1;
                    end
                end

                // ------------------------------------------------------
                // Hold PD_SCK high past the 80us power-down threshold.
                // ------------------------------------------------------
                ST_PD_HOLD: begin
                    pd_sck <= 1'b1;
                    loadcell_status[LC_STATUS_BUSY_BIT] <= 1'b0;

                    if (us_tick) begin
                        if (us_cnt >= PD_HOLD_LAST) begin
                            us_cnt <= {US_CNT_W{1'b0}};
                            loadcell_status[LC_STATUS_PD_BIT] <= 1'b1;
                            state  <= ST_PD_IDLE;
                            `DBG_LOG(("[LC ] %0t powered down", $time));
                        end else begin
                            us_cnt <= us_cnt + 1'b1;
                        end
                    end
                end

                // ------------------------------------------------------
                // Asleep. Releasing PD_SCK wakes the chip, which resumes
                // with the setup conditions it had before the
                // power-down.
                // ------------------------------------------------------
                ST_PD_IDLE: begin
                    pd_sck <= 1'b1;
                    loadcell_status[LC_STATUS_PD_BIT] <= 1'b1;

                    if (!ctrl_pd) begin
                        pd_sck      <= 1'b0;
                        pd_armed    <= 1'b0;
                        pulse_index <= 5'd0;
                        loadcell_status[LC_STATUS_PD_BIT] <= 1'b0;
                        state <= ST_IDLE;
                        `DBG_LOG(("[LC ] %0t wake", $time));
                    end
                end

                default: state <= ST_IDLE;
            endcase
        end
    end
endmodule
