module timer_mux
    (
        input  wire        sys_clk,
        input  wire        reset,
        output wire mux_sel1,
        output wire mux_sel2,
        output wire mux_sel3,
        output wire mux_sel4
    );

    // Explicitly instantiate SB_IO with an output register (PIN_OUTPUT_REGISTERED = 6'b0101_00)
    SB_IO #(.PIN_TYPE(6'b0101_00)) io_sel1 (
        .PACKAGE_PIN(mux_sel1),
        .OUTPUT_CLK(sys_clk),
        .D_OUT_0(reset ? 1'b1 : 1'b0)
    );

    SB_IO #(.PIN_TYPE(6'b0101_00)) io_sel2 (
        .PACKAGE_PIN(mux_sel2),
        .OUTPUT_CLK(sys_clk),
        .D_OUT_0(reset ? 1'b1 : 1'b0)
    );

    SB_IO #(.PIN_TYPE(6'b0101_00)) io_sel3 (
        .PACKAGE_PIN(mux_sel3),
        .OUTPUT_CLK(sys_clk),
        .D_OUT_0(reset ? 1'b1 : 1'b0)
    );

    SB_IO #(.PIN_TYPE(6'b0101_00)) io_sel4 (
        .PACKAGE_PIN(mux_sel4),
        .OUTPUT_CLK(sys_clk),
        .D_OUT_0(reset ? 1'b1 : 1'b0)
    );

endmodule