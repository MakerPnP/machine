`include "src/main/logging.svh"

module loadcell (
    input  wire        reset,
    input  wire        sys_clk,

    input  wire        bus_stb,
    input  wire        bus_we,
    input  wire [7:0]  bus_addr,
    input  wire [31:0] bus_din,
    output reg  [31:0] bus_dout,
    output reg         bus_ack,

    output reg         s0,
    output reg         s1,
    output reg         pd_sck,
    input  wire        dout
);

    `include "src/main/io/loadcell_regs.svh"
    `include "src/main/io/loadcell_shared.svh"

    // enable, ODR control (uses S0, S1), start continuous conversion (clears ready, powers up the device, etc), power-down mode, etc.
    reg [8:0] loadcell_ctrl;

    // bit 0 - enabled
    // bit 1 - value ready
    reg [1:0] loadcell_status;

    // copy of the last buffer, for use by the bus
    reg [23:0] loadcell_value;

    // buffer for reading into
    reg [23:0] loadcell_buffer;

    always @(posedge sys_clk) begin
        if (reset) begin
            loadcell_ctrl   <= 32'd0;
            loadcell_value  <= 24'd0;
            s0              <= 1'b0;
            s1              <= 1'b0;
            pd_sck          <= 1'b0;
            bus_dout        <= 32'h00000000;
            bus_ack         <= 1'b0;
        end else begin
            // TODO use bus interface, read load-cell, apply to value.
        end
    end
endmodule
