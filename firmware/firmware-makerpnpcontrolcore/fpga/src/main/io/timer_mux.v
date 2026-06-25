
module timer_mux
    (
        input  wire        sys_clk,
        input  wire        reset,
        output wire mux_sel1,
        output wire mux_sel2,
        output wire mux_sel3,
        output wire mux_sel4
    );

reg[3:0] mux_sel;

assign mux_sel1 = mux_sel[0];
assign mux_sel2 = mux_sel[1];
assign mux_sel3 = mux_sel[2];
assign mux_sel4 = mux_sel[3];

always @(posedge sys_clk) begin
    if (reset) begin
        mux_sel <= 4'b1111;
    end else begin
        mux_sel <= 4'b0000;
    end
end

endmodule
