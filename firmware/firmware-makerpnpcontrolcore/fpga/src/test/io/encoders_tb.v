`timescale 1ns/1ps

`include "src/test/assertions.svh"

module encoders_tb;

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
    encoders dut (
        .reset(RESET),
        .sys_clk(TCXO),

        .bus_we(we),
        .bus_addr(addr),
        .bus_din(din),
        .bus_dout(dout),

        // TODO
        //.encoder_hardware_pins(),

        .debug(debug)
    );

    // Clock generation: 100 MHz simulated clock (10ns period)
    always #5 TCXO = ~TCXO;

    // Simulation control
    initial begin
        $dumpfile("encoders_tb.vcd");
        $dumpvars(0, encoders_tb);

        // simulate pull-ups
        USER_0 = 1;
        USER_1 = 1;

        // reset pulse
        RESET = 1;
        #20;
        RESET = 0;

        #50;

        addr = 6'h00;
        #10;
        `ASSERT_EQ(dout, 32'd0, "0x%08h", "ENCODER_CTRL default status");

        // TODO wrap this is a function
        addr = 6'h00;
        din = {24'd0, 8'b0000_0001};
        we = 1'b1;
        #10;
        we = 1'b0;
        #50;

        `ASSERT_EQ(dout[0], 1'b0, "0b%1b", "ENCODER_CTRL reset flag not cleared");

        // read the encoders

        addr = 6'h20;
        #10;
        `ASSERT_EQ(dout, 32'd0, "0x%08h", "ENC_1 invalid");
        addr = 6'h24;
        #10;
        `ASSERT_EQ(dout, 32'd0, "0x%08h", "ENC_2 invalid");
        addr = 6'h28;
        #10;
        `ASSERT_EQ(dout, 32'd0, "0x%08h", "ENC_3 invalid");
        addr = 6'h2C;
        #10;
        `ASSERT_EQ(dout, 32'd0, "0x%08h", "ENC_4 invalid");
        addr = 6'h30;
        #10;
        `ASSERT_EQ(dout, 32'd0, "0x%08h", "ENC_5 invalid");
        addr = 6'h34;
        #10;
        `ASSERT_EQ(dout, 32'd0, "0x%08h", "ENC_6 invalid");
        addr = 6'h38;
        #10;
        `ASSERT_EQ(dout, 32'hffff_ffff, "0x%08h", "Read from invalid address");

        // TODO simulate quadrature a/b/n signals and verify counter
        // TODO debounce inputs

        report();
        $finish;
    end

endmodule