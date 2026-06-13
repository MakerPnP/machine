`timescale 1ns/1ps

`include "src/test/assertions.svh"

module clock_out_tb;

    // Testbench signals
    reg RESET;
    reg TCXO = 0;

    wire FPGA_CLK_1;
    wire FPGA_CLK_2;
    wire FPGA_CLK_3;
    wire FPGA_CLK_4;

    // Instantiate the DUT (DUT = Device Under Test)
    clock_out dut (
        .sys_clk(TCXO),
        .reset(RESET),
        .clock_out1(FPGA_CLK_1),
        .clock_out2(FPGA_CLK_2),
        .clock_out3(FPGA_CLK_3),
        .clock_out4(FPGA_CLK_4)
    );

    // Clock generation: 100 MHz simulated clock (10ns period)
    always #5 TCXO = ~TCXO;

    // Simulation control
    initial begin
        $dumpfile("clock_out_tb.vcd");
        $dumpvars(0, clock_out_tb);

        // reset pulse
        RESET = 1;
        #10;
        `ASSERT_EQ(FPGA_CLK_1, 1'd1);
        `ASSERT_EQ(FPGA_CLK_2, 1'd1);
        `ASSERT_EQ(FPGA_CLK_3, 1'd1);
        `ASSERT_EQ(FPGA_CLK_4, 1'd1);

        #10;
        RESET = 0;

        #10;
        `ASSERT_EQ(FPGA_CLK_1, 1'd0);
        `ASSERT_EQ(FPGA_CLK_2, 1'd0);
        `ASSERT_EQ(FPGA_CLK_3, 1'd0);
        `ASSERT_EQ(FPGA_CLK_4, 1'd0);

        // Run simulation for some time
        #100;

        report();
        $finish;
    end

endmodule