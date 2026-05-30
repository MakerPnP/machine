
module wake
    (
        input reset,
        input nwake_in,
        output reg nwake_1
    );

always @(*) begin
    if (reset) begin
        nwake_1 = 1;
    end else begin
        nwake_1 = nwake_in;
    end
end

endmodule
