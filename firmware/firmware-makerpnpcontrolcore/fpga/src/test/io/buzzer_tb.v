`timescale 1ns/1ps

`include "src/test/assertions.svh"

module buzzer_tb;

    // Testbench signals
    reg RESET;
    reg TCXO = 0;

    `include "src/test/bus_io.svh"

    wire BUZZER;

    wire [15:0] debug;

    reg [31:0] result;

    // Instantiate the DUT (DUT = Device Under Test)
    buzzer dut (
        .reset(RESET),
        .sys_clk(TCXO),

        .bus_stb(stb),
        .bus_we(we),
        .bus_addr(addr),
        .bus_din(din),
        .bus_dout(dout),
        .bus_ack(ack),

        .buzzer(BUZZER),

        .debug(debug)
    );

    // Clock generation: 100 MHz simulated clock (10ns period)
    always #5 TCXO = ~TCXO;

    // Simulation control
    initial begin
        $dumpfile("buzzer_tb.vcd");
        $dumpvars(0, buzzer_tb);

        sys_reset();
        bus_init();

        bus_write(6'd0, {24'd0, 8'b0000_0000});

        $display("Buzzer. enabled: %d", BUZZER);
        `ASSERT_EQ(BUZZER, 1'b0, "0b%1b", "BUZZER not disabled");

        bus_read(6'd0, result);
        `ASSERT_EQ(result, 32'h0000_0000, "0x%08h", "BUZZER_CTRL not updated");

        bus_write(6'd0, {24'd0, 8'b0000_0001});

        $display("Buzzer. enabled: %d", BUZZER);
        `ASSERT_EQ(BUZZER, 1'b1, "0b%1b", "BUZZER not enabled");

        bus_read(6'd0, result);
        `ASSERT_EQ(result, 32'h0000_0001, "0x%08h", "BUZZER_CTRL not updated");

        report();
        $finish;
    end

endmodule