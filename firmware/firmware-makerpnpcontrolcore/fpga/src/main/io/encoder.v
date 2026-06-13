// encoders.v
// Example of an internal module managing registers in real time
module encoder(
    input  wire        sys_clk,
    input  wire        reset,
    input  wire        set,
    input  wire [31:0] set_value,
    output reg [31:0] count,
    input  wire [2:0]  abz
);

    // --------------------------------------------------
    // 1. Synchronize inputs (2FF)
    // --------------------------------------------------
    reg [2:0] abz_sync_0 = 3'b000;
    reg [2:0] abz_sync_1 = 3'b000;

    always @(posedge sys_clk) begin
        abz_sync_0 <= abz;
        abz_sync_1 <= abz_sync_0;
    end

    // --------------------------------------------------
    // 2. Quadrature decode (x4)
    // --------------------------------------------------
    reg [2:0] prev_abz;// = 3'b000;
    wire [2:0] curr_abz = abz_sync_1;

    wire [3:0] transition = {prev_abz[2:1], curr_abz[2:1]};

    wire inc = (transition == 4'b0001) |
               (transition == 4'b0111) |
               (transition == 4'b1110) |
               (transition == 4'b1000);

    wire dec = (transition == 4'b0010) |
               (transition == 4'b1011) |
               (transition == 4'b1101) |
               (transition == 4'b0100);

    // --------------------------------------------------
    // 3. Index (Z) rising edge detect
    // --------------------------------------------------
    wire z_rise = (curr_abz[0] == 1'b1) && (prev_abz[0] == 1'b0);

    reg initialized = 0;

    // --- 3. Internal Business Logic / CDC Core ---
    always @(posedge sys_clk) begin
        if (reset) begin
            count <= 0;
            prev_abz <= 3'b000;
            initialized <= 1'b0;
        end else begin
            if (set) begin
                count <= set_value;
            end else begin
                prev_abz <= curr_abz;
                if (!initialized) begin
                    initialized <= 1'b1;
                end else begin

                    if (z_rise) begin
                        count <= 0;
                    end else begin
                        if (inc) begin
                            count <= count + 1;
                        end else if (dec) begin
                            count <= count - 1;
                        end
                    end
                end
            end
        end
    end
endmodule
