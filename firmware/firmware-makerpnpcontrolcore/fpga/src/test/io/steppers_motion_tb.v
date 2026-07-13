`timescale 1ns/1ps

`include "src/test/assertions.svh"

module steppers_motion_tb;

    reg RESET;
    reg TCXO = 0;
    `include "src/test/bus_io.svh"
    `include "src/main/io/steppers_regs.svh"
    `include "src/main/io/steppers_shared.svh"

    wire [7:0] step_pins;
    wire [7:0] dir_pins;
    wire [1:0] bank_enable_pins;
    reg global_motor_en;

    reg [31:0] result;

    reg stepper_clk;

    stepper_clk stepper_clk_inst (
        .sys_clk(TCXO),
        .reset(RESET),
        .stepper_clk(stepper_clk)
    );

    steppers dut (
        .sys_clk(TCXO),
        .reset(RESET),

        .stepper_clk(stepper_clk),

        .bus_stb(stb),
        .bus_we(we),
        .bus_addr(addr),
        .bus_din(din),
        .bus_dout(dout),
        .bus_ack(ack),

        .step_pins(step_pins),
        .dir_pins(dir_pins),
        .bank_enable_pins(bank_enable_pins),
        .global_motor_en(global_motor_en)
    );

    // ============================================================
    // SYSTEM TIMING DEFINITIONS & DERIVED PARAMETERS
    // ============================================================
    localparam SYS_CLK_FREQ_HZ     = 50_000_000;
    localparam BASE_STEP_FREQ_HZ   = 100_000;
    localparam HALF_SYS_PERIOD_NS  = 10;
    // 50 MHz clock toggle period

    // Real-time nanoseconds per single master system clock cycle
    localparam NS_PER_SYS_CYCLE    = HALF_SYS_PERIOD_NS * 2;

    // How many master sys_clk cycles occur per stepper_clk tick - the
    // conversion factor between the RTL's period fields (in ticks) and
    // capture_duration's units (sys_clk cycles).
    localparam TICKS_TO_SYS_CYCLES = SYS_CLK_FREQ_HZ / BASE_STEP_FREQ_HZ;

    // 50 MHz Master Clock Generation
    always #HALF_SYS_PERIOD_NS TCXO = ~TCXO;

    // Capture Storage Array Architecture
    reg [31:0] capture_duration[2048];
    reg        capture_dir[2048];
    integer    capture_count = 0;

    // Golden model of stepper_core.v's period recurrence: same starting
    // value, same per-step +/- delta_magnitude, same floor-at-1 (period
    // register never below 1) and same 16-bit saturate-on-add behavior as
    // alu_add_sub/next_period in the RTL. Returns the period (in ticks)
    // that produces step `step_index` (0-indexed) within a segment, so
    // expected inter-step timing can be asserted exactly rather than with
    // a placeholder value. Recomputed from scratch each call rather than
    // building an array - segments here are at most ~10 steps, and a
    // scalar-only function avoids the array/queue task-port support
    // Icarus doesn't have.
    function automatic integer period_at_step;
        input integer start_period;
        input integer delta_magnitude;
        input integer period_increasing;
        input integer step_index;
        integer i;
        integer current;
        begin
            current = start_period;
            for (i = 0; i < step_index; i = i + 1) begin
                if (period_increasing) begin
                    if (current + delta_magnitude > 65535) current = 65535;
                    else current = current + delta_magnitude;
                end else begin
                    if (delta_magnitude >= current) current = 1;
                    else current = current - delta_magnitude;
                end
            end
            period_at_step = current;
        end
    endfunction

    // Safety ceiling for wait_for_captures below - not a timing prediction,
    // just a bound so a genuine DUT bug (steps never firing) ends the
    // simulation instead of hanging forever. The wait itself is event-
    // driven and returns as soon as the expected steps are captured,
    // regardless of this value.
    localparam SAFETY_TIMEOUT_NS = 30_000_000;

    // Blocks until capture_count reaches `target`, checking every 100ns -
    // event-driven completion instead of a fixed #(...) delay sized for
    // the worst case. Falls through on timeout (rather than hanging) and
    // leaves the mismatch for the caller's own capture_count assertion to
    // report.
    task automatic wait_for_captures(input integer target, input integer timeout_ns);
        integer waited;
        begin
            waited = 0;
            while (capture_count < target && waited < timeout_ns) begin
                #100;
                waited = waited + 100;
            end
        end
    endtask

    // ============================================================
    // TRAPEZOIDAL PROFILE PLANNER (mirrors the Rust MCU-side planner:
    // scalar parameters in, RampSegments out - here written into these
    // plain fixed-size module-scope arrays instead of a struct/Vec, since
    // Icarus doesn't support dynamic arrays/queues as task ports). Plain
    // module-scope arrays (not task ports) sidestep that entirely - same
    // pattern already used for capture_duration/capture_dir above.
    // ============================================================
    localparam MAX_SEGMENTS  = 32;
    localparam N_SUB         = 8;
    localparam PERIOD_MAX    = 65535.0;

    reg  [15:0] seg_sp[0:MAX_SEGMENTS-1];
    reg  [15:0] seg_dm[0:MAX_SEGMENTS-1];
    integer     seg_n[0:MAX_SEGMENTS-1];
    reg         seg_increasing[0:MAX_SEGMENTS-1];
    reg         seg_halt[0:MAX_SEGMENTS-1];
    integer     num_segments;
    integer     plan_total_steps;

    // Plans a simple trapezoidal profile (constant acceleration up to
    // v_max, or as far as target_steps allows; cruise; constant
    // deceleration back to rest) from just (target_steps, v_max, a_max) -
    // same three parameters the Rust planner takes, minus jerk (this is
    // deliberately the plain trapezoidal case, not jerk-limited).
    //
    // A linear-per-step period ramp does not give constant acceleration
    // (velocity = 1/period is a reciprocal), so each ramp is evaluated at
    // N_SUB equally-spaced time boundaries against the true physics
    // (constant acceleration a => v(t) = a*t, step n at t_n = sqrt(2n/a)),
    // and only the (small) gap within each boundary pair is approximated
    // as linear - the same approach as planner.rs's append_quad_phase.
    task automatic plan_trapezoid(
        input real target_steps,
        input real v_max,
        input real a_max
    );
        real period_min;
        real n_ramp_full;
        real v_peak;
        real n_ramp;
        real n_cruise;
        real t_ramp;
        real t_b[0:N_SUB];
        real bp[0:N_SUB];
        real raw_counts[0:N_SUB-1];
        integer counts[0:N_SUB-1];
        integer i;
        integer sum_c;
        integer diff;
        integer best_idx;
        integer sp_i, ep_i;
        integer n_i;
        begin
            num_segments = 0;
            plan_total_steps = 0;

            period_min = 1.0 / v_max;
            n_ramp_full = (v_max * v_max) / (2.0 * a_max);

            if (2.0 * n_ramp_full <= target_steps) begin
                v_peak   = v_max;
                n_ramp   = n_ramp_full;
                n_cruise = target_steps - 2.0 * n_ramp;
            end else begin
                v_peak   = $sqrt(a_max * target_steps);
                n_ramp   = target_steps / 2.0;
                n_cruise = 0.0;
            end

            t_ramp = v_peak / a_max;

            for (i = 0; i <= N_SUB; i = i + 1) begin
                t_b[i] = t_ramp * i / N_SUB;
            end

            for (i = 0; i <= N_SUB; i = i + 1) begin
                if (t_b[i] <= 0.0) begin
                    bp[i] = -1.0; // singular (v=0); filled below once counts[0] is known
                end else begin
                    bp[i] = 1.0 / (a_max * t_b[i]);
                    if (bp[i] > PERIOD_MAX) bp[i] = PERIOD_MAX;
                end
            end

            for (i = 0; i < N_SUB; i = i + 1) begin
                raw_counts[i] = 0.5 * a_max * (t_b[i+1]*t_b[i+1] - t_b[i]*t_b[i]);
                counts[i] = $rtoi(raw_counts[i] + 0.5);
            end
            sum_c = 0;
            for (i = 0; i < N_SUB; i = i + 1) sum_c = sum_c + counts[i];
            diff = $rtoi(n_ramp + 0.5) - sum_c;
            best_idx = 0;
            for (i = 1; i < N_SUB; i = i + 1) begin
                if (counts[i] > counts[best_idx]) best_idx = i;
            end
            counts[best_idx] = counts[best_idx] + diff;

            if (bp[0] < 0.0) begin
                if (counts[0] > 0) bp[0] = t_b[1] / $itor(counts[0]);
                else bp[0] = PERIOD_MAX;
            end

            // accel: rest -> v_peak
            for (i = 0; i < N_SUB; i = i + 1) begin
                n_i = counts[i];
                if (n_i > 0) begin
                    sp_i = $rtoi(bp[i] + 0.5);
                    ep_i = $rtoi(bp[i+1] + 0.5);
                    if (sp_i < 1) sp_i = 1;
                    if (ep_i < 1) ep_i = 1;
                    seg_sp[num_segments]         = sp_i[15:0];
                    seg_dm[num_segments]         = (sp_i - ep_i) / n_i;
                    seg_n[num_segments]          = n_i;
                    seg_increasing[num_segments] = 1'b0; // PERIOD_DECREASING
                    seg_halt[num_segments]       = 1'b0;
                    num_segments     = num_segments + 1;
                    plan_total_steps = plan_total_steps + n_i;
                end
            end

            // cruise at v_peak
            if (n_cruise > 0.5) begin
                sp_i = $rtoi((1.0/v_peak) + 0.5);
                if (sp_i < 1) sp_i = 1;
                n_i = $rtoi(n_cruise + 0.5);
                seg_sp[num_segments]         = sp_i[15:0];
                seg_dm[num_segments]         = 16'd0;
                seg_n[num_segments]          = n_i;
                seg_increasing[num_segments] = 1'b0;
                seg_halt[num_segments]       = 1'b0;
                num_segments     = num_segments + 1;
                plan_total_steps = plan_total_steps + n_i;
            end

            // decel: v_peak -> rest (exact mirror of accel, reversed)
            for (i = N_SUB - 1; i >= 0; i = i - 1) begin
                n_i = counts[i];
                if (n_i > 0) begin
                    sp_i = $rtoi(bp[i+1] + 0.5);
                    ep_i = $rtoi(bp[i] + 0.5);
                    if (sp_i < 1) sp_i = 1;
                    if (ep_i < 1) ep_i = 1;
                    seg_sp[num_segments]         = sp_i[15:0];
                    seg_dm[num_segments]         = (ep_i - sp_i) / n_i;
                    seg_n[num_segments]          = n_i;
                    seg_increasing[num_segments] = 1'b1; // PERIOD_INCREASING
                    seg_halt[num_segments]       = 1'b0;
                    num_segments     = num_segments + 1;
                    plan_total_steps = plan_total_steps + n_i;
                end
            end

            // last segment brings the profile to a full stop
            seg_halt[num_segments - 1] = 1'b1;
        end
    endtask

    // Writes every planned segment to the bus (TX_CONFIG then each
    // CTST/SPDM pair), printing each segment's data to the console first -
    // mirrors motion_control.rs logging each RampSegment before executing
    // it.
    task automatic write_planned_segments;
        integer i;
        reg [1:0] cmd_i;
        begin
            $display("[PLAN] %0d segments, %0d steps total:", num_segments, plan_total_steps);
            bus_write(REG_STEP_TX_CONFIG, {16'h0000, 8'h00, num_segments[7:0]});
            #200;

            for (i = 0; i < num_segments; i = i + 1) begin
                cmd_i = seg_halt[i] ? CMD_MOVE_HALT : CMD_MOVE;
                $display("  segment %0d: SP=%0d DM=%0d n=%0d %s %s",
                          i, seg_sp[i], seg_dm[i], seg_n[i],
                          seg_increasing[i] ? "INCREASING" : "DECREASING",
                          seg_halt[i] ? "MOVE_HALT" : "MOVE");
                bus_write(REG_STEP_SEG_CTST, {4'd0, seg_increasing[i], DIR_NORMAL, cmd_i, seg_n[i][23:0]});
                bus_write(REG_STEP_SEG_SPDM, {seg_sp[i], seg_dm[i]});
                #200;
            end
        end
    endtask

    time       last_step_time = 0;
    time       current_time = 0;
    reg        monitoring_active = 0;
    // Automated Step Capture Monitor Loop
    always @(posedge step_pins[0]) begin
        if (monitoring_active && capture_count < 2048) begin
            current_time = $time;
            if (capture_count == 0) begin
                capture_duration[capture_count] = 0;
            end else begin
                // Express the duration dynamically in master system clock cycles
                capture_duration[capture_count] = (current_time - last_step_time) / NS_PER_SYS_CYCLE;
            end
            capture_dir[capture_count] = dir_pins[0];
            capture_count = capture_count + 1;
            last_step_time = current_time;
        end
    end

    reg [7:0] test_index = 0;
    initial begin
        $dumpfile("steppers_motion_tb.vcd");
        $dumpvars(0, steppers_motion_tb);
        // Hardware Initialization
        global_motor_en = 1'b1;
        sys_reset();
        bus_init();

        #200;
        // ============================================================
        $display("TEST %0d: Single step", test_index);
        test_index += 1;
        // ============================================================
        begin : SINGLE_POINT_STEP_TEST
            reg [31:0] status_read;
            reg [31:0] position_read;
            integer k;

            // Dynamic Trajectory Parameters
            integer target_steps = 1;
            integer start_period = 1;
            integer delta_magnitude = 1;
            capture_count = 0;
            monitoring_active = 1;

            // 1) Configure Transmit Framework for Motor 0 (1 Point)
            $display("[TX_CONFIG] Configuring Motor 0 for a single target position profile...");
            bus_write(REG_STEP_TX_CONFIG, {16'h0000, 8'h00, 8'h01});
            #200;

            // 2) Send payload
            bus_write(REG_STEP_SEG_CTST, {6'd0, CMD_MOVE_HALT, target_steps[23:0]});
            bus_write(REG_STEP_SEG_SPDM, {start_period[15:0], delta_magnitude[15:0]});
            #200;

            // 3) Strobe Start command utilizing REG_STEP_CTRL
            $display("[CTRL] Issuing Start Strobe to Core 0...");
            bus_write(REG_STEP_CTRL, 32'h0000_0100);

            // 4) Verify dynamic engine status transitions to MOVING - the
            // shared run engine services motors round-robin (up to 10
            // sys_clk cycles, 200ns, before it reaches this one), so this
            // isn't instantaneous the way a per-motor engine would be.
            #200;
            bus_read(REG_STEP_STATUS_0, status_read);
            `ASSERT_EQ(status_read[0], 1'b1, "0x%08h", "Motor 0 status expected to register MOVING (1) upon start strobe activation.");
            // Wait for the step to actually be captured (event-driven)
            wait_for_captures(target_steps, SAFETY_TIMEOUT_NS);
            // 5) Verify dynamic engine status transitions to IDLE/STOPPED after time-to-complete
            bus_read(REG_STEP_STATUS_0, status_read);
            `ASSERT_EQ(status_read[0], 1'b0, "0x%08h", "Motor 0 status expected to register STOPPED (0) after completing steps.");

            monitoring_active = 0;
            // 6) Assert total count and timing separation intervals
            $display("[VERIFY] Evaluating recorded capture matrices...");
            `ASSERT_EQ(capture_count, target_steps, "%0d", "Total generated output steps mismatch target input profile configuration.");
            for (k = 0; k < capture_count; k = k + 1) begin
                $display("  -> Step Index %0d: Delta = %0d sys_clk cycles, DIR = %b", k, capture_duration[k], capture_dir[k]);
                `ASSERT_EQ(capture_dir[k], 1'b0, "%b", $sformatf("Direction mismatch at step phase array element %0d (Expected Forward)", k));
                if (k > 0) begin
                    `ASSERT_EQ(capture_duration[k], period_at_step(start_period, delta_magnitude, PERIOD_DECREASING, k) * TICKS_TO_SYS_CYCLES, "%0d", $sformatf("Step time interval mismatch at historical sequence point %0d", k));
                end
            end

            // 7) Assert the FPGA's own tracked position matches exactly -
            // a single forward step from a fresh reset.
            bus_read(REG_STEP_POS_0, position_read);
            `ASSERT_EQ($signed(position_read), 1, "%0d", "Motor 0 final position mismatch after a single forward step.");
        end

        // ============================================================
        $display("TEST %0d: Multi-Point with Intentional Pause", test_index);
        test_index += 1;
        // ============================================================
        begin : MULTI_POINT_PAUSE_TEST
            reg [31:0] status_read;
            reg [31:0] position_read;
            integer k;

            sys_reset();

            // 1) Configure Transmit Framework for Motor 0 (3 Points Total)
            $display("[TX_CONFIG] Configuring Motor 0 for 3 distinct trajectory profile segments...");
            bus_write(REG_STEP_TX_CONFIG, {16'h0000, 8'h00, 8'h03});
            #200;

            // 2) Load 3 point trajectory segments (CTST steps field is a
            //    relative step count for that segment, not an absolute
            //    coordinate)
            bus_write(REG_STEP_SEG_CTST,    {5'd0, DIR_NORMAL, CMD_MOVE_HALT_WAIT, 24'd10});
            bus_write(REG_STEP_SEG_SPDM,    {16'd1, 16'd1});
            #200;

            bus_write(REG_STEP_SEG_CTST,    {4'd0, PERIOD_DECREASING, DIR_REVERSE, CMD_MOVE, 24'd6});
            bus_write(REG_STEP_SEG_SPDM,    {16'd2, 16'd2});
            #200;

            bus_write(REG_STEP_SEG_CTST,    {4'd0, PERIOD_DECREASING, DIR_REVERSE, CMD_MOVE_HALT, 24'd4});
            bus_write(REG_STEP_SEG_SPDM,    {16'd3, 16'd3});
            #200;

            // --------------------------------------------------------------------
            // PHASE 1: Capture Point 1 Steps and Verify Segment Stop Boundary
            // --------------------------------------------------------------------
            capture_count = 0;
            last_step_time = $time;
            monitoring_active = 1;

            $display("[PHASE 1] Issuing Start Strobe to Core 0...");
            bus_write(REG_STEP_CTRL, 32'h0000_0100);

            // Wait for point 1's 10 steps to be captured (event-driven)
            wait_for_captures(10, SAFETY_TIMEOUT_NS);

            // Verify core engine dropped into PAUSED state via command descriptor boundary
            bus_read(REG_STEP_STATUS_0, status_read);
            `ASSERT_EQ(status_read[0], 1'b0, "0x%08h", "[PHASE 1] Motor 0 failed to enter PAUSED state after Point 1 execution.");

            monitoring_active = 0;

            // Evaluate Recorded Performance Phase 1
            $display("[PHASE 1 VERIFY] Evaluating recorded capture ...");
            `ASSERT_EQ(capture_count, 10, "%0d", "[PHASE 1] Generated step count error for Point 1.");
            for (k = 0; k < capture_count; k = k + 1) begin
                $display("  -> Phase 1, Step Index %0d: Delta = %0d sys_clk cycles, DIR = %b", k, capture_duration[k], capture_dir[k]);
                `ASSERT_EQ(capture_dir[k], 1'b0, "%b", $sformatf("[PHASE 1] Expected FORWARD direction (0) at index %0d", k));
                if (k > 0) begin
                    // point 1: SP=1, DM=1
                    `ASSERT_EQ(capture_duration[k], period_at_step(1, 1, PERIOD_DECREASING, k) * TICKS_TO_SYS_CYCLES, "%0d", $sformatf("[PHASE 1] Step time period drift at step index %0d", k));
                end
            end

            // Position after 10 forward (DIR_NORMAL) steps from a fresh reset
            bus_read(REG_STEP_POS_0, position_read);
            `ASSERT_EQ($signed(position_read), 10, "%0d", "[PHASE 1] Motor 0 position mismatch after 10 forward steps.");

            // --------------------------------------------------------------------
            // PHASE 2: Resume Sequence and Capture Points 2 and 3 Seamlessly
            // --------------------------------------------------------------------
            capture_count = 0;
            last_step_time = $time;
            monitoring_active = 1;

            $display("[PHASE 2] Issuing Resume Strobe via Start Command...");
            bus_write(REG_STEP_CTRL, 32'h0000_0100);

            // Wait for points 2 & 3's streaming pipeline steps to be
            // captured (6 + 4 = 10 steps total), event-driven
            wait_for_captures(10, SAFETY_TIMEOUT_NS);

            // Verify engine has completely finished profile segments and returned to full IDLE/STOPPED
            bus_read(REG_STEP_STATUS_0, status_read);
            `ASSERT_EQ(status_read[0], 1'b0, "0x%08h", "[PHASE 2] Motor 0 failed to return to IDLE/STOPPED status after profile tail.");

            monitoring_active = 0;

            // Evaluate Recorded Performance for Phase 2
            $display("[PHASE 2 VERIFY] Evaluating seamless streaming capture ...");
            `ASSERT_EQ(capture_count, 10, "%0d", "[PHASE 2] Combined steps mismatch across streaming segments.");
            for (k = 0; k < capture_count; k = k + 1) begin
                $display("  -> Phase 2, Step Index %0d: Delta = %0d sys_clk cycles, DIR = %b", k, capture_duration[k], capture_dir[k]);
                `ASSERT_EQ(capture_dir[k], 1'b1, "%b", $sformatf("[PHASE 2] Expected REVERSE direction (1) at index %0d", k));
                if (k > 0) begin
                    if (k < 6) begin
                        // point 2: SP=2, DM=2 - local step index k
                        `ASSERT_EQ(capture_duration[k], period_at_step(2, 2, PERIOD_DECREASING, k) * TICKS_TO_SYS_CYCLES, "%0d", $sformatf("[PHASE 2] Step period drift at step index %0d", k));
                    end else begin
                        // point 3: SP=3, DM=3 - fresh reload at the segment
                        // boundary, so its own local step index is k - 6
                        `ASSERT_EQ(capture_duration[k], period_at_step(3, 3, PERIOD_DECREASING, k - 6) * TICKS_TO_SYS_CYCLES, "%0d", $sformatf("[PHASE 2] Step period drift at step index %0d", k));
                    end
                end
            end

            // Position back at 0: 10 forward (Phase 1) then 10 reverse
            // (Phase 2) is a net round trip.
            bus_read(REG_STEP_POS_0, position_read);
            `ASSERT_EQ($signed(position_read), 0, "%0d", "[PHASE 2] Motor 0 position mismatch after the round trip (expected net 0).");
        end

        // ============================================================
        $display("TEST %0d: Trapezoidal Capture and CSV Export", test_index);
        test_index += 1;
        // ============================================================
        begin : SCURVE_CAPTURE_TEST
            reg [31:0] status_read;
            reg [31:0] position_read;
            integer k;
            integer csv_captured_file;
            integer total_steps;

            sys_reset();

            // 1) & 2) Plan a trapezoidal profile from just three input
            //    parameters (target_steps, v_max, a_max) - mirroring the
            //    Rust MCU-side planner - instead of hand-picked segment
            //    values, then write whatever it generates to the bus
            //    (TX_CONFIG followed by each segment's CTST/SPDM), with
            //    every segment printed to the console first.
            $display("[TX_CONFIG] Configuring Motor 0 for a planned trapezoidal profile...");
            plan_trapezoid(100.0, 0.2, 0.0005); // target_steps, v_max (steps/tick), a_max (steps/tick^2)
            total_steps = plan_total_steps;
            write_planned_segments();

            // 3) Capture, as per TEST 1
            capture_count = 0;
            last_step_time = $time;
            monitoring_active = 1;

            $display("[CTRL] Issuing Start Strobe to Core 0...");
            bus_write(REG_STEP_CTRL, 32'h0000_0100);

            wait_for_captures(total_steps, SAFETY_TIMEOUT_NS);

            bus_read(REG_STEP_STATUS_0, status_read);
            `ASSERT_EQ(status_read[0], 1'b0, "0x%08h", "[TRAPEZOID] Motor 0 failed to return to STOPPED after profile completion.");

            monitoring_active = 0;

            $display("[TRAPEZOID VERIFY] Evaluating captured profile ...");
            `ASSERT_EQ(capture_count, total_steps, "%0d", "[TRAPEZOID] Captured step count mismatch against the planned profile.");
            for (k = 0; k < capture_count; k = k + 1) begin
                `ASSERT_EQ(capture_dir[k], 1'b0, "%b", $sformatf("[TRAPEZOID] Expected FORWARD direction (0) at index %0d", k));
            end

            // Entirely forward motion, so final position == the planned
            // total step count (dynamic, not hardcoded, since the profile
            // is itself parameterized).
            bus_read(REG_STEP_POS_0, position_read);
            `ASSERT_EQ($signed(position_read), total_steps, "%0d", "[TRAPEZOID] Motor 0 final position mismatch against the planned profile.");

            // 4) Export Physical Step Capture Log to CSV. Velocity,
            //    acceleration and jerk are recovered as successive
            //    discrete derivatives of the captured inter-step timing:
            //    velocity = 1 step / dt, acceleration = dv/dt, jerk =
            //    da/dt - each divided by the same real elapsed time
            //    (capture_duration converted from sys_clk cycles to
            //    seconds), not just the raw difference of the previous
            //    quantity.
            csv_captured_file = $fopen("captured_scurve.csv", "w");
            $fwrite(csv_captured_file, "Time,Step,IntervalCycles,Velocity,Accel,Jerk\n");
            begin
                real dt_s;
                real last_v, cur_v;
                real last_a, cur_a;
                real cur_j;
                real t;

                t = 0.0;
                last_v = 0.0;
                last_a = 0.0;

                for (k = 0; k < capture_count; k = k + 1) begin
                    if (capture_duration[k] > 0) begin
                        dt_s  = $itor(capture_duration[k]) / $itor(SYS_CLK_FREQ_HZ);
                        cur_v = 1.0 / dt_s;
                    end else begin
                        dt_s  = 0.0;
                        cur_v = 0.0;
                    end

                    if (dt_s > 0.0) begin
                        cur_a = (cur_v - last_v) / dt_s;
                        cur_j = (cur_a - last_a) / dt_s;
                    end else begin
                        cur_a = 0.0;
                        cur_j = 0.0;
                    end

                    t = t + dt_s;

                    $fwrite(csv_captured_file, "%f,%0d,%0d,%f,%f,%f\n",
                        t, k, capture_duration[k], cur_v, cur_a, cur_j);

                    last_v = cur_v;
                    last_a = cur_a;
                end
            end
            $fclose(csv_captured_file);
            $display("[CAPTURE] Exported %0d actual steps to captured_scurve.csv", capture_count);
        end

        report();
        $finish;
    end

endmodule
