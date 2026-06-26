`timescale 1ns/1ps

`include "src/test/assertions.svh"

module int_leds_mem_tb;

    // Testbench signals
    reg RESET;
    reg TCXO = 0;

    `include "src/test/bus_io.svh"

    wire FPGA_ACT;
    wire MCU_ACT;

    reg [15:0] mem_addr;
    reg [31:0] mem_din;
    reg [31:0] mem_dout;
    wire       mem_valid;
    reg        mem_en = 0;
    reg        mem_we = 0;

    reg        led_we;
    reg        led_stb;
    reg [5:0]  led_addr;
    reg [31:0] led_din;
    reg [31:0] led_dout;
    reg        led_ack;

    wire [15:0] debug;

    leds dut (
        .reset(RESET),
        .sys_clk(TCXO),

        .bus_stb(led_stb),
        .bus_we(led_we),
        .bus_addr(led_addr),
        .bus_din(led_din),
        .bus_dout(led_dout),
        .bus_ack(led_ack),

        .mcu_act(MCU_ACT),
        .fpga_act(FPGA_ACT),

        .debug(debug)
    );

    memory memory_map_inst (
        .reset(RESET),
        .clk_a(TCXO),
        .en_a(mem_en),
        .we_a(mem_we),
        .addr_a(mem_addr),
        .din_a(mem_din),
        .dout_a(mem_dout),
        .valid_a(mem_valid),

        .led_stb(led_stb),
        .led_we(led_we),
        .led_addr(led_addr),
        .led_din(led_din),
        .led_dout(led_dout),
        .led_ack(led_ack)
    );

    // Clock generation: 100 MHz simulated clock (10ns period)
    always #5 TCXO = ~TCXO;


    task memory_write;
        input [15:0] address;
        input [31:0] data;
        begin
            @(negedge TCXO);
            mem_addr = address;
            mem_din  = data;
            mem_we   = 1'b1;
            mem_en   = 1'b1;

            @(negedge TCXO);
            mem_we   = 1'b0;
            mem_en   = 1'b0;

            // memory.v accepts the request, emits the downstream write strobe,
            // then leds.v consumes its internal strobe and updates outputs.
            repeat (8) @(posedge TCXO);
        end
    endtask

    // Simulation control
    initial begin
        $dumpfile("int_leds_mem_tb.vcd");
        $dumpvars(0, int_leds_mem_tb);

        mem_addr = 16'd0;
        mem_din  = 32'd0;
        mem_en   = 1'b0;
        mem_we   = 1'b0;

        // reset pulse
        RESET = 1;
        repeat (4) @(posedge TCXO);
        RESET = 0;
        repeat (4) @(posedge TCXO);

        memory_write(16'h0040, 32'd0);

        #100;
        $display("LEDs. mcu: %d, fpga: %d", MCU_ACT, FPGA_ACT);
        `ASSERT_EQ(FPGA_ACT, 1'b0, "0b%1b", "FPGA_ACT mismatch");
        `ASSERT_EQ(MCU_ACT, 1'b0, "0b%1b", "MCU_ACT mismatch");

        memory_write(16'h0040, {24'd0, 8'b0000_0001});

        $display("LEDs. mcu: %d, fpga: %d", MCU_ACT, FPGA_ACT);
        `ASSERT_EQ(FPGA_ACT, 1'b1, "0b%1b", "FPGA_ACT mismatch");
        `ASSERT_EQ(MCU_ACT, 1'b0, "0b%1b", "MCU_ACT mismatch");

        memory_write(16'h0040, {24'd0, 8'b0000_0010});

        $display("LEDs. mcu: %d, fpga: %d", MCU_ACT, FPGA_ACT);
        `ASSERT_EQ(FPGA_ACT, 1'b0, "0b%1b", "FPGA_ACT mismatch");
        `ASSERT_EQ(MCU_ACT, 1'b1, "0b%1b", "MCU_ACT mismatch");

        memory_write(16'h0040, {24'd0, 8'b0000_0011});

        $display("LEDs. mcu: %d, fpga: %d", MCU_ACT, FPGA_ACT);
        `ASSERT_EQ(MCU_ACT, 1'b1, "0b%1b", "MCU_ACT mismatch");
        `ASSERT_EQ(FPGA_ACT, 1'b1, "0b%1b", "FPGA_ACT mismatch");

        report();
        $finish;
    end

endmodule