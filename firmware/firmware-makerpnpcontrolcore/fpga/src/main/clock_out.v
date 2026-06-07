
module clock_out
    (
        input reset,
        output reg clock_out1,
        output reg clock_out2,
        output reg clock_out3,
        output reg clock_out4
    );

// for now, keep the clock signals LOW after reset
// the TMC5160 says: "CLK - CLK input. Tie to GND using short wire for internal clock or supply external clock"
// See TMC5160 Datasheet, Rev 1.18, 26.2 Using an External Clock.

always @(*) begin
    if (reset) begin
        clock_out1 = 1;
        clock_out2 = 1;
        clock_out3 = 1;
        clock_out4 = 1;
    end else begin
        clock_out1 = 0;
        clock_out2 = 0;
        clock_out3 = 0;
        clock_out4 = 0;
    end
end

endmodule
