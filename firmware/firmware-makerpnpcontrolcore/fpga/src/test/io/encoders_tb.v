`timescale 1ns/1ps

`include "src/test/assertions.svh"

module encoders_tb;

    // Testbench signals
    reg RESET;
    reg TCXO = 0;

    reg [5:0]  addr;
    reg [31:0] din;
    reg [31:0] dout;
    reg        we;

    wire [15:0] debug;

    reg [2:0] ENC_ABZ [6] = '{
        3'b000,
        3'b000,
        3'b000,
        3'b000,
        3'b000,
        3'b000
    };

    // Instantiate the DUT (DUT = Device Under Test)
    encoders dut (
        .reset(RESET),
        .sys_clk(TCXO),

        .bus_we(we),
        .bus_addr(addr),
        .bus_din(din),
        .bus_dout(dout),

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

        // reset pulse
        RESET = 1;
        #20;
        RESET = 0;

        #50;

        addr = 6'h00;
        #10;
        `ASSERT_EQ(dout, 32'd0, "0x%08h", "ENCODER_CTRL default status");

        // TODO wrap this is a function
        addr = 6'h00;
        din = {24'd0, 8'b0000_0001};
        we = 1'b1;
        #10;
        we = 1'b0;
        #50;

        `ASSERT_EQ(dout[0], 1'b0, "0b%1b", "ENCODER_CTRL reset flag not cleared");

        // read the encoder

        addr = 6'h20;
        #10;
        $display("ENC after reset: %0d", dout);
        `ASSERT_EQ(dout, 32'd0, "0x%08h", "ENC_COUNT invalid");

        // TODO debounce inputs

        // ----------------------------------------
        // Encoder counting test
        // ----------------------------------------

        // Enable encoder (assuming ctrl bit already set earlier)

        // Step forward 10 steps
        repeat (10) step_forward();

        // Read encoder value
        addr = 6'h20;
        #10;
        $display("ENC value after +10 steps: %0d", dout);

        // Expect 40 if x4 decoding (10 * 4)
        `ASSERT_EQ(dout, 32'd40, "%0d", "ENC forward count failed");

        // Step backward 5 steps
        repeat (5) step_backward();

        addr = 6'h20;
        #10;
        $display("ENC value after -5 steps: %0d", dout);

        `ASSERT_EQ(dout, 32'd20, "%0d", "ENC backward count failed");

        // ----------------------------------------
        // Index reset test
        // ----------------------------------------

        pulse_index();

        #50;

        addr = 6'h20;
        #10;
        $display("ENC value z pulse: %0d", dout);
        `ASSERT_EQ(dout, 32'd0, "%0d", "ENC index reset failed");

        transition_forwards();

        // Read encoder value
        addr = 6'h20;
        #10;
        $display("ENC value after +1 steps: %0d", dout);
        `ASSERT_EQ(dout, 32'd1, "%0d", "Count after z pulse + 1 forward step incorrect");

        pulse_index();

        #50;

        addr = 6'h20;
        #10;
        $display("ENC value z pulse: %0d", dout);
        `ASSERT_EQ(dout, 32'd0, "%0d", "ENC index reset failed");

        transition_backwards();

        // Read encoder value (wraps around)
        addr = 6'h20;
        #10;
        $display("ENC value after +1 steps: %0d", dout);
        `ASSERT_EQ(dout, 32'hffff_ffff, "%0d", "Count after z pulse + 1 reverse step incorrect");

        // ----------------------------------------
        // Set counters
        // ----------------------------------------

        begin : SET_COUNTERS
            integer i;

            for (i = 0; i < 6; i = i + 1) begin
                addr = 6'h04 + (i * 4);
                din = 32'hbaad_beef;
                we = 1'b1;
                #10;
                we = 1'b0;
                #50;

                addr = 6'h20 + (i * 4);
                #100;
                $display("ENC counter after ENC_SET_VALUE_%1d: %0d", i, dout);

                `ASSERT_EQ(dout, 32'hbaad_beef, "0x%08h", $sformatf("ENC_SET_VALUE_%1d failed", i));
            end
        end

        report();
        $finish;
    end

endmodule