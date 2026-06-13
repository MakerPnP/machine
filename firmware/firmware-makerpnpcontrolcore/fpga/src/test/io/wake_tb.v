`timescale 1ns/1ps

`include "src/test/assertions.svh"

module wake_tb;

    // Testbench signals
    reg RESET;
    reg TCXO = 0;

    reg NWAKE_IN;
    wire NWAKE_1;

    // Instantiate the DUT (DUT = Device Under Test)
    wake dut (
        .sys_clk(TCXO),
        .reset(RESET),
        .nwake_in(NWAKE_IN),
        .nwake_1(NWAKE_1)
    );

    // Clock generation: 100 MHz simulated clock (10ns period)
    always #5 TCXO = ~TCXO;

    // Simulation control
    initial begin
        $dumpfile("wake_tb.vcd");
        $dumpvars(0, wake_tb);

        // reset pulse, with NWAKE_IN=HIGH
        RESET = 1;
        NWAKE_IN = 1;
        #10;
        // during reset pulse, NWAKE_IN goes LOW, but this should not be reflected on the output, until after reset goes LOW
        NWAKE_IN = 0;
        #1;
        `ASSERT_EQ(NWAKE_1, 1'd1);
        #9;
        RESET = 0;

        #10;
        `ASSERT_EQ(NWAKE_1, 1'd0);

        // Run simulation for some time
        #100;

        `ASSERT_EQ(NWAKE_1, 1'd0);

        report();
        $finish;
    end

endmodule