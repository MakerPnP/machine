`timescale 1ns/1ps

`include "src/test/assertions.svh"

module steppers_tb;

    reg RESET;
    reg TCXO = 0;

    `include "src/test/bus_io.svh"
    `include "src/main/io/steppers_regs.svh"
    `include "src/main/io/steppers_shared.svh"

    reg [3:0] step_pins;
    reg [3:0] dir_pins;
    reg global_motor_en;

    wire [15:0] debug;

    reg [31:0] result;

    steppers dut (
        .sys_clk(TCXO),
        .reset(RESET),

        .bus_stb(stb),
        .bus_we(we),
        .bus_addr(addr),
        .bus_din(din),
        .bus_dout(dout),
        .bus_ack(ack),

        .step_pins(step_pins),
        .dir_pins(dir_pins),
        .global_motor_en(global_motor_en)
    );

    always #10 TCXO = ~TCXO; // 50 MHz

    reg [7:0] test_index = 0;

    initial begin
        $dumpfile("steppers_tb.vcd");
        $dumpvars(0, steppers_tb);

        sys_reset();
        bus_init();

        // ============================================================
        $display("TEST: CTRL enable");
        test_index += 1;
        // Expect: output enabled BUT no STEP/DIR signals
        // ============================================================
        begin : CONFIGURE_AND_ENABLE

            bus_write(REG_STEP_CTRL, 32'b0000_0000_0000_0000_0000_0000_0000_0001);

            #200;

            // TODO check that step/dirs are all low
        end

        // ============================================================
        $display("TEST: SET POINTS (Motor 0)");
        test_index += 1;
        // ============================================================
        begin : SET_POINTS

            // motor 0
            // 2 points
            bus_write(REG_STEP_TX_CONFIG, {8'h00, 8'h00, 8'h00, 8'h02});

            #200;
        end

        // ============================================================
        $display("TEST: Write and verify streaming Point Data");
        test_index += 1;
        // ============================================================
        begin : STREAM_DATA_1
            integer i;

            // embed a 2 bit command into the upper 2 bits of the position
            reg [31:0] control_and_steps[6] = '{
                { 8'h00, 24'h111111 },
                { 8'h05, 24'h222222 },
                { 8'h50, 24'h333333 },
                { 8'hA5, 24'h444444 },
                { 8'h5A, 24'h555555 },
                { 8'hFF, 24'h666666 }
            };


            reg [31:0] start_period_and_delta_magnitude[6] = '{
                {16'ha511, 16'h0110},
                {16'hff22, 16'h0220},
                {16'h5a34, 16'h0330},
                {16'hff44, 16'h0440},
                {16'ha555, 16'h0550},
                {16'hff66, 16'h0660}
            };

            begin : STEAM_DATA_WRITE
                $display("TEST: Write streaming Point Data");

                for (i = 0; i < 6; i = i + 1) begin
                    bus_write(REG_STEP_SEG_CTST, control_and_steps[i]);
                    bus_write(REG_STEP_SEG_SPDM, start_period_and_delta_magnitude[i]);

                    // XXX
                    #100;
                end
            end

            begin : STREAM_DATA_READ
                reg [31:0] ctps_verify;
                reg [31:0] spdm_verify;

                $display("TEST: Verify streaming Point Data");

                for (i = 0; i < 6; i = i + 1) begin
                    bus_read(REG_STEP_SEG_CTST, ctps_verify);
                    bus_read(REG_STEP_SEG_SPDM, spdm_verify);

                    // XXX
                    #100;

                    `ASSERT_EQ(ctps_verify, control_and_steps[i], "0x%08h", $sformatf("control_and_steps mismatch at index %d", i));
                    `ASSERT_EQ(spdm_verify, start_period_and_delta_magnitude[i], "0x%08h", $sformatf("start_period_and_delta_magnitude mismatch at index %d", i));
                end
            end
        end

        // ============================================================
        $display("TEST: Exhaustive memory exercise - all motors, all segments");
        test_index += 1;
        // Writes and reads back dut.MAX_SEG segments for every one of the
        // 8 motors - every (motor, segment, word) address the segment
        // table can express gets exercised at least once. Uses a
        // deterministic, recomputable value per (motor, segment) rather
        // than a fixed table, so it automatically adapts if MAX_SEG is
        // ever changed again.
        // ============================================================
        begin : EXHAUSTIVE_MEMORY_TEST
            integer m, i;
            reg [31:0] ctst_expected, spdm_expected;
            reg [31:0] ctst_readback, spdm_readback;

            for (m = 0; m < 8; m = m + 1) begin
                $display("  [WRITE] motor %0d: writing all %0d segments...", m, dut.MAX_SEG);
                bus_write(REG_STEP_TX_CONFIG, {16'h0000, {5'd0, m[2:0]}, dut.MAX_SEG});

                for (i = 0; i < dut.MAX_SEG; i = i + 1) begin
                    ctst_expected = ((m & 8'hF) << 28) | ((i & 8'hFF) << 8) | ((m ^ i) & 8'hFF);
                    spdm_expected = ((i & 16'hFFFF) << 16) | (((m << 8) | (i ^ 8'hAA)) & 16'hFFFF);
                    bus_write(REG_STEP_SEG_CTST, ctst_expected);
                    bus_write(REG_STEP_SEG_SPDM, spdm_expected);
                end

                $display("  [READ]  motor %0d: reading all %0d segments back...", m, dut.MAX_SEG);
                for (i = 0; i < dut.MAX_SEG; i = i + 1) begin
                    ctst_expected = ((m & 8'hF) << 28) | ((i & 8'hFF) << 8) | ((m ^ i) & 8'hFF);
                    spdm_expected = ((i & 16'hFFFF) << 16) | (((m << 8) | (i ^ 8'hAA)) & 16'hFFFF);
                    bus_read(REG_STEP_SEG_CTST, ctst_readback);
                    bus_read(REG_STEP_SEG_SPDM, spdm_readback);
                    `ASSERT_EQ(ctst_readback, ctst_expected, "0x%08h", $sformatf("motor %0d segment %0d CTST mismatch", m, i));
                    `ASSERT_EQ(spdm_readback, spdm_expected, "0x%08h", $sformatf("motor %0d segment %0d SPDM mismatch", m, i));
                end
            end
        end

        // ============================================================
        // END
        // ============================================================
        report();
        $finish;
    end

endmodule
