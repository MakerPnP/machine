module core_top (
    input  TCXO,      // H16 Bank 1 50 MHz TCXO
    output FPGA_ACT,
    (* PULLUP = 1 *)
    input NWAKE_IN,
    output NWAKE_1,
    output MUX_SEL1,
    output MUX_SEL2,
    output MUX_SEL3,
    output MUX_SEL4,
    output FPGA_CLK_1,
    output FPGA_CLK_2,
    output FPGA_CLK_3,
    output FPGA_CLK_4,
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
    .nwake_1(NWAKE_1),
    .nwake_2(NWAKE_2),
    .nwake_3(NWAKE_3),
    .nwake_4(NWAKE_4)
);

timer_mux u_timer_mux (
    .reset(reset),
    .mux_sel1(MUX_SEL1),
    .mux_sel2(MUX_SEL2),
    .mux_sel3(MUX_SEL3),
    .mux_sel4(MUX_SEL4)
);

clock_out u_clock_out (
    .reset(reset),
    .clock_out1(FPGA_CLK_1),
    .clock_out2(FPGA_CLK_2),
    .clock_out3(FPGA_CLK_3),
    .clock_out4(FPGA_CLK_4)
);

endmodule