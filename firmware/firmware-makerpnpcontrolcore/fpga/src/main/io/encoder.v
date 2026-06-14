// encoders.v
// Purely synchronous module managing encoder registers in real time
module encoder(
    input  wire        sys_clk,
    input  wire        reset,
    input  wire        set,
    input  wire [15:0] set_value,
    output reg  [15:0] count,
    input  wire [2:0]  abz
);

    // --------------------------------------------------
    // 1. Synchronize inputs (2FF)
    // --------------------------------------------------
    reg [2:0] abz_sync_0;
    reg [2:0] abz_sync_1;

    always @(posedge sys_clk) begin
        if (reset) begin
            abz_sync_0 <= 3'b000;
            abz_sync_1 <= 3'b000;
        end else begin
            abz_sync_0 <= abz;
            abz_sync_1 <= abz_sync_0;
        end
    end

    // --------------------------------------------------
    // 2. Synchronous Quadrature Decode & Counter Logic
    // --------------------------------------------------
    reg [2:0] prev_abz;
    reg       initialized;

    // Look-ahead wires for the current clock cycle's evaluation
    wire [3:0] transition = {prev_abz[2:1], abz_sync_1[2:1]};
    wire       z_rise     = abz_sync_1[0] & ~prev_abz[0];

    wire inc = (transition == 4'b0001) |
               (transition == 4'b0111) |
               (transition == 4'b1110) |
               (transition == 4'b1000);

    wire dec = (transition == 4'b0010) |
               (transition == 4'b1011) |
               (transition == 4'b1101) |
               (transition == 4'b0100);

    // --- Core Sequential Logic ---
    always @(posedge sys_clk) begin
        if (reset) begin
            count       <= 15'd0;
            prev_abz    <= 3'b000;
            initialized <= 1'b0;
        end else begin
            // Update history register every clock cycle to track edges
            prev_abz <= abz_sync_1;

            if (set) begin
                count <= set_value;
            end else if (!initialized) begin
                initialized <= 1'b1;
            end else if (z_rise) begin
                count <= 15'd0;
            end else begin
                // Synchronous accumulation happens here safely on the clock edge
                if (inc) begin
                    count <= count + 1;
                end else if (dec) begin
                    count <= count - 1;
                end
            end
        end
    end

endmodule