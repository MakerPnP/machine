`timescale 1ns/1ps

`include "src/test/assertions.svh"

module buzzer_tb;

    // Testbench signals
    reg RESET;
    reg TCXO = 0;
    wire BUZZER;

    reg [5:0]  addr;
    reg [31:0] din;
    reg [31:0] dout;
    reg        we;

    wire [15:0] debug;

    // Instantiate the DUT (DUT = Device Under Test)
    buzzer dut (
        .reset(RESET),
        .sys_clk(TCXO),

        .bus_we(we),
        .bus_addr(addr),
        .bus_din(din),
        .bus_dout(dout),

        .buzzer(BUZZER),

        .debug(debug)
    );

    // Clock generation: 100 MHz simulated clock (10ns period)
    always #5 TCXO = ~TCXO;

    // Simulation control
    initial begin
        $dumpfile("buzzer_tb.vcd");
        $dumpvars(0, buzzer_tb);

        // reset pulse
        RESET = 1;
        #20;
        RESET = 0;

        // Run simulation for some time
        #100;

        // TODO wrap this is a function
        addr = 6'd0;
        din = {24'd0, 8'b0000_0000};
        we = 1'b1;
        #10;
        we = 1'b0;
        #10;

        #100;

        $display("Buzzer. enabled: %d", BUZZER);
        `ASSERT_EQ(dout, 32'h0000_0000, "0x%08h", "BUZZER_CTRL not updated");
        `ASSERT_EQ(BUZZER, 1'b0, "0b%1b", "BUZZER not disabled");


        // TODO wrap this is a function
        addr = 6'd0;
        din = {24'd0, 8'b0000_0001};
        we = 1'b1;
        #10;
        we = 1'b0;
        #10;

        #100;

        $display("Buzzer. enabled: %d", BUZZER);
        `ASSERT_EQ(dout, 32'h0000_0001, "0x%08h", "BUZZER_CTRL not updated");
        `ASSERT_EQ(BUZZER, 1'b1, "0b%1b", "BUZZER not enabled");

        report();
        $finish;
    end

endmodule