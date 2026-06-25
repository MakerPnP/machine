
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

reg[3:0] clock_out;

assign clock_out1 = clock_out[0];
assign clock_out2 = clock_out[1];
assign clock_out3 = clock_out[2];
assign clock_out4 = clock_out[3];

always @(posedge sys_clk) begin
    if (reset) begin
        clock_out <= 4'b1111;
    end else begin
        clock_out <= 4'b0000;
    end
end

endmodule
