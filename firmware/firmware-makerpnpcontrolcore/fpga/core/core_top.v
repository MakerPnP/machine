module core_top (
    input  TCXO,      // H16 Bank 1 50 MHz TCXO
    output FPGA_ACT,
    (* PULLUP = 1 *)
    input NWAKE_IN,
    output NWAKE_1
);

wire clk_100;
wire locked;
wire reset;

wire wake_1;

assign reset = ~locked;

// ----------------------
// PLL
// ----------------------
pll u_pll (
    .clock_in(TCXO),
    .clock_out(clk_100),
    .locked(locked)
);

// ----------------------
// Application logic
// ----------------------
blink u_blink (
    .reset(reset),
    .clk(clk_100),
    .led(FPGA_ACT)
);

wake u_wake (
    .reset(reset),
    .nwake_in(NWAKE_IN),
    .nwake_1(NWAKE_1)
);

endmodule