`timescale 1ns/1ps

`include "src/test/assertions.svh"

module clock_out_tb;

    // Testbench signals
    wire FPGA_CLK_1;
    wire FPGA_CLK_2;
    wire FPGA_CLK_3;
    wire FPGA_CLK_4;
    reg RESET = 1;

    // Instantiate the DUT (DUT = Device Under Test)
    clock_out dut (
        .reset(RESET),
        .clock_out1(FPGA_CLK_1),
        .clock_out2(FPGA_CLK_2),
        .clock_out3(FPGA_CLK_3),
        .clock_out4(FPGA_CLK_4)
    );

    // Simulation control
    initial begin
        $dumpfile("clock_out_tb.vcd");
        $dumpvars(0, clock_out_tb);

        // reset pulse
        RESET = 1;
        #20;
        RESET = 0;

        // Run simulation for some time
        #100;

        `ASSERT_EQ(FPGA_CLK_1, 1'd0);
        `ASSERT_EQ(FPGA_CLK_2, 1'd0);
        `ASSERT_EQ(FPGA_CLK_3, 1'd0);
        `ASSERT_EQ(FPGA_CLK_4, 1'd0);

        report();

        $finish;
    end

endmodule