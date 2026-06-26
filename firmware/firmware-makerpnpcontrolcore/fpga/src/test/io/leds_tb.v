`timescale 1ns/1ps

`include "src/test/assertions.svh"

module leds_tb;

    // Testbench signals
    reg RESET;
    reg TCXO = 0;

    `include "src/test/bus_io.svh"

    wire FPGA_ACT;
    wire MCU_ACT;

    wire [15:0] debug;

    reg [31:0] result;

    // Instantiate the DUT (DUT = Device Under Test)
    leds dut (
        .reset(RESET),
        .sys_clk(TCXO),

        .bus_stb(stb),
        .bus_we(we),
        .bus_addr(addr),
        .bus_din(din),
        .bus_dout(dout),
        .bus_ack(ack),

        .mcu_act(MCU_ACT),
        .fpga_act(FPGA_ACT),

        .debug(debug)
    );

    // Clock generation: 100 MHz simulated clock (10ns period)
    always #5 TCXO = ~TCXO;

    // Simulation control
    initial begin
        $dumpfile("leds_tb.vcd");
        $dumpvars(0, leds_tb);

        sys_reset();
        bus_init();

        // Run simulation for some time
        #100;

        bus_write(6'h00, {24'd0, 8'b0000_0000});

        $display("LEDs. mcu: %d, fpga: %d", MCU_ACT, FPGA_ACT);
        `ASSERT_EQ(MCU_ACT, 1'b0, "0b%1b", "MCU_ACT not disabled");
        `ASSERT_EQ(FPGA_ACT, 1'b0, "0b%1b", "FPGA_ACT not disabled");

        bus_read(6'h00, result);
        `ASSERT_EQ(result, 32'h0000_0000, "0x%08h", "LED_CTRL not updated");


        bus_write(6'h00, {24'd0, 8'b0000_0011});

        $display("LEDs. mcu: %d, fpga: %d", MCU_ACT, FPGA_ACT);
        `ASSERT_EQ(MCU_ACT, 1'b1, "0b%1b", "MCU_ACT not enabled");
        `ASSERT_EQ(FPGA_ACT, 1'b1, "0b%1b", "FPGA_ACT not enabled");

        bus_read(6'h00, result);
        `ASSERT_EQ(dout, 32'h0000_0003, "0x%08h", "LED_CTRL not updated");

        report();
        $finish;
    end

endmodule