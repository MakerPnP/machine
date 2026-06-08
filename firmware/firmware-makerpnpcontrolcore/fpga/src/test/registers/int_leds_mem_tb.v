`timescale 1ns/1ps

`include "src/test/assertions.svh"

module int_leds_mem_tb;

    // Testbench signals
    reg RESET;
    reg TCXO = 0;

    wire FPGA_ACT;
    wire MCU_ACT;

    reg [15:0] mem_addr;
    reg [31:0] mem_din;
    reg [31:0] mem_dout;
    reg        mem_we   = 0;

    reg [5:0]  led_addr;
    reg [31:0] led_din;
    reg [31:0] led_dout;
    reg        led_we;

    wire [15:0] debug;

    leds dut (
        .reset(RESET),
        .sys_clk(TCXO),

        .bus_we(led_we),
        .bus_addr(led_addr),
        .bus_din(led_din),
        .bus_dout(led_dout),

        .mcu_act(MCU_ACT),
        .fpga_act(FPGA_ACT),

        .debug(debug)
    );

    memory memory_map_inst (
        .reset(RESET),
        .clk_a(TCXO),
        .we_a(mem_we),
        .addr_a(mem_addr),
        .din_a(mem_din),
        .dout_a(mem_dout),

        .led_we(led_we),
        .led_addr(led_addr),
        .led_din(led_din),
        .led_dout(led_dout)
    );

    // Clock generation: 100 MHz simulated clock (10ns period)
    always #5 TCXO = ~TCXO;

    // Simulation control
    initial begin
        $dumpfile("int_leds_mem_tb.vcd");
        $dumpvars(0, int_leds_mem_tb);

        // reset pulse
        RESET = 1;
        #20;
        RESET = 0;

        // Run simulation for some time
        #100;
        mem_we = 1'b1;
        mem_addr = 12'h040;
        mem_din = 32'd0;

        // hold the write strobe for one clock cycle
        #10;
        mem_we = 1'b0;

        // Run to let clocks sync
        #100;
        $display("LEDs. mcu: %d, fpga: %d", MCU_ACT, FPGA_ACT);
        `ASSERT_EQ(FPGA_ACT, 1'b0, "0b%1b", "FPGA_ACT mismatch");
        `ASSERT_EQ(MCU_ACT, 1'b0, "0b%1b", "MCU_ACT mismatch");

        mem_we = 1'b1;
        mem_din = {24'd0, 8'b0000_0001};

        // hold the write strobe for one clock cycle
        #10;
        mem_we = 1'b0;

        // Run to let clocks sync
        #100;
        $display("LEDs. mcu: %d, fpga: %d", MCU_ACT, FPGA_ACT);
        `ASSERT_EQ(FPGA_ACT, 1'b1, "0b%1b", "FPGA_ACT mismatch");
        `ASSERT_EQ(MCU_ACT, 1'b0, "0b%1b", "MCU_ACT mismatch");

        mem_we = 1'b1;
        mem_din = {24'd0, 8'b0000_0010};

        // hold the write strobe for one clock cycle
        #10;
        mem_we = 1'b0;

        // Run to let clocks sync
        #100;
        $display("LEDs. mcu: %d, fpga: %d", MCU_ACT, FPGA_ACT);
        `ASSERT_EQ(FPGA_ACT, 1'b0, "0b%1b", "FPGA_ACT mismatch");
        `ASSERT_EQ(MCU_ACT, 1'b1, "0b%1b", "MCU_ACT mismatch");

        mem_we = 1'b1;
        mem_din = {24'd0, 8'b0000_0011};

        // hold the write strobe for one clock cycle
        #10;
        mem_we = 1'b0;

        // Run to let clocks sync
        #100;
        $display("LEDs. mcu: %d, fpga: %d", MCU_ACT, FPGA_ACT);
        `ASSERT_EQ(MCU_ACT, 1'b1, "0b%1b", "MCU_ACT mismatch");
        `ASSERT_EQ(FPGA_ACT, 1'b1, "0b%1b", "FPGA_ACT mismatch");

        report();
        $finish;
    end

endmodule