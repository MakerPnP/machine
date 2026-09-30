`timescale 1ns/1ps

`include "src/test/assertions.svh"

// timer_pwm_tb.v
//
// Full 12-channel integration testbench for timer_pwm.v: the flexible
// many-to-one channel-to-timer mapping (PM1-4 -> TIM1-4 respectively,
// OT1-2 -> TIM1, OT3-4 -> TIM2, OT5-6 -> TIM3, OT7-8 -> TIM4), static
// output levels (enable/polarity/global-disable), and the TIM_SYNC-gated
// simultaneous start of independently-configured, independently-
// polarized timers. Register-level/default-value coverage and the
// per-timer counting algorithm itself are exercised by timer_tb.v.
module timer_pwm_tb;

    reg RESET;
    reg SYS_CLK = 0;
    `include "src/test/bus_io.svh"
    `include "src/main/io/timer_regs.svh"
    `include "src/main/io/timer_shared.svh"

    wire [3:0] pm_out;
    wire [7:0] ot_out;
    wire       ot_en;

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
    // timer_pwm.v's 12 PWM channels share one comparator, time-
    // multiplexed round-robin, so any one channel's pin can lag a config
    // change by up to 12 sys_clk cycles - wait this long (with margin)
    // after any write that could change a pin's expected level, before
    // checking it.
    localparam PWM_SETTLE_NS = 13 * NS_PER_SYS_CYCLE;
    always #10 SYS_CLK = ~SYS_CLK; // 20ns period -> 50 MHz

    // Channel 0-3 = PM1-4, channel 4-11 = OT1-8 - see timer_shared.svh.
    // Fixed for this whole file: PM1-4 -> TIM1-4 respectively, OT1-2 ->
    // TIM1, OT3-4 -> TIM2, OT5-6 -> TIM3, OT7-8 -> TIM4.
    integer channel_timer_tbl [0:11];

    // Returns the live pin level for channel `ch`, hiding the pm_out/
    // ot_out split so both tests can loop over all 12 channels uniformly.
    function automatic channel_pin;
        input integer ch;
        begin
            if (ch < 4) channel_pin = pm_out[ch];
            else        channel_pin = ot_out[ch - 4];
        end
    endfunction

    // Golden model, identical to timer_tb.v's (see that file for the
    // rationale) - kept local rather than shared, matching the existing
    // convention of per-testbench golden-model helpers (e.g.
    // steppers_motion_tb.v's period_at_step).
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

    function automatic level_for;
        input integer cnt;
        input integer cmp;
        input         polarity;
        begin
            level_for = (cnt < cmp) ? polarity : !polarity;
        end
    endfunction

    // Extends simulate_timer_cnt with TIM_SYNC's shared prescaler stage
    // ahead of the per-timer one - see timer_tb.v's identical function
    // for the full rationale.
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

    // See timer_tb.v's identical function for the full rationale: the 12
    // PWM channels share one comparator, time-multiplexed round-robin.
    // pwm_level[ch] updates a full two cycles after cmp_scan itself
    // reads ch (pwm_scan_addr_d lags cmp_scan by a cycle, and the update
    // gated by it lags pwm_scan_addr_d by another) - empirically
    // confirmed cycle-by-cycle against the RTL. Relative to a baseline
    // where cmp_scan reads scan_ref, the channel serviced by the update
    // landing K cycles later is (scan_ref + K - 2) mod 12, and that
    // update's comparator reads tim_cnt as of K-1 cycles (a blocking
    // read of a nonblocking-updated signal in the same edge). Valid
    // whenever n >= 12.
    function automatic integer last_scan_update_cycle;
        input integer channel;
        input integer scan_ref;
        input integer n;
        integer delta;
        integer k;
        begin
            delta = (channel - scan_ref + 2 + 12) % 12;
            if (delta == 0) delta = 12;
            k = delta + 12 * ((n - delta) / 12);
            last_scan_update_cycle = k - 1;
        end
    endfunction

    // Configures PWM channel `ch`'s CTRL + CMP registers in one call.
    task automatic config_channel;
        input integer ch;
        input         enable;
        input         polarity;
        input integer timer_src;
        input integer cmp;
        begin
            bus_write(pwm_ctrl_reg(ch[3:0]), pwm_ctrl_word(enable, polarity, timer_src[1:0]));
            bus_write(pwm_cmp_reg(ch[3:0]), cmp[7:0]);
        end
    endtask

    // Configures timer `idx`: RESET+prescaler first (parking presc_cnt
    // and CNT at a known point), then the initial CNT, then ARR, then
    // ENABLE - without touching CNT/presc_cnt again. Same sequence
    // timer_tb.v's FUNCTIONAL_TIMING_TEST uses by hand.
    task automatic config_timer;
        input integer idx;
        input integer prescaler;
        input integer arr;
        input integer cnt_init;
        input         enable;
        begin
            bus_write(tim_ctrl_reg(idx[1:0]), tim_ctrl_word(1'b0, 1'b1, prescaler[7:0]));
            bus_write(tim_cnt_reg(idx[1:0]), cnt_init[7:0]);
            bus_write(tim_arr_reg(idx[1:0]), arr[7:0]);
            bus_write(tim_ctrl_reg(idx[1:0]), tim_ctrl_word(enable, 1'b0, prescaler[7:0]));
        end
    endtask

    integer ch, tsrc;
    reg [7:0] test_index = 0;

    initial begin
        $dumpfile("timer_pwm_tb.vcd");
        $dumpvars(0, timer_pwm_tb);

        sys_reset();
        bus_init();

        channel_timer_tbl[0] = 0; channel_timer_tbl[1] = 1; channel_timer_tbl[2] = 2; channel_timer_tbl[3] = 3;
        channel_timer_tbl[4] = 0; channel_timer_tbl[5] = 0; channel_timer_tbl[6] = 1; channel_timer_tbl[7] = 1;
        channel_timer_tbl[8] = 2; channel_timer_tbl[9] = 2; channel_timer_tbl[10] = 3; channel_timer_tbl[11] = 3;

        // ============================================================
        $display("TEST %0d: Static output level per PWM channel (enable/polarity/global disable)", test_index);
        test_index = test_index + 1;
        // ============================================================
        begin : STATIC_OUTPUT_LEVEL_TEST
            // A1: reset defaults - every pin LOW.
            for (ch = 0; ch < 12; ch = ch + 1) begin
                `ASSERT_EQ(channel_pin(ch), 1'b0, "%0d", $sformatf("Channel %0d should be LOW at reset", ch + 1));
            end

            // A2: global output enable on, every channel still
            // individually disabled - still all LOW.
            bus_write(REG_PWM_CTRL, 32'h1);
            `ASSERT_EQ(ot_en, 1'b0, "%0d", "OT_EN should be active (low) once PWM_CTRL.OUTPUT_ENABLE=1");
            for (ch = 0; ch < 12; ch = ch + 1) begin
                `ASSERT_EQ(channel_pin(ch), 1'b0, "%0d",
                           $sformatf("Channel %0d should stay LOW while its own ENABLE=0", ch + 1));
            end

            // A3: one channel at a time, alternating polarity - every
            // OTHER channel must stay LOW while only the one under test
            // reflects its configured polarity (every timer is stopped
            // at CNT=0, CMP=50>0, so CNT<CMP always here). PWM_SETTLE_NS
            // gives the shared, round-robin comparator (see timer_pwm.v)
            // time to have revisited every channel at least once since
            // the last config write, before checking any pin.
            for (ch = 0; ch < 12; ch = ch + 1) begin
                config_channel(ch, 1'b1, ch[0], channel_timer_tbl[ch], 8'd50);
                #PWM_SETTLE_NS;

                `ASSERT_EQ(channel_pin(ch), ch[0], "%0d",
                           $sformatf("Channel %0d static level mismatch for polarity=%0d", ch + 1, ch[0]));

                config_channel(ch, 1'b0, ch[0], channel_timer_tbl[ch], 8'd50);
                #PWM_SETTLE_NS;
                `ASSERT_EQ(channel_pin(ch), 1'b0, "%0d",
                           $sformatf("Channel %0d should return LOW once disabled", ch + 1));
            end

            // A4: every channel enabled together, and the global disable
            // still overrides all 12 at once.
            for (ch = 0; ch < 12; ch = ch + 1)
                config_channel(ch, 1'b1, ch[0], channel_timer_tbl[ch], 8'd50);
            #PWM_SETTLE_NS;

            for (ch = 0; ch < 12; ch = ch + 1) begin
                `ASSERT_EQ(channel_pin(ch), ch[0], "%0d",
                           $sformatf("Channel %0d level mismatch with every channel enabled together", ch + 1));
            end

            bus_write(REG_PWM_CTRL, 32'h0);
            #PWM_SETTLE_NS; // let every channel's shared-comparator slot re-evaluate with OUTPUT_ENABLE now 0
            `ASSERT_EQ(ot_en, 1'b1, "%0d", "OT_EN should isolate OT1-8 again once PWM_CTRL.OUTPUT_ENABLE=0");
            for (ch = 0; ch < 12; ch = ch + 1) begin
                `ASSERT_EQ(channel_pin(ch), 1'b0, "%0d",
                           $sformatf("Channel %0d should be forced LOW by the global output disable", ch + 1));
            end

            bus_write(REG_PWM_CTRL, 32'h1); // leave enabled for the next test
        end

        // ============================================================
        $display("TEST %0d: TIM_SYNC-gated simultaneous start, per-timer polarity, dynamic levels", test_index);
        test_index = test_index + 1;
        // ============================================================
        begin : SYNCED_START_TEST
            integer prescaler_tbl     [0:3];
            integer arr_tbl           [0:3];
            integer polarity_of_timer [0:3]; // TIM1/TIM3 normal, TIM2/TIM4 inverted
            integer cnt_ref           [0:3];
            integer presc_ref         [0:3];
            integer cmp_scan_ref;
            integer expected_cnt;
            integer update_cycle;
            reg     expected_lvl;
            time    t_ref;

            bus_write(REG_TIM_SYNC, 32'h0); // make sure sync starts disabled

            prescaler_tbl[0] = 0; prescaler_tbl[1] = 1; prescaler_tbl[2] = 2; prescaler_tbl[3] = 3;
            arr_tbl[0] = 10; arr_tbl[1] = 10; arr_tbl[2] = 10; arr_tbl[3] = 10;
            polarity_of_timer[0] = PWM_POLARITY_NORMAL;   // TIM1
            polarity_of_timer[1] = PWM_POLARITY_INVERTED; // TIM2
            polarity_of_timer[2] = PWM_POLARITY_NORMAL;   // TIM3
            polarity_of_timer[3] = PWM_POLARITY_INVERTED; // TIM4

            // Every channel's own polarity follows its mapped timer's;
            // CMP=5 (mid-point of ARR=10) for all 12.
            for (ch = 0; ch < 12; ch = ch + 1) begin
                config_channel(ch, 1'b1, polarity_of_timer[channel_timer_tbl[ch]], channel_timer_tbl[ch], 8'd5);
            end

            // All 4 timers configured differently (distinct prescaler,
            // common ARR, CNT=0) and individually enabled, but TIM_SYNC
            // is still 0 - every one of them must stay frozen.
            for (tsrc = 0; tsrc < 4; tsrc = tsrc + 1) begin
                config_timer(tsrc, prescaler_tbl[tsrc], arr_tbl[tsrc], 0, 1'b1);
            end

            // ---- verify initial output levels (TIM_SYNC still 0) -----
            // CNT=0 < CMP=5 on every timer, so every channel sits at its
            // own polarity's "before compare" level: TIM1/TIM3-mapped
            // channels LOW, TIM2/TIM4-mapped channels HIGH. Settle first
            // so every channel's shared-comparator slot has re-evaluated
            // with its final configuration.
            #PWM_SETTLE_NS;
            `ASSERT_EQ(pm_out, 4'b1010, "0b%04b", "Initial PM levels mismatch before TIM_SYNC");
            `ASSERT_EQ(ot_out, 8'b11001100, "0b%08b", "Initial OT levels mismatch before TIM_SYNC");

            // ---- enable global sync, advance time, check dynamic levels ----
            bus_write(REG_TIM_SYNC, 32'h1);

            // Settle past the bus handshake's own latency, then snapshot
            // the actual internal state as this test's baseline (see
            // timer_tb.v's FUNCTIONAL_TIMING_TEST for why).
            #(5 * NS_PER_SYS_CYCLE + 1);
            t_ref = $time;
            for (tsrc = 0; tsrc < 4; tsrc = tsrc + 1) begin
                cnt_ref[tsrc]   = dut.tim_cnt[tsrc];
                presc_ref[tsrc] = dut.tim_presc_cnt[tsrc];
            end
            cmp_scan_ref = dut.cmp_scan;

            // >=12 cycles, so every channel's shared-comparator slot has
            // come up at least once since the baseline.
            #(15 * NS_PER_SYS_CYCLE);
            for (tsrc = 0; tsrc < 4; tsrc = tsrc + 1) begin
                expected_cnt = simulate_timer_cnt(prescaler_tbl[tsrc], arr_tbl[tsrc], cnt_ref[tsrc], presc_ref[tsrc], 15);
                `ASSERT_EQ(dut.tim_cnt[tsrc], expected_cnt[7:0], "%0d",
                           $sformatf("TIM%0d CNT mismatch after advancing time", tsrc + 1));
            end

            for (ch = 0; ch < 12; ch = ch + 1) begin
                tsrc = channel_timer_tbl[ch];
                // pwm_level only updates once every 12 cycles (shared,
                // time-multiplexed comparator) - reflects tim_cnt as of
                // the most recent cycle the scanner served this channel.
                update_cycle = last_scan_update_cycle(ch, cmp_scan_ref, 15);
                expected_cnt = simulate_timer_cnt(prescaler_tbl[tsrc], arr_tbl[tsrc], cnt_ref[tsrc], presc_ref[tsrc], update_cycle);
                expected_lvl = level_for(expected_cnt, 5, polarity_of_timer[tsrc]);
                `ASSERT_EQ(channel_pin(ch), expected_lvl, "%0d",
                           $sformatf("Channel %0d level mismatch after advancing time", ch + 1));
            end

            bus_write(REG_TIM_SYNC, 32'h0);
            bus_write(REG_PWM_CTRL, 32'h0);
            for (tsrc = 0; tsrc < 4; tsrc = tsrc + 1) bus_write(tim_ctrl_reg(tsrc[1:0]), 32'h0);
            for (ch = 0; ch < 12; ch = ch + 1) bus_write(pwm_ctrl_reg(ch[3:0]), 32'h0);
        end

        // ============================================================
        $display("TEST %0d: TIM_SYNC shared prescaler composes across all 4 timers/12 channels", test_index);
        test_index = test_index + 1;
        // ============================================================
        begin : GLOBAL_PRESCALER_TEST
            integer prescaler_tbl     [0:3];
            integer arr_tbl           [0:3];
            integer polarity_of_timer [0:3]; // TIM1/TIM3 normal, TIM2/TIM4 inverted
            integer cnt_ref           [0:3];
            integer presc_ref         [0:3];
            integer cmp_scan_ref;
            integer global_presc_ref;
            integer global_prescaler;
            integer expected_cnt;
            integer update_cycle;
            reg     expected_lvl;
            time    t_ref;

            bus_write(REG_TIM_SYNC, 32'h0); // make sure sync starts disabled

            global_prescaler = 3; // shared divide-by-4 ahead of every timer

            prescaler_tbl[0] = 0; prescaler_tbl[1] = 1; prescaler_tbl[2] = 2; prescaler_tbl[3] = 3;
            arr_tbl[0] = 10; arr_tbl[1] = 10; arr_tbl[2] = 10; arr_tbl[3] = 10;
            polarity_of_timer[0] = PWM_POLARITY_NORMAL;   // TIM1
            polarity_of_timer[1] = PWM_POLARITY_INVERTED; // TIM2
            polarity_of_timer[2] = PWM_POLARITY_NORMAL;   // TIM3
            polarity_of_timer[3] = PWM_POLARITY_INVERTED; // TIM4

            for (ch = 0; ch < 12; ch = ch + 1) begin
                config_channel(ch, 1'b1, polarity_of_timer[channel_timer_tbl[ch]], channel_timer_tbl[ch], 8'd5);
            end

            for (tsrc = 0; tsrc < 4; tsrc = tsrc + 1) begin
                config_timer(tsrc, prescaler_tbl[tsrc], arr_tbl[tsrc], 0, 1'b1);
            end

            bus_write(REG_PWM_CTRL, 32'h1); // SYNCED_START_TEST's cleanup left this disabled

            #PWM_SETTLE_NS;
            `ASSERT_EQ(pm_out, 4'b1010, "0b%04b", "Initial PM levels mismatch before TIM_SYNC");
            `ASSERT_EQ(ot_out, 8'b11001100, "0b%08b", "Initial OT levels mismatch before TIM_SYNC");

            // Start every timer AND arm the shared prescaler in the same
            // write - both fields live in TIM_SYNC.
            bus_write(REG_TIM_SYNC, tim_sync_word(1'b1, global_prescaler[5:0]));

            #(5 * NS_PER_SYS_CYCLE + 1);
            t_ref = $time;
            for (tsrc = 0; tsrc < 4; tsrc = tsrc + 1) begin
                cnt_ref[tsrc]   = dut.tim_cnt[tsrc];
                presc_ref[tsrc] = dut.tim_presc_cnt[tsrc];
            end
            cmp_scan_ref     = dut.cmp_scan;
            global_presc_ref = dut.tim_sync_presc_cnt;

            // 100 cycles: with the shared divide-by-4 on top of each
            // timer's own (divide 1..4), the slowest timer here needs up
            // to 16 cycles/tick, so this comfortably covers several
            // ticks per timer and clears the >=12-cycle requirement for
            // the comparator scanner.
            #(100 * NS_PER_SYS_CYCLE);
            for (tsrc = 0; tsrc < 4; tsrc = tsrc + 1) begin
                expected_cnt = simulate_timer_cnt_global(global_prescaler, global_presc_ref,
                                                          prescaler_tbl[tsrc], arr_tbl[tsrc], cnt_ref[tsrc], presc_ref[tsrc], 100);
                `ASSERT_EQ(dut.tim_cnt[tsrc], expected_cnt[7:0], "%0d",
                           $sformatf("TIM%0d CNT mismatch after advancing time", tsrc + 1));
            end

            for (ch = 0; ch < 12; ch = ch + 1) begin
                tsrc = channel_timer_tbl[ch];
                update_cycle = last_scan_update_cycle(ch, cmp_scan_ref, 100);
                expected_cnt = simulate_timer_cnt_global(global_prescaler, global_presc_ref,
                                                          prescaler_tbl[tsrc], arr_tbl[tsrc], cnt_ref[tsrc], presc_ref[tsrc], update_cycle);
                expected_lvl = level_for(expected_cnt, 5, polarity_of_timer[tsrc]);
                `ASSERT_EQ(channel_pin(ch), expected_lvl, "%0d",
                           $sformatf("Channel %0d level mismatch after advancing time", ch + 1));
            end

            bus_write(REG_TIM_SYNC, 32'h0);
            bus_write(REG_PWM_CTRL, 32'h0);
            for (tsrc = 0; tsrc < 4; tsrc = tsrc + 1) bus_write(tim_ctrl_reg(tsrc[1:0]), 32'h0);
            for (ch = 0; ch < 12; ch = ch + 1) bus_write(pwm_ctrl_reg(ch[3:0]), 32'h0);
        end

        report();
        $finish;
    end

endmodule
