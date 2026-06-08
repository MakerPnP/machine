`timescale 1ns/1ps

`include "src/test/assertions.svh"

module io_tb;

    // Testbench signals
    reg RESET;
    reg TCXO = 0;
    reg USER_0;
    reg USER_1;

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

        .user_0(USER_0),
        .user_1(USER_1),

        .debug(debug)
    );

    // Clock generation: 100 MHz simulated clock (10ns period)
    always #5 TCXO = ~TCXO;

    // Simulation control
    initial begin
        $dumpfile("io_tb.vcd");
        $dumpvars(0, io_tb);

        // simulate pull-ups
        USER_0 = 1;
        USER_1 = 1;

        // reset pulse
        RESET = 1;
        #20;
        RESET = 0;

        #50;

        addr = 6'h04;
        #10;

        $display("Buttons. user_0: %d, user_1: %d", (dout[1:0] & 2'b01) >> 0, (dout[1:0] & 2'b10) >> 1);
        `ASSERT_EQ(dout, {30'd0, 2'b00}, "0x%08h", "IO_STATUS not updated");

        $display("simulate button 0 press (active low)");
        USER_0 = 0;
        #20;

        $display("Buttons. user_0: %d, user_1: %d", (dout[1:0] & 2'b01) >> 0, (dout[1:0] & 2'b10) >> 1);
        `ASSERT_EQ(dout, {30'd0, 2'b01}, "0x%08h", "IO_STATUS not updated");

        $display("simulate button 0 release (active low)");
        USER_0 = 1;
        #20;

        $display("Buttons. user_0: %d, user_1: %d", (dout[1:0] & 2'b01) >> 0, (dout[1:0] & 2'b10) >> 1);
        `ASSERT_EQ(dout, {30'd0, 2'b00}, "0x%08h", "IO_STATUS not updated");

        $display("simulate button 1 press (active low)");
        USER_1 = 0;
        #20;

        $display("Buttons. user_0: %d, user_1: %d", (dout[1:0] & 2'b01) >> 0, (dout[1:0] & 2'b10) >> 1);
        `ASSERT_EQ(dout, {30'd0, 2'b10}, "0x%08h", "IO_STATUS not updated");

        $display("simulate button 1 release (active low)");
        USER_1 = 1;
        #20;

        $display("Buttons. user_0: %d, user_1: %d", (dout[1:0] & 2'b01) >> 0, (dout[1:0] & 2'b10) >> 1);
        `ASSERT_EQ(dout, {30'd0, 2'b00}, "0x%08h", "IO_STATUS not updated");

        report();
        $finish;
    end

endmodule