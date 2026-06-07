
module wake
    (
        input reset,
        input nwake_in,
        output reg nwake_1,
        output reg nwake_2,
        output reg nwake_3,
        output reg nwake_4
    );

always @(*) begin
    if (reset) begin
        nwake_1 = 1;
        nwake_2 = 1;
        nwake_3 = 1;
        nwake_4 = 1;
    end else begin
        nwake_1 = nwake_in;
        nwake_2 = nwake_in;
        nwake_3 = nwake_in;
        nwake_4 = nwake_in;
    end
end

endmodule
