
module timer_mux
    (
        input reset,
        output reg mux_sel1,
        output reg mux_sel2,
        output reg mux_sel3,
        output reg mux_sel4
    );

always @(*) begin
    if (reset) begin
        mux_sel1 = 1;
        mux_sel2 = 1;
        mux_sel3 = 1;
        mux_sel4 = 1;
    end else begin
        mux_sel1 = 0;
        mux_sel2 = 0;
        mux_sel3 = 0;
        mux_sel4 = 0;
    end
end

endmodule
