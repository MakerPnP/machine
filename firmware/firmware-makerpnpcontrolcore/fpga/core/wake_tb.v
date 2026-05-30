`timescale 1ns/1ps

module wake_tb;

    // Testbench signals
    reg NWAKE_IN;
    wire NWAKE_1;
    reg RESET = 1;

    // Instantiate your design (DUT = Device Under Test)
    wake dut (
        .reset(RESET),
        .nwake_in(NWAKE_IN),
        .nwake_1(NWAKE_1)
    );

    // Simulation control
    initial begin
        $dumpfile("wake.vcd");
        $dumpvars(0, wake_tb);

        // reset pulse, with NWAKE_IN=HIGH
        RESET = 1;
        NWAKE_IN = 1;
        #10;
        // during reset pulse, NWAKE_IN goes LOW, but this should not be reflected on the output, until after reset goes LOW
        NWAKE_IN = 0;
        #10;
        RESET = 0;

        // Run simulation for some time
        #100;

        $finish;
    end

endmodule