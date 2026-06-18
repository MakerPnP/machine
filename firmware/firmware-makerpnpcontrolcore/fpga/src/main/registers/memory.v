module memory (
    input              reset,
    input  wire        clk_a,
    input  wire        en_a,
    input  wire        we_a,
    input  wire [15:0] addr_a,
    input  wire [31:0] din_a,
    output reg  [31:0] dout_a,
    output reg         valid_a,

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
    input  wire [31:0] encoder_dout,

    // Bus Interface to WS2812 Module 0
    output wire        ws0_we,
    output wire [5:0]  ws0_addr,
    output wire [31:0] ws0_din,
    input  wire [31:0] ws0_dout,

    // Bus Interface to WS2812 Module 0
    output wire        ws1_we,
    output wire [5:0]  ws1_addr,
    output wire [31:0] ws1_din,
    input  wire [31:0] ws1_dout
);

    localparam [31:0] IDENT   = 32'hFA_CE_B0_0B;
    localparam [31:0] VERSION = 32'h01_02_03_04;
    localparam [31:0] MARKER  = 32'hDE_AD_C0_DE;

    localparam [2:0] TARGET_NONE    = 3'd0;
    localparam [2:0] TARGET_LED     = 3'd1;
    localparam [2:0] TARGET_IO      = 3'd2;
    localparam [2:0] TARGET_BUZZER  = 3'd3;
    localparam [2:0] TARGET_ENCODER = 3'd4;
    localparam [2:0] TARGET_WS0     = 3'd5;
    localparam [2:0] TARGET_WS1     = 3'd6;

    reg        led_we_r;
    reg        io_we_r;
    reg        buzzer_we_r;
    reg        encoder_we_r;
    reg        ws0_we_r;
    reg        ws1_we_r;

    reg [5:0]  led_addr_r;
    reg [5:0]  io_addr_r;
    reg [5:0]  buzzer_addr_r;
    reg [5:0]  encoder_addr_r;
    reg [5:0]  ws0_addr_r;
    reg [5:0]  ws1_addr_r;

    reg [31:0] led_din_r;
    reg [31:0] io_din_r;
    reg [31:0] buzzer_din_r;
    reg [31:0] encoder_din_r;
    reg [31:0] ws0_din_r;
    reg [31:0] ws1_din_r;

    reg        req_valid_r;
    reg        req_we_r;
    reg [15:0] req_addr_r;
    reg [31:0] req_din_r;
    reg [2:0]  req_target_r;

    reg        rsp_valid_r;
    reg [15:0] rsp_addr_r;
    reg [2:0]  rsp_target_r;
    reg [31:0] global_dout_r;

    assign led_we      = led_we_r;
    assign io_we       = io_we_r;
    assign buzzer_we   = buzzer_we_r;
    assign encoder_we  = encoder_we_r;
    assign ws0_we      = ws0_we_r;
    assign ws1_we      = ws1_we_r;

    assign led_addr     = led_addr_r;
    assign io_addr      = io_addr_r;
    assign buzzer_addr  = buzzer_addr_r;
    assign encoder_addr = encoder_addr_r;
    assign ws0_addr     = ws0_addr_r;
    assign ws1_addr     = ws1_addr_r;

    assign led_din     = led_din_r;
    assign io_din      = io_din_r;
    assign buzzer_din  = buzzer_din_r;
    assign encoder_din = encoder_din_r;
    assign ws0_din     = ws0_din_r;
    assign ws1_din     = ws1_din_r;

    wire [2:0] target_a =
        // 0x0040
        (addr_a[15:6] == 10'h001) ? TARGET_LED :
        // 0x0080
        (addr_a[15:6] == 10'h002) ? TARGET_IO :
        // 0x00C0
        (addr_a[15:6] == 10'h003) ? TARGET_BUZZER :
        // 0x0100
        (addr_a[15:6] == 10'h004) ? TARGET_ENCODER :
        // 0x0140
        (addr_a[15:6] == 10'h005) ? TARGET_WS0 :
        // 0x0180
        (addr_a[15:6] == 10'h006) ? TARGET_WS1 :
                                    TARGET_NONE;

    always @(posedge clk_a) begin
        if (reset) begin
            dout_a       <= 32'h00000000;
            valid_a      <= 1'b0;

            req_valid_r  <= 1'b0;
            req_we_r     <= 1'b0;
            req_addr_r   <= 16'd0;
            req_din_r    <= 32'd0;
            req_target_r <= TARGET_NONE;

            rsp_valid_r   <= 1'b0;
            rsp_addr_r    <= 16'd0;
            rsp_target_r  <= TARGET_NONE;
            global_dout_r <= 32'hAA55AA55;

            led_we_r     <= 1'b0;
            io_we_r      <= 1'b0;
            buzzer_we_r  <= 1'b0;
            encoder_we_r <= 1'b0;
            ws0_we_r     <= 1'b0;
            ws1_we_r     <= 1'b0;

            led_addr_r     <= 6'd0;
            io_addr_r      <= 6'd0;
            buzzer_addr_r  <= 6'd0;
            encoder_addr_r <= 6'd0;
            ws0_addr_r     <= 6'd0;
            ws1_addr_r     <= 6'd0;

            led_din_r     <= 32'd0;
            io_din_r      <= 32'd0;
            buzzer_din_r  <= 32'd0;
            encoder_din_r <= 32'd0;
            ws0_din_r     <= 32'd0;
            ws1_din_r     <= 32'd0;
        end else begin
            valid_a <= 1'b0;

            // Stage 0: capture incoming RAM-like port request.
            req_valid_r  <= en_a;
            req_we_r     <= we_a;
            req_addr_r   <= addr_a;
            req_din_r    <= din_a;
            req_target_r <= target_a;

            // Predecode global/default register data one stage before dout_a.
            case (addr_a)
                16'h0000: global_dout_r <= IDENT;
                16'h0004: global_dout_r <= VERSION;
                16'h01FC: global_dout_r <= MARKER;
                default:  global_dout_r <= 32'hAA55AA55;
            endcase

            // Stage 2 defaults.
            rsp_valid_r  <= 1'b0;
            rsp_addr_r   <= req_addr_r;
            rsp_target_r <= req_target_r;

            // Default write strobes low; asserted for exactly one clk_a cycle.
            led_we_r     <= 1'b0;
            io_we_r      <= 1'b0;
            buzzer_we_r  <= 1'b0;
            encoder_we_r <= 1'b0;
            ws0_we_r     <= 1'b0;
            ws1_we_r     <= 1'b0;

            // Stage 1: service the previously captured request.
            // For peripheral reads, this drives the peripheral address.
            // The selected peripheral dout is captured in Stage 2.
            if (req_valid_r) begin
                rsp_valid_r <= !req_we_r;

                case (req_target_r)
                    TARGET_LED: begin
                        led_addr_r <= req_addr_r[5:0];
                        led_din_r  <= req_din_r;
                        led_we_r   <= req_we_r;
                    end

                    TARGET_IO: begin
                        io_addr_r <= req_addr_r[5:0];
                        io_din_r  <= req_din_r;
                        io_we_r   <= req_we_r;
                    end

                    TARGET_BUZZER: begin
                        buzzer_addr_r <= req_addr_r[5:0];
                        buzzer_din_r  <= req_din_r;
                        buzzer_we_r   <= req_we_r;
                    end

                    TARGET_ENCODER: begin
                        encoder_addr_r <= req_addr_r[5:0];
                        encoder_din_r  <= req_din_r;
                        encoder_we_r   <= req_we_r;
                    end

                    TARGET_WS0: begin
                        ws0_addr_r <= req_addr_r[5:0];
                        ws0_din_r  <= req_din_r;
                        ws0_we_r   <= req_we_r;
                    end

                    TARGET_WS1: begin
                        ws1_addr_r <= req_addr_r[5:0];
                        ws1_din_r  <= req_din_r;
                        ws1_we_r   <= req_we_r;
                    end

                    default: begin
                    end
                endcase
            end

            // Stage 2: capture read data after peripheral address has settled.
            if (rsp_valid_r) begin
                valid_a <= 1'b1;

                case (rsp_target_r)
                    TARGET_LED: begin
                        dout_a <= led_dout;
                    end

                    TARGET_IO: begin
                        dout_a <= io_dout;
                    end

                    TARGET_BUZZER: begin
                        dout_a <= buzzer_dout;
                    end

                    TARGET_ENCODER: begin
                        dout_a <= encoder_dout;
                    end

                    TARGET_WS0: begin
                        dout_a <= ws0_dout;
                    end

                    TARGET_WS1: begin
                        dout_a <= ws1_dout;
                    end

                    default: begin
                        dout_a <= global_dout_r;
                    end
                endcase
            end
        end
    end

endmodule