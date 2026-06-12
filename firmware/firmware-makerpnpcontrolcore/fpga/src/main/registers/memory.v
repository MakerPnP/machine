module memory (
    input              reset,
    input  wire        clk_a,
    input  wire        we_a,
    input  wire [15:0] addr_a,
    input  wire [31:0] din_a,
    output reg  [31:0] dout_a,

    // Bus Interface to LED Module
    output wire        led_we,
    output wire [5:0]  led_addr,
    output wire [31:0] led_din,
    input  wire [31:0] led_dout,

    // Bus Interface to IO Module
    output wire        io_we,
    output wire [5:0]  io_addr,
    output wire [31:0] io_din,
    input  wire [31:0] io_dout,

    // Bus Interface to Buzzer Module
    output wire        buzzer_we,
    output wire [5:0]  buzzer_addr,
    output wire [31:0] buzzer_din,
    input  wire [31:0] buzzer_dout,

    // Bus Interface to Encoders Module
    output wire        encoder_we,
    output wire [5:0]  encoder_addr,
    output wire [31:0] encoder_din,
    input  wire [31:0] encoder_dout
);

    localparam [31:0] IDENT   = 32'hFA_CE_B0_0B;
    localparam [31:0] VERSION = 32'h01_02_03_04;
    localparam [31:0] MARKER  = 32'hDE_AD_C0_DE;

    // --- 1. Address Range Decoding (Combinational Demux) ---
    // Check ranges and routing criteria.

    // 6 bits per space
    wire is_led_space     = (addr_a >= 16'h0040 && addr_a < 16'h0080);
    wire is_io_space      = (addr_a >= 16'h0080 && addr_a < 16'h00C0);
    wire is_buzzer_space  = (addr_a >= 16'h00C0 && addr_a < 16'h0100);
    wire is_encoder_space = (addr_a >= 16'h0100 && addr_a < 16'h0140);

    // Assign Write Enables only if the module is being targeted
    assign led_we        = we_a && is_led_space;
    assign io_we         = we_a && is_io_space;
    assign buzzer_we     = we_a && is_buzzer_space;
    assign encoder_we   = we_a && is_encoder_space;

    // Pass down the relative/sub-address offset
    assign led_addr        = addr_a[5:0];
    assign io_addr         = addr_a[5:0];
    assign buzzer_addr     = addr_a[5:0];
    assign encoder_addr    = addr_a[5:0];

    // Pass down data straight through
    assign led_din         = din_a;
    assign io_din          = din_a;
    assign buzzer_din      = din_a;
    assign encoder_din     = din_a;

    // --- 2. Centralized Combinational Read Routing ---
    always @(*) begin
        if (is_led_space) begin
            dout_a = led_dout;
        end else if (is_io_space) begin
            dout_a = io_dout;
        end else if (is_buzzer_space) begin
            dout_a = buzzer_dout;
        end else if (is_encoder_space) begin
            dout_a = encoder_dout;
        end else begin
            // Fallback for core/global registers or defaults
            case (addr_a)
                16'h0000: dout_a = IDENT;
                16'h0004: dout_a = VERSION;
                16'h01FC: dout_a = MARKER;
                default: dout_a = 32'hAA55AA55;
            endcase
        end
    end

endmodule
