`timescale 1ns/1ps

module blink_tb;

    // Testbench signals
    reg TCXO = 0;
    wire FPGA_ACT;
    reg RESET = 1;

    // Instantiate the DUT (DUT = Device Under Test)
    blink #(
        .SPEED(10)   // small number for fast simulation
    ) dut (
        .clk(TCXO),
        .reset(RESET),
        .led(FPGA_ACT)
    );

    // Clock generation: 100 MHz simulated clock (10ns period)
    always #5 TCXO = ~TCXO;

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


        $finish;
    end

endmodule