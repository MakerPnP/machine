
module clock_out
    (
        input  wire        sys_clk,
        input  wire        reset,
        output wire clock_out1,
        output wire clock_out2,
        output wire clock_out3,
        output wire clock_out4
    );

    // for now, keep the clock signals LOW after reset
    // the TMC5160 says: "CLK - CLK input. Tie to GND using short wire for internal clock or supply external clock"
    // See TMC5160 Datasheet, Rev 1.18, 26.2 Using an External Clock.

    // The (* keep *) attribute prevents Yosys from merging these identical flip-flops since the pins are spread across the chip
    (* keep *) reg[3:0] clock_out;

    always @(posedge sys_clk) begin
        if (reset) begin
            clock_out <= 4'b1111;
        end else begin
            clock_out <= 4'b0000;
        end
    end

    // Explicitly instantiate SB_IO with an output register (PIN_OUTPUT_REGISTERED = 6'b0101_00)
    SB_IO #(.PIN_TYPE(6'b0101_00)) io_clock_out1 (
        .PACKAGE_PIN(clock_out1),
        .OUTPUT_CLK(sys_clk),
        .D_OUT_0(reset ? 1'b1 : clock_out[0])
    );

    SB_IO #(.PIN_TYPE(6'b0101_00)) io_clock_out2 (
        .PACKAGE_PIN(clock_out2),
        .OUTPUT_CLK(sys_clk),
        .D_OUT_0(reset ? 1'b1 : clock_out[1])
    );

    SB_IO #(.PIN_TYPE(6'b0101_00)) io_clock_out3 (
        .PACKAGE_PIN(clock_out3),
        .OUTPUT_CLK(sys_clk),
        .D_OUT_0(reset ? 1'b1 : clock_out[2])
    );

    SB_IO #(.PIN_TYPE(6'b0101_00)) io_clock_out4 (
        .PACKAGE_PIN(clock_out4),
        .OUTPUT_CLK(sys_clk),
        .D_OUT_0(reset ? 1'b1 : clock_out[3])
    );

endmodule
