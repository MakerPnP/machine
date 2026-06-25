
module wake
    (
        input  wire        sys_clk,
        input  wire        reset,
        input              nwake_in,
        output wire nwake_1,
        output wire nwake_2,
        output wire nwake_3,
        output wire nwake_4
    );

    reg nwake_in_r;

    always @(posedge sys_clk) begin
        if (reset) begin
            nwake_in_r = 1'b1;
        end else begin
            nwake_in_r = nwake_in;
        end
    end

    // Explicitly instantiate SB_IO with an output register (PIN_OUTPUT_REGISTERED = 6'b0101_00)
    SB_IO #(.PIN_TYPE(6'b0101_00)) io_wake1 (
        .PACKAGE_PIN(nwake_1),
        .OUTPUT_CLK(sys_clk),
        .D_OUT_0(nwake_in_r)
    );

    SB_IO #(.PIN_TYPE(6'b0101_00)) io_wake2 (
        .PACKAGE_PIN(nwake_2),
        .OUTPUT_CLK(sys_clk),
        .D_OUT_0(nwake_in_r)
    );

    SB_IO #(.PIN_TYPE(6'b0101_00)) io_wake3 (
        .PACKAGE_PIN(nwake_3),
        .OUTPUT_CLK(sys_clk),
        .D_OUT_0(nwake_in_r)
    );

    SB_IO #(.PIN_TYPE(6'b0101_00)) io_wake4 (
        .PACKAGE_PIN(nwake_4),
        .OUTPUT_CLK(sys_clk),
        .D_OUT_0(nwake_in_r)
    );

endmodule
