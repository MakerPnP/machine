`timescale 1ns / 1ps

`include "src/test/assertions.svh"

module int_core_top_tb;

    reg TCXO = 0;
    // Simulated clock generation
    always #20 TCXO = ~TCXO;

    //
    // QuadSPI 1
    //
    reg clk = 0;
    reg cs_n = 1;
    wire [3:0] io;

    reg [3:0] io_drive;
    reg io_en = 0;

    assign io = io_en ? io_drive : 4'bz;

    //
    // LED outputs
    //
    reg MCU_ACT;
    reg FPGA_ACT;

    //
    // BUZZER output
    //
    reg BUZZER;

    //
    // WS2812 RGB(W) LED outputs
    //
    reg RGB_PORTS;
    reg RGB_UP_CAM;

    //
    // Digital Inputs
    //

    // active low buttons (inverted)
    reg [1:0] BTN = 2'd1;
    // active high (non-inverted)
    reg [1:0] IAK = 2'd0;
    reg [7:0] DIN = 8'd0;

    //
    // Digital Outputs
    //
    reg [1:0] OEC;

    // encoders
    reg [2:0] ENCODER_A = 3'd0;
    reg [2:0] ENCODER_B = 3'd0;
    reg [2:0] ENCODER_C = 3'd0;
    reg [2:0] ENCODER_X = 3'd0;
    reg [2:0] ENCODER_Y = 3'd0;
    reg [2:0] ENCODER_Z = 3'd0;

    core_top uut (
        .TCXO(TCXO),
        .QUADSPI1_CLK(clk),
        .QUADSPI1_NCS(cs_n),
        .QUADSPI1_IO(io),
        .FPGA_ACT(FPGA_ACT),
        .MCU_ACT(MCU_ACT),
        .BUZZER(BUZZER),
        .BTN(BTN),
        .IAK(IAK),
        .DIN(DIN),
        .OEC(OEC),
        .ENCODER_A(ENCODER_A),
        .ENCODER_B(ENCODER_B),
        .ENCODER_C(ENCODER_C),
        .ENCODER_X(ENCODER_X),
        .ENCODER_Y(ENCODER_Y),
        .ENCODER_Z(ENCODER_Z),
        .RGB_PORTS(RGB_PORTS),
        .RGB_UP_CAM(RGB_UP_CAM)
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

    task read_long_word_data_be;
        output [31:0] r_data;
        begin
            read_byte_data(r_data[31:24]);
            read_byte_data(r_data[23:16]);
            read_byte_data(r_data[15:8]);
            read_byte_data(r_data[7:0]);
        end
    endtask

    task read_long_word_data_le;
        output [31:0] r_data;
        begin
            read_byte_data(r_data[7:0]);
            read_byte_data(r_data[15:8]);
            read_byte_data(r_data[23:16]);
            read_byte_data(r_data[31:24]);
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
    reg [31:0] read_word;
    integer i;

    initial begin
        $dumpfile("int_core_top_tb.vcd");
        $dumpvars(0, int_core_top_tb);

        // MCU will drive these signals high on startup via interal pull-ups.
        cs_n = 1;
        clk = 1;
        // MCU will not drive this signals until a transfer begins.
        io_en = 0;
        io_drive = 4'b0;

        #500;

        // -------------------------------------------------------------
        $display("--- Test 1: Reading IDENT & VERSION Sequentially ---");
        // -------------------------------------------------------------

        cs_n  = 0;
        io_en = 1;
        send_command_byte(8'h10);
        send_address_word(16'h0000);
        dummy_phase();

        read_long_word_data_be(read_word);
        $display("IDENT Reg Data: 0x%08h", read_word);
        `ASSERT_EQ(read_word, 32'hfaceb00b, "0x%08h", "Ident mismatch (BE)");

        read_long_word_data_be(read_word);
        $display("VERSION Reg Data: 0x%08h", read_word);
        `ASSERT_EQ(read_word, 32'h01020304, "0x%08h", "Version mismatch (BE)");
        cs_n = 1;

        #100;

        cs_n  = 0;
        io_en = 1;
        send_command_byte(8'h11);
        send_address_word(16'h0000);
        dummy_phase();

        read_long_word_data_le(read_word);
        $display("IDENT Reg Data: 0x%08h", read_word);
        `ASSERT_EQ(read_word, 32'hfaceb00b, "0x%08h", "Ident mismatch (LE)");

        read_long_word_data_le(read_word);
        $display("VERSION Reg Data: 0x%08h", read_word);
        `ASSERT_EQ(read_word, 32'h01020304, "0x%08h", "Version mismatch (LE)");
        cs_n = 1;

        #100;

        // -------------------------------------------------------------
        $display("--- Test 2: Simulating Pressed Buttons and readback ---");
        // -------------------------------------------------------------

        // Simulate buttons being pressed (inverted)
        BTN[0] = 0;
        BTN[1] = 0;
        // non-inverted IO
        IAK[0] = 1;
        IAK[1] = 1;
        #100;

        cs_n  = 0;
        io_en = 1;
        send_command_byte(8'h10);
        send_address_word(16'h0084);
        dummy_phase();
        read_long_word_data_be(read_word);
        cs_n = 1;

        `ASSERT_EQ(read_word, 32'h0000_000F, "0x%08h", "IO_IN_1 Readout mismatch");

        #100;

        // Simulate buttons being released (inverted)
        BTN[0] = 1;
        BTN[1] = 1;
        // non-inverted IO
        IAK[0] = 0;
        IAK[1] = 0;
        #100;

        cs_n  = 0;
        io_en = 1;
        send_command_byte(8'h10);
        send_address_word(16'h0084);
        dummy_phase();
        read_long_word_data_be(read_word);
        cs_n = 1;

        `ASSERT_EQ(read_word, 32'h0000_0000, "0x%08h", "IO_IN_1 Readout mismatch");

        #100;

        // -------------------------------------------------------------
        $display("--- Test 3: Simulate changing DIN and readback ---");
        // -------------------------------------------------------------

        // Bit pattern 1
        DIN = 8'hA5;
        #100;

        cs_n  = 0;
        io_en = 1;
        send_command_byte(8'h10);
        send_address_word(16'h0088);
        dummy_phase();
        read_long_word_data_be(read_word);
        cs_n = 1;

        `ASSERT_EQ(read_word, 32'h0000_00A5, "0x%08h", "IO_IN_2 Readout mismatch");

        #100;

        // Bit pattern 2
        DIN = 8'h5A;
        #100;

        cs_n  = 0;
        io_en = 1;
        send_command_byte(8'h10);
        send_address_word(16'h0088);
        dummy_phase();
        read_long_word_data_be(read_word);
        cs_n = 1;

        `ASSERT_EQ(read_word, 32'h0000_005A, "0x%08h", "IO_IN_1 Readout mismatch");

        #100;

        // -------------------------------------------------------------
        $display("--- Test 4: Writing DOUT ---");
        // -------------------------------------------------------------

        cs_n  = 0;
        io_en = 1;
        send_command_byte(8'h90);
        send_address_word(16'h0090);
        send_long_word(32'h0000_0003);
        cs_n = 1;

        // Allow the sys_clk domain several cycles to flush out the strobe
        repeat (5) @(posedge TCXO);

        `ASSERT_EQ(OEC, 2'b11, "0b%2b", "OEC mismatch");

        #100;

        // -------------------------------------------------------------
        $display("--- Test 5: Writing LED ---");
        // -------------------------------------------------------------

        cs_n  = 0;
        io_en = 1;
        send_command_byte(8'h90);
        send_address_word(16'h0040);
        send_long_word(32'h0000_0003);
        cs_n = 1;

        // Allow the sys_clk domain several cycles to flush out the strobe
        repeat (5) @(posedge TCXO);

        `ASSERT_EQ(MCU_ACT, 1'b1, "0b%1b", "MCU_ACT mismatch");
        `ASSERT_EQ(FPGA_ACT, 1'b1, "0b%1b", "FPGA_ACT mismatch");

        #100;

        // -------------------------------------------------------------
        $display("--- Test 6: Writing BUZZER_CTRL ---");
        // -------------------------------------------------------------

        cs_n  = 0;
        io_en = 1;
        send_command_byte(8'h90);
        send_address_word(16'h00C0);
        send_long_word(32'h0000_0001);
        cs_n = 1;

        // Allow the sys_clk domain several cycles to flush out the strobe
        repeat (5) @(posedge TCXO);

        `ASSERT_EQ(BUZZER, 1'b1, "0b%1b", "BUZZER mismatch");

        #100;

        // -------------------------------------------------------------
        $display("--- Test 7: Continuous Read of Encoders 1 to 6 (24 Bytes) ---");
        // -------------------------------------------------------------

        // TODO generate encoder signals to increase the encoder counters

        cs_n = 0;
        io_en = 1;
        send_command_byte(8'h10);
        send_address_word(16'h0120);
        dummy_phase();

        for (i = 0; i <= 5; i = i + 1) begin
            read_long_word_data_be(read_word);
            $display("Encoder %0d value: 0x%08h", i + 1, read_word);
            `ASSERT_EQ(read_word, 32'h0, "0x%08h", $sformatf("Encoder %0d mismatch", i));
        end
        cs_n = 1;

        #100;

        // -------------------------------------------------------------
        $display("--- Test 8: Writing 0x01 to CONFIG_1 to Reset Encoders ---");
        // -------------------------------------------------------------

        cs_n  = 0;
        io_en = 1;
        send_command_byte(8'h90);
        send_address_word(16'h0100);
        send_long_word(32'h0000_0001);
        cs_n = 1;

        // Allow the sys_clk domain several cycles to flush out the strobe
        repeat (5) @(posedge TCXO);

        #100;

        // Verify Encoders were reset
        cs_n  = 0;
        io_en = 1;
        send_command_byte(8'h10);
        send_address_word(16'h0120);
        dummy_phase();

        for (i = 0; i <= 5; i = i + 1) begin
            read_long_word_data_be(read_word);
            $display("Encoder %0d value: 0x%08h", i + 1, read_word);
            `ASSERT_EQ(read_word, 32'h0, "0x%08h", $sformatf("Encoder %0d not reset", i));
        end

        cs_n = 1;

        #100;

        // -------------------------------------------------------------
        $display("--- Test 9: Wrap around and register map boundary ---");
        // -------------------------------------------------------------
        begin
            reg [31:0] expected_data [3] = '{
                // data from second to last address.
                32'haa55aa55,
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
                read_long_word_data_be(read_word);

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
