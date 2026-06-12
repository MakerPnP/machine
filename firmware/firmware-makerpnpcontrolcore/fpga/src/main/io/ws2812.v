module ws2812 #(
    parameter MAX_LEDS = 256
)(
    input  wire        sys_clk,
    input  wire        reset,

    // =========================
    // BUS INTERFACE
    // =========================
    input  wire        bus_we,
    input  wire [5:0]  bus_addr,
    input  wire [31:0] bus_din,
    output reg  [31:0] bus_dout,

    // =========================
    // WS OUTPUT
    // =========================
    output reg         ws_out
);

    always @(*) begin
        if (reset) begin
            ws_out = 1'b1;
            bus_dout = 32'd0;
        end
        else begin
            ws_out = 1'b0;
            bus_dout = 32'd0;
        end
    end

endmodule
