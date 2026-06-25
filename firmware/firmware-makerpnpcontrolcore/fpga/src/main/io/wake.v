
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

    assign nwake_1 = nwake_in_r;
    assign nwake_2 = nwake_in_r;
    assign nwake_3 = nwake_in_r;
    assign nwake_4 = nwake_in_r;

    always @(posedge sys_clk) begin
        if (reset) begin
            nwake_in_r = 1'b1;
        end else begin
            nwake_in_r = nwake_in;
        end
    end

endmodule
