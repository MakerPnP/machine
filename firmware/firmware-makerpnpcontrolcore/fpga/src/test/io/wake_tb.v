`timescale 1ns/1ps

`include "src/test/assertions.svh"

module wake_tb;

    // Testbench signals
    reg RESET;
    reg SYS_CLK = 0;

    reg NWAKE_IN;
    wire NWAKE_1;

    // Instantiate the DUT (DUT = Device Under Test)
    wake dut (
        .sys_clk(SYS_CLK),
        .reset(RESET),
        .nwake_in(NWAKE_IN),
        .nwake_1(NWAKE_1)
    );

    always #10 SYS_CLK = ~SYS_CLK; // (10 * 2) = 20ns period -> 50 MHz

    // Simulation control
    initial begin
        $dumpfile("wake_tb.vcd");
        $dumpvars(0, wake_tb);

        // reset pulse, with NWAKE_IN=HIGH
        RESET = 1;
        NWAKE_IN = 1;
        #20;
        // during reset pulse, NWAKE_IN goes LOW, but this should not be reflected on the output, until after reset goes LOW
        NWAKE_IN = 0;
        #1;
        `ASSERT_EQ(NWAKE_1, 1'd1);
        #19;
        RESET = 0;

        #20;
        `ASSERT_EQ(NWAKE_1, 1'd0);

        // Run simulation for some time
        #100;

        `ASSERT_EQ(NWAKE_1, 1'd0);

        report();
        $finish;
    end

endmodule