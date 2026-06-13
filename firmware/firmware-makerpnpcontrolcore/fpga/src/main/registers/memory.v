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

    reg        led_we_r;
    reg        io_we_r;
    reg        buzzer_we_r;
    reg        encoder_we_r;

    reg [5:0]  led_addr_r;
    reg [5:0]  io_addr_r;
    reg [5:0]  buzzer_addr_r;
    reg [5:0]  encoder_addr_r;

    reg [31:0] led_din_r;
    reg [31:0] io_din_r;
    reg [31:0] buzzer_din_r;
    reg [31:0] encoder_din_r;

    assign led_we      = led_we_r;
    assign io_we       = io_we_r;
    assign buzzer_we   = buzzer_we_r;
    assign encoder_we  = encoder_we_r;

    assign led_addr     = led_addr_r;
    assign io_addr      = io_addr_r;
    assign buzzer_addr  = buzzer_addr_r;
    assign encoder_addr = encoder_addr_r;

    assign led_din     = led_din_r;
    assign io_din      = io_din_r;
    assign buzzer_din  = buzzer_din_r;
    assign encoder_din = encoder_din_r;

    wire is_led_space     = (addr_a >= 16'h0040 && addr_a < 16'h0080);
    wire is_io_space      = (addr_a >= 16'h0080 && addr_a < 16'h00C0);
    wire is_buzzer_space  = (addr_a >= 16'h00C0 && addr_a < 16'h0100);
    wire is_encoder_space = (addr_a >= 16'h0100 && addr_a < 16'h0140);

    always @(posedge clk_a) begin
        if (reset) begin
            dout_a <= 32'h00000000;

            led_we_r     <= 1'b0;
            io_we_r      <= 1'b0;
            buzzer_we_r  <= 1'b0;
            encoder_we_r <= 1'b0;

            led_addr_r     <= 6'd0;
            io_addr_r      <= 6'd0;
            buzzer_addr_r  <= 6'd0;
            encoder_addr_r <= 6'd0;

            led_din_r     <= 32'd0;
            io_din_r      <= 32'd0;
            buzzer_din_r  <= 32'd0;
            encoder_din_r <= 32'd0;
        end else begin
            // Default write strobes low; asserted for exactly one clk_a cycle.
            led_we_r     <= 1'b0;
            io_we_r      <= 1'b0;
            buzzer_we_r  <= 1'b0;
            encoder_we_r <= 1'b0;

            // Synchronously drive downstream bus address/data.
            if (is_led_space) begin
                led_addr_r <= addr_a[5:0];
                led_din_r  <= din_a;
                led_we_r   <= we_a;
                dout_a     <= led_dout;
            end else if (is_io_space) begin
                io_addr_r <= addr_a[5:0];
                io_din_r  <= din_a;
                io_we_r   <= we_a;
                dout_a    <= io_dout;
            end else if (is_buzzer_space) begin
                buzzer_addr_r <= addr_a[5:0];
                buzzer_din_r  <= din_a;
                buzzer_we_r   <= we_a;
                dout_a        <= buzzer_dout;
            end else if (is_encoder_space) begin
                encoder_addr_r <= addr_a[5:0];
                encoder_din_r  <= din_a;
                encoder_we_r   <= we_a;
                dout_a         <= encoder_dout;
            end else begin
                case (addr_a)
                    16'h0000: dout_a <= IDENT;
                    16'h0004: dout_a <= VERSION;
                    16'h01FC: dout_a <= MARKER;
                    default:  dout_a <= 32'hAA55AA55;
                endcase
            end
        end
    end

endmodule