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
    output reg         led_stb,
    output wire        led_we,
    output wire [5:0]  led_addr,
    output wire [31:0] led_din,
    input  wire [31:0] led_dout,
    input  wire        led_ack,

    // Bus Interface to IO Module
    output reg         io_stb,
    output wire        io_we,
    output wire [5:0]  io_addr,
    output wire [31:0] io_din,
    input  wire [31:0] io_dout,
    input  wire        io_ack,

    // Bus Interface to Buzzer Module
    output reg         buzzer_stb,
    output wire        buzzer_we,
    output wire [5:0]  buzzer_addr,
    output wire [31:0] buzzer_din,
    input  wire [31:0] buzzer_dout,
    input  wire        buzzer_ack,

    // Bus Interface to Encoders Module
    output reg         encoder_stb,
    output wire        encoder_we,
    output wire [5:0]  encoder_addr,
    output wire [31:0] encoder_din,
    input  wire [31:0] encoder_dout,
    input  wire        encoder_ack,

    // Bus Interface to WS2812 Module 0
    output reg         ws0_stb,
    output wire        ws0_we,
    output wire [5:0]  ws0_addr,
    output wire [31:0] ws0_din,
    input  wire [31:0] ws0_dout,
    input  wire        ws0_ack,

    // Bus Interface to WS2812 Module 1
    output reg         ws1_stb,
    output wire        ws1_we,
    output wire [5:0]  ws1_addr,
    output wire [31:0] ws1_din,
    input  wire [31:0] ws1_dout,
    input  wire        ws1_ack

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

    // Static Control Registers
    reg        io_we_r, led_we_r, buzzer_we_r, encoder_we_r, ws0_we_r, ws1_we_r;
    reg [5:0]  io_addr_r, led_addr_r, buzzer_addr_r, encoder_addr_r, ws0_addr_r, ws1_addr_r;
    reg [31:0] io_din_r, led_din_r, buzzer_din_r, encoder_din_r, ws0_din_r, ws1_din_r;

    // Pipeline tracking elements
    reg        req_valid_r;
    reg        req_we_r;
    reg [15:0] req_addr_r;
    reg [31:0] req_din_r;
    reg [2:0]  req_target_r;
    reg        rsp_valid_r;
    reg [2:0]  rsp_target_r;
    reg [31:0] global_dout_r;

    // REGISTERED Bus State Tracking
    reg        bus_busy;

    // Drive structural wires cleanly
    assign led_we        = led_we_r;        assign led_addr      = led_addr_r;      assign led_din       = led_din_r;
    assign io_we         = io_we_r;         assign io_addr       = io_addr_r;       assign io_din        = io_din_r;
    assign buzzer_we     = buzzer_we_r;     assign buzzer_addr   = buzzer_addr_r;   assign buzzer_din    = buzzer_din_r;
    assign encoder_we    = encoder_we_r;    assign encoder_addr  = encoder_addr_r;  assign encoder_din   = encoder_din_r;
    assign ws0_we        = ws0_we_r;        assign ws0_addr      = ws0_addr_r;      assign ws0_din       = ws0_din_r;
    assign ws1_we        = ws1_we_r;        assign ws1_addr      = ws1_addr_r;      assign ws1_din       = ws1_din_r;

    // Fast, localized combinatorial target decode
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

    wire active_ack = led_ack | io_ack | buzzer_ack | encoder_ack | ws0_ack | ws1_ack;

    // These evaluate completely independently of bus_busy or valid_a logic loops
    wire ws0_select       = req_valid_r && (req_target_r == TARGET_WS0);
    wire ws1_select       = req_valid_r && (req_target_r == TARGET_WS1);
    wire led_select       = req_valid_r && (req_target_r == TARGET_LED);
    wire io_select        = req_valid_r && (req_target_r == TARGET_IO);
    wire buzzer_select    = req_valid_r && (req_target_r == TARGET_BUZZER);
    wire encoder_select   = req_valid_r && (req_target_r == TARGET_ENCODER);

    // Main Bus Pipeline Logic
    always @(posedge clk_a) begin
        if (reset) begin
            dout_a          <= 32'h00000000;
            valid_a         <= 1'b0;
            bus_busy        <= 1'b0;

            req_valid_r     <= 1'b0;
            req_we_r        <= 1'b0;
            req_addr_r      <= 16'd0;
            req_din_r       <= 32'd0;
            req_target_r    <= TARGET_NONE;
            rsp_valid_r     <= 1'b0;
            rsp_target_r    <= TARGET_NONE;
            global_dout_r   <= 32'hAA55AA55;

            led_stb <= 1'b0;
            io_stb <= 1'b0;
            buzzer_stb <= 1'b0;
            encoder_stb <= 1'b0;
            ws0_stb <= 1'b0;
            ws1_stb <= 1'b0;

            led_we_r <= 1'b0;
            io_we_r <= 1'b0;
            buzzer_we_r <= 1'b0;
            encoder_we_r <= 1'b0;
            ws0_we_r <= 1'b0;
            ws1_we_r <= 1'b0;
        end else begin
            valid_a <= 1'b0;

            // =================================================================
            // STATE A: Bus is Busy / Waiting for an Active Peripheral Handshake
            // =================================================================
            if (bus_busy) begin
                if (active_ack) begin
                    // Handshake resolved! Capture response and release the bus pipeline
                    bus_busy    <= 1'b0;
                    rsp_valid_r <= 1'b0;
                    valid_a     <= !req_we_r; // Assert master read valid if this was a read cycle

                    case (rsp_target_r)
                        TARGET_LED:      dout_a <= led_dout;
                        TARGET_IO:       dout_a <= io_dout;
                        TARGET_BUZZER:   dout_a <= buzzer_dout;
                        TARGET_ENCODER:  dout_a <= encoder_dout;
                        TARGET_WS0:      dout_a <= ws0_dout;
                        TARGET_WS1:      dout_a <= ws1_dout;
                        default:         dout_a <= global_dout_r;
                    endcase
                end
            end
            // =================================================================
            // STATE B: Bus is Free / Normal Pipelined Flow
            // =================================================================
            else begin
                // --- STAGE 0: Fetch master interface ports ---
                req_valid_r  <= en_a;
                req_we_r     <= we_a;
                req_addr_r   <= addr_a;
                req_din_r    <= din_a;
                req_target_r <= target_a;

                case (addr_a)
                    16'h0000: global_dout_r <= IDENT;
                    16'h0004: global_dout_r <= VERSION;
                    16'h01FC: global_dout_r <= MARKER;
                    default:  global_dout_r <= 32'hAA55AA55;
                endcase

                // --- STAGE 1: Dispatch Decoded Operations ---
                if (req_valid_r) begin
                    rsp_target_r <= req_target_r;
                    if (req_target_r == TARGET_NONE) begin
                        // Internal layouts complete in exactly 1 cycle without handshakes
                        valid_a     <= !req_we_r;
                        dout_a      <= global_dout_r;
                        rsp_valid_r <= 1'b0;
                    end else begin
                        // Peripheral transaction initiated: engage the registered stall interlock
                        bus_busy    <= 1'b1;
                        rsp_valid_r <= 1'b1;
                    end
                end else begin
                    rsp_target_r <= TARGET_NONE;
                end
            end

            // =================================================================
            // DECOUPLED PERIPHERAL REGISTER DISPATCH
            // =================================================================
            // We clear strobe signals if the bus isn't locked up or when active_ack clears them
            if (bus_busy && active_ack) begin
                led_stb <= 1'b0; io_stb <= 1'b0; buzzer_stb <= 1'b0; encoder_stb <= 1'b0;
                ws0_stb <= 1'b0; ws1_stb <= 1'b0;
            end

            // If the bus is free and a valid request matches, latch it instantly!
            if (!bus_busy) begin
                if (ws0_select) begin
                    ws0_addr_r <= req_addr_r[5:0];
                    ws0_din_r  <= req_din_r;
                    ws0_we_r   <= req_we_r;
                    ws0_stb    <= 1'b1;
                end
                if (ws1_select) begin
                    ws1_addr_r <= req_addr_r[5:0];
                    ws1_din_r  <= req_din_r;
                    ws1_we_r   <= req_we_r;
                    ws1_stb    <= 1'b1;
                end
                if (led_select) begin
                    led_addr_r <= req_addr_r[5:0];
                    led_din_r  <= req_din_r;
                    led_we_r   <= req_we_r;
                    led_stb    <= 1'b1;
                end
                if (io_select) begin
                    io_addr_r  <= req_addr_r[5:0];
                    io_din_r   <= req_din_r;
                    io_we_r    <= req_we_r;
                    io_stb     <= 1'b1;
                end
                if (buzzer_select) begin
                    buzzer_addr_r <= req_addr_r[5:0];
                    buzzer_din_r  <= req_din_r;
                    buzzer_we_r   <= req_we_r;
                    buzzer_stb    <= 1'b1;
                end
                if (encoder_select) begin
                    encoder_addr_r <= req_addr_r[5:0];
                    encoder_din_r  <= req_din_r;
                    encoder_we_r   <= req_we_r;
                    encoder_stb    <= 1'b1;
                end
            end
        end
    end

endmodule