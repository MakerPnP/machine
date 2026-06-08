
module la
    (
        input reset,
        input sys_clk,
        output wire [15:0] la_io,
        input wire [7:0] la_src_in,
        input wire [15:0] la_in
    );

    localparam SRC_DISABLED = 8'd0;
    localparam SRC_COUNTER = 8'd1;
    localparam SRC_INPUT = 8'd2;

    reg [15:0] counter = 0;
    reg [7:0] la_src;

    always @(posedge sys_clk) begin
        if (reset) begin
            counter <= 0;
            la_src <= SRC_DISABLED;
        end else begin
            la_src <= la_src_in;
            counter <= counter + 1;
        end
    end

    function [15:0] la_mux;
        input [7:0] sel;
        input [15:0] counter;
        input [15:0] la_in;

        begin
            case (sel)
                SRC_COUNTER: la_mux = counter;
                SRC_INPUT:   la_mux = la_in;
                default:     la_mux = 16'd0;
            endcase
        end
    endfunction

    assign la_io = la_mux(la_src, counter, la_in);
endmodule

