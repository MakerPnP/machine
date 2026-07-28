`timescale 1ns/1ps

`include "src/test/assertions.svh"

module blink_tb;

    // Testbench signals
    reg SYS_CLK = 0;
    wire FPGA_ACT;
    reg RESET = 1;

    // Instantiate the DUT (DUT = Device Under Test)
    blink #(
        .SPEED(10)   // small number for fast simulation
    ) dut (
        .clk(SYS_CLK),
        .reset(RESET),
        .led(FPGA_ACT)
    );

    always #10 SYS_CLK = ~SYS_CLK; // (10 * 2) = 20ns period -> 50 MHz

    // Simulation control
    initial begin
        $dumpfile("blink_tb.vcd");
        $dumpvars(0, blink_tb);

        // reset pulse
        RESET = 1;
        #20;
        RESET = 0;

        // Run simulation for some time
        #2500;

        $display("LED: %d", FPGA_ACT);

        report();
        $finish;
    end

endmodule