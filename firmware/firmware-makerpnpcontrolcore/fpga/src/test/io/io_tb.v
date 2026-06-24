`timescale 1ns/1ps

`include "src/test/assertions.svh"

module io_tb;

    // Testbench signals
    reg RESET;
    reg TCXO = 0;
    reg [1:0] BTN;
    reg [7:0] DIN = 8'd0;
    reg [1:0] IAK;
    reg [1:0] OEC;
    reg [1:0] ADC_MUX;

    reg [5:0]  addr;
    reg [31:0] din;
    reg [31:0] dout;
    reg        we;

    wire [15:0] debug;

    // Instantiate the DUT (DUT = Device Under Test)
    io dut (
        .reset(RESET),
        .sys_clk(TCXO),

        .bus_we(we),
        .bus_addr(addr),
        .bus_din(din),
        .bus_dout(dout),

        .btn(BTN),
        .iak(IAK),
        .din(DIN),
        .oec(OEC),
        .adc_mux(ADC_MUX),

        .debug(debug)
    );

    // Clock generation: 100 MHz simulated clock (10ns period)
    always #5 TCXO = ~TCXO;

    // Simulation control
    initial begin
        $dumpfile("io_tb.vcd");
        $dumpvars(0, io_tb);

        we = 0;

        // simulate pull-ups (active-low)
        BTN[0] = 1;
        BTN[1] = 1;
        IAK[0] = 1;
        IAK[1] = 1;

        // reset pulse
        RESET = 1;
        #20;
        RESET = 0;

        #50;

        addr = 6'h04;
        #10;

        $display("default state");
        $display("iak1: %d, iak2: %d, user_0: %d, user_1: %d", (dout[3:2] & 2'b01) >> 0, (dout[3:2] & 2'b10) >> 1, (dout[1:0] & 2'b01) >> 0, (dout[1:0] & 2'b10) >> 1);
        `ASSERT_EQ(dout, {28'd0, 2'b00, 2'b00}, "0x%08h", "IO_IN_1 not updated");

        $display("simulate button 0 press (active low)");
        BTN[0] = 0;
        #20;

        $display("iak1: %d, iak2: %d, user_0: %d, user_1: %d", (dout[3:2] & 2'b01) >> 0, (dout[3:2] & 2'b10) >> 1, (dout[1:0] & 2'b01) >> 0, (dout[1:0] & 2'b10) >> 1);
        `ASSERT_EQ(dout, {28'd0, 2'b00, 2'b01}, "0x%08h", "IO_IN_1 not updated");

        $display("simulate button 0 release (active low)");
        BTN[0] = 1;
        #20;

        $display("iak1: %d, iak2: %d, user_0: %d, user_1: %d", (dout[3:2] & 2'b01) >> 0, (dout[3:2] & 2'b10) >> 1, (dout[1:0] & 2'b01) >> 0, (dout[1:0] & 2'b10) >> 1);
        `ASSERT_EQ(dout, {28'd0, 2'b00, 2'b00}, "0x%08h", "IO_IN_1 not updated");

        $display("simulate button 1 press (active low)");
        BTN[1] = 0;
        #20;

        $display("iak1: %d, iak2: %d, user_0: %d, user_1: %d", (dout[3:2] & 2'b01) >> 0, (dout[3:2] & 2'b10) >> 1, (dout[1:0] & 2'b01) >> 0, (dout[1:0] & 2'b10) >> 1);
        `ASSERT_EQ(dout, {28'd0, 2'b00, 2'b10}, "0x%08h", "IO_IN_1 not updated");

        $display("simulate button 1 release (active low)");
        BTN[1] = 1;
        #20;

        $display("iak1: %d, iak2: %d, user_0: %d, user_1: %d", (dout[3:2] & 2'b01) >> 0, (dout[3:2] & 2'b10) >> 1, (dout[1:0] & 2'b01) >> 0, (dout[1:0] & 2'b10) >> 1);
        `ASSERT_EQ(dout, {28'd0, 2'b00, 2'b00}, "0x%08h", "IO_IN_1 not updated");

        $display("simulate iak1 active (active low)");
        IAK[0] = 0;
        #20;

        $display("iak1: %d, iak2: %d, user_0: %d, user_1: %d", (dout[3:2] & 2'b01) >> 0, (dout[3:2] & 2'b10) >> 1, (dout[1:0] & 2'b01) >> 0, (dout[1:0] & 2'b10) >> 1);
        `ASSERT_EQ(dout, {28'd0, 2'b01, 2'b00}, "0x%08h", "IO_IN_1 not updated");

        $display("simulate iak1 inactive (active low)");
        IAK[0] = 1;
        #20;

        $display("iak1: %d, iak2: %d, user_0: %d, user_1: %d", (dout[3:2] & 2'b01) >> 0, (dout[3:2] & 2'b10) >> 1, (dout[1:0] & 2'b01) >> 0, (dout[1:0] & 2'b10) >> 1);
        `ASSERT_EQ(dout, {28'd0, 2'b00, 2'b00}, "0x%08h", "IO_IN_1 not updated");

        $display("simulate iak2 active (active low)");
        IAK[1] = 0;
        #20;

        $display("iak1: %d, iak2: %d, user_0: %d, user_1: %d", (dout[3:2] & 2'b01) >> 0, (dout[3:2] & 2'b10) >> 1, (dout[1:0] & 2'b01) >> 0, (dout[1:0] & 2'b10) >> 1);
        `ASSERT_EQ(dout, {28'd0, 2'b10, 2'b00}, "0x%08h", "IO_IN_1 not updated");

        $display("simulate iak2 inactive (active low)");
        IAK[1] = 1;
        #20;

        $display("iak1: %d, iak2: %d, user_0: %d, user_1: %d", (dout[3:2] & 2'b01) >> 0, (dout[3:2] & 2'b10) >> 1, (dout[1:0] & 2'b01) >> 0, (dout[1:0] & 2'b10) >> 1);
        `ASSERT_EQ(dout, {28'd0, 2'b00, 2'b00}, "0x%08h", "IO_IN_1 not updated");


        $display("change outputs - pattern 1");
        we = 1;
        addr = 5'h10;
        din = 32'h0000_0201;
        #10;
        we = 0;

        #20;

        `ASSERT_EQ(OEC, 2'b01, "0x%02b", "OEC mismatch");
        `ASSERT_EQ(dout, 32'h0000_0201, "0x%08h", "dout mismatch");


        $display("change outputs - pattern 2");
        we = 1;
        addr = 5'h10;
        din = 32'h0000_0102;
        #10;
        we = 0;

        #20;

        `ASSERT_EQ(OEC, 2'b10, "0x%02b", "OEC mismatch");
        `ASSERT_EQ(dout, 32'h0000_0102, "0x%08h", "dout mismatch");

        report();
        $finish;
    end

endmodule