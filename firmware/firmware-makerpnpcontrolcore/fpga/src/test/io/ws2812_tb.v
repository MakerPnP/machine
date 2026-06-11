`timescale 1ns/1ps

`include "src/test/assertions.svh"

module ws2812_tb;

    // ============================================================
    // CLOCK / RESET
    // ============================================================
    reg RESET;
    reg TCXO = 0;

    always #5 TCXO = ~TCXO; // 100 MHz

    // ============================================================
    // BUS SIGNALS
    // ============================================================
    reg  [5:0]  addr;
    reg  [31:0] din;
    wire [31:0] dout;
    reg         we;

    wire ws_out;
    wire [15:0] debug;

    // ============================================================
    // DUT
    // ============================================================
    ws2812 dut (
        .sys_clk(TCXO),
        .reset(RESET),

        .bus_we(we),
        .bus_addr(addr),
        .bus_din(din),
        .bus_dout(dout),

        .ws_out(ws_out)
    );

    // ============================================================
    // BUS WRITE TASK
    // ============================================================
    task write(input [4:0] a, input [31:0] d);
    begin
        addr = a;
        din  = d;
        we   = 1;
        #10;
        we   = 0;
        #10;
    end
    endtask

    // ============================================================
    // BUS READ TASK
    // ============================================================
    task read(input [4:0] a);
    begin
        addr = a;
        we   = 0;
        #10;
    end
    endtask

    // ============================================================
    // TEST SEQUENCE
    // ============================================================
    initial begin
        $dumpfile("ws2812_tb.vcd");
        $dumpvars(0, ws2812_tb);

        // --------------------------------------------------------
        // RESET
        // --------------------------------------------------------
        RESET = 1;
        we = 0;
        addr = 0;
        din = 0;

        #50;
        RESET = 0;

        #50;

        // ============================================================
        // TEST 1:
        // Configure MODE + ENABLE
        // Expect: output enabled BUT no LED transmission yet
        // ============================================================
        begin : CONFIGURE_AND_ENABLE
            $display("TEST 1: CTRL enable + mode set");

            write(5'h00, 32'b0000_0000_0000_0000_0000_0000_0000_0001);
            // mode = RGB (00), enable = 1

            #200;

            `ASSERT_EQ(ws_out, 1'b0, "%d", "WS output should remain idle before data stream");

            $display("PASS: CTRL sets state but no LED output yet");
        end

        // ============================================================
        // TEST 2:
        // Configure NUM_LEDS = 16
        // ============================================================
        begin : SET_NUM_LEDS
            $display("TEST 2: NUM_LEDS = 16");

            write(5'h04, 32'd16);

            #200;

            // still no output activity yet
            `ASSERT_EQ(ws_out, 1'b0, "%d", "WS output still idle before data");

            $display("PASS: NUM_LEDS configured, still no transmission");
        end

        // ============================================================
        // TEST 3:
        // STREAM RGB DATA (16 LEDs total)
        // Each LED = 0xRRGGBB
        // ============================================================
        begin : STREAM_DATA_1
            integer idx;

            $display("TEST 3: Streaming RGB data");


            for (idx = 0; idx < 16; idx = idx + 1) begin
                write(5'h10, {8'h00, idx[7:0], 8'h10, 8'h20});
            end


        end

        // ============================================================
        // TEST 4:
        // Verify WS2812 waveform is active
        // We check that signal is NOT stuck low
        // ============================================================
        begin : WS2812_WAVEFORM_TEST

            integer i;
            integer high_count;
            integer low_count;
            integer bit_index;
            integer total_bits;

            reg last_ws;
            reg bit_value;

            last_ws = 0;
            high_count = 0;
            low_count = 0;
            bit_index = 0;

            total_bits = 24 * 4; // we only expect first few LEDs to validate


            // ------------------------------------------------------------
            // Wait for first rising edge
            // ------------------------------------------------------------
            $display("WAITING FOR WS OUTPUT ACTIVITY...");

            fork
                begin
                    #100000;
                    $display("TIMEOUT: WS2812 never started");
                    $finish;
                end

                begin
                    wait (ws_out !== 1'b0);
                end
            join_any
            disable fork;

            $display("WS OUTPUT START DETECTED");

            // ------------------------------------------------------------
            // Sample bits
            // ------------------------------------------------------------
            for (bit_index = 0; bit_index < total_bits; bit_index = bit_index + 1) begin

                high_count = 0;
                low_count  = 0;

                // wait for bit start (rising edge)
                @(posedge ws_out);

                // measure HIGH duration
                while (ws_out == 1'b1) begin
                    high_count = high_count + 1;
                    #1;
                end

                // measure LOW duration
                while (ws_out == 1'b0) begin
                    low_count = low_count + 1;
                    #1;
                end

                // --------------------------------------------------------
                // CLASSIFY BIT BASED ON DUTY CYCLE
                // --------------------------------------------------------
                if (high_count > low_count) begin
                    bit_value = 1;
                end else begin
                    bit_value = 0;
                end

                $display("BIT %0d: HIGH=%0d LOW=%0d => %b",
                         bit_index, high_count, low_count, bit_value);

                // --------------------------------------------------------
                // TIMING ASSERTIONS (RELATIVE, NOT ABSOLUTE)
                // --------------------------------------------------------

                // sanity: WS2812 bits are never extremely short
                `ASSERT_GT(high_count + low_count, 10,
                    "WS2812 bit too short (timing broken)");

                // sanity: high phase exists
                `ASSERT_GT(high_count, 0,
                    "Missing high pulse in WS2812 bit");
            end

            $display("WS2812 WAVEFORM TEST COMPLETE");

        end
        // ============================================================
        // END
        // ============================================================
        $display("ALL TESTS COMPLETE");
        $finish;
    end

endmodule
