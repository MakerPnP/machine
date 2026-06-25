/**
 * PLL configuration
 *
 * This Verilog module was generated automatically
 * using the icepll tool from the IceStorm project.
 * Use at your own risk.
 *
 * Given input frequency:        50.000 MHz
 * Requested output frequency:   50.000 MHz
 * Achieved output frequency:    50.000 MHz
 */

`ifdef SIM
`timescale 1ns / 1ps
module pll(
    input  clock_in,
    output reg clock_out,
    output reg locked
);
    initial begin
        $display("Using simulated PLL");

        clock_out = 0;
        locked = 0;

        // wait a bit to simulate lock time
        #100 locked = 1;
        $display("locked");

        // generate 1x clock (adjust timing to match your testbench timescale)
        forever #5 clock_out = ~clock_out;
    end
endmodule
`else
module pll(
	input  clock_in,
	output clock_out,
	output locked
	);

SB_PLL40_CORE #(
		.FEEDBACK_PATH("SIMPLE"),
.FEEDBACK_PATH("SIMPLE"),
        .DIVR(4'b0000),         // DIVR =  0
        .DIVF(7'b0001111),      // DIVF = 15
        .DIVQ(3'b100),          // DIVQ =  4
        .FILTER_RANGE(3'b100)   // FILTER_RANGE = 4
) uut (
		.LOCK(locked),
		.RESETB(1'b1),
		.BYPASS(1'b0),
		.REFERENCECLK(clock_in),
		.PLLOUTCORE(clock_out)
		);

endmodule
`endif