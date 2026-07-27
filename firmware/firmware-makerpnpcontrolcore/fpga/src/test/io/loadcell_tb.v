`timescale 1ns/1ps

`include "src/test/assertions.svh"

module loadcell_tb;

    reg RESET;
    reg TCXO = 0;

    `include "src/test/bus_io.svh"
    `include "src/main/io/loadcell_regs.svh"
    `include "src/main/io/loadcell_shared.svh"

    reg lc_s0;
    reg lc_s1;
    reg lc_pd_clk;
    reg lc_dout;

    reg [31:0] value;
    reg [7:0] status;

    loadcell dut (
        .sys_clk(TCXO),
        .reset(RESET),

        .bus_stb(stb),
        .bus_we(we),
        .bus_addr(addr),
        .bus_din(din),
        .bus_dout(dout),
        .bus_ack(ack),

        .s0(lc_s0),
        .s1(lc_s1),
        .pd_sck(lc_pd_sck),
        .dout(lc_dout)
    );

    always #10 TCXO = ~TCXO; // 50 MHz

    reg [7:0] test_index = 0;

    initial begin
        $dumpfile("loadcell_tb.vcd");
        $dumpvars(0, loadcell_tb);

        sys_reset();
        bus_init();

        // ============================================================
        $display("TEST: SINGLE_CONVERSION");
        test_index += 1;
        // ============================================================
        begin : SINGLE_CONVERSION

            // TODO
            bus_write(REG_LC_CTRL, 32'b0000_0000_0000_0000_0000_0000_0000_0000);

            // TODO assert S0/S1 pins
            // TODO simulate HX717 (set expected value for HX717 simulator, capture clock pulses on pd_sck, set dout) and simulate powerdown, etc.

            // TODO poll for status, with timeout based on ODR choice, clock pulse length and other known timing requirements
            bus_read(REG_LC_STATUS, status);

            bus_read(REG_LC_VALUE, value);

            // TODO assert value is the same as simulated HX717s value (24 bit)
        end

        // ============================================================
        // END
        // ============================================================
        report();
        $finish;
    end

endmodule
