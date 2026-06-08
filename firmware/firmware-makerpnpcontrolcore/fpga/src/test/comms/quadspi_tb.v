// NOTE: addresses are incremented after reads/writes, tests assert the NEXT address
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

    // Interconnect Wires between isolated QuadSPI Core and Memory Map Decoder
    wire [15:0] mem_addr;
    wire [31:0] mem_din;
    wire [31:0] mem_dout;
    wire        mem_we;

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
    reg [31:0] read_word;
    integer i;

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
        // TEST 1: Sequential Read
        // -------------------------------------------------------------
        $display("--- Test 1: Sequential Read ---");
        cs_n  = 0;
        io_en = 1;
        send_command_byte(8'h10);
        send_address_word(16'h0000);
        dummy_phase();

        read_long_word_data(read_word);
        `ASSERT_EQ(mem_addr, 16'h0004, "0x%08h", "address mismatch");
        read_long_word_data(read_word);
        `ASSERT_EQ(mem_addr, 16'h0008, "0x%08h", "address mismatch");

        cs_n = 1;

        #100;

        // -------------------------------------------------------------
        // TEST 2: Sequential Write
        // -------------------------------------------------------------
        $display("--- Test 3: Sequential Write ---");

        cs_n  = 0;
        io_en = 1;
        send_command_byte(8'h90);
        send_address_word(16'h1234);
        send_long_word(32'ha1b2_c3d4);

        `ASSERT_EQ(mem_addr, 16'h1234, "0x%04h", "address mismatch");
        `ASSERT_EQ(mem_din, 32'ha1b2_c3d4, "0x%08h", "data mismatch");

        send_long_word(32'he5f6_0718);

        `ASSERT_EQ(mem_addr, 16'h1238, "0x%04h", "address mismatch");
        `ASSERT_EQ(mem_din, 32'he5f6_0718, "0x%08h", "data mismatch");

        cs_n = 1;

        // Minimum sys_clk domain cycles to flush out the write
        repeat (3) @(posedge TCXO);

        `ASSERT_EQ(mem_addr, 16'h123C, "0x%04h", "address mismatch");

        #100;

        // -------------------------------------------------------------
        // TEST 3: Wrap around and register map boundary
        // -------------------------------------------------------------
        $display("--- Test 3: Wrap around and register map boundary ---");
        begin
            reg [15:0] address = 16'h01f8;
            reg [15:0] expected_addresses[4] = '{
                16'h01fc,
                16'h0000,
                16'h0004,
                16'h0008
            };

            cs_n  = 0;
            io_en = 1;
            send_command_byte(8'h10);
            send_address_word(address);
            dummy_phase();

            for (i = 0; i < 4; i = i + 1) begin
                read_long_word_data(read_word);

                $display("Address: 0x%3h", address);

                `ASSERT_EQ(mem_addr, expected_addresses[i], "0x%04h", "address mismatch");

                address = address + 16'd4;
            end
        end

        cs_n = 1;

        #100;
        report();
        $finish;
    end

endmodule
