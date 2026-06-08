// quadspi_tb.v
// Isolated Unit Testbench bypassing core_top.v (Strict Verilog-2001)
`timescale 1ns / 1ps

`include "src/test/assertions.svh"

module quadspi_tb;

    reg RESET;
    reg TCXO = 0;
    // Clock generation: 50 MHz simulated clock (10ns period)
    always #10 TCXO = ~TCXO;

    // Testbench MCU Emulation Wires
    reg clk = 0;
    reg cs_n = 1;
    wire [3:0] io;
    reg [3:0] io_drive;
    reg io_en = 0;

    wire [5:0] encoder_hardware_pins = 0;
    // Bidirectional Bus Tri-state Setup
    assign io = io_en ? io_drive : 4'bz;

    // Local Testbench Signals to mimic internal FPGA modules
    reg  [31:0] mock_reg_io_in_1;
    reg  [31:0] mock_enc             [0:5];

    // Interconnect Wires between isolated QuadSPI Core and Memory Map Decoder
    wire [11:0] mem_addr;
    wire [31:0] mem_din;
    wire [31:0] mem_dout;
    wire        mem_we;

    wire        strobe_led_update;
    wire [31:0] led_ctrl;
    wire        strobe_encoder_reset;

    // Direct instantiation of your isolated modules under test (UUT)
    quadspi qspi_uut (
        .clk_sys(TCXO),
        .sck(clk),
        .cs_n(cs_n),
        .io(io),
        .mem_addr(mem_addr),
        .mem_din(mem_din),
        .mem_dout(mem_dout),
        .mem_we(mem_we)
    );

    memory memory_map_uut (
        .reset (RESET),
        .clk_a (TCXO),
        .addr_a(mem_addr),
        .we_a  (mem_we),
        .din_a (mem_din),
        .dout_a(mem_dout),

        // Route testbench mock variables directly into read multiplexer
        // IO (buttons)
        .reg_io_in_1(mock_reg_io_in_1),

        // Encoders
        .enc_1(mock_enc[0]),
        .enc_2(mock_enc[1]),
        .enc_3(mock_enc[2]),
        .enc_4(mock_enc[3]),
        .enc_5(mock_enc[4]),
        .enc_6(mock_enc[5]),

        // Catch output strobes directly for evaluation
        .strobe_led_update(strobe_led_update),
        .led_ctrl(led_ctrl),
        .strobe_encoder_reset(strobe_encoder_reset)
    );

    // Clock generator helper - Starts from 1, pulls low, then drives high
    task clock_tick;
        begin
            clk = 0;
            #50;
            clk = 1;
            #50;
        end
    endtask

    task send_byte;
        input [7:0] value;
        begin
            io_drive = value[7:4];
            clock_tick();
            io_drive = value[3:0];
            clock_tick();
        end
    endtask

    task send_word;
        input [15:0] value;
        begin
            send_byte(value[15:8]);
            send_byte(value[7:0]);
        end
    endtask

    task send_long_word;
        input [31:0] value;
        begin
            send_byte(value[31:24]);
            send_byte(value[23:16]);
            send_byte(value[15:8]);
            send_byte(value[7:0]);
        end
    endtask

    // Drives high nibble, ticks clock, drives low nibble, ticks clock
    task send_command_byte;
        input [7:0] cmd_val;
        begin
            send_byte(cmd_val);
        end
    endtask

    task send_address_word;
        input [15:0] address;
        begin
            send_byte(address[15:8]);
            send_byte(address[7:0]);
        end
    endtask

    task read_byte_data;
        output [7:0] r_data;
        reg [3:0] nh;
        reg [3:0] nl;
        begin
            // Phase 1: High Nibble
            clk = 0;
            #50;  // Falling edge: Slave stabilizes next data nibble
            // SAMPLE JUST BEFORE RISING EDGE.
            nh  = io;
            clk = 1;
            #50;  // Rising edge: Master samples the data

            // Phase 2: Low Nibble
            clk = 0;
            #50;  // Falling edge: Slave stabilizes next data nibble
            // SAMPLE JUST BEFORE RISING EDGE.
            nl  = io;
            clk = 1;
            #50;  // Rising edge: Master samples the data

            r_data = {nh, nl};
        end
    endtask

    task read_long_word_data;
        output [31:0] r_data;
        begin
            read_byte_data(r_data[31:24]);
            read_byte_data(r_data[23:16]);
            read_byte_data(r_data[15:8]);
            read_byte_data(r_data[7:0]);
        end
    endtask

    task dummy_phase;
        integer d;
        begin
            io_en = 0;  // Hand over the bus to the slave module
            for (d = 0; d < 8; d = d + 1) begin
                clock_tick();
            end
        end
    endtask

    // Testbench execution variables
    reg [7:0] read_byte;
    reg [7:0] b0, b1, b2, b3;
    reg [31:0] read_word;
    integer i;

    // Emulate what the encoders module does do when it handles a reset strobe

    always @(posedge TCXO) begin
        if (strobe_encoder_reset) begin
            $display("Resetting mock encoders");
            mock_enc[0] = 32'd0;
            mock_enc[1] = 32'd0;
            mock_enc[2] = 32'd0;
            mock_enc[3] = 32'd0;
            mock_enc[4] = 32'd0;
            mock_enc[5] = 32'd0;
        end
    end

    initial begin
        $dumpfile("quadspi_tb.vcd");
        $dumpvars(0, quadspi_tb);

        // MCU will drive these signals high on startup via interal pull-ups.
        cs_n = 1;
        clk = 1;
        // MCU will not drive this signals until a transfer begins.
        io_en = 0;
        io_drive = 4'b0;

        // reset pulse
        RESET = 1;
        #20;
        RESET = 0;

        #500;

        // -------------------------------------------------------------
        // TEST 1: Continuous Read (IDENT + VERSION) - 8 Bytes Total
        // -------------------------------------------------------------
        $display("--- Test 1: Reading IDENT & VERSION Sequentially ---");
        cs_n  = 0;
        io_en = 1;
        send_command_byte(8'h10);
        send_address_word(16'h0000);
        dummy_phase();

        read_long_word_data(read_word);
        $display("IDENT Reg Data: 0x%08h", read_word);
        `ASSERT_EQ(read_word, 32'hfaceb00b, "0x%08h", "Ident mismatch");

        read_long_word_data(read_word);
        $display("VERSION Reg Data: 0x%08h", read_word);
        `ASSERT_EQ(read_word, 32'h01020304, "0x%08h", "Version mismatch");
        cs_n = 1;

        #100;

        // -------------------------------------------------------------
        // TEST 2: Read Button Status Bits (IO_IN_1 Address 0x24)
        // -------------------------------------------------------------
        $display("--- Test 2: Simulating Pressed Buttons inside TB and Reading ---");
        // Simulate buttons being pressed on fabric side
        mock_reg_io_in_1 = 32'h03;
        #100;

        cs_n  = 0;
        io_en = 1;
        send_command_byte(8'h10);
        send_address_word(16'h0024);
        dummy_phase();
        read_long_word_data(read_word);
        cs_n = 1;

        `ASSERT_EQ(read_word, 32'h0000_0003, "0x%08h", "IO_IN_1 Readout mismatch");

        #100;

        // -------------------------------------------------------------
        // TEST 3: Write LED Status Bits (Address 0x20)
        // -------------------------------------------------------------
        $display("--- Test 3: Writing LED State & Evaluating Strobes ---");

        begin : test3_block
            reg led_strobe_caught;
            led_strobe_caught = 1'b0;

            fork
                // Thread A: Send the write data byte
                begin
                    cs_n  = 0;
                    io_en = 1;
                    send_command_byte(8'h90);
                    send_address_word(16'h0020);
                    send_long_word(32'h0000_0003);
                    cs_n = 1;

                    // Allow the sys_clk domain several cycles to flush out the strobe
                    repeat (5) @(posedge TCXO);
                end

                // Thread B: Wait for the strobe to fire concurrently
                begin
                    @(posedge strobe_led_update);
                    led_strobe_caught = 1'b1;
                end
            join

            // Step past the evaluation delta plane to display results safely
            if (led_strobe_caught) begin
                $display("Strobe LED signal caught");
            end else begin
                $error("ERROR: Strobe LED signal missing");
            end
            `ASSERT_EQ(led_strobe_caught, 1);
        end


        `ASSERT_EQ(led_ctrl, 8'h03, "0x%02h", "LED_CTRL mismatch");

        #100;

        // -------------------------------------------------------------
        // TEST 4: Read LED Status Bits (Address 0x20)
        // -------------------------------------------------------------
        $display("--- Test 4: Reading back the LED State ---");
        #100;

        cs_n  = 0;
        io_en = 1;
        send_command_byte(8'h10);
        send_address_word(16'h0020);
        dummy_phase();
        read_long_word_data(read_word);
        cs_n = 1;

        `ASSERT_EQ(read_word, led_ctrl, "0x%02h", "LED_CTRL readback mismatch");

        #100;

        // -------------------------------------------------------------
        // TEST 5: Continuous Multi-Byte Read of All 6 Encoders (24 Bytes)
        // -------------------------------------------------------------
        $display("--- Test 5: Continuous Read of Encoders 1 to 6 (24 Bytes) ---");
        // mock encoder variable counters

        // Initialize the array
        mock_enc[0] = 32'h11223344;
        mock_enc[1] = 32'h55667788;
        mock_enc[2] = 32'h99AABBCC;
        mock_enc[3] = 32'hDDEEFF00;
        mock_enc[4] = 32'h12345678;
        mock_enc[5] = 32'h87654321;

        cs_n = 0;
        io_en = 1;
        send_command_byte(8'h10);
        send_address_word(16'h0040);
        dummy_phase();

        for (i = 0; i <= 5; i = i + 1) begin
            read_long_word_data(read_word);
            $display("Encoder %0d value: 0x%08h", i + 1, read_word);
            `ASSERT_EQ(read_word, mock_enc[i], "0x%08h", $sformatf("Encoder %0d mismatch", i));
        end
        cs_n = 1;

        #100;

        // -------------------------------------------------------------
        // TEST 6: Reset All Encoders to 0 via CONFIG_1 (Address 0x10)
        // -------------------------------------------------------------
        $display("--- Test 6: Writing 0x01 to CONFIG_1 to Reset Encoders ---");

        begin : test6_block
            reg reset_strobe_caught;
            reset_strobe_caught = 1'b0;

            fork
                // Thread A: Setup and execute the SPI transaction entirely within the fork
                begin
                    cs_n  = 0;
                    io_en = 1;
                    send_command_byte(8'h90);
                    send_address_word(16'h0010);
                    send_long_word(32'h0000_0001);
                    cs_n = 1;

                    // Allow the sys_clk domain several cycles to flush out the strobe
                    repeat (5) @(posedge TCXO);
                end

                // Thread B: Wait concurrently for the strobe to fire
                begin
                    @(posedge strobe_encoder_reset);
                    reset_strobe_caught = 1'b1;
                end
            join

            // Evaluate findings

            if (reset_strobe_caught) begin
                $display("Strobe Encoder Reset signal detected");
            end else begin
                $error("ERROR: Strobe Encoder Reset signal missing");
            end
            `ASSERT_EQ(reset_strobe_caught, 1);
        end

        #100;

        // Re-verify Encoder 1 has cleared out
        cs_n  = 0;
        io_en = 1;
        send_command_byte(8'h10);
        send_address_word(16'h0040);
        dummy_phase();
        read_byte_data(read_byte);
        cs_n = 1;

        `ASSERT_EQ(read_byte, 8'h00, "0x%02h", "Encoder 1 was not reset");

        #100;

        // -------------------------------------------------------------
        // TEST 7: Wrap around and register map boundary
        // -------------------------------------------------------------
        $display("--- Test 7: Wrap around and register map boundary ---");
        begin
            reg [31:0] expected_data [3] = '{
                // data from second to last address.
                32'hFFFF_FFFF,
                // marker at last address.
                32'hDEAD_C0DE,
                // ident from first address, as address should wrap round to 0 at 0x200
                32'hFACE_B00B
            };
            reg [15:0] address = 16'h01f8;

            cs_n  = 0;
            io_en = 1;
            send_command_byte(8'h10);
            send_address_word(address);
            dummy_phase();


            for (i = 0; i < 3; i = i + 1) begin
                read_long_word_data(read_word);

                $display("Address: 0x%3h, Value:  0x%h", address, read_word);
                `ASSERT_EQ(read_word, expected_data[i], "0x%02h", "Value mismatch");

                address = address + 16'd4;
            end
        end

        cs_n = 1;

        #100;
        report();
        $finish;
    end

endmodule
