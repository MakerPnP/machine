
module wake
    (
        input  wire        sys_clk,
        input  wire        reset,
        input              nwake_in,
        output reg nwake_1,
        output reg nwake_2,
        output reg nwake_3,
        output reg nwake_4
    );

    reg nwake_in_r;

    always @(*) begin
        nwake_in_r = nwake_in;
    end

    always @(posedge sys_clk) begin
        if (reset) begin
            nwake_1 = 1;
            nwake_2 = 1;
            nwake_3 = 1;
            nwake_4 = 1;
        end else begin
            nwake_1 <= nwake_in_r;
            nwake_2 <= nwake_in_r;
            nwake_3 <= nwake_in_r;
            nwake_4 <= nwake_in_r;
        end
    end

endmodule
