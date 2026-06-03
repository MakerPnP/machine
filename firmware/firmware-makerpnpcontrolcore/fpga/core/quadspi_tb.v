// quadspi_tb.v
// Isolated Unit Testbench bypassing core_top.v (Strict Verilog-2001)
`timescale 1ns/1ps

module quadspi_tb;

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
    reg [7:0]  mock_reg_io_in_1;
    reg [31:0] mock_enc_1;
    reg [31:0] mock_enc_2;
    reg [31:0] mock_enc_3;
    reg [31:0] mock_enc_4;
    reg [31:0] mock_enc_5;
    reg [31:0] mock_enc_6;

    // Interconnect Wires between isolated QuadSPI Core and Memory Map Decoder
    wire [11:0] mem_addr;
    wire [7:0]  mem_din;
    wire [7:0]  mem_dout;
    wire        mem_we;

    wire        strobe_led_update;
    wire [7:0]  led_out;
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
        .clk_a(TCXO), // Driven straight by the QSPI Bus Master clock
        .addr_a(mem_addr),
        .we_a(mem_we),
        .din_a(mem_din),
        .dout_a(mem_dout),

        // Route testbench mock variables directly into read multiplexer
        // IO (buttons)
        .reg_io_in_1(mock_reg_io_in_1),

        // Encoders
        .enc_1(mock_enc_1), .enc_2(mock_enc_2), .enc_3(mock_enc_3),
        .enc_4(mock_enc_4), .enc_5(mock_enc_5), .enc_6(mock_enc_6),

        // Catch output strobes directly for evaluation
        .strobe_led_update(strobe_led_update),
        .led_out(led_out),
        .strobe_encoder_reset(strobe_encoder_reset)
    );

    // Clock generator helper - Starts from 1, pulls low, then drives high
    task clock_tick;
        begin
            clk = 0; #50;
            clk = 1; #50;
        end
    endtask

    // Drives high nibble, ticks clock, drives low nibble, ticks clock
    task send_command_byte;
        input [7:0] cmd_val;
        begin
            io_drive = cmd_val[7:4]; clock_tick();
            io_drive = cmd_val[3:0]; clock_tick();
        end
    endtask

    task send_address_word;
        input [15:0] addr_val;
        begin
            io_drive = addr_val[15:12]; clock_tick();
            io_drive = addr_val[11:8];  clock_tick();
            io_drive = addr_val[7:4];   clock_tick();
            io_drive = addr_val[3:0];   clock_tick();
        end
    endtask

    task send_byte;
        input [7:0] cmd_val;
        begin
            io_drive = cmd_val[7:4]; clock_tick();
            io_drive = cmd_val[3:0]; clock_tick();
        end
    endtask

    task read_byte_data;
        output [7:0] r_data;
        reg [3:0] nh;
        reg [3:0] nl;
        begin
            // Phase 1: High Nibble
            clk = 0; #50; // Falling edge: Slave stabilizes next data nibble
            // SAMPLE JUST BEFORE RISING EDGE.
            nh = io;
            clk = 1; #50; // Rising edge: Master samples the data

            // Phase 2: Low Nibble
            clk = 0; #50; // Falling edge: Slave stabilizes next data nibble
            // SAMPLE JUST BEFORE RISING EDGE.
            nl = io;
            clk = 1; #50; // Rising edge: Master samples the data

            r_data = {nh, nl};
        end
    endtask

    task dummy_phase;
        integer d;
        begin
            io_en = 0; // Hand over the bus to the slave module
            for (d = 0; d < 8; d = d + 1) begin
                clock_tick();
            end
        end
    endtask

    // Testbench execution variables
    reg [7:0] read_byte;
    reg [7:0] b0, b1, b2, b3;
    integer i;

    // Emulate what the encoders module does do when it handles a reset strobe

    always @(posedge TCXO) begin
        if (strobe_encoder_reset) begin
            $display("Resetting mock encoders");
            mock_enc_1 = 32'd0;
            mock_enc_2 = 32'd0;
            mock_enc_3 = 32'd0;
            mock_enc_4 = 32'd0;
            mock_enc_5 = 32'd0;
            mock_enc_6 = 32'd0;
        end
    end

    initial begin
        $dumpfile("quadspi_tb.vcd");
        $dumpvars(0, quadspi_tb);

        // Pre-configure initial state of mock hardware elements

        // mock io input status
        mock_reg_io_in_1 = 8'h00;

        // mock encoder variable counters
        mock_enc_1 = 32'h11223344;
        mock_enc_2 = 32'h55667788;
        mock_enc_3 = 32'h99AABBCC;
        mock_enc_4 = 32'hDDEEFF00;
        mock_enc_5 = 32'h12345678;
        mock_enc_6 = 32'h87654321;

        // MCU will drive these signals high on startup via interal pull-ups.
        cs_n = 1;
        clk = 1;
        // MCU will not drive this signals until a transfer begins.
        io_en = 0;
        io_drive = 4'b0;

        #500;

        // -------------------------------------------------------------
        // TEST 1: Continuous Read (IDENT + VERSION) - 8 Bytes Total
        // -------------------------------------------------------------
        $display("--- Test 1: Reading IDENT & VERSION Sequentially ---");
        cs_n = 0; io_en = 1;
        send_command_byte(8'h10);
        send_address_word(16'h0000);
        dummy_phase();

        $write("IDENT Reg Data: ");
        for (i = 0; i < 4; i = i + 1) begin
            read_byte_data(read_byte); $write("%h", read_byte);
        end
        $write("\nVERSION Reg Data: ");
        for (i = 0; i < 4; i = i + 1) begin
            read_byte_data(read_byte); $write("%h", read_byte);
        end
        $write("\n");
        cs_n = 1;

        #100;

        // -------------------------------------------------------------
        // TEST 2: Read Button Status Bits (IO_IN_1 Address 0x24)
        // -------------------------------------------------------------
        $display("--- Test 2: Simulating Pressed Buttons inside TB and Reading ---");
        mock_reg_io_in_1 = 8'h03; // Simulate buttons being pressed on fabric side
        #100;

        cs_n = 0; io_en = 1;
        send_command_byte(8'h10);
        send_address_word(16'h0024);
        dummy_phase();
        read_byte_data(read_byte);
        $display("IO_IN_1 Readout: Byte = 0x%h (Expected 0x03)", read_byte);
        cs_n = 1;

        #100;

        // -------------------------------------------------------------
        // TEST 3: Write LED Status Bits (Address 0x20)
        // -------------------------------------------------------------
        $display("--- Test 3: Writing LED State & Evaluating Strobes ---");
        // We drop CS_N manually here to evaluate strobes *before* reset wipes it

        // Track flag to see if we caught the strobe concurrently
        begin : test3_block
            reg led_strobe_caught;
            led_strobe_caught = 1'b0;

            fork
                // Thread A: Send the write data byte
                begin
                    cs_n = 0; io_en = 1;
                    send_command_byte(8'h90);
                    send_address_word(16'h0020);
                    send_byte(8'h01);
                    // Add a safety buffer of system clock cycles while CS_N is still low
                    repeat (5) @(posedge TCXO);
                    cs_n = 1;
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
                $display("ERROR: Strobe LED signal missing");
            end
        end


        $display("Strobe LED Data: 0x%02h (Expected 01)", led_out);

        #100;

        // -------------------------------------------------------------
        // TEST 4: Read LED Status Bits (Address 0x20)
        // -------------------------------------------------------------
        $display("--- Test 4: Reading back the LED State ---");
        #100;

        cs_n = 0; io_en = 1;
        send_command_byte(8'h10);
        send_address_word(16'h0020);
        dummy_phase();
        read_byte_data(read_byte);
        $display("LEDs Readout: Byte = 0x%02h (Expected 0x01)", read_byte);
        cs_n = 1;

        #100;

        // -------------------------------------------------------------
        // TEST 5: Continuous Multi-Byte Read of All 6 Encoders (24 Bytes)
        // -------------------------------------------------------------
        $display("--- Test 5: Continuous Read of Encoders 1 to 6 (24 Bytes) ---");
        cs_n = 0; io_en = 1;
        send_command_byte(8'h10);
        send_address_word(16'h0040);
        dummy_phase();

        for (i = 1; i <= 6; i = i + 1) begin
            read_byte_data(b0); read_byte_data(b1);
            read_byte_data(b2); read_byte_data(b3);
            $display("  Encoder %0d Count Value: 0x%h", i, {b0, b1, b2, b3});
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
                    cs_n = 0; io_en = 1;
                    send_command_byte(8'h90);
                    send_address_word(16'h0010);
                    send_byte(8'h01);

                    // Allow the 50MHz domain several cycles to flush out the strobe while CS_N is held low
                    repeat (5) @(posedge TCXO);
                    cs_n = 1;
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
                $display("ERROR: Strobe Encoder Reset signal missing");
            end
        end

        #100;

        // Re-verify Encoder 1 has cleared out
        cs_n = 0; io_en = 1;
        send_command_byte(8'h10);
        send_address_word(16'h0040);
        dummy_phase();
        read_byte_data(read_byte);
        $display("Encoder 1 post-reset Byte 0 verification: 0x%h (Expected 00)", read_byte);
        cs_n = 1;

        #100;

        $finish;
    end

endmodule