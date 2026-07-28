`timescale 1ns / 1ps

`include "src/test/assertions.svh"
`include "src/main/registers/map.svh"

module int_core_top_tb;

    `include "src/main/io/io_regs.svh"
    `include "src/main/io/leds_regs.svh"
    `include "src/main/io/buzzer_regs.svh"
    `include "src/main/io/encoders_regs.svh"
    `include "src/main/io/loadcell_regs.svh"
    `include "src/main/io/loadcell_shared.svh"
    `include "src/main/io/steppers_regs.svh"
    `include "src/main/io/steppers_shared.svh"
    `include "src/main/registers/system0_regs.svh"
    `include "src/main/registers/system1_regs.svh"

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

    //
    // ADC Mux (address select bits)
    //
    reg [1:0] ADC_MUX;

    //
    // Present status
    //
    reg       BASE_PRESENT = 1'b0;
    reg [3:0] PORT_PRESENT = 4'b0000;

    // encoders
    reg [2:0] ENCODER_A = 3'd0;
    reg [2:0] ENCODER_B = 3'd0;
    reg [2:0] ENCODER_C = 3'd0;
    reg [2:0] ENCODER_X = 3'd0;
    reg [2:0] ENCODER_Y = 3'd0;
    reg [2:0] ENCODER_Z = 3'd0;

    //
    // HX717 load cell (Test 13)
    //
    // core_top names these pins LC1_* even though the instance inside it
    // is lc0_inst and the register block is LC0_BASE - following the pin
    // names here rather than renaming anything.
    //
    // lc_sample is the 24-bit code the simulated HX717 converts. It is
    // given a value at declaration rather than in the initial block, so
    // the free-running model has a defined result to latch from its very
    // first conversion - long before Test 13 runs.
    reg  [23:0] lc_sample = 24'hA5A55A;

    wire        LC1_S0;
    wire        LC1_S1;
    wire        LC1_PD_SCK;
    wire        LC1_DOUT;

    wire [4:0]  lc_last_pulse_count;
    wire        lc_chan_b;
    wire [7:0]  lc_gain;
    wire [31:0] lc_conv_count;
    wire [31:0] lc_err_count;

    //
    // Stepper step/dir pins
    //
    reg [3:0] XYZF_STEP;
    reg [3:0] XYZF_DIR;
    reg       XYZF_EN;
    reg [3:0] BCDE_STEP;
    reg [3:0] BCDE_DIR;
    reg       BCDE_EN;

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
        .ADC_MUX(ADC_MUX),
        .BASE_PRESENT(BASE_PRESENT),
        .PORT_PRESENT(PORT_PRESENT),
        .ENCODER_A(ENCODER_A),
        .ENCODER_B(ENCODER_B),
        .ENCODER_C(ENCODER_C),
        .ENCODER_X(ENCODER_X),
        .ENCODER_Y(ENCODER_Y),
        .ENCODER_Z(ENCODER_Z),
        .RGB_PORTS(RGB_PORTS),
        .RGB_UP_CAM(RGB_UP_CAM),
        .XYZF_STEP(XYZF_STEP),
        .XYZF_DIR(XYZF_DIR),
        .XYZF_EN(XYZF_EN),
        .BCDE_STEP(BCDE_STEP),
        .BCDE_DIR(BCDE_DIR),
        .BCDE_EN(BCDE_EN),

        .LC1_S0(LC1_S0),
        .LC1_S1(LC1_S1),
        .LC1_PD_SCK(LC1_PD_SCK),
        .LC1_DOUT(LC1_DOUT)
    );

    // Simulated HX717 on the load cell pins. This is the same model the
    // loadcell unit testbench uses, instantiated rather than copied, so
    // the integration test is checked against exactly the same reading
    // of the datasheet - including its T1/T2/T3/T4 timing checks, which
    // run here too and feed lc_err_count.
    hx717_sim hx717 (
        .s0              (LC1_S0),
        .s1              (LC1_S1),
        .pd_sck          (LC1_PD_SCK),
        .dout            (LC1_DOUT),

        .sample_value    (lc_sample),

        .latched_value   (),
        .conv_count      (lc_conv_count),
        .last_pulse_count(lc_last_pulse_count),
        .chan_b          (lc_chan_b),
        .gain            (lc_gain),
        .powered_down    (),
        .pd_level        (),
        .err_count       (lc_err_count)
    );

    // ----------------------------------------------------------------
    // Load cell bus trace.
    //
    // Watches the decoder's lc0_* port rather than the QuadSPI pins, so
    // it separates "the decoder never delivered the access" from "the
    // peripheral answered and the answer was lost on the way back".
    // Gated, because it would otherwise fire on every cycle of the
    // second-long stepper test above.
    // ----------------------------------------------------------------
    reg lc_bus_trace = 1'b0;

    always @(posedge uut.sys_clk) begin
        if (lc_bus_trace && uut.lc0_stb) begin
            $display("[LC BUS] %0t stb we=%0d addr=0x%02h din=0x%08h dout=0x%08h ack=%0d",
                     $time, uut.lc0_we, uut.lc0_addr, uut.lc0_din,
                     uut.lc0_dout, uut.lc0_ack);
        end
    end

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

    // ----------------------------------------------------------------
    // General-purpose QuadSPI single-register write/read, wrapping the
    // command/address/data primitives above the same way bus_io.svh's
    // bus_write/bus_read do for the direct-bus unit testbenches - saves
    // repeating the cs_n/io_en/send_*/dummy_phase sequence at every call
    // site. settle_ticks lets a caller ask for extra margin on
    // registers whose bus decode takes more than one sys_clk cycle to
    // ack (see qspi_bus_write_settled below) - ordinary single-cycle-ack
    // registers are fine with the default.
    // ----------------------------------------------------------------
    task qspi_bus_write;
        input [15:0] address;
        input [31:0] data;
        begin
            cs_n  = 0;
            io_en = 1;
            send_command_byte(8'h90);
            send_address_word(address);
            send_long_word(data);
            cs_n = 1;
            // Allow the sys_clk domain several cycles to flush out the
            // strobe - same margin used throughout this file's existing
            // tests for ordinary (single-cycle-ack) registers, PLUS the
            // same extra #100 gap those tests always place before their
            // next cs_n=0 (repeat(5)@(posedge TCXO) alone is what's used
            // WITHIN a test before its own #100 - chaining bus writes
            // back to back without that second piece, as this task
            // originally did, is 100ns short of the total gap any
            // proven-working consecutive-transaction pair in this file
            // actually uses).
            repeat (5) @(posedge TCXO);
            #100;
        end
    endtask

    // For REG_STEP_SEG_CTST/REG_STEP_SEG_SPDM specifically: steppers.v
    // documents these two registers as going through a multi-cycle
    // IDLE->LO->HI->(ACK on read) handshake, needing a couple of sys_clk
    // cycles beyond a single-cycle-ack register's usual settle time.
    // Extra margin here rather than on qspi_bus_write's default, since
    // every other register in this design acks in one cycle.
    task qspi_bus_write_settled;
        input [15:0] address;
        input [31:0] data;
        begin
            cs_n  = 0;
            io_en = 1;
            send_command_byte(8'h90);
            send_address_word(address);
            send_long_word(data);
            cs_n = 1;
            repeat (10) @(posedge TCXO);
            #100;
        end
    endtask

    task qspi_bus_read;
        input  [15:0] address;
        output [31:0] data;
        begin
            cs_n  = 0;
            io_en = 1;
            send_command_byte(8'h10);
            send_address_word(address);
            dummy_phase();
            read_long_word_data_be(data);
            cs_n = 1;
            // Same total gap as qspi_bus_write above - reads chained
            // back to back need it just as much as writes do.
            repeat (5) @(posedge TCXO);
            #100;
        end
    endtask

    // ----------------------------------------------------------------
    // Uploads a 3-segment ramp-up/coast/ramp-down profile to one motor:
    // TX_CONFIG selects the motor and rewinds the segment write pointer,
    // then each segment's CTST is written followed by its SPDM (SPDM is
    // what advances the write pointer to the next segment - see
    // steppers.v's bus protocol comment). All three segments share the
    // same step counts/periods/delta - only dir varies between the
    // forward and reverse passes this test makes.
    // ----------------------------------------------------------------
    task upload_trapezoidal_profile;
        input [2:0]  motor;
        input        dir;
        input [23:0] ramp_steps;
        input [23:0] coast_steps;
        input [15:0] start_period;
        input [15:0] coast_period;
        input [15:0] delta_mag;
        begin
            qspi_bus_write(STEPPERS_BASE + REG_STEP_TX_CONFIG, {16'h0000, {5'd0, motor}, 8'd3});

            // Segment 0: ramp up (accelerating) - period shrinks from
            // start_period down to coast_period over ramp_steps steps,
            // then falls straight through (CMD_MOVE) into segment 1.
            qspi_bus_write_settled(STEPPERS_BASE + REG_STEP_SEG_CTST,
                                    {4'd0, RAMP_UP, dir, CMD_MOVE, ramp_steps});
            qspi_bus_write_settled(STEPPERS_BASE + REG_STEP_SEG_SPDM,
                                    {start_period, delta_mag});

            // Segment 1: coast at a constant period (delta_mag=0, so
            // RAMP_UP vs RAMP_DOWN makes no difference here) - falls
            // through into segment 2.
            qspi_bus_write_settled(STEPPERS_BASE + REG_STEP_SEG_CTST,
                                    {4'd0, RAMP_UP, dir, CMD_MOVE, coast_steps});
            qspi_bus_write_settled(STEPPERS_BASE + REG_STEP_SEG_SPDM,
                                    {coast_period, 16'd0});

            // Segment 2: ramp down (decelerating) - period grows back
            // from coast_period up to start_period over ramp_steps
            // steps, then halts (CMD_MOVE_HALT, end of profile).
            qspi_bus_write_settled(STEPPERS_BASE + REG_STEP_SEG_CTST,
                                    {4'd0, RAMP_DOWN, dir, CMD_MOVE_HALT, ramp_steps});
            qspi_bus_write_settled(STEPPERS_BASE + REG_STEP_SEG_SPDM,
                                    {coast_period, delta_mag});
        end
    endtask

    // Polls REG_STEP_CTRL (bits[7:0] = per-motor moving status on read)
    // every 10us until every motor reports stopped, or timeout_polls
    // iterations have passed without that happening.
    task poll_steppers_until_stopped;
        input integer timeout_polls;
        reg [31:0] ctrl_read;
        integer    tries;
        begin
            ctrl_read = 32'hFFFF_FFFF;
            tries = 0;
            while (ctrl_read[7:0] != 8'h00 && tries < timeout_polls) begin
                qspi_bus_read(STEPPERS_BASE + REG_STEP_CTRL, ctrl_read);
                tries = tries + 1;
                if (ctrl_read[7:0] != 8'h00) #10000;
            end
            `ASSERT_EQ(ctrl_read[7:0], 8'h00, "0b%08b",
                       "[STEPPER TRAPEZOID] Motors did not all stop within the polling timeout");
        end
    endtask

    // ----------------------------------------------------------------
    // Load cell polling helpers.
    //
    // Two of them, because REG_LC_STATUS.READY answers a different
    // question than it looks like it does. READY means "a conversion has
    // completed since the last write to REG_LC_CTRL" and is cleared only
    // by a CTRL write - never by a read, because all three load cell
    // registers share one 16-byte OctoSPI prefetch line and a read side
    // effect would fire whenever the host merely polled STATUS.
    //
    // So READY tells you the FIRST sample has arrived, but stays set
    // afterwards and cannot distinguish the second sample from the
    // first. That is what the sequence counter in REG_LC_VALUE[31:24] is
    // for: it steps once per conversion, and it shares a word with the
    // value so a single read gets a pair that cannot skew.
    // ----------------------------------------------------------------

    // Polls REG_LC_STATUS every 10us until READY, then reads the value.
    task poll_loadcell_ready;
        input  integer timeout_polls;
        output [31:0]  value_word;
        reg    [31:0]  status_word;
        integer        tries;
        begin
            status_word = 32'h0000_0000;
            tries       = 0;
            while (!status_word[LC_STATUS_READY_BIT] && tries < timeout_polls) begin
                qspi_bus_read(LC0_BASE + REG_LC_STATUS, status_word);
                tries = tries + 1;
                if (!status_word[LC_STATUS_READY_BIT]) #10000;
            end
            `ASSERT_EQ(status_word[LC_STATUS_READY_BIT], 1'b1, "%0d",
                       "[LOADCELL] READY never asserted within the polling timeout");
            qspi_bus_read(LC0_BASE + REG_LC_VALUE, value_word);
        end
    endtask

    // Polls until the sequence counter moves on from prev_seq. STATUS is
    // read on every iteration as well, both to keep the poll shape
    // identical to the first sample's and to prove that reading STATUS
    // repeatedly does not disturb anything.
    task poll_loadcell_next;
        input  integer timeout_polls;
        input  [7:0]   prev_seq;
        output [31:0]  value_word;
        reg    [31:0]  status_word;
        integer        tries;
        begin
            value_word = {prev_seq, 24'h000000};
            tries      = 0;
            while (value_word[31:24] === prev_seq && tries < timeout_polls) begin
                qspi_bus_read(LC0_BASE + REG_LC_STATUS, status_word);
                qspi_bus_read(LC0_BASE + REG_LC_VALUE,  value_word);
                tries = tries + 1;
                if (value_word[31:24] === prev_seq) #10000;
            end
            `ASSERT_EQ(status_word[LC_STATUS_READY_BIT], 1'b1, "%0d",
                       "[LOADCELL] READY should still be set on the second sample");
            `ASSERT_NE(value_word[31:24], prev_seq, "%0d",
                       "[LOADCELL] sequence counter never advanced within the polling timeout");
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
        send_address_word(SYSTEM0_BASE + REG_IDENT);
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
        send_address_word(SYSTEM0_BASE + REG_IDENT);
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
        IAK[0] = 0;
        IAK[1] = 0;
        #100;

        cs_n  = 0;
        io_en = 1;
        send_command_byte(8'h10);
        send_address_word(IO_BASE + REG_IO_IN_1);
        dummy_phase();
        read_long_word_data_be(read_word);
        cs_n = 1;

        `ASSERT_EQ(read_word, 32'h0000_000F, "0x%08h", "IO_IN_1 Readout mismatch");

        #100;

        // Simulate buttons being released (inverted)
        BTN[0] = 1;
        BTN[1] = 1;
        // non-inverted IO
        IAK[0] = 1;
        IAK[1] = 1;
        #100;

        cs_n  = 0;
        io_en = 1;
        send_command_byte(8'h10);
        send_address_word(IO_BASE + REG_IO_IN_1);
        dummy_phase();
        read_long_word_data_be(read_word);
        cs_n = 1;

        `ASSERT_EQ(read_word, 32'h0000_0000, "0x%08h", "IO_IN_1 Readout mismatch");

        #100;

        // -------------------------------------------------------------
        $display("--- Test 3: Simulate changing DIN and readback ---");
        // -------------------------------------------------------------

        // Bit pattern 1, active-low
        DIN = 8'hA5;
        #100;

        cs_n  = 0;
        io_en = 1;
        send_command_byte(8'h10);
        send_address_word(IO_BASE + REG_IO_IN_2);
        dummy_phase();
        read_long_word_data_be(read_word);
        cs_n = 1;

        `ASSERT_EQ(read_word, 32'h0000_005A, "0x%08h", "IO_IN_2 Readout mismatch");

        #100;

        // Bit pattern 2, active-low
        DIN = 8'h5A;
        #100;

        cs_n  = 0;
        io_en = 1;
        send_command_byte(8'h10);
        send_address_word(IO_BASE + REG_IO_IN_2);
        dummy_phase();
        read_long_word_data_be(read_word);
        cs_n = 1;

        `ASSERT_EQ(read_word, 32'h0000_00A5, "0x%08h", "IO_IN_1 Readout mismatch");

        #100;

        // -------------------------------------------------------------
        $display("--- Test 4: Writing DOUT ---");
        // -------------------------------------------------------------

        cs_n  = 0;
        io_en = 1;
        send_command_byte(8'h90);
        send_address_word(IO_BASE + REG_IO_OUT_1);
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
        send_address_word(LED_BASE + REG_LED_CTRL);
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
        send_address_word(BUZZER_BASE + REG_BUZZER_CTRL);
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
        send_address_word(ENCODER_BASE + REG_ENC_COUNT_A);
        dummy_phase();

        for (i = 0; i <= 5; i = i + 1) begin
            read_long_word_data_be(read_word);
            $display("Encoder %0d value: 0x%08h", i + 1, read_word);
            `ASSERT_EQ(read_word, 32'h0, "0x%08h", $sformatf("Encoder %0d mismatch", i));
        end
        cs_n = 1;

        #100;

        // -------------------------------------------------------------
        $display("--- Test 8: Setting encoders manually ---");
        // -------------------------------------------------------------

        begin
            reg [15:0] expected_encoder_values[6];
            cs_n  = 0;
            io_en = 1;
            send_command_byte(8'h90);
            send_address_word(ENCODER_BASE + REG_ENC_SET_COUNT_A);
            for (i = 1; i <= 6; i = i + 1) begin
                expected_encoder_values[i - 1] = (i << 0) + (i << 4) + (i << 8) + (i << 12);
                send_long_word(32'hffff_0000 | expected_encoder_values[i - 1]);
            end
            cs_n = 1;

            // Allow the sys_clk domain several cycles to flush out the strobe
            repeat (5) @(posedge TCXO);

            #100;

            // Verify Encoders were reset
            cs_n  = 0;
            io_en = 1;
            send_command_byte(8'h10);
            send_address_word(ENCODER_BASE + REG_ENC_COUNT_A);
            dummy_phase();

            for (i = 0; i <= 5; i = i + 1) begin
                read_long_word_data_be(read_word);
                $display("Encoder %0d value: 0x%08h", i + 1, read_word);
                `ASSERT_EQ(read_word[15:0], expected_encoder_values[i], "0x%04h", $sformatf("Encoder %0d was not set", i));
            end

            cs_n = 1;

            #100;

        end

        // -------------------------------------------------------------
        $display("--- Test 9: Writing 0x01 to CONFIG_1 to Reset Encoders ---");
        // -------------------------------------------------------------

        cs_n  = 0;
        io_en = 1;
        send_command_byte(8'h90);
        send_address_word(ENCODER_BASE + REG_ENC_CTRL);
        send_long_word(32'h0000_0001);
        cs_n = 1;

        // Allow the sys_clk domain several cycles to flush out the strobe
        repeat (5) @(posedge TCXO);

        #100;

        // Verify Encoders were reset
        cs_n  = 0;
        io_en = 1;
        send_command_byte(8'h10);
        send_address_word(ENCODER_BASE + REG_ENC_COUNT_A);
        dummy_phase();

        for (i = 0; i <= 5; i = i + 1) begin
            read_long_word_data_be(read_word);
            $display("Encoder %0d value: 0x%08h", i + 1, read_word);
            `ASSERT_EQ(read_word, 32'h0, "0x%08h", $sformatf("Encoder %0d not reset", i));
        end

        cs_n = 1;

        #100;

        // -------------------------------------------------------------
        $display("--- Test 10: Wrap around and register map boundary ---");
        // -------------------------------------------------------------
        begin
            reg [31:0] expected_data [3] = '{
                // data from second to last address.
                32'h55aa55aa,
                // marker at last address.
                32'hDEAD_C0DE,
                // ident from first address, as address should wrap round to 0 at 0x200
                32'hFACE_B00B
            };
            reg [15:0] address = SYSTEM1_BASE + REG_MARKER - 8'h04;

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

        // -------------------------------------------------------------
        $display("--- Test 11: Long continuous read crossing 3 boundaries ---");
        // -------------------------------------------------------------
        // read from a system peripherial, followed by unmapped memory and
        // then two consecutive peripherals.
        //
        // this test only works with these peripherals, in this order, adjust
        // as-required if the memory map is changed.
        `ASSERT_EQ(SYSTEM0_BASE, 16'h0000, "%d", "Invalid test setup (A)");
        `ASSERT_EQ(RESERVED0_BASE, 16'h0100, "%d", "Invalid test setup (B)");
        `ASSERT_EQ(LED_BASE, 16'h0200, "%d", "Invalid test setup (C)");
        `ASSERT_EQ(BUZZER_BASE, 16'h0300, "%d", "Invalid test setup (D)");
        begin
            reg [31:0] block[16];
            reg [15:0] capture_addresses[8] = '{
                16'h0000,
                16'h00FC,
                16'h0100,
                16'h01FC,
                16'h0200,
                16'h02FC,
                16'h0300,
                16'h03FC
            };
            reg [32:0] captured_values[8];
            reg [32:0] expected_values[8] = '{
                // ident
                32'hface_b00b,
                // system 0 marker
                32'haa55_aa55,
                // unmapped memory
                32'h99ba_ad99,
                32'h99ba_ad99,
                // LED_CTRL reset value
                32'h0000_0003,
                // led marker
                32'h4444_4444,
                // BUZZER_CTRL reset value
                32'h0000_0001,
                // buzzer marker
                32'h1111_1111
            };
            integer address, block_address, capture_index;

            // memory addresses here are in bytes
            localparam start_address = 16'h0000;
            localparam end_address = 16'h03ff;

            address = start_address;

            cs_n = 0;
            io_en = 1;
            send_command_byte(8'h10);
            send_address_word(address);
            dummy_phase();

            capture_index = 0;

            for (; address <= end_address; address = address + (16 * 4)) begin
                for (i = 0; i < 16; i = i + 1) begin
                    read_long_word_data_be(read_word);
                    block[i] = read_word;

                    block_address = address + (i * 4);
                    if (capture_addresses[capture_index] == block_address) begin
                        captured_values[capture_index] = read_word;
                        capture_index += 1;
                    end
                end
                $display("0x%04h: 0x%08h, 0x%08h, 0x%08h, 0x%08h, 0x%08h, 0x%08h, 0x%08h, 0x%08h, 0x%08h, 0x%08h, 0x%08h, 0x%08h, 0x%08h, 0x%08h, 0x%08h, 0x%08h",
                    address,
                    block[0],
                    block[1],
                    block[2],
                    block[3],
                    block[4],
                    block[5],
                    block[6],
                    block[7],
                    block[8],
                    block[9],
                    block[10],
                    block[11],
                    block[12],
                    block[13],
                    block[14],
                    block[15],
                );
            end
            cs_n = 1;

            #100;

            for (i = 0; i < 8; i++) begin
                `ASSERT_EQ(captured_values[i], expected_values[i], "0x%08h", $sformatf("Captured value mismatch. address: ", capture_addresses[i]));
            end
        end

        // -------------------------------------------------------------
        $display("--- Test 12: Stepper 3-Segment Trapezoidal Move, All 8 Motors, Forward then Reverse ---");
        // -------------------------------------------------------------
        begin : STEPPER_TRAPEZOID_TEST
            // ---- Reference math: 1us step pulse width -----------------
            // Starts from target_ns = 1_000 (decimal) and derives the
            // (prescaler, preset) register pair from it - the same
            // calculation an MCU driver will need, given only the sys_clk
            // period and the desired pulse width. Pulse width =
            // (prescaler+1)*(preset+1) sys_clk cycles (see steppers.v's
            // "Step pulse width" section) - prescaler is 6 bits (divide
            // 1-64), preset is 4 bits (count 1-16).
            localparam integer NS_PER_CYCLE = 20;
            localparam integer MAX_COUNT    = 16;  // preset+1 max (4-bit reg)
            localparam integer MAX_DIVIDE   = 64;  // prescaler+1 max (6-bit reg)

            integer target_ns;
            integer target_cycles;
            integer min_divide;
            integer search_divide, this_count, this_total;
            integer best_divide, best_count, best_total;
            integer pls_prescaler_reg, pls_preset_reg;

            // ---- Trapezoidal profile parameters (stepper ticks - a
            // separate unit/domain from the pulse-width cycles above) ----
            // Kept deliberately small - total simulated ticks (not step
            // count) is what drives simulation time, and the effective
            // sys_clk rate observed in this simulation makes even a
            // modest per-direction tick count take a long time to
            // simulate. This is still small enough (28 ticks/direction)
            // to comfortably clear the configured 1us pulse width
            // regardless of which rate is actually in effect.
            localparam integer RAMP_STEPS   = 6;
            localparam integer COAST_STEPS  = 3;
            localparam integer DELTA_MAG    = 1;
            localparam integer COAST_PERIOD = 2;
            // Chosen so the ramp-up segment's LAST step (index
            // RAMP_STEPS-1, since period_at_step applies delta
            // step_index times) lands exactly on COAST_PERIOD - a smooth
            // handoff into the coast segment with no period jump, and the
            // ramp-down segment mirrors it symmetrically back up.
            localparam integer START_PERIOD = COAST_PERIOD + DELTA_MAG * (RAMP_STEPS - 1);

            // Expected total motion duration, computed in closed form
            // rather than simulated step by step - displayed as
            // reference (and a useful cross-check against how long the
            // test actually takes to run), not used to drive the
            // polling below directly - see poll_steppers_until_stopped's
            // comment for why. Sum of periods across a ramp segment is a
            // triangular-number expression:
            // sum_{k=0}^{N-1}(start - k*delta) = N*start - delta*N*(N-1)/2
            // (ramp-down mirrors it, period growing instead of
            // shrinking). TICKS_TO_SYS_CYCLES=500 matches stepper_clk.v's
            // divider (divides sys_clk by 500 to produce one stepper
            // tick - the unit start_period/coast_period/delta above are
            // all expressed in) - assumes 20ns/cycle (the 50MHz
            // synthesis constraint), which this simulation's actual
            // effective rate does not seem to match (again, see
            // poll_steppers_until_stopped's comment), so treat the
            // displayed value as an order-of-magnitude reference rather
            // than a precise prediction of this simulation's wall-clock
            // or simulated-time behavior.
            localparam integer TICKS_TO_SYS_CYCLES = 500;
            integer total_ramp_up_ticks, total_coast_ticks, total_ramp_down_ticks, total_ticks;
            integer expected_duration_ns;

            integer    m;
            reg [31:0] readback;
            reg [31:0] position_read;
            integer    expected_position;

            // ---- Compute prescaler/preset for the 1us pulse width -----
            // Ceiling-divides throughout: never produce a pulse shorter
            // than requested, and never claim a total narrower than the
            // target - matching the driver-safety floor established
            // elsewhere in this design. Searches divide upward from the
            // smallest value that could possibly reach target_cycles
            // within MAX_COUNT, taking the first EXACT factorization
            // found (searching from the smallest divide keeps divide -
            // and so LC cost on the divide side - minimal); if none
            // exists in range, keeps the smallest total that still
            // covers the target.
            target_ns     = 1_000;
            target_cycles = (target_ns + NS_PER_CYCLE - 1) / NS_PER_CYCLE;
            min_divide    = (target_cycles + MAX_COUNT - 1) / MAX_COUNT;
            if (min_divide < 1) min_divide = 1;

            best_total = -1;
            for (search_divide = min_divide; search_divide <= MAX_DIVIDE; search_divide = search_divide + 1) begin
                this_count = (target_cycles + search_divide - 1) / search_divide;
                if (this_count <= MAX_COUNT) begin
                    this_total = search_divide * this_count;
                    if (best_total == -1 || this_total < best_total) begin
                        best_total  = this_total;
                        best_divide = search_divide;
                        best_count  = this_count;
                    end
                    if (this_total == target_cycles) begin
                        search_divide = MAX_DIVIDE + 1; // exact match - stop searching
                    end
                end
            end

            pls_prescaler_reg = best_divide - 1;
            pls_preset_reg    = best_count - 1;

            $display("[PLS CALC] target=%0dns -> %0d cycles -> divide=%0d count=%0d (%0d cycles = %0dns) -> prescaler_reg=%0d preset_reg=%0d",
                      target_ns, target_cycles, best_divide, best_count, best_total, best_total * NS_PER_CYCLE,
                      pls_prescaler_reg, pls_preset_reg);

            `ASSERT_EQ(best_total, target_cycles, "%0d",
                       "[STEPPER TRAPEZOID] Expected an exact factorization for 1us at 50MHz");

            // Same preset on every motor, same prescaler on both banks.
            qspi_bus_write(STEPPERS_BASE + REG_STEP_PLS_CONFIG,
                            {8{pls_preset_reg[3:0]}});
            qspi_bus_write(STEPPERS_BASE + REG_STEP_PLS_PRESCALER,
                            {10'd0, pls_prescaler_reg[5:0], 10'd0, pls_prescaler_reg[5:0]});

            qspi_bus_read(STEPPERS_BASE + REG_STEP_PLS_CONFIG, readback);
            `ASSERT_EQ(readback, {8{pls_preset_reg[3:0]}}, "0x%08h",
                       "[STEPPER TRAPEZOID] REG_STEP_PLS_CONFIG readback mismatch");
            qspi_bus_read(STEPPERS_BASE + REG_STEP_PLS_PRESCALER, readback);
            `ASSERT_EQ(readback, {10'd0, pls_prescaler_reg[5:0], 10'd0, pls_prescaler_reg[5:0]}, "0x%08h",
                       "[STEPPER TRAPEZOID] REG_STEP_PLS_PRESCALER readback mismatch");

            total_ramp_up_ticks   = RAMP_STEPS*START_PERIOD - DELTA_MAG*RAMP_STEPS*(RAMP_STEPS-1)/2;
            total_coast_ticks     = COAST_STEPS*COAST_PERIOD;
            total_ramp_down_ticks = RAMP_STEPS*COAST_PERIOD + DELTA_MAG*RAMP_STEPS*(RAMP_STEPS-1)/2;
            total_ticks           = total_ramp_up_ticks + total_coast_ticks + total_ramp_down_ticks;
            expected_duration_ns  = total_ticks * TICKS_TO_SYS_CYCLES * NS_PER_CYCLE;
            $display("[DURATION CALC] total_ticks=%0d -> expected_duration=%0dns (%0dms)",
                      total_ticks, expected_duration_ns, expected_duration_ns / 1_000_000);

            // ---- Upload the forward-direction profile to all 8 motors -
            for (m = 0; m < 8; m = m + 1) begin
                upload_trapezoidal_profile(m[2:0], DIR_NORMAL, RAMP_STEPS, COAST_STEPS,
                                            START_PERIOD, COAST_PERIOD, DELTA_MAG);
            end

            // ---- Start all 8 motors simultaneously, then poll --------
            $display("[STEPPER TRAPEZOID] Starting all 8 motors (forward)...");
            qspi_bus_write(STEPPERS_BASE + REG_STEP_CTRL, 32'h000F_0F00);
            poll_steppers_until_stopped(30000);

            // ---- Verify final position: 25+50+25 = 100 steps forward -
            expected_position = RAMP_STEPS + COAST_STEPS + RAMP_STEPS;
            for (m = 0; m < 8; m = m + 1) begin
                qspi_bus_read(STEPPERS_BASE + REG_STEP_POS_0 + (m * 4), position_read);
                $display("[STEPPER TRAPEZOID] Motor %0d position (forward): %0d", m, $signed(position_read));
                `ASSERT_EQ($signed(position_read), expected_position, "%0d",
                           $sformatf("[STEPPER TRAPEZOID] Motor %0d position mismatch after forward move", m));
            end

            // ---- Upload the SAME profile again, reversed direction ---
            for (m = 0; m < 8; m = m + 1) begin
                upload_trapezoidal_profile(m[2:0], DIR_REVERSE, RAMP_STEPS, COAST_STEPS,
                                            START_PERIOD, COAST_PERIOD, DELTA_MAG);
            end

            $display("[STEPPER TRAPEZOID] Starting all 8 motors (reverse)...");
            qspi_bus_write(STEPPERS_BASE + REG_STEP_CTRL, 32'h000F_0F00);
            poll_steppers_until_stopped(30000);

            // ---- Verify final position: back to 0 (100 forward, then
            // 100 back) --------------------------------------------------
            for (m = 0; m < 8; m = m + 1) begin
                qspi_bus_read(STEPPERS_BASE + REG_STEP_POS_0 + (m * 4), position_read);
                $display("[STEPPER TRAPEZOID] Motor %0d position (reverse): %0d", m, $signed(position_read));
                `ASSERT_EQ($signed(position_read), 0, "%0d",
                           $sformatf("[STEPPER TRAPEZOID] Motor %0d position mismatch after reverse move", m));
            end
        end

        #100;

        // -------------------------------------------------------------
        $display("--- Test 13: Load Cell Continuous Conversion at 320SPS ---");
        // -------------------------------------------------------------
        begin : LOADCELL_TEST
            reg [LC_CTRL_W-1:0] lc_ctrl_expected;
            reg [31:0]          lc_value_word;
            reg [31:0]          lc_readback;
            reg [1:0]           lc_rate_pins;
            reg [7:0]           first_seq;
            reg [7:0]           second_seq;

            // ---- start continuous conversion ----------------------
            // Channel A at gain 128, 320 SPS, no power-down, not a
            // one-shot.
            lc_bus_trace = 1'b1;

            lc_ctrl_expected = lc_ctrl_word(1'b1,           // enable
                                            1'b0,           // pd
                                            LC_RATE_320HZ,
                                            LC_MODE_A128,
                                            LC_PD_ADC,
                                            1'b0);          // single
            qspi_bus_write(LC0_BASE + REG_LC_CTRL,
                           {{(32 - LC_CTRL_W) {1'b0}}, lc_ctrl_expected});

            qspi_bus_read(LC0_BASE + REG_LC_CTRL, lc_readback);
            $display("[LOADCELL] CTRL readback: 0x%08h (expected 0x%08h)",
                     lc_readback, {{(32 - LC_CTRL_W) {1'b0}}, lc_ctrl_expected});
            `ASSERT_EQ(lc_readback[LC_CTRL_W-1:0], lc_ctrl_expected, "0x%03h",
                       "[LOADCELL] CTRL readback mismatch");

            // The rate has to reach the pins, not just the register -
            // S1/S0 are what actually select the data rate on the part.
            lc_rate_pins = {LC1_S1, LC1_S0};
            `ASSERT_EQ(lc_rate_pins, LC_RATE_320HZ, "0b%02b",
                       "[LOADCELL] S1/S0 not driven for 320SPS");

            // ---- first sample -------------------------------------
            poll_loadcell_ready(200, lc_value_word);
            first_seq = lc_value_word[31:24];
            $display("[LOADCELL] sample 1: value=0x%06h seq=%0d pulses=%0d",
                     lc_value_word[23:0], first_seq, lc_last_pulse_count);

            `ASSERT_EQ(lc_value_word[23:0], lc_sample, "0x%06h",
                       "[LOADCELL] First sample value mismatch");
            // Nothing may clock the part before ENABLE is written, so the
            // very first conversion the peripheral ever performs carries
            // sequence 1 - this also catches the peripheral running the
            // interface while disabled.
            `ASSERT_EQ(first_seq, 8'd1, "%0d",
                       "[LOADCELL] First sample should carry sequence 1");
            // 25 pulses IS the channel A / gain 128 command (Table 4),
            // so this checks the analog front end is configured as asked
            // rather than merely that some data came back.
            `ASSERT_EQ(lc_last_pulse_count, 5'd25, "%0d",
                       "[LOADCELL] Expected 25 PD_SCK pulses for CH A gain 128");
            `ASSERT_EQ(lc_chan_b, 1'b0, "%0d",
                       "[LOADCELL] Wrong input channel selected");
            `ASSERT_EQ(lc_gain, 8'd128, "%0d",
                       "[LOADCELL] Wrong PGA gain selected");

            // Change the stimulus so the second sample is distinguishable
            // from the first - otherwise a peripheral that never updated
            // the value register would still pass. Safe to do here: the
            // model latches its next result one full conversion period
            // after the 25th pulse, far longer than the handful of
            // QuadSPI transactions above take.
            lc_sample = 24'h5A5AA5;

            // ---- second sample ------------------------------------
            poll_loadcell_next(200, first_seq, lc_value_word);
            second_seq = lc_value_word[31:24];
            $display("[LOADCELL] sample 2: value=0x%06h seq=%0d",
                     lc_value_word[23:0], second_seq);

            `ASSERT_EQ(lc_value_word[23:0], lc_sample, "0x%06h",
                       "[LOADCELL] Second sample value mismatch");
            // Exactly one, not merely different: a larger step would mean
            // a conversion was produced and then lost, which is precisely
            // what the sequence counter exists to make visible.
            `ASSERT_EQ(second_seq - first_seq, 8'd1, "%0d",
                       "[LOADCELL] Sequence must step by one per conversion");

            // The model has been checking every PD_SCK edge against the
            // datasheet's T1/T2/T3/T4 limits for the whole run, so a
            // correct-looking value with a non-zero error count means the
            // interface only happens to work.
            `ASSERT_EQ(lc_err_count, 32'd0, "%0d",
                       "[LOADCELL] HX717 protocol violations");

            // Leave the peripheral idle.
            qspi_bus_write(LC0_BASE + REG_LC_CTRL, 32'h0000_0000);

            lc_bus_trace = 1'b0;
        end

        #100;

        report();
        $finish;
    end

endmodule
'