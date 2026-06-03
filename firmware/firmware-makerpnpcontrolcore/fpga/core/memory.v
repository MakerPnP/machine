// memory.v
// Streamlined architecture with unified synchronous state registers and
// an instantaneous combinational read-back multiplexer.
module memory (
    input  wire        clk_a,       // Driven by System Clock (TCXO / clk_sys)
    input  wire [11:0] addr_a,      // Address space from QSPI
    input  wire        we_a,        // Write Enable from QSPI
    input  wire [7:0]  din_a,       // Data from MCU
    output reg  [7:0]  dout_a,      // Data out to MCU (Combinational Read Routing)

    // Read Connections FROM internal modules
    input  wire [7:0]  reg_io_in_1, // From Buttons
    input  wire [31:0] enc_1,       // From Encoder Module 1
    input  wire [31:0] enc_2,       // From Encoder Module 2
    input  wire [31:0] enc_3,       // From Encoder Module 3
    input  wire [31:0] enc_4,       // From Encoder Module 4
    input  wire [31:0] enc_5,       // From Encoder Module 5
    input  wire [31:0] enc_6,       // From Encoder Module 6

    // Write Connections TO internal modules
    output reg         strobe_led_update,
    output reg [7:0]   led_out,
    output reg         strobe_encoder_reset // Triggers an encoder reset
);

    // Hardcoded Read-Only Constants
    localparam [31:0] IDENT   = 32'hFA_CE_B0_0B;
    localparam [31:0] VERSION = 32'h01_02_03_04; // Major, Minor, Patch, Build

    // -----------------------------------------------------------------
    // 1. INSTANTANEOUS COMBINATIONAL READ MULTIPLEXER
    // -----------------------------------------------------------------
    // Keeps read paths clean and free of multi-clock synchronization lag
    always @(*) begin
        case (addr_a)
            // IDENT read out (Byte by Byte, Big Endian match)
            12'h000: dout_a = IDENT[31:24];
            12'h001: dout_a = IDENT[23:16];
            12'h002: dout_a = IDENT[15:8];
            12'h003: dout_a = IDENT[7:0];

            // VERSION read out
            12'h004: dout_a = VERSION[31:24];
            12'h005: dout_a = VERSION[23:16];
            12'h006: dout_a = VERSION[15:8];
            12'h007: dout_a = VERSION[7:0];

            // LED outputs readback
            12'h020: dout_a = led_out;

            // IO INPUTS (Buttons)
            12'h024: dout_a = reg_io_in_1;

            // ENCODER 1 (32-bit layout sliced into bytes)
            12'h040: dout_a = enc_1[31:24];
            12'h041: dout_a = enc_1[23:16];
            12'h042: dout_a = enc_1[15:8];
            12'h043: dout_a = enc_1[7:0];

            // ENCODER 2
            12'h044: dout_a = enc_2[31:24];
            12'h045: dout_a = enc_2[23:16];
            12'h046: dout_a = enc_2[15:8];
            12'h047: dout_a = enc_2[7:0];

            // ENCODER 3
            12'h048: dout_a = enc_3[31:24];
            12'h049: dout_a = enc_3[23:16];
            12'h04A: dout_a = enc_3[15:8];
            12'h04B: dout_a = enc_3[7:0];

            // ENCODER 4
            12'h04C: dout_a = enc_4[31:24];
            12'h04D: dout_a = enc_4[23:16];
            12'h04E: dout_a = enc_4[15:8];
            12'h04F: dout_a = enc_4[7:0];

            // ENCODER 5
            12'h050: dout_a = enc_5[31:24];
            12'h051: dout_a = enc_5[23:16];
            12'h052: dout_a = enc_5[15:8];
            12'h053: dout_a = enc_5[7:0];

            // ENCODER 6
            12'h054: dout_a = enc_6[31:24];
            12'h055: dout_a = enc_6[23:16];
            12'h056: dout_a = enc_6[15:8];
            12'h057: dout_a = enc_6[7:0];

            // END OF MEMORY (0x1FC)
            12'h1FC: dout_a = 8'hDE;
            12'h1FD: dout_a = 8'hAD;
            12'h1FE: dout_a = 8'hC0;
            12'h1FF: dout_a = 8'hDE;

            default: dout_a = 8'hFF; // Reserved/unmapped space default
        endcase
    end

    // -----------------------------------------------------------------
    // 2. UNIFIED SYNCHRONOUS WRITE TARGET AND STROBE ENGINE
    // -----------------------------------------------------------------
    // Storage flip-flops and command pulses belong together on the clock edge
    always @(posedge clk_a) begin
        // Strobes default to 0; they automatically de-assert on the next system clock edge
        strobe_led_update    <= 1'b0;
        strobe_encoder_reset <= 1'b0;

        if (we_a) begin
            // Sim-only logging using non-blocking friendly formats
            $strobe("(strobe)writing to address: 0x%04h, data: 0x%02h", addr_a, din_a);
            $display("(display)writing to address: 0x%04h, data: 0x%02h", addr_a, din_a);

            case (addr_a)
                12'h010: begin
                    if (din_a[0] == 1'b1) begin
                        strobe_encoder_reset <= 1'b1; // Registered clean 1-cycle pulse
                        $display("resetting encoders");
                    end
                end
                12'h020: begin
                    led_out           <= din_a;        // Saves stable configuration state
                    strobe_led_update <= 1'b1;         // Registered clean 1-cycle pulse
                end
                default: begin end
            endcase
        end
    end

endmodule