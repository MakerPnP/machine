`include "src/main/logging.svh"

// timer_pwm.v
//
// 4 independent STM32-timer-style 8-bit counters (source clock -> 8-bit
// prescaler -> 8-bit auto-reload) feeding 12 flexible PWM output
// channels (PM1-4, OT1-8), each channel independently mapped to any of
// the 4 timers with its own 8-bit compare value and output polarity.
//
// INVARIANT: like every other peripheral behind memory.v, NO REGISTER
// HERE MAY HAVE A READ SIDE EFFECT - see memory.v's header comment.
// TIMx_CTRL's RESET bit is therefore never stored as persistent state:
// a write with that bit set produces a same-cycle reset pulse and the
// bit itself always reads back 0, exactly like REG_LC_CTRL.SINGLE in
// loadcell.v.
//
// ARCHITECTURE
// ---------------------------------------------------------------------
// - TIM_SYNC is a single global gate: a timer only counts while BOTH
//   its own TIMx_CTRL.ENABLE and TIM_SYNC.ENABLE are set. This lets a
//   host configure all 4 timers (prescaler/ARR/initial CNT) and each
//   timer's own ENABLE bit first, completely at rest, then start every
//   enabled timer on the same sys_clk edge by writing TIM_SYNC last -
//   the same "arm everything, then fire together" shape as steppers.v's
//   start-strobe bitmap, but for phase-aligning free-running counters
//   instead of one-shot motion profiles.
// - TIM_SYNC.PRESCALER is a SHARED 6-bit prescaler ahead of all 4
//   timers' own 8-bit prescalers (a timer only advances on a sys_clk
//   cycle this stage "ticks" on), multiplying rather than replacing
//   them: effective divide to one timer TICK is (TIM_SYNC.PRESCALER+1)
//   * (TIMx_CTRL.PRESCALER+1) sys_clk cycles, and a full period is that
//   times (TIMx_ARR+1) ticks - up to 64*256*256 = 4,194,304 sys_clk
//   cycles (50MHz/4194304 ~= 11.9Hz floor, versus ~763Hz with the
//   8-bit-only per-timer prescaler alone). It costs one instance of the
//   shared stage instead of widening all 4 timers' own prescaler
//   fields, at the cost of the 4 timers no longer being able to pick
//   arbitrarily different frequencies spanning the FULL range
//   simultaneously - they all still share this one divide factor, only
//   their own 8-bit prescaler/ARR vary independently on top of it.
//   Like the per-timer prescaler, it's parked at its configured reload
//   value (not counting) whenever TIM_SYNC.ENABLE is 0, so every
//   timer's phase relative to the shared stage is deterministic the
//   instant sync starts, the same guarantee TIMx_CTRL.RESET gives each
//   timer's own counter.
// - Each timer counts 0..ARR then wraps to 0 (STM32 upcounting mode).
//   TIMx_CNT is directly host-writable (while a timer is stopped, this
//   is how "different initial counter values" gets set up before the
//   first TIM_SYNC-gated start), and TIMx_CTRL.RESET forces it back to
//   0 immediately, independent of counting state.
// - PWM_CTRLx.TIMER_SRC selects, per channel, which timer's CNT that
//   channel's comparator watches - deliberately many-to-one (e.g. two
//   OT outputs can share one timer's frequency while each keeps its
//   own duty cycle), since PM1-4/OT1-8 may drive identical or entirely
//   different hardware (see the module header comment in core_top.v).
// - Per channel: pin = (CNT < CMP) ? active_level : idle_level, where
//   active_level is HIGH for POLARITY=0 (normal) and LOW for POLARITY=1
//   (inverted) - a pure function of the channel's own timer's CNT, its
//   own CMP, and its own POLARITY. The active phase is therefore the
//   FIRST CMP cycles of the period (conventional PWM mode-1 shape):
//   CMP directly sets the active (duty) width, and that width always
//   starts at CNT=0. This already produces the right behaviour "when a
//   timer resets" (natural ARR wrap or an explicit TIMx_CTRL.RESET): the
//   instant CNT reads 0 again, the same comparison re-evaluates to
//   active_level (assuming CMP != 0) with no separate reset-handling
//   logic needed.
// - PWM_CTRL (global, offset 0x00) is the "disable ALL outputs" master
//   switch required for OT1-8's electrical safety: OUTPUT_ENABLE=0 (the
//   reset default) forces every one of the 12 pins LOW regardless of
//   per-channel ENABLE/POLARITY, and drives OT_EN to its inactive
//   level, tri-stating the SN74HCT245 that isolates OT1-8 from the
//   FPGA. PM1-4 have no such buffer (their ADUM120N0 isolators are
//   always enabled per the hardware description), so forcing their
//   pins LOW is the only "disable" available to them - which is
//   sufficient, since LOW is the inactive/safe level either way.
//
// PWM CHANNEL STORAGE: 12 channels x (enable, polarity, timer_src, 8-bit
// compare) started out as flip-flops decoded by a 24-branch case
// statement (PWM_CTRL1-12/PWM_CMP1-12) - on an iCE40HX8K this measured
// ~990 LUT4/144 SB_CARRY for the whole peripheral, tipping a design that
// otherwise fit comfortably (6326/7680 LC, 82%) up to 7546/7680 (98%),
// which nextpnr could not place. Sharing the 12 parallel comparators
// round-robin, and various register-layout reshuffles, each measured
// only partial relief (the underlying amount of state read back on the
// bus is the same either way). The fix that actually worked: this
// state now lives in one genuine SB_RAM40_4K block RAM instead - the
// device had 12 of 32 EBRs completely idle (62% used), and a block
// RAM's address decode is native hardware, not LUT-synthesized, so an
// address range check plus one subtraction replaces the 24-way decode
// entirely. It also happens to force the shared comparator below into
// exactly the round-robin scheme that was independently worth trying,
// as a free consequence of the RAM having only one read port rather
// than a separately-designed scheme. iCE40 EBR contents are loaded from
// the bitstream at configuration time (SB_RAM40_4K's INIT_0-INIT_F
// parameters, defaulted to zero here), so this is a genuine hardware
// zero at power-on, not just a simulation convenience - PWM_CTRLn/
// PWM_CMPn keep reading back 0 after reset exactly as before. The 24
// register addresses and bit layouts are completely unchanged; only the
// internal implementation and (for PWM_CTRLn/PWM_CMPn specifically) the
// read latency changed - see the RAM/arbitration comment below.
// ---------------------------------------------------------------------

module timer_pwm (
    input  wire        reset,
    input  wire        sys_clk,

    // Bus Slave Interface
    input  wire        bus_stb,
    input  wire        bus_we,
    input  wire [7:0]  bus_addr,
    input  wire [31:0] bus_din,
    output reg  [31:0] bus_dout,
    output reg         bus_ack,

    // Physical Hardware Interfaces
    output wire [3:0]  pm_out,   // PM1-4
    output wire [7:0]  ot_out,   // OT1-8
    output wire        ot_en     // SN74HCT245 /OE, active low
);

    `include "src/main/io/timer_regs.svh"
    `include "src/main/io/timer_shared.svh"

    // ------------------------------------------------------------------
    // Global registers
    // ------------------------------------------------------------------
    reg pwm_output_enable;
    reg tim_sync_enable;
    reg [5:0] tim_sync_prescaler;   // shared prescaler ahead of all 4 timers
    reg [5:0] tim_sync_presc_cnt;   // internal divide-down counter for it

    // Combinational: this sys_clk cycle is a "global tick" - the one
    // cycle in every (tim_sync_prescaler+1) that the 4 timers' own
    // prescale/tick logic below is allowed to advance on.
    wire tim_sync_tick = tim_sync_enable && (tim_sync_presc_cnt == 6'd0);

    // ------------------------------------------------------------------
    // Per-timer state (index 0-3 == TIM1-4)
    // ------------------------------------------------------------------
    reg        tim_enable    [0:3];
    reg [7:0]  tim_prescaler [0:3];
    reg [7:0]  tim_arr       [0:3];
    reg [7:0]  tim_cnt       [0:3];
    reg [7:0]  tim_presc_cnt [0:3]; // internal divide-down counter

    // Registered comparator output per PWM channel (index 0-3 == PM1-4,
    // index 4-11 == OT1-8) - the actual pin state. Each channel's own
    // enable/polarity/timer_src/compare configuration lives in block
    // RAM now (see below), not here.
    reg pwm_level [0:11];

    integer i;

    // ------------------------------------------------------------------
    // PWM channel storage: one SB_RAM40_4K, addressed 0-11 by channel,
    // holding both that channel's CTRL fields and its CMP value in one
    // 16-bit word - see the module header comment for why. Word layout
    // (internal only - never bus-visible directly):
    //   [3:0]   CTRL nibble: {timer_src[1:0], polarity, enable}
    //   [11:4]  CMP value (8 bits)
    //   [15:12] unused
    // A host write to PWM_CTRLn or PWM_CMPn updates only its own nibble
    // (CTRL) or byte (CMP) via the RAM's write MASK, leaving the other
    // half of the word untouched.
    //
    // Port arbitration: the RAM has exactly one read port, shared
    // between the bus (an occasional host read of PWM_CTRLn/PWM_CMPn)
    // and the comparator scanner below (which wants continuous access,
    // cycling through every channel). The bus wins whenever a read is
    // actually in flight; otherwise the scanner uses it. Since
    // SB_RAM40_4K's read output is registered (RADDR must be stable one
    // full cycle before RDATA reflects it), *_valid_d below are 1-cycle-
    // delayed copies of "who drove RADDR last cycle", used to correctly
    // attribute the RAM's current output once it arrives - the same
    // shared-port-arbitration shape steppers.v uses for its segment
    // table (see that file's "SEGMENT READBACK" comment), except here
    // losing the odd cycle to the bus just means the scanner revisits
    // that channel on its next pass ~12 cycles later, utterly
    // negligible against any realistic PWM period.
    // ------------------------------------------------------------------
    wire        pwm_ch_sel    = bus_stb && (bus_addr >= REG_PWM_CTRL1) && (bus_addr <= REG_PWM_CMP12);
    wire [3:0]  pwm_ch_idx    = (bus_addr - REG_PWM_CTRL1) >> 3; // 0..11, 8 bytes/channel
    wire        pwm_ch_is_cmp = bus_addr[2];                     // CTRLn at +0, CMPn at +4
    wire        bus_owns_pwm_port = pwm_ch_sel && !bus_we;       // a bus READ in flight wins the port

    reg  [3:0]  cmp_scan;             // round-robin channel the comparator wants this cycle
    reg         bus_pwm_read_valid_d; // last cycle's RADDR belonged to a bus read
    reg         pwm_ch_is_cmp_d;      // ...and whether it was CTRL or CMP
    reg         pwm_scan_valid_d;     // last cycle's RADDR belonged to the comparator scan
    reg  [3:0]  pwm_scan_addr_d;      // ...and which channel

    wire [3:0]  pwm_raddr = bus_owns_pwm_port ? pwm_ch_idx : cmp_scan;
    wire [15:0] pwm_rdata;

    // A host write commits combinationally the instant it's decoded -
    // RAM writes need no extra latency, only reads do (the registered
    // RDATA path above/below).
    wire        pwm_wr_en   = pwm_ch_sel && bus_we && !bus_ack;
    wire [15:0] pwm_wr_mask = pwm_ch_is_cmp ? 16'hF00F : 16'hFFF0;
    wire [15:0] pwm_wr_data = pwm_ch_is_cmp
        ? {4'd0, bus_din[7:0], 4'd0}
        : {12'd0, bus_din[PWM_CTRL_TIMER_SRC_LSB +: PWM_CTRL_TIMER_SRC_W],
                  bus_din[PWM_CTRL_POLARITY_BIT], bus_din[PWM_CTRL_ENABLE_BIT]};

    SB_RAM40_4K #(.WRITE_MODE(0), .READ_MODE(0)) pwm_ram (
        .RCLK(sys_clk), .RCLKE(1'b1), .RE(1'b1),
        .RADDR({7'b0, pwm_raddr}), .RDATA(pwm_rdata),
        .WCLK(sys_clk), .WCLKE(1'b1), .WE(pwm_wr_en),
        .WADDR({7'b0, pwm_ch_idx}), .WDATA(pwm_wr_data), .MASK(pwm_wr_mask)
    );

    wire       pwm_rd_enable    = pwm_rdata[0];
    wire       pwm_rd_polarity  = pwm_rdata[1];
    wire [1:0] pwm_rd_timer_src = pwm_rdata[3:2];
    wire [7:0] pwm_rd_cmp       = pwm_rdata[11:4];

    // Scratch (blocking-assigned) variables for the comparator - same
    // pattern as steppers.v's scratch registers.
    reg cmp_active;
    reg cmp_level;

    assign pm_out = {pwm_level[3], pwm_level[2], pwm_level[1], pwm_level[0]};
    assign ot_out = {pwm_level[11], pwm_level[10], pwm_level[9], pwm_level[8],
                      pwm_level[7], pwm_level[6], pwm_level[5], pwm_level[4]};
    // Active-low: OUTPUT_ENABLE=1 drives OT_EN low (SN74HCT245 outputs
    // active); OUTPUT_ENABLE=0 (reset default) drives it high (isolated).
    assign ot_en = !pwm_output_enable;

    always @(posedge sys_clk) begin
        if (reset) begin
            bus_dout          <= 32'h00000000;
            bus_ack           <= 1'b0;

            pwm_output_enable <= 1'b0; // safe default: outputs disabled, OT_EN isolated
            tim_sync_enable   <= 1'b0;
            tim_sync_prescaler <= 6'd0;
            tim_sync_presc_cnt <= 6'd0;

            for (i = 0; i < 4; i = i + 1) begin
                tim_enable[i]    <= 1'b0;
                tim_prescaler[i] <= 8'd0;
                tim_arr[i]       <= 8'd0;
                tim_cnt[i]       <= 8'd0;
                tim_presc_cnt[i] <= 8'd0;
            end

            for (i = 0; i < 12; i = i + 1) begin
                pwm_level[i] <= 1'b0;
            end

            cmp_scan             <= 4'd0;
            bus_pwm_read_valid_d <= 1'b0;
            pwm_ch_is_cmp_d      <= 1'b0;
            pwm_scan_valid_d     <= 1'b0;
            pwm_scan_addr_d      <= 4'd0;
        end else begin
            // ========================================================
            // Shared prescaler ahead of all 4 timers (TIM_SYNC.
            // PRESCALER). Parked at its configured reload value (not
            // counting) whenever TIM_SYNC.ENABLE is 0, so every timer's
            // phase relative to it is deterministic the instant sync
            // starts - the same guarantee TIMx_CTRL.RESET gives each
            // timer's own counter.
            // ========================================================
            if (tim_sync_enable) begin
                if (tim_sync_tick)
                    tim_sync_presc_cnt <= tim_sync_prescaler;
                else
                    tim_sync_presc_cnt <= tim_sync_presc_cnt - 6'd1;
            end else begin
                tim_sync_presc_cnt <= tim_sync_prescaler;
            end

            // ========================================================
            // Free-running counters. Only advance on a shared "global
            // tick" (tim_sync_tick above) - a same-cycle bus write to a
            // timer's own registers (below, later in program order)
            // overrides these nonblocking assignments for that timer.
            // ========================================================
            for (i = 0; i < 4; i = i + 1) begin
                if (tim_sync_tick && tim_enable[i]) begin
                    if (tim_presc_cnt[i] == 8'd0) begin
                        tim_presc_cnt[i] <= tim_prescaler[i];
                        if (tim_cnt[i] == tim_arr[i])
                            tim_cnt[i] <= 8'd0;
                        else
                            tim_cnt[i] <= tim_cnt[i] + 8'd1;
                    end else begin
                        tim_presc_cnt[i] <= tim_presc_cnt[i] - 8'd1;
                    end
                end
            end

            // ========================================================
            // PWM RAM port arbitration bookkeeping - see the port
            // declaration comment above for what these track.
            // ========================================================
            bus_pwm_read_valid_d <= bus_owns_pwm_port;
            pwm_ch_is_cmp_d      <= pwm_ch_is_cmp;
            pwm_scan_valid_d     <= !bus_owns_pwm_port;
            pwm_scan_addr_d      <= cmp_scan;
            cmp_scan             <= (cmp_scan == 4'd11) ? 4'd0 : cmp_scan + 4'd1;

            // ========================================================
            // Bus slave interface. Every register acks in one cycle
            // except PWM_CTRLn/PWM_CMPn reads, which need one extra
            // cycle for the RAM's registered output to catch up with a
            // freshly-presented address (writes commit combinationally
            // via pwm_wr_en above, no extra latency). No register here
            // has a read side effect.
            // ========================================================
            if (bus_stb) begin
                if (pwm_ch_sel && !bus_we) begin
                    if (bus_pwm_read_valid_d) begin
                        bus_dout <= pwm_ch_is_cmp_d
                            ? {24'd0, pwm_rd_cmp}
                            : {24'd0, 2'd0, pwm_rd_timer_src, 2'd0, pwm_rd_polarity, pwm_rd_enable};
                        bus_ack  <= 1'b1;
                    end
                    // else: address just started settling into the RAM's
                    // registered output this cycle - ack next cycle.
                end else if (!bus_ack) begin
                    bus_ack <= 1'b1;
                    if (bus_we) begin
                        `DBG_LOG(("timer_pwm bus write. addr: %02x, value: %08h", bus_addr, bus_din));
                        case (bus_addr)
                            REG_PWM_CTRL: begin
                                pwm_output_enable <= bus_din[PWM_GLOBAL_OUTPUT_ENABLE_BIT];
                            end

                            REG_TIM_SYNC: begin
                                tim_sync_enable    <= bus_din[TIM_SYNC_ENABLE_BIT];
                                tim_sync_prescaler <= bus_din[TIM_SYNC_PRESCALER_LSB +: TIM_SYNC_PRESCALER_W];
                            end

                            REG_TIM1_CTRL: begin
                                tim_enable[0]    <= bus_din[TIM_CTRL_ENABLE_BIT];
                                tim_prescaler[0] <= bus_din[TIM_CTRL_PRESCALER_LSB +: TIM_CTRL_PRESCALER_W];
                                if (bus_din[TIM_CTRL_RESET_BIT]) begin
                                    tim_cnt[0]       <= 8'd0;
                                    tim_presc_cnt[0] <= bus_din[TIM_CTRL_PRESCALER_LSB +: TIM_CTRL_PRESCALER_W];
                                end
                            end
                            REG_TIM1_ARR: tim_arr[0] <= bus_din[7:0];
                            REG_TIM1_CNT: tim_cnt[0] <= bus_din[7:0];

                            REG_TIM2_CTRL: begin
                                tim_enable[1]    <= bus_din[TIM_CTRL_ENABLE_BIT];
                                tim_prescaler[1] <= bus_din[TIM_CTRL_PRESCALER_LSB +: TIM_CTRL_PRESCALER_W];
                                if (bus_din[TIM_CTRL_RESET_BIT]) begin
                                    tim_cnt[1]       <= 8'd0;
                                    tim_presc_cnt[1] <= bus_din[TIM_CTRL_PRESCALER_LSB +: TIM_CTRL_PRESCALER_W];
                                end
                            end
                            REG_TIM2_ARR: tim_arr[1] <= bus_din[7:0];
                            REG_TIM2_CNT: tim_cnt[1] <= bus_din[7:0];

                            REG_TIM3_CTRL: begin
                                tim_enable[2]    <= bus_din[TIM_CTRL_ENABLE_BIT];
                                tim_prescaler[2] <= bus_din[TIM_CTRL_PRESCALER_LSB +: TIM_CTRL_PRESCALER_W];
                                if (bus_din[TIM_CTRL_RESET_BIT]) begin
                                    tim_cnt[2]       <= 8'd0;
                                    tim_presc_cnt[2] <= bus_din[TIM_CTRL_PRESCALER_LSB +: TIM_CTRL_PRESCALER_W];
                                end
                            end
                            REG_TIM3_ARR: tim_arr[2] <= bus_din[7:0];
                            REG_TIM3_CNT: tim_cnt[2] <= bus_din[7:0];

                            REG_TIM4_CTRL: begin
                                tim_enable[3]    <= bus_din[TIM_CTRL_ENABLE_BIT];
                                tim_prescaler[3] <= bus_din[TIM_CTRL_PRESCALER_LSB +: TIM_CTRL_PRESCALER_W];
                                if (bus_din[TIM_CTRL_RESET_BIT]) begin
                                    tim_cnt[3]       <= 8'd0;
                                    tim_presc_cnt[3] <= bus_din[TIM_CTRL_PRESCALER_LSB +: TIM_CTRL_PRESCALER_W];
                                end
                            end
                            REG_TIM4_ARR: tim_arr[3] <= bus_din[7:0];
                            REG_TIM4_CNT: tim_cnt[3] <= bus_din[7:0];

                            // PWM_CTRLn/PWM_CMPn writes commit via the
                            // combinational pwm_wr_en/pwm_wr_data/
                            // pwm_wr_mask feeding the RAM instance above
                            // (already asserted this exact cycle) - just
                            // ack here, nothing further to do.
                            default: begin end
                        endcase
                    end else begin
                        case (bus_addr)
                            REG_PWM_CTRL: bus_dout <= {31'd0, pwm_output_enable};
                            REG_TIM_SYNC: bus_dout <= tim_sync_word(tim_sync_enable, tim_sync_prescaler);

                            REG_TIM1_CTRL: bus_dout <= tim_ctrl_word(tim_enable[0], 1'b0, tim_prescaler[0]);
                            REG_TIM1_ARR:  bus_dout <= {24'd0, tim_arr[0]};
                            REG_TIM1_CNT:  bus_dout <= {24'd0, tim_cnt[0]};

                            REG_TIM2_CTRL: bus_dout <= tim_ctrl_word(tim_enable[1], 1'b0, tim_prescaler[1]);
                            REG_TIM2_ARR:  bus_dout <= {24'd0, tim_arr[1]};
                            REG_TIM2_CNT:  bus_dout <= {24'd0, tim_cnt[1]};

                            REG_TIM3_CTRL: bus_dout <= tim_ctrl_word(tim_enable[2], 1'b0, tim_prescaler[2]);
                            REG_TIM3_ARR:  bus_dout <= {24'd0, tim_arr[2]};
                            REG_TIM3_CNT:  bus_dout <= {24'd0, tim_cnt[2]};

                            REG_TIM4_CTRL: bus_dout <= tim_ctrl_word(tim_enable[3], 1'b0, tim_prescaler[3]);
                            REG_TIM4_ARR:  bus_dout <= {24'd0, tim_arr[3]};
                            REG_TIM4_CNT:  bus_dout <= {24'd0, tim_cnt[3]};

                            // PWM_CTRLn/PWM_CMPn reads are intercepted
                            // above (they need the extra RAM-latency
                            // cycle) and never reach this case.
                            default: bus_dout <= 32'hDEAD7000;
                        endcase
                    end
                end
            end else begin
                bus_ack <= 1'b0;
            end

            // ========================================================
            // Shared comparator: whenever the RAM's registered output
            // belongs to the scanner rather than a bus read
            // (pwm_scan_valid_d), update that channel's pin. A disabled
            // channel (its own CTRL.ENABLE, or the global PWM_CTRL.
            // OUTPUT_ENABLE) is forced LOW regardless of polarity, per
            // spec.
            // ========================================================
            if (pwm_scan_valid_d) begin
                cmp_active = pwm_output_enable && pwm_rd_enable;
                cmp_level  = (tim_cnt[pwm_rd_timer_src] < pwm_rd_cmp) ? !pwm_rd_polarity : pwm_rd_polarity;
                pwm_level[pwm_scan_addr_d] <= cmp_active ? cmp_level : 1'b0;
            end
        end
    end

endmodule
