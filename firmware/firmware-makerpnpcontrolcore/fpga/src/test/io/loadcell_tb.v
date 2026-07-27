`timescale 1ns / 1ps
`include "src/test/assertions.svh"

// loadcell_tb.v
//
// Drives the loadcell peripheral against hx717_sim, a behavioural model
// of the HX717 built from the datasheet timing.
//
// The model independently checks every datasheet timing parameter and
// counts violations, so err_count == 0 at the end of the run is as much
// a part of the pass criterion as the register assertions - it is what
// distinguishes "the driver produced the right number" from "the driver
// produced the right number while clocking the part out of spec".
//
// Build:
//   iverilog -g2012 -DSIM -o build/loadcell_tb -I. \
//            src/test/loadcell_tb.v src/test/hx717_sim.v src/main/io/loadcell.v
//   vvp build/loadcell_tb

module loadcell_tb;

    // ------------------------------------------------------------------
    // Clock and reset. sys_clk is the 50 MHz TCXO.
    // ------------------------------------------------------------------
    reg TCXO  = 1'b0;
    reg RESET = 1'b0;

    always #10 TCXO = ~TCXO;    // 20ns period -> 50 MHz

    // Bus master signals + bus_write/bus_read/sys_reset tasks. Note this
    // declares `dout` as the bus read-data register, which is why the
    // HX717 serial data line is called hx_dout below.
    `include "src/test/bus_io.svh"

    `include "src/main/io/loadcell_regs.svh"
    `include "src/main/io/loadcell_shared.svh"

    // ------------------------------------------------------------------
    // DUT <-> HX717 model interconnect
    // ------------------------------------------------------------------
    wire lc_s0;
    wire lc_s1;
    wire lc_pd_sck;
    wire hx_dout;

    reg  [23:0] sample_value;

    wire [23:0] hx_latched_value;
    wire [31:0] hx_conv_count;
    wire [4:0]  hx_last_pulse_count;
    wire        hx_chan_b;
    wire [7:0]  hx_gain;
    wire        hx_powered_down;
    wire [1:0]  hx_pd_level;
    wire [31:0] hx_err_count;

    loadcell dut (
        .reset   (RESET),
        .sys_clk (TCXO),

        .bus_stb (stb),
        .bus_we  (we),
        .bus_addr(addr),
        .bus_din (din),
        .bus_dout(dout),
        .bus_ack (ack),

        .s0      (lc_s0),
        .s1      (lc_s1),
        .pd_sck  (lc_pd_sck),
        .dout    (hx_dout)
    );

    hx717_sim hx (
        .s0              (lc_s0),
        .s1              (lc_s1),
        .pd_sck          (lc_pd_sck),
        .dout            (hx_dout),

        .sample_value    (sample_value),

        .latched_value   (hx_latched_value),
        .conv_count      (hx_conv_count),
        .last_pulse_count(hx_last_pulse_count),
        .chan_b          (hx_chan_b),
        .gain            (hx_gain),
        .powered_down    (hx_powered_down),
        .pd_level        (hx_pd_level),
        .err_count       (hx_err_count)
    );

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------
    reg [31:0] rd;
    reg [31:0] st_before;
    reg [31:0] val_before;
    reg [7:0]  seq_a;
    integer    prefetch;
    integer    conv_mark;
    reg [4:0]  pulse_mark;
    reg        ready_ok;

    task lc_ctrl_write(
        input       en,
        input       pd,
        input [1:0] rate,
        input [1:0] mode,
        input [1:0] pdmode,
        input       single
    );
        begin
            bus_write(REG_LC_CTRL,
                      {{(32-LC_CTRL_W){1'b0}},
                       lc_ctrl_word(en, pd, rate, mode, pdmode, single)});
        end
    endtask

    // Poll REG_LC_STATUS until READY appears. Returns 0 on give-up so a
    // hung DUT produces a failed assertion rather than a hung run.
    task lc_wait_ready(output ok);
        integer polls;
        reg [31:0] st;
        begin
            ok    = 1'b0;
            polls = 0;
            while (!ok && polls < 4000) begin
                bus_read(REG_LC_STATUS, st);
                if (st[LC_STATUS_READY_BIT]) ok = 1'b1;
                else begin
                    polls = polls + 1;
                    #2000;      // 2us between polls
                end
            end
        end
    endtask

    // Wait for the PD_SCK train to finish.
    //
    // READY is raised as soon as the 24th data bit lands, which is
    // deliberately *before* the trailing channel-select pulses go out -
    // so anything that inspects the pulse count has to wait for the
    // train itself, not just for the data. Detecting quiet on the pin
    // rather than reading a status bit keeps this independent of the
    // DUT's internal state encoding.
    task lc_wait_train_done;
        integer quiet;
        begin
            quiet = 0;
            while (quiet < 20) begin
                #500;                       // 0.5us
                if (lc_pd_sck) quiet = 0;
                else           quiet = quiet + 1;
            end
        end
    endtask

    // Poll REG_LC_STATUS until the peripheral reports itself asleep.
    // The model powers down at the datasheet's 80us threshold, but the
    // DUT holds PD_SCK for LC_PD_HOLD_US before claiming it - so the
    // model's flag leads the register by ~20us and the two must be
    // checked in that order.
    task lc_wait_pd(output ok);
        integer polls;
        reg [31:0] st;
        begin
            ok    = 1'b0;
            polls = 0;
            while (!ok && polls < 2000) begin
                bus_read(REG_LC_STATUS, st);
                if (st[LC_STATUS_PD_BIT]) ok = 1'b1;
                else begin
                    polls = polls + 1;
                    #2000;
                end
            end
        end
    endtask

    // Wait for the sequence counter in REG_LC_VALUE[31:24] to move on
    // from `seq0`. This is the whole new-sample mechanism now that reads
    // are side-effect free - there is no flag to consume. The result is
    // left in `rd` so the caller can check the value that came with it,
    // which matters: value and counter arrive in one 32-bit read and
    // therefore cannot skew against each other.
    task lc_wait_new_sample(input [7:0] seq0, output ok);
        integer polls;
        begin
            ok    = 1'b0;
            polls = 0;
            while (!ok && polls < 4000) begin
                bus_read(REG_LC_VALUE, rd);
                if (rd[31:24] !== seq0) ok = 1'b1;
                else begin
                    polls = polls + 1;
                    #2000;
                end
            end
        end
    endtask

    // Wait until every conversion the model serves carries `v`, then
    // read it back through the bus. Waiting two conversion boundaries
    // guarantees the sample in flight when `v` was applied has been
    // fully retired, so the comparison is against `v` directly rather
    // than against a live model output that could race the read.
    task lc_expect_sample(input [23:0] v);
        reg ok;
        reg [7:0] seq0;
        begin
            sample_value = v;
            conv_mark    = hx_conv_count;
            wait (hx_conv_count >= conv_mark + 2);
            bus_read(REG_LC_VALUE, rd);
            seq0 = rd[31:24];
            lc_wait_new_sample(seq0, ok);
            `ASSERT_EQ(ok, 1'b1, "%0d", "sequence counter never advanced")
            `ASSERT_EQ(rd[23:0], v, "%06h", "sample readback")
        end
    endtask

    // Run one conversion in `mode` and check the resulting pulse count,
    // channel and gain. The pulse count IS the channel/gain command on
    // this part, so these three are really one assertion in three parts.
    task lc_check_mode(input [1:0] mode, input [4:0] exp_pulses,
                       input exp_chan_b, input [7:0] exp_gain);
        reg ok;
        begin
            lc_ctrl_write(1'b1, 1'b0, LC_RATE_320HZ, mode, LC_PD_ADC, 1'b0);
            // Two conversions: the first train still carries the
            // previous mode's pulse count, since the count selects the
            // configuration for the *next* conversion.
            conv_mark = hx_conv_count;
            wait (hx_conv_count >= conv_mark + 2);
            bus_read(REG_LC_VALUE, rd);
            lc_wait_ready(ok);
            `ASSERT_EQ(ok, 1'b1, "%0d", "ready never asserted for mode")
            lc_wait_train_done;
            `ASSERT_EQ(hx_last_pulse_count, exp_pulses, "%0d", "PD_SCK pulse count")
            `ASSERT_EQ(hx_chan_b, exp_chan_b, "%0d", "selected channel")
            `ASSERT_EQ(hx_gain, exp_gain, "%0d", "selected gain")
        end
    endtask

    // ------------------------------------------------------------------
    // Test sequence
    // ------------------------------------------------------------------
    initial begin
        $dumpfile("loadcell_tb.vcd");
        $dumpvars(0, loadcell_tb);

        sample_value = 24'h123456;
        bus_init;
        sys_reset;

        // --------------------------------------------------------------
        $display("--- reset state ---");
        bus_read(REG_LC_CTRL, rd);
        `ASSERT_EQ(rd, 32'h0, "%08h", "CTRL clears on reset")
        bus_read(REG_LC_STATUS, rd);
        `ASSERT_EQ(rd, 32'h0, "%08h", "STATUS clears on reset")
        bus_read(REG_LC_VALUE, rd);
        `ASSERT_EQ(rd, 32'h0, "%08h", "VALUE clears on reset")
        `ASSERT_EQ(lc_pd_sck, 1'b0, "%0d", "PD_SCK idles low (device awake)")

        bus_read(8'hFC, rd);
        `ASSERT_EQ(rd, 32'hDEADBEEF, "%08h", "unmapped address")

        // --------------------------------------------------------------
        $display("--- rate pins (Table 3) ---");
        lc_ctrl_write(1'b0, 1'b0, LC_RATE_10HZ, LC_MODE_A128, LC_PD_ADC, 1'b0);
        #200;
        `ASSERT_EQ({lc_s1, lc_s0}, LC_RATE_10HZ, "%02b", "S1/S0 for 10Hz")
        lc_ctrl_write(1'b0, 1'b0, LC_RATE_20HZ, LC_MODE_A128, LC_PD_ADC, 1'b0);
        #200;
        `ASSERT_EQ({lc_s1, lc_s0}, LC_RATE_20HZ, "%02b", "S1/S0 for 20Hz")
        lc_ctrl_write(1'b0, 1'b0, LC_RATE_80HZ, LC_MODE_A128, LC_PD_ADC, 1'b0);
        #200;
        `ASSERT_EQ({lc_s1, lc_s0}, LC_RATE_80HZ, "%02b", "S1/S0 for 80Hz")
        lc_ctrl_write(1'b0, 1'b0, LC_RATE_320HZ, LC_MODE_A128, LC_PD_ADC, 1'b0);
        #200;
        `ASSERT_EQ({lc_s1, lc_s0}, LC_RATE_320HZ, "%02b", "S1/S0 for 320Hz")

        // Disabled means idle: no clocking at all.
        pulse_mark = hx_last_pulse_count;
        #100_000;
        `ASSERT_EQ(hx_last_pulse_count, pulse_mark, "%0d",
                   "no PD_SCK activity while disabled")

        // --------------------------------------------------------------
        $display("--- continuous conversion, CH A gain 128 ---");
        lc_ctrl_write(1'b1, 1'b0, LC_RATE_320HZ, LC_MODE_A128, LC_PD_ADC, 1'b0);

        bus_read(REG_LC_STATUS, rd);
        `ASSERT_EQ(rd[LC_STATUS_ENABLED_BIT], 1'b1, "%0d", "STATUS.ENABLED")

        lc_wait_ready(ready_ok);
        `ASSERT_EQ(ready_ok, 1'b1, "%0d", "first sample never became ready")
        lc_wait_train_done;
        `ASSERT_EQ(hx_last_pulse_count, 5'd25, "%0d", "25 pulses for CH A gain 128")

        // The sequence counter must advance by exactly one per
        // conversion - a jump of more than one is how the host detects
        // that it missed samples, so the step size has to be exact.
        bus_read(REG_LC_VALUE, rd);
        seq_a = rd[31:24];
        lc_wait_new_sample(seq_a, ready_ok);
        `ASSERT_EQ(ready_ok, 1'b1, "%0d", "sequence counter never advanced")
        `ASSERT_EQ(rd[31:24] - seq_a, 8'd1, "%0d",
                   "sequence counter steps by one per conversion")

        // --------------------------------------------------------------
        $display("--- sample values ---");
        lc_expect_sample(24'h000000);
        lc_expect_sample(24'h7FFFFF);   // positive full scale
        lc_expect_sample(24'h800000);   // negative full scale
        lc_expect_sample(24'hFFFFFF);   // -1
        lc_expect_sample(24'hA5A55A);   // alternating pattern, catches
                                        // bit-order and off-by-one errors

        // --------------------------------------------------------------
        $display("--- channel / gain selection (Table 4) ---");
        lc_check_mode(LC_MODE_A128, 5'd25, 1'b0, 8'd128);
        lc_check_mode(LC_MODE_B64,  5'd26, 1'b1, 8'd64);
        lc_check_mode(LC_MODE_A64,  5'd27, 1'b0, 8'd64);
        lc_check_mode(LC_MODE_B8,   5'd28, 1'b1, 8'd8);

        // --------------------------------------------------------------
        $display("--- power-down: ADC only (25-28 pulses) ---");
        // Currently in B8 / 28 pulses.
        lc_ctrl_write(1'b1, 1'b1, LC_RATE_320HZ, LC_MODE_B8, LC_PD_ADC, 1'b0);
        wait (hx_powered_down === 1'b1);
        `ASSERT_EQ(hx_last_pulse_count, 5'd28, "%0d",
                   "shallow power-down keeps the channel/gain pulse count")
        `ASSERT_EQ(hx_pd_level, LC_PD_ADC, "%0d", "power-down depth")
        `ASSERT_EQ(lc_pd_sck, 1'b1, "%0d", "PD_SCK held high while asleep")

        lc_wait_pd(ready_ok);
        `ASSERT_EQ(ready_ok, 1'b1, "%0d", "STATUS.PD while asleep")

        $display("--- wake ---");
        lc_ctrl_write(1'b1, 1'b0, LC_RATE_320HZ, LC_MODE_A128, LC_PD_ADC, 1'b0);
        wait (hx_powered_down === 1'b0);
        `ASSERT_EQ(lc_pd_sck, 1'b0, "%0d", "PD_SCK released on wake")
        // The datasheet promises the chip resumes with the setup it had
        // before sleeping, so it should still be on CH B gain 8 here.
        `ASSERT_EQ(hx_chan_b, 1'b1, "%0d", "channel preserved across power-down")
        `ASSERT_EQ(hx_gain, 8'd8, "%0d", "gain preserved across power-down")

        lc_wait_ready(ready_ok);
        `ASSERT_EQ(ready_ok, 1'b1, "%0d", "conversions did not resume after wake")

        // --------------------------------------------------------------
        $display("--- power-down: ADC + regulator (29 pulses) ---");
        lc_ctrl_write(1'b1, 1'b1, LC_RATE_320HZ, LC_MODE_A128, LC_PD_ADC_REG, 1'b0);
        wait (hx_powered_down === 1'b1);
        `ASSERT_EQ(hx_last_pulse_count, 5'd29, "%0d", "29 pulses")
        `ASSERT_EQ(hx_pd_level, LC_PD_ADC_REG, "%0d", "power-down depth")

        lc_ctrl_write(1'b1, 1'b0, LC_RATE_320HZ, LC_MODE_A128, LC_PD_ADC, 1'b0);
        wait (hx_powered_down === 1'b0);

        // --------------------------------------------------------------
        $display("--- power-down: everything (30 pulses) ---");
        lc_ctrl_write(1'b1, 1'b1, LC_RATE_320HZ, LC_MODE_A128, LC_PD_ALL, 1'b0);
        wait (hx_powered_down === 1'b1);
        `ASSERT_EQ(hx_last_pulse_count, 5'd30, "%0d", "30 pulses")
        `ASSERT_EQ(hx_pd_level, LC_PD_ALL, "%0d", "power-down depth")

        lc_ctrl_write(1'b0, 1'b0, LC_RATE_320HZ, LC_MODE_A128, LC_PD_ADC, 1'b0);
        wait (hx_powered_down === 1'b0);
        // Apply the next stimulus before the model finishes its first
        // post-wake conversion: with the DUT disabled nobody will clock
        // that result out, so whatever is latched here is exactly what
        // the single shot below will retrieve.
        sample_value = 24'h0F0F0F;
        #400_000;                               // > one conversion period

        // --------------------------------------------------------------
        $display("--- single shot ---");
        bus_read(REG_LC_VALUE, rd);             // note the sequence
        // ENABLE stays 0; SINGLE alone must produce exactly one train.
        lc_ctrl_write(1'b0, 1'b0, LC_RATE_320HZ, LC_MODE_A128, LC_PD_ADC, 1'b1);

        bus_read(REG_LC_CTRL, rd);
        `ASSERT_EQ(rd[LC_CTRL_SINGLE_BIT], 1'b0, "%0d", "SINGLE is self-clearing")

        lc_wait_ready(ready_ok);
        `ASSERT_EQ(ready_ok, 1'b1, "%0d", "single shot never completed")
        bus_read(REG_LC_VALUE, rd);
        `ASSERT_EQ(rd[23:0], 24'h0F0F0F, "%06h", "single shot value")

        // ...and then nothing further happens. READY stays set - it now
        // means "a sample landed under the current configuration" and is
        // only cleared by a CTRL write - so the thing that must not move
        // is the sequence counter.
        bus_read(REG_LC_VALUE, rd);
        seq_a = rd[31:24];
        #600_000;
        bus_read(REG_LC_VALUE, rd);
        `ASSERT_EQ(rd[31:24], seq_a, "%0d",
                   "single shot must not keep converting")
        bus_read(REG_LC_STATUS, rd);
        `ASSERT_EQ(rd[LC_STATUS_BUSY_BIT], 1'b0, "%0d",
                   "single shot must return to idle")

        // --------------------------------------------------------------
        $display("--- prefetch safety: reads must have no side effects ---");
        // The OctoSPI fills a 16-byte FIFO line aligned to 16 bytes, so
        // touching ANY register in this peripheral makes the controller
        // issue reads of 0x00/0x04/0x08/0x0C. Emulate that burst and
        // confirm nothing moves. This is the regression test for the
        // original design, where the 0x08 read in this burst silently
        // cleared READY - so merely polling STATUS destroyed the flag
        // being polled.
        //
        // The DUT is idle here (single shot finished, ENABLE clear), so
        // every register is genuinely static and any change is the
        // peripheral's fault rather than a new conversion landing.
        bus_read(REG_LC_STATUS, st_before);
        bus_read(REG_LC_VALUE,  val_before);
        `ASSERT_EQ(st_before[LC_STATUS_READY_BIT], 1'b1, "%0d",
                   "READY should be set before the prefetch burst")

        for (prefetch = 0; prefetch < 4; prefetch = prefetch + 1) begin
            bus_read(8'h00, rd);
            bus_read(8'h04, rd);
            bus_read(8'h08, rd);
            bus_read(8'h0c, rd);
        end

        bus_read(REG_LC_STATUS, rd);
        `ASSERT_EQ(rd, st_before, "%08h",
                   "STATUS unchanged by repeated whole-line reads")
        `ASSERT_EQ(rd[LC_STATUS_READY_BIT], 1'b1, "%0d",
                   "READY survives a prefetch that includes VALUE")
        bus_read(REG_LC_VALUE, rd);
        `ASSERT_EQ(rd, val_before, "%08h",
                   "VALUE and sequence unchanged by repeated reads")

        // READY is cleared by a WRITE, which a prefetching master cannot
        // issue by accident.
        lc_ctrl_write(1'b0, 1'b0, LC_RATE_320HZ, LC_MODE_A128, LC_PD_ADC, 1'b0);
        bus_read(REG_LC_STATUS, rd);
        `ASSERT_EQ(rd[LC_STATUS_READY_BIT], 1'b0, "%0d",
                   "READY cleared by a CTRL write")

        // --------------------------------------------------------------
        $display("--- data-ready watchdog ---");
        force hx_dout = 1'b1;                   // DOUT stuck high: no part
        lc_ctrl_write(1'b1, 1'b0, LC_RATE_320HZ, LC_MODE_A128, LC_PD_ADC, 1'b0);
        #((LC_TIMEOUT_US + 500) * 1000);
        bus_read(REG_LC_STATUS, rd);
        `ASSERT_EQ(rd[LC_STATUS_TIMEOUT_BIT], 1'b1, "%0d",
                   "TIMEOUT set when DOUT never falls")
        release hx_dout;

        // A control write acknowledges the timeout and rearms.
        lc_ctrl_write(1'b1, 1'b0, LC_RATE_320HZ, LC_MODE_A128, LC_PD_ADC, 1'b0);
        bus_read(REG_LC_STATUS, rd);
        `ASSERT_EQ(rd[LC_STATUS_TIMEOUT_BIT], 1'b0, "%0d",
                   "TIMEOUT cleared by a CTRL write")

        lc_expect_sample(24'h5A5A5A);

        // --------------------------------------------------------------
        $display("--- shutdown ---");
        lc_ctrl_write(1'b0, 1'b0, LC_RATE_10HZ, LC_MODE_A128, LC_PD_ADC, 1'b0);
        #100_000;
        `ASSERT_EQ(lc_pd_sck, 1'b0, "%0d", "PD_SCK low when idle and awake")

        // --------------------------------------------------------------
        // The model has been checking T1/T2/T3/T4 and the pulse-count
        // limit on every edge for the whole run.
        `ASSERT_EQ(hx_err_count, 32'd0, "%0d", "HX717 protocol violations")

        report();
        $display("loadcell_tb complete at %0t", $time);
        $finish;
    end

    // Global watchdog: a hung handshake should fail the run, not stall
    // the regression.
    initial begin
        #100_000_000;                // 100ms
        $display("\033[31mloadcell_tb: global timeout\033[0m");
        $fatal(1);
    end

endmodule
