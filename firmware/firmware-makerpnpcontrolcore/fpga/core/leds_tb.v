`timescale 1ns/1ps

module leds_tb;

    // Testbench signals
    reg RESET;
    reg TCXO = 0;
    wire FPGA_ACT;
    wire MCU_ACT;

    reg [7:0] led_out;
    reg strobe_led_update = 1'b0;

    // Instantiate the DUT (DUT = Device Under Test)
    leds dut (
        .reset(RESET),
        .sys_clk(TCXO),
        .led_out(led_out),
        .strobe_led_update(strobe_led_update),
        .mcu_act(MCU_ACT),
        .fpga_act(FPGA_ACT)
    );

    // Clock generation: 100 MHz simulated clock (10ns period)
    always #5 TCXO = ~TCXO;

    // Simulation control
    initial begin
        $dumpfile("leds.vcd");
        $dumpvars(0, leds_tb);

        // reset pulse
        RESET = 1;
        #20;
        RESET = 0;

        // Run simulation for some time
        #100;

        led_out = 8'b0000_0000;
        // hold the strobe for a few clock cycles
        strobe_led_update = 1'b1;
        #100;
        strobe_led_update = 1'b0;
        #100;

        $display("LEDs. mcu: %d, fpga: %d", MCU_ACT, FPGA_ACT);

        // Run simulation for some time
        #100;

        led_out = 8'b0000_0011;
        // hold the strobe for a few clock cycles
        strobe_led_update = 1'b1;
        #100;
        strobe_led_update = 1'b0;
        #100;

        $display("LEDs. mcu: %d, fpga: %d", MCU_ACT, FPGA_ACT);

        $finish;
    end

endmodule