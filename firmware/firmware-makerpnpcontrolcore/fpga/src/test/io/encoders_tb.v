`timescale 1ns/1ps

`include "src/test/assertions.svh"

module encoders_tb;

    // Testbench signals
    reg RESET;
    reg TCXO = 0;

    `include "src/test/bus_io.svh"

    reg [2:0] ENC_ABZ [6] = '{
        3'b000,
        3'b000,
        3'b000,
        3'b000,
        3'b000,
        3'b000
    };

    wire [15:0] debug;

    reg [31:0] result;

    // Instantiate the DUT (DUT = Device Under Test)
    encoders dut (
        .reset(RESET),
        .sys_clk(TCXO),

        .bus_stb(stb),
        .bus_we(we),
        .bus_addr(addr),
        .bus_din(din),
        .bus_dout(dout),
        .bus_ack(ack),

        .abz_a(ENC_ABZ[0]),
        .abz_b(ENC_ABZ[1]),
        .abz_c(ENC_ABZ[2]),
        .abz_x(ENC_ABZ[3]),
        .abz_y(ENC_ABZ[4]),
        .abz_z(ENC_ABZ[5]),

        .debug(debug)
    );

    // Clock generation: 100 MHz simulated clock (10ns period)
    always #5 TCXO = ~TCXO;

    // 2 bits so it wraps round when it overflows
    reg [1:0] transition_index = 0;

    // quadrature encoding values. check the datasheet for the motor/encoder to determine which way the encoder rotates
    reg [1:0] transition_values [4] = '{
        2'b00,
        2'b01,
        2'b11,
        2'b10
    };

    task transition_forwards;
    begin
        transition_index += 1;
        ENC_ABZ[0][2:1] = transition_values[transition_index]; #20;
    end
    endtask

    task transition_backwards;
    begin
        transition_index -= 1;
        ENC_ABZ[0][2:1] = transition_values[transition_index]; #20;
    end
    endtask

    task step_forward;
    begin
        repeat (4) transition_forwards;
    end
    endtask

    task step_backward;
    begin
        repeat (4) transition_backwards;
    end
    endtask

    task pulse_index;
    begin
        ENC_ABZ[0][0] = 1;  // Z high
        #20;
        ENC_ABZ[0][0] = 0;  // Z low
    end
    endtask

    // Simulation control
    initial begin
        $dumpfile("encoders_tb.vcd");
        $dumpvars(0, encoders_tb);

        sys_reset();
        bus_init();

        bus_read(6'h00, result);
        `ASSERT_EQ(result, 32'd0, "0x%08h", "ENCODER_CTRL default status");

        bus_write(6'h00, {24'd0, 8'b0000_0001});
        bus_read(6'h00, result);

        //#50;

        `ASSERT_EQ(result[0], 1'b0, "0b%1b", "ENCODER_CTRL reset flag not cleared");

        // read the encoder

        bus_read(6'h00, result);
        $display("ENC after reset: %0d", result);
        `ASSERT_EQ(result, 32'd0, "0x%08h", "ENC_COUNT invalid");

        // ----------------------------------------
        // Encoder counting test
        // ----------------------------------------

        // Enable encoder (assuming ctrl bit already set earlier)

        // Step forward 10 steps
        repeat (10) step_forward();

        // Read encoder value
        bus_read(6'h20, result);

        $display("ENC value after +10 steps: %0d", result);

        // Expect 40 if x4 decoding (10 * 4)
        `ASSERT_EQ(result, 32'd40, "%0d", "ENC forward count failed");

        // Step backward 5 steps
        repeat (5) step_backward();

        bus_read(6'h20, result);
        $display("ENC value after -5 steps: %0d", result);

        `ASSERT_EQ(result, 32'd20, "%0d", "ENC backward count failed");

        // ----------------------------------------
        // Index reset test
        // ----------------------------------------

        pulse_index();
        #20;

        bus_read(6'h20, result);
        $display("ENC value z pulse: %0d", result);
        `ASSERT_EQ(result, 32'd0, "%0d", "ENC index reset failed");

        transition_forwards();
        #20;

        // Read encoder value
        bus_read(6'h20, result);


        $display("ENC value after +1 steps: %0d", result);
        `ASSERT_EQ(result, 32'd1, "%0d", "Count after z pulse + 1 forward step incorrect");

        pulse_index();
        #20;

        bus_read(6'h20, result);
        $display("ENC value z pulse: %0d", result);
        `ASSERT_EQ(result, 32'd0, "%0d", "ENC index reset failed");

        transition_backwards();
        #20;

        // Read encoder value (wraps around)
        bus_read(6'h20, result);
        $display("ENC value after +1 steps: %0d", result);
        `ASSERT_EQ(result, 32'h0000_ffff, "%0d", "Count after z pulse + 1 reverse step incorrect");

        // ----------------------------------------
        // Set counters
        // ----------------------------------------

        begin : SET_COUNTERS
            integer i;

            for (i = 0; i < 6; i = i + 1) begin

                bus_write(6'h04 + (i * 4), 32'hbaad_beef);
                bus_read(6'h20 + (i * 4), result);
                $display("ENC counter after ENC_SET_VALUE_%1d: %0d", i, result);

                `ASSERT_EQ(result, 32'h0000_beef, "0x%08h", $sformatf("ENC_SET_VALUE_%1d failed", i));
            end
        end

        report();
        $finish;
    end

endmodule