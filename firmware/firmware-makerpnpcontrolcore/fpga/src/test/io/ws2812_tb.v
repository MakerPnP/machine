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

    reg [31:0] expected [0:15];
    integer bit_count;
    reg bitstream [0:2047]; // enough for 85 LEDs max (2048 bits safe)

    integer decoded_leds;
    reg [23:0] led_data [0:255];

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
                expected[idx] = {8'h00, idx[7:0], 8'h10, 8'h20};
                $display("index: %d, value: 0x%08h", idx, expected[idx]);
                write(5'h10, expected[idx]);
            end


        end

        // ============================================================
        // TEST 4:
        // Decode WS2812 waveform is correct
        // ============================================================
        begin : WS2812_PROTOCOL_TEST

            integer i;
            integer high_count;
            integer low_count;
            integer bit_index;

            integer led_index;
            integer b;

            integer cycle_count;
            integer max_cycles_per_bit;

            reg bit_value;

            bit_count   = 0;
            led_index   = 0;

            $display("WAITING FOR WS OUTPUT ACTIVITY...");

            fork
                begin
                    #100000;
                    $error("TIMEOUT: WS2812 never started");
                    report();
                    $finish;
                end

                begin
                    wait (ws_out == 1'b1);
                end
            join_any
            disable fork;

            $display("WS OUTPUT START DETECTED");

            // ============================================================
            // STEP 1: CAPTURE BITS
            // ============================================================
            max_cycles_per_bit = 1500; // 1.5uS at 100Mhz
            while (led_index < 15) begin

                high_count = 0;
                low_count  = 0;
                cycle_count = 0;

                while (ws_out == 1'b1 && cycle_count < max_cycles_per_bit) begin
                    high_count = high_count + 1;
                    cycle_count = cycle_count + 1;
                    #1;
                end

                while (ws_out == 1'b0 && cycle_count < max_cycles_per_bit) begin
                    low_count = low_count + 1;
                    cycle_count = cycle_count + 1;
                    #1;
                end
                $display("bit cycle_count: %d, high_count: %d, low_count: %d", cycle_count, high_count, low_count);

                // classify bit
                // TODO improve this, since it doesn't check the actual timings.
                if (high_count > low_count)
                    bit_value = 1;
                else
                    bit_value = 0;

                bitstream[bit_count] = bit_value;
                bit_count = bit_count + 1;

                // once we have 24 bits → form LED
                if (bit_count % 24 == 0) begin

                    led_index = bit_count / 24 - 1;

                    led_data[led_index] = {
                        bitstream[bit_count-24],
                        bitstream[bit_count-23],
                        bitstream[bit_count-22],
                        bitstream[bit_count-21],
                        bitstream[bit_count-20],
                        bitstream[bit_count-19],
                        bitstream[bit_count-18],
                        bitstream[bit_count-17],
                        bitstream[bit_count-16],
                        bitstream[bit_count-15],
                        bitstream[bit_count-14],
                        bitstream[bit_count-13],
                        bitstream[bit_count-12],
                        bitstream[bit_count-11],
                        bitstream[bit_count-10],
                        bitstream[bit_count-9],
                        bitstream[bit_count-8],
                        bitstream[bit_count-7],
                        bitstream[bit_count-6],
                        bitstream[bit_count-5],
                        bitstream[bit_count-4],
                        bitstream[bit_count-3],
                        bitstream[bit_count-2],
                        bitstream[bit_count-1]
                    };

                    $display("LED %0d decoded: %h",
                             led_index, led_data[led_index]);
                end
            end

            // ============================================================
            // STEP 2: VERIFY AGAINST EXPECTED MODEL
            // ============================================================
            $display("COMPARING DECODED LED DATA...");

            for (i = 0; i < 16; i = i + 1) begin

                // NOTE: adjust ordering if your pack_pixel differs
                `ASSERT_EQ(led_data[i][23:0],
                           expected[i][23:0],
                           "%h",
                           "LED mismatch at index");

            end

            `ASSERT_EQ(ws_out, 0, "%d",
                "WS2812 should be low after last LED");

        end

        // ============================================================
        // TEST 4:
        // Ensure reset pulse is present before next frame
        // ============================================================

        begin : RESET_PULSE_CHECK

            integer low_cycles;
            integer max_cycles;
            reg last_ws;

            low_cycles = 0;
            max_cycles = 10000; // 100us margin at 100MHz

            $display("TEST: WS2812 RESET pulse validation");

            low_cycles = 0;

            // ------------------------------------------------------------
            // Measure low time
            // ------------------------------------------------------------
            while (ws_out == 1'b0) begin
                low_cycles = low_cycles + 1;
                @(posedge TCXO);

                if (low_cycles > max_cycles) begin
                    $error("ERROR: reset pulse exceeds expected window (or stuck low)");
                    report();
                    $finish;
                end
            end

            // ------------------------------------------------------------
            // Convert cycles → time check
            // ------------------------------------------------------------
            $display("RESET LOW duration = %0d cycles", low_cycles);

            `ASSERT_GE(low_cycles, 50, "%d",
                "WS2812 reset pulse too short (<50us @ 100MHz)");

            $display("PASS: WS2812 reset pulse valid");

        end
        // ============================================================
        // END
        // ============================================================
        report();
        $finish;
    end

endmodule
