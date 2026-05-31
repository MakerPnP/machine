`timescale 1ns/1ps

module timer_mux_tb;

    // Testbench signals
    wire MUX_SEL1;
    wire MUX_SEL2;
    wire MUX_SEL3;
    wire MUX_SEL4;
    reg RESET = 1;

    // Instantiate the DUT (DUT = Device Under Test)
    timer_mux dut (
        .reset(RESET),
        .mux_sel1(MUX_SEL1),
        .mux_sel2(MUX_SEL2),
        .mux_sel3(MUX_SEL3),
        .mux_sel4(MUX_SEL4)
    );

    // Simulation control
    initial begin
        $dumpfile("timer_mux.vcd");
        $dumpvars(0, timer_mux_tb);

        // reset pulse
        RESET = 1;
        #20;
        RESET = 0;

        // Run simulation for some time
        #100;

        $finish;
    end

endmodule