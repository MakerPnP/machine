`timescale 1ns/1ps

`include "src/test/assertions.svh"

// timer_tb.v
//
// Register-level and timer-counting testbench for timer_pwm.v. Exercises
// every register's default value and read/write behaviour, the
// self-clearing TIMx_CTRL.RESET strobe, and the free-running counter +
// per-channel compare logic (using PWM_CTRL1-4 mapped 1:1 to TIM1-4 as
// the observable "compare output" for each timer). Full 12-channel/
// flexible-mapping/polarity coverage lives in timer_pwm_tb.v.
module timer_tb;

    reg RESET;
    reg SYS_CLK = 0;
    `include "src/test/bus_io.svh"
    `include "src/main/io/timer_regs.svh"
    `include "src/main/io/timer_shared.svh"

    wire [3:0] pm_out;
    wire [7:0] ot_out;
    wire       ot_en;

    reg [31:0] result;

    timer_pwm dut (
        .reset(RESET),
        .sys_clk(SYS_CLK),

        .bus_stb(stb),
        .bus_we(we),
        .bus_addr(addr),
        .bus_din(din),
        .bus_dout(dout),
        .bus_ack(ack),

        .pm_out(pm_out),
        .ot_out(ot_out),
        .ot_en(ot_en)
    );

    localparam NS_PER_SYS_CYCLE = 20;
    always #10 SYS_CLK = ~SYS_CLK; // 20ns period -> 50 MHz

    // ------------------------------------------------------------------
    // Golden model of timer_pwm.v's per-cycle counting algorithm: given a
    // (prescaler, arr) config and a live (cnt, presc_cnt) starting point,
    // returns the CNT value exactly `cycles` sys_clk edges later. Mirrors
    // the RTL's tick/reload/wrap logic exactly, the same role
    // steppers_motion_tb.v's period_at_step plays for steppers.v.
    // ------------------------------------------------------------------
    function automatic integer simulate_timer_cnt;
        input integer prescaler;
        input integer arr;
        input integer start_cnt;
        input integer start_presc_cnt;
        input integer cycles;
        integer k, cnt, presc;
        begin
            cnt   = start_cnt;
            presc = start_presc_cnt;
            for (k = 0; k < cycles; k = k + 1) begin
                if (presc == 0) begin
                    presc = prescaler;
                    if (cnt == arr) cnt = 0;
                    else cnt = cnt + 1;
                end else begin
                    presc = presc - 1;
                end
            end
            simulate_timer_cnt = cnt;
        end
    endfunction

    // Extends simulate_timer_cnt with TIM_SYNC's shared prescaler stage
    // ahead of the per-timer one: the per-timer prescale/tick logic only
    // advances on a sys_clk cycle the shared stage "ticks" on (mirrors
    // timer_pwm.v's tim_sync_tick gating exactly).
    function automatic integer simulate_timer_cnt_global;
        input integer global_prescaler;
        input integer global_presc_start;
        input integer prescaler;
        input integer arr;
        input integer start_cnt;
        input integer start_presc_cnt;
        input integer cycles;
        integer k, cnt, presc, gpresc;
        begin
            cnt    = start_cnt;
            presc  = start_presc_cnt;
            gpresc = global_presc_start;
            for (k = 0; k < cycles; k = k + 1) begin
                if (gpresc == 0) begin
                    gpresc = global_prescaler;
                    if (presc == 0) begin
                        presc = prescaler;
                        if (cnt == arr) cnt = 0;
                        else cnt = cnt + 1;
                    end else begin
                        presc = presc - 1;
                    end
                end else begin
                    gpresc = gpresc - 1;
                end
            end
            simulate_timer_cnt_global = cnt;
        end
    endfunction

    // Mirrors timer_pwm.v's per-channel comparator: pin = polarity while
    // CNT < CMP, else the opposite level.
    function automatic level_for;
        input integer cnt;
        input integer cmp;
        input         polarity;
        begin
            level_for = (cnt < cmp) ? polarity : !polarity;
        end
    endfunction

    // timer_pwm.v's 12 PWM channels share one comparator, time-
    // multiplexed round-robin (cmp_scan, free-running every cycle,
    // wrapping 0..11) rather than each channel recomputing every cycle -
    // see timer_pwm.v's module header. Both pwm_scan_addr_d (<= cmp_scan)
    // and the pwm_level update gated by it (an `if (pwm_scan_valid_d)
    // pwm_level[pwm_scan_addr_d] <= ...` reading pwm_scan_addr_d's value
    // from BEFORE that same edge) are one cycle behind the signal that
    // feeds them, so pwm_level[ch] actually updates a full two cycles
    // after cmp_scan itself reads ch - empirically confirmed cycle-by-
    // cycle against the RTL (see the git history of this comment for
    // the trace): relative to a baseline where cmp_scan reads scan_ref,
    // the channel serviced by the update landing K cycles later
    // (K=1,2,3,...) is (scan_ref + K - 2) mod 12. That same update's
    // comparator logic reads tim_cnt combinationally (a blocking read
    // of a signal the counting logic elsewhere updates via nonblocking
    // assignment in the very same edge), so it sees tim_cnt as of K-1
    // cycles, not K. Given how many cycles have elapsed since the
    // baseline (n), this returns the `cycles` argument simulate_timer_
    // cnt needs to reproduce exactly what that comparator saw at its
    // most recent update to channel ch - valid whenever n >= 12 (one
    // full scan), which every caller here guarantees.
    function automatic integer last_scan_update_cycle;
        input integer channel;
        input integer scan_ref;
        input integer n;
        integer delta;
        integer k;
        begin
            delta = (channel - scan_ref + 2 + 12) % 12; // smallest K>=0 serviced == channel
            if (delta == 0) delta = 12;                 // need K>=1 (K-1 must be a capturable cycle)
            k = delta + 12 * ((n - delta) / 12);         // largest such K <= n
            last_scan_update_cycle = k - 1;
        end
    endfunction

    integer t, c;
    reg [7:0] test_index = 0;

    initial begin
        $dumpfile("timer_tb.vcd");
        $dumpvars(0, timer_tb);

        sys_reset();
        bus_init();

        // ============================================================
        $display("TEST %0d: Register default values", test_index);
        test_index = test_index + 1;
        // ============================================================
        begin : DEFAULTS_TEST
            bus_read(REG_PWM_CTRL, result);
            `ASSERT_EQ(result, 32'h0, "0x%08h", "REG_PWM_CTRL default mismatch");

            bus_read(REG_TIM_SYNC, result);
            `ASSERT_EQ(result, 32'h0, "0x%08h", "REG_TIM_SYNC default mismatch");

            for (t = 0; t < 4; t = t + 1) begin
                bus_read(tim_ctrl_reg(t[1:0]), result);
                `ASSERT_EQ(result, 32'h0, "0x%08h", $sformatf("TIM%0d_CTRL default mismatch", t + 1));
                bus_read(tim_arr_reg(t[1:0]), result);
                `ASSERT_EQ(result, 32'h0, "0x%08h", $sformatf("TIM%0d_ARR default mismatch", t + 1));
                bus_read(tim_cnt_reg(t[1:0]), result);
                `ASSERT_EQ(result, 32'h0, "0x%08h", $sformatf("TIM%0d_CNT default mismatch", t + 1));
            end

            for (c = 0; c < 12; c = c + 1) begin
                bus_read(pwm_ctrl_reg(c[3:0]), result);
                `ASSERT_EQ(result, 32'h0, "0x%08h", $sformatf("PWM_CTRL%0d default mismatch", c + 1));
                bus_read(pwm_cmp_reg(c[3:0]), result);
                `ASSERT_EQ(result, 32'h0, "0x%08h", $sformatf("PWM_CMP%0d default mismatch", c + 1));
            end

            `ASSERT_EQ(pm_out, 4'b0000, "0b%04b", "PM outputs should be LOW at reset");
            `ASSERT_EQ(ot_out, 8'b0, "0b%08b", "OT outputs should be LOW at reset");
            `ASSERT_EQ(ot_en, 1'b1, "%0d", "OT_EN should be inactive (high, isolated) at reset");
        end

        // ============================================================
        $display("TEST %0d: Write and read-back every register", test_index);
        test_index = test_index + 1;
        // ============================================================
        begin : READBACK_TEST
            reg [31:0] w;

            bus_write(REG_PWM_CTRL, 32'h1);
            bus_read(REG_PWM_CTRL, result);
            `ASSERT_EQ(result, 32'h1, "0x%08h", "REG_PWM_CTRL readback mismatch");
            bus_write(REG_PWM_CTRL, 32'h0);

            bus_write(REG_TIM_SYNC, tim_sync_word(1'b1, 6'd0));
            bus_read(REG_TIM_SYNC, result);
            `ASSERT_EQ(result, tim_sync_word(1'b1, 6'd0), "0x%08h", "REG_TIM_SYNC readback mismatch");

            // Exercise the shared prescaler field specifically.
            bus_write(REG_TIM_SYNC, tim_sync_word(1'b0, 6'd37));
            bus_read(REG_TIM_SYNC, result);
            `ASSERT_EQ(result, tim_sync_word(1'b0, 6'd37), "0x%08h",
                       "REG_TIM_SYNC.PRESCALER readback mismatch");

            bus_write(REG_TIM_SYNC, 32'h0);

            for (t = 0; t < 4; t = t + 1) begin
                // CTRL: enable=1, reset=0, a distinct prescaler per timer.
                w = tim_ctrl_word(1'b1, 1'b0, (8'h10 + t));
                bus_write(tim_ctrl_reg(t[1:0]), w);
                bus_read(tim_ctrl_reg(t[1:0]), result);
                `ASSERT_EQ(result, w, "0x%08h", $sformatf("TIM%0d_CTRL readback mismatch", t + 1));

                bus_write(tim_arr_reg(t[1:0]), (8'h40 + t));
                bus_read(tim_arr_reg(t[1:0]), result);
                `ASSERT_EQ(result, (32'h40 + t), "0x%08h", $sformatf("TIM%0d_ARR readback mismatch", t + 1));

                bus_write(tim_cnt_reg(t[1:0]), (8'h05 + t));
                bus_read(tim_cnt_reg(t[1:0]), result);
                `ASSERT_EQ(result, (32'h05 + t), "0x%08h", $sformatf("TIM%0d_CNT readback mismatch", t + 1));

                // Disable again so nothing free-runs during later tests.
                bus_write(tim_ctrl_reg(t[1:0]), 32'h0);
            end

            for (c = 0; c < 12; c = c + 1) begin
                w = pwm_ctrl_word(1'b1, c[0], c[1:0]);
                bus_write(pwm_ctrl_reg(c[3:0]), w);
                bus_read(pwm_ctrl_reg(c[3:0]), result);
                `ASSERT_EQ(result, w, "0x%08h", $sformatf("PWM_CTRL%0d readback mismatch", c + 1));

                bus_write(pwm_cmp_reg(c[3:0]), (8'h20 + c));
                bus_read(pwm_cmp_reg(c[3:0]), result);
                `ASSERT_EQ(result, (32'h20 + c), "0x%08h", $sformatf("PWM_CMP%0d readback mismatch", c + 1));

                bus_write(pwm_ctrl_reg(c[3:0]), 32'h0);
            end
        end

        // ============================================================
        $display("TEST %0d: TIMx_CTRL.RESET self-clears and resets CNT", test_index);
        test_index = test_index + 1;
        // ============================================================
        begin : RESET_BIT_TEST
            reg [31:0] ctrl_read;

            for (t = 0; t < 4; t = t + 1) begin
                // Give it a nonzero starting count while stopped.
                bus_write(tim_cnt_reg(t[1:0]), 8'hAA);
                bus_read(tim_cnt_reg(t[1:0]), result);
                `ASSERT_EQ(result, 32'h000000AA, "0x%08h", $sformatf("TIM%0d_CNT pre-reset value mismatch", t + 1));

                // Write CTRL with RESET=1 (bit 1) alongside ENABLE=0.
                bus_write(tim_ctrl_reg(t[1:0]), (32'h1 << TIM_CTRL_RESET_BIT));
                bus_read(tim_ctrl_reg(t[1:0]), ctrl_read);
                `ASSERT_EQ(ctrl_read[TIM_CTRL_RESET_BIT], 1'b0, "%0d",
                           $sformatf("TIM%0d_CTRL.RESET should self-clear", t + 1));

                bus_read(tim_cnt_reg(t[1:0]), result);
                `ASSERT_EQ(result, 32'h0, "0x%08h", $sformatf("TIM%0d_CNT should be 0 after RESET", t + 1));
            end
        end

        // ============================================================
        $display("TEST %0d: Independent prescalers/reload/initial counts, TIM_SYNC-gated start, per-timer compare output", test_index);
        test_index = test_index + 1;
        // ============================================================
        begin : FUNCTIONAL_TIMING_TEST
            integer prescaler_tbl [0:3];
            integer arr_tbl       [0:3];
            integer cnt_init_tbl  [0:3];
            integer cmp_tbl       [0:3];
            integer cnt_ref       [0:3];
            integer presc_ref     [0:3];
            integer cmp_scan_ref;
            integer expected_cnt;
            integer update_cycle;
            reg     expected_lvl;
            time    t_ref;

            sys_reset();

            prescaler_tbl[0] = 0; prescaler_tbl[1] = 1; prescaler_tbl[2] = 2; prescaler_tbl[3] = 3;
            arr_tbl[0]       = 4; arr_tbl[1]       = 5; arr_tbl[2]       = 6; arr_tbl[3]       = 7;
            cnt_init_tbl[0]  = 0; cnt_init_tbl[1]  = 1; cnt_init_tbl[2]  = 2; cnt_init_tbl[3]  = 3;
            cmp_tbl[0]       = 2; cmp_tbl[1]       = 3; cmp_tbl[2]       = 4; cmp_tbl[3]       = 5;

            // Configure each timer: RESET+prescaler first (parks presc_cnt
            // and CNT at a known point), then the initial CNT, then ARR,
            // then ENABLE (without touching CNT/presc_cnt again) - exactly
            // the "different prescaler, different reload, different
            // initial counter, then enable" sequence.
            for (t = 0; t < 4; t = t + 1) begin
                bus_write(tim_ctrl_reg(t[1:0]), tim_ctrl_word(1'b0, 1'b1, prescaler_tbl[t][7:0]));
                bus_write(tim_cnt_reg(t[1:0]), cnt_init_tbl[t][7:0]);
                bus_write(tim_arr_reg(t[1:0]), arr_tbl[t][7:0]);
                bus_write(tim_ctrl_reg(t[1:0]), tim_ctrl_word(1'b1, 1'b0, prescaler_tbl[t][7:0]));

                // PWM channel t (PWM_CTRL(t+1) / PM(t+1)) mapped 1:1 to
                // timer t, normal polarity - its output IS this timer's
                // observable compare signal.
                bus_write(pwm_ctrl_reg(t[3:0]), pwm_ctrl_word(1'b1, PWM_POLARITY_NORMAL, t[1:0]));
                bus_write(pwm_cmp_reg(t[3:0]), cmp_tbl[t][7:0]);
            end

            // Global output enable, still not sync'd - every timer must
            // be frozen at its configured initial value.
            bus_write(REG_PWM_CTRL, 32'h1);

            for (t = 0; t < 4; t = t + 1) begin
                `ASSERT_EQ(dut.tim_cnt[t], cnt_init_tbl[t][7:0], "%0d",
                           $sformatf("TIM%0d frozen CNT mismatch before TIM_SYNC", t + 1));
            end
            `ASSERT_EQ(pm_out, 4'b0000, "0b%04b",
                       "PM outputs should all read their pre-compare (LOW, normal polarity) level before TIM_SYNC");

            // Start every timer together.
            bus_write(REG_TIM_SYNC, 32'h1);

            // Settle well past the bus handshake's own latency (an exact
            // multiple of the clock period, plus a 1ns guard against
            // sampling a hierarchical register reference on the very
            // edge it updates), then snapshot the ACTUAL internal state
            // as this test's baseline - the golden model below simulates
            // forward from whatever this live state turns out to be,
            // so no assumption about exact bus latency is needed.
            #(5 * NS_PER_SYS_CYCLE + 1);
            t_ref = $time;
            for (t = 0; t < 4; t = t + 1) begin
                cnt_ref[t]   = dut.tim_cnt[t];
                presc_ref[t] = dut.tim_presc_cnt[t];
            end
            cmp_scan_ref = dut.cmp_scan;

            // Checkpoint 1: 15 more sys_clk cycles (>=12, so every
            // channel's shared-comparator slot has come up at least once
            // since the baseline - see last_scan_update_cycle above).
            #(15 * NS_PER_SYS_CYCLE);
            for (t = 0; t < 4; t = t + 1) begin
                expected_cnt = simulate_timer_cnt(prescaler_tbl[t], arr_tbl[t], cnt_ref[t], presc_ref[t], 15);
                `ASSERT_EQ(dut.tim_cnt[t], expected_cnt[7:0], "%0d",
                           $sformatf("TIM%0d CNT mismatch at checkpoint 1", t + 1));

                // pwm_level only updates once every 12 cycles (the shared
                // comparator is time-multiplexed round-robin across all
                // 12 channels) - it reflects tim_cnt as of the most
                // recent cycle the scanner served this channel, not the
                // checkpoint's own cycle count.
                update_cycle = last_scan_update_cycle(t, cmp_scan_ref, 15);
                expected_cnt = simulate_timer_cnt(prescaler_tbl[t], arr_tbl[t], cnt_ref[t], presc_ref[t], update_cycle);
                expected_lvl = level_for(expected_cnt, cmp_tbl[t], PWM_POLARITY_NORMAL);
                `ASSERT_EQ(pm_out[t], expected_lvl, "%0d",
                           $sformatf("PM%0d compare output mismatch at checkpoint 1", t + 1));
            end

            // Checkpoint 2: 15 more cycles (30 total past the baseline) -
            // every timer here wraps through ARR (5 ticks) well before
            // this point, and every channel's shared-comparator slot has
            // come up at least twice more.
            #(15 * NS_PER_SYS_CYCLE);
            for (t = 0; t < 4; t = t + 1) begin
                expected_cnt = simulate_timer_cnt(prescaler_tbl[t], arr_tbl[t], cnt_ref[t], presc_ref[t], 30);
                `ASSERT_EQ(dut.tim_cnt[t], expected_cnt[7:0], "%0d",
                           $sformatf("TIM%0d CNT mismatch at checkpoint 2", t + 1));

                update_cycle = last_scan_update_cycle(t, cmp_scan_ref, 30);
                expected_cnt = simulate_timer_cnt(prescaler_tbl[t], arr_tbl[t], cnt_ref[t], presc_ref[t], update_cycle);
                expected_lvl = level_for(expected_cnt, cmp_tbl[t], PWM_POLARITY_NORMAL);
                `ASSERT_EQ(pm_out[t], expected_lvl, "%0d",
                           $sformatf("PM%0d compare output mismatch at checkpoint 2", t + 1));
            end

            // Restore a safe state for anything run after this test.
            bus_write(REG_TIM_SYNC, 32'h0);
            bus_write(REG_PWM_CTRL, 32'h0);
            for (t = 0; t < 4; t = t + 1) begin
                bus_write(tim_ctrl_reg(t[1:0]), 32'h0);
                bus_write(pwm_ctrl_reg(t[3:0]), 32'h0);
            end
        end

        // ============================================================
        $display("TEST %0d: TIM_SYNC shared prescaler composes with each timer's own prescaler", test_index);
        test_index = test_index + 1;
        // ============================================================
        begin : GLOBAL_PRESCALER_TEST
            integer prescaler_tbl [0:3];
            integer arr_tbl       [0:3];
            integer cnt_init_tbl  [0:3];
            integer cmp_tbl       [0:3];
            integer cnt_ref       [0:3];
            integer presc_ref     [0:3];
            integer cmp_scan_ref;
            integer global_presc_ref;
            integer global_prescaler;
            integer expected_cnt;
            integer update_cycle;
            reg     expected_lvl;
            time    t_ref;

            sys_reset();

            global_prescaler = 3; // shared divide-by-4 ahead of every timer

            prescaler_tbl[0] = 0; prescaler_tbl[1] = 1; prescaler_tbl[2] = 2; prescaler_tbl[3] = 3;
            arr_tbl[0]       = 4; arr_tbl[1]       = 5; arr_tbl[2]       = 6; arr_tbl[3]       = 7;
            cnt_init_tbl[0]  = 0; cnt_init_tbl[1]  = 1; cnt_init_tbl[2]  = 2; cnt_init_tbl[3]  = 3;
            cmp_tbl[0]       = 2; cmp_tbl[1]       = 3; cmp_tbl[2]       = 4; cmp_tbl[3]       = 5;

            // Same per-timer configuration sequence as FUNCTIONAL_TIMING_
            // TEST above - the shared prescaler is independent of it,
            // armed separately (with ENABLE) in the TIM_SYNC write below.
            for (t = 0; t < 4; t = t + 1) begin
                bus_write(tim_ctrl_reg(t[1:0]), tim_ctrl_word(1'b0, 1'b1, prescaler_tbl[t][7:0]));
                bus_write(tim_cnt_reg(t[1:0]), cnt_init_tbl[t][7:0]);
                bus_write(tim_arr_reg(t[1:0]), arr_tbl[t][7:0]);
                bus_write(tim_ctrl_reg(t[1:0]), tim_ctrl_word(1'b1, 1'b0, prescaler_tbl[t][7:0]));

                bus_write(pwm_ctrl_reg(t[3:0]), pwm_ctrl_word(1'b1, PWM_POLARITY_NORMAL, t[1:0]));
                bus_write(pwm_cmp_reg(t[3:0]), cmp_tbl[t][7:0]);
            end

            bus_write(REG_PWM_CTRL, 32'h1);

            for (t = 0; t < 4; t = t + 1) begin
                `ASSERT_EQ(dut.tim_cnt[t], cnt_init_tbl[t][7:0], "%0d",
                           $sformatf("TIM%0d frozen CNT mismatch before TIM_SYNC", t + 1));
            end
            `ASSERT_EQ(pm_out, 4'b0000, "0b%04b",
                       "PM outputs should all read their pre-compare (LOW, normal polarity) level before TIM_SYNC");

            // Start every timer AND arm the shared prescaler in the same
            // write - both fields live in TIM_SYNC.
            bus_write(REG_TIM_SYNC, tim_sync_word(1'b1, global_prescaler[5:0]));

            #(5 * NS_PER_SYS_CYCLE + 1);
            t_ref = $time;
            for (t = 0; t < 4; t = t + 1) begin
                cnt_ref[t]   = dut.tim_cnt[t];
                presc_ref[t] = dut.tim_presc_cnt[t];
            end
            cmp_scan_ref     = dut.cmp_scan;
            global_presc_ref = dut.tim_sync_presc_cnt;

            // Checkpoint 1: 100 sys_clk cycles - with the shared divide-
            // by-4 stage on top of each timer's own (divide 1..4), the
            // slowest timer here needs up to 4*4=16 cycles per tick, so
            // 100 cycles comfortably covers several ticks on every timer
            // (and clears the >=12-cycle requirement for the comparator
            // scanner - see last_scan_update_cycle above).
            #(100 * NS_PER_SYS_CYCLE);
            for (t = 0; t < 4; t = t + 1) begin
                expected_cnt = simulate_timer_cnt_global(global_prescaler, global_presc_ref,
                                                          prescaler_tbl[t], arr_tbl[t], cnt_ref[t], presc_ref[t], 100);
                `ASSERT_EQ(dut.tim_cnt[t], expected_cnt[7:0], "%0d",
                           $sformatf("TIM%0d CNT mismatch at checkpoint 1", t + 1));

                update_cycle = last_scan_update_cycle(t, cmp_scan_ref, 100);
                expected_cnt = simulate_timer_cnt_global(global_prescaler, global_presc_ref,
                                                          prescaler_tbl[t], arr_tbl[t], cnt_ref[t], presc_ref[t], update_cycle);
                expected_lvl = level_for(expected_cnt, cmp_tbl[t], PWM_POLARITY_NORMAL);
                `ASSERT_EQ(pm_out[t], expected_lvl, "%0d",
                           $sformatf("PM%0d compare output mismatch at checkpoint 1", t + 1));
            end

            // Checkpoint 2: 100 more cycles (200 total past the baseline).
            #(100 * NS_PER_SYS_CYCLE);
            for (t = 0; t < 4; t = t + 1) begin
                expected_cnt = simulate_timer_cnt_global(global_prescaler, global_presc_ref,
                                                          prescaler_tbl[t], arr_tbl[t], cnt_ref[t], presc_ref[t], 200);
                `ASSERT_EQ(dut.tim_cnt[t], expected_cnt[7:0], "%0d",
                           $sformatf("TIM%0d CNT mismatch at checkpoint 2", t + 1));

                update_cycle = last_scan_update_cycle(t, cmp_scan_ref, 200);
                expected_cnt = simulate_timer_cnt_global(global_prescaler, global_presc_ref,
                                                          prescaler_tbl[t], arr_tbl[t], cnt_ref[t], presc_ref[t], update_cycle);
                expected_lvl = level_for(expected_cnt, cmp_tbl[t], PWM_POLARITY_NORMAL);
                `ASSERT_EQ(pm_out[t], expected_lvl, "%0d",
                           $sformatf("PM%0d compare output mismatch at checkpoint 2", t + 1));
            end

            // Restore a safe state for anything run after this test.
            bus_write(REG_TIM_SYNC, 32'h0);
            bus_write(REG_PWM_CTRL, 32'h0);
            for (t = 0; t < 4; t = t + 1) begin
                bus_write(tim_ctrl_reg(t[1:0]), 32'h0);
                bus_write(pwm_ctrl_reg(t[3:0]), 32'h0);
            end
        end

        report();
        $finish;
    end

endmodule
