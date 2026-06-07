`timescale 1ns/1ps

`include "src/test/assertions.svh"

module int_leds_mem_tb;

    // Testbench signals
    reg RESET;
    reg TCXO = 0;
    wire FPGA_ACT;
    wire MCU_ACT;

    reg [7:0] led_ctrl;
    reg strobe_led_update;


    // Local Testbench Signals to mimic internal FPGA modules
    reg [7:0]  mock_reg_io_in_1;
    reg [31:0] mock_enc_1;
    reg [31:0] mock_enc_2;
    reg [31:0] mock_enc_3;
    reg [31:0] mock_enc_4;
    reg [31:0] mock_enc_5;
    reg [31:0] mock_enc_6;

    // Interconnect Wires between isolated QuadSPI Core and Memory Map Decoder
    reg [11:0] mem_addr;
    reg [7:0]  mem_din;
    reg [7:0]  mem_dout;
    reg        mem_we   = 0;

    wire        strobe_encoder_reset;

    leds leds_int (
        .reset(RESET),
        .sys_clk(TCXO),
        .led_ctrl(led_ctrl),
        .strobe_update(strobe_led_update),
        .mcu_act(MCU_ACT),
        .fpga_act(FPGA_ACT)
    );

    memory memory_int (
        .reset(RESET),
        .clk_a(TCXO),
        .addr_a(mem_addr),
        .we_a(mem_we),
        .din_a(mem_din),
        .dout_a(mem_dout),

        // Route testbench mock variables directly into read multiplexer
        // IO (buttons)
        .reg_io_in_1(mock_reg_io_in_1),

        // Encoders
        .enc_1(mock_enc_1), .enc_2(mock_enc_2), .enc_3(mock_enc_3),
        .enc_4(mock_enc_4), .enc_5(mock_enc_5), .enc_6(mock_enc_6),

        // Catch output strobes directly for evaluation
        .strobe_led_update(strobe_led_update),
        .led_ctrl(led_ctrl),
        .strobe_encoder_reset(strobe_encoder_reset)
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
        mem_addr = 12'h020;
        mem_din = 8'b0000_0000;

        // hold the write strobe for one clock cycle
        #10;
        mem_we = 1'b0;

        // Run to let clocks sync
        #100;
        $display("LEDs. mcu: %d, fpga: %d", MCU_ACT, FPGA_ACT);

        mem_we = 1'b1;
        mem_addr = 12'h020;
        mem_din = 8'b0000_0001;

        // hold the write strobe for one clock cycle
        #10;
        mem_we = 1'b0;

        // Run to let clocks sync
        #100;
        $display("LEDs. mcu: %d, fpga: %d", MCU_ACT, FPGA_ACT);

        mem_we = 1'b1;
        mem_addr = 12'h020;
        mem_din = 8'b0000_0010;

        // hold the write strobe for one clock cycle
        #10;
        mem_we = 1'b0;

        // Run to let clocks sync
        #100;
        $display("LEDs. mcu: %d, fpga: %d", MCU_ACT, FPGA_ACT);

        mem_we = 1'b1;
        mem_addr = 12'h020;
        mem_din = 8'b0000_0011;

        // hold the write strobe for one clock cycle
        #10;
        mem_we = 1'b0;

        // Run to let clocks sync
        #100;
        $display("LEDs. mcu: %d, fpga: %d", MCU_ACT, FPGA_ACT);

        report();
        $finish;
    end

endmodule