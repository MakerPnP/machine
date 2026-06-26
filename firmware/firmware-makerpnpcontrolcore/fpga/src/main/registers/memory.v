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
    output wire [7:0]  led_addr,
    output wire [31:0] led_din,
    input  wire [31:0] led_dout,
    input  wire        led_ack,

    // Bus Interface to IO Module
    output reg         io_stb,
    output wire        io_we,
    output wire [7:0]  io_addr,
    output wire [31:0] io_din,
    input  wire [31:0] io_dout,
    input  wire        io_ack,

    // Bus Interface to Buzzer Module
    output reg         buzzer_stb,
    output wire        buzzer_we,
    output wire [7:0]  buzzer_addr,
    output wire [31:0] buzzer_din,
    input  wire [31:0] buzzer_dout,
    input  wire        buzzer_ack,

    // Bus Interface to Encoders Module
    output reg         encoder_stb,
    output wire        encoder_we,
    output wire [7:0]  encoder_addr,
    output wire [31:0] encoder_din,
    input  wire [31:0] encoder_dout,
    input  wire        encoder_ack,

    // Bus Interface to WS2812 Module 0
    output reg         ws0_stb,
    output wire        ws0_we,
    output wire [7:0]  ws0_addr,
    output wire [31:0] ws0_din,
    input  wire [31:0] ws0_dout,
    input  wire        ws0_ack,

    // Bus Interface to WS2812 Module 1
    output reg         ws1_stb,
    output wire        ws1_we,
    output wire [7:0]  ws1_addr,
    output wire [31:0] ws1_din,
    input  wire [31:0] ws1_dout,
    input  wire        ws1_ack

);

    `include "src/main/registers/map.svh"
    `include "src/main/registers/system0_regs.svh"
    `include "src/main/registers/system1_regs.svh"

    localparam [31:0] IDENT   = 32'hFA_CE_B0_0B;
    localparam [31:0] VERSION = 32'h01_02_03_04;
    localparam [31:0] MARKER  = 32'hDE_AD_C0_DE;

    localparam PERIPHERAL_BITS = 8;
    localparam ADDRESS_BITS = 8;

    localparam [7:0] TARGET_SYSTEM0 = SYSTEM0_BASE >> PERIPHERAL_BITS;
    localparam [7:0] TARGET_LED     = LED_BASE >> PERIPHERAL_BITS;
    localparam [7:0] TARGET_IO      = IO_BASE >> PERIPHERAL_BITS;
    localparam [7:0] TARGET_BUZZER  = BUZZER_BASE >> PERIPHERAL_BITS;
    localparam [7:0] TARGET_ENCODER = ENCODER_BASE >> PERIPHERAL_BITS;
    localparam [7:0] TARGET_WS0     = WS0_BASE >> PERIPHERAL_BITS;
    localparam [7:0] TARGET_WS1     = WS1_BASE >> PERIPHERAL_BITS;
    localparam [7:0] TARGET_SYSTEM1 = SYSTEM1_BASE >> PERIPHERAL_BITS;

    // Static Control Registers
    reg        io_we_r, led_we_r, buzzer_we_r, encoder_we_r, ws0_we_r, ws1_we_r;
    reg [8:0]  io_addr_r, led_addr_r, buzzer_addr_r, encoder_addr_r, ws0_addr_r, ws1_addr_r;
    reg [31:0] io_din_r, led_din_r, buzzer_din_r, encoder_din_r, ws0_din_r, ws1_din_r;

    // Pipeline tracking elements
    reg        req_valid_r;
    reg        req_we_r;
    reg [7:0]  req_addr_r;
    reg [31:0] req_din_r;
    reg [7:0]  req_target_r;
    reg        rsp_valid_r;
    reg [7:0]  rsp_target_r;
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
    wire [7:0] target_a = addr_a[15:8];

    reg system0_ack = 0;
    reg system0_stb = 0;
    reg system1_ack = 0;
    reg system1_stb = 0;
    reg unmapped_stb = 0;
    reg unmapped_ack = 0;

    wire active_ack = led_ack | io_ack | buzzer_ack | encoder_ack | ws0_ack | ws1_ack | system0_ack | system1_ack | unmapped_ack;

    // These evaluate completely independently of bus_busy or valid_a logic loops
    wire system0_select   = (req_target_r == TARGET_SYSTEM0);
    wire system1_select   = (req_target_r == TARGET_SYSTEM1);
    wire ws0_select       = (req_target_r == TARGET_WS0);
    wire ws1_select       = (req_target_r == TARGET_WS1);
    wire led_select       = (req_target_r == TARGET_LED);
    wire io_select        = (req_target_r == TARGET_IO);
    wire buzzer_select    = (req_target_r == TARGET_BUZZER);
    wire encoder_select   = (req_target_r == TARGET_ENCODER);

    wire unmapped_select = !(
        system0_select |
        system1_select |
        ws0_select |
        ws1_select |
        io_select |
        led_select |
        buzzer_select |
        encoder_select
    );

    // Main Bus Pipeline Logic
    always @(posedge clk_a) begin
        if (reset) begin
            dout_a          <= 32'h00000000;
            valid_a         <= 1'b0;
            bus_busy        <= 1'b0;

            req_valid_r     <= 1'b0;
            req_we_r        <= 1'b0;
            req_addr_r      <= 8'd0;
            req_din_r       <= 32'd0;
            req_target_r    <= 32'd0;
            rsp_valid_r     <= 1'b0;
            rsp_target_r    <= 32'd0;
            global_dout_r   <= 32'd0;

            led_stb <= 1'b0;
            io_stb <= 1'b0;
            buzzer_stb <= 1'b0;
            encoder_stb <= 1'b0;
            ws0_stb <= 1'b0;
            ws1_stb <= 1'b0;
            system0_stb <= 1'b0;
            system1_stb <= 1'b0;
            unmapped_stb <= 1'b0;

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

                    // Assert master read valid if this was a read cycle
                    valid_a     <= !req_we_r;

                    case (rsp_target_r)
                        TARGET_LED:      dout_a <= led_dout;
                        TARGET_IO:       dout_a <= io_dout;
                        TARGET_BUZZER:   dout_a <= buzzer_dout;
                        TARGET_ENCODER:  dout_a <= encoder_dout;
                        TARGET_WS0:      dout_a <= ws0_dout;
                        TARGET_WS1:      dout_a <= ws1_dout;
                        // SYSTEM0/SYSTEM1 or un-mapped
                        default:         dout_a <= global_dout_r;
                    endcase

                    if (system0_ack) begin
                        system0_ack <= 1'b0;
                    end

                    if (system1_ack) begin
                        system1_ack <= 1'b0;
                    end

                    if (unmapped_ack) begin
                        unmapped_ack <= 1'b0;
                    end
                end
            end
            // =================================================================
            // STATE B: Bus is Free / Normal Pipelined Flow
            // =================================================================
            else begin
                // --- STAGE 0: Fetch master interface ports ---
                req_valid_r  <= en_a;
                req_we_r     <= we_a;
                req_addr_r   <= addr_a[7:0];
                req_din_r    <= din_a;
                req_target_r <= target_a;

                // --- STAGE 1: Dispatch Decoded Operations ---
                if (req_valid_r) begin
                    rsp_target_r <= req_target_r;
                    // Peripheral transaction initiated: engage the registered stall interlock
                    bus_busy    <= 1'b1;
                    rsp_valid_r <= 1'b1;
                end
            end

            // =================================================================
            // DECOUPLED PERIPHERAL REGISTER DISPATCH
            // =================================================================
            // We clear strobe signals if the bus isn't locked up or when active_ack clears them
            if (bus_busy && active_ack) begin
                led_stb <= 1'b0;
                io_stb <= 1'b0;
                buzzer_stb <= 1'b0;
                encoder_stb <= 1'b0;
                ws0_stb <= 1'b0;
                ws1_stb <= 1'b0;
                system0_stb <= 1'b0;
                system1_stb <= 1'b0;
                unmapped_stb <= 1'b0;
            end

            // If the bus is free and a valid request matches, latch it instantly!
            if (!bus_busy && req_valid_r) begin
                if (ws0_select) begin
                    ws0_addr_r <= req_addr_r;
                    ws0_din_r  <= req_din_r;
                    ws0_we_r   <= req_we_r;
                    ws0_stb    <= 1'b1;
                end
                if (ws1_select) begin
                    ws1_addr_r <= req_addr_r;
                    ws1_din_r  <= req_din_r;
                    ws1_we_r   <= req_we_r;
                    ws1_stb    <= 1'b1;
                end
                if (led_select) begin
                    led_addr_r <= req_addr_r;
                    led_din_r  <= req_din_r;
                    led_we_r   <= req_we_r;
                    led_stb    <= 1'b1;
                end
                if (io_select) begin
                    io_addr_r  <= req_addr_r;
                    io_din_r   <= req_din_r;
                    io_we_r    <= req_we_r;
                    io_stb     <= 1'b1;
                end
                if (buzzer_select) begin
                    buzzer_addr_r <= req_addr_r;
                    buzzer_din_r  <= req_din_r;
                    buzzer_we_r   <= req_we_r;
                    buzzer_stb    <= 1'b1;
                end
                if (encoder_select) begin
                    encoder_addr_r <= req_addr_r;
                    encoder_din_r  <= req_din_r;
                    encoder_we_r   <= req_we_r;
                    encoder_stb    <= 1'b1;
                end
                // FUTURE consider making system0 and system1 real peripherals
                if (system0_select) begin
                    system0_stb <= 1'b1;
                end
                if (system1_select) begin
                    system1_stb <= 1'b1;
                end
                if (unmapped_select) begin
                    unmapped_stb <= 1'b1;
                end
            end
        end

        if (system0_stb && !system0_ack) begin
            //$display("system0 read");
            system0_ack <= 1'b1;
            case (req_addr_r)
                REG_IDENT: global_dout_r <= IDENT;
                REG_VERSION: global_dout_r <= VERSION;
                default: global_dout_r <= 32'hAA55AA55;
            endcase
        end

        if (system1_stb && !system1_ack) begin
            //$display("system1 read");
            system1_ack <= 1'b1;
            case (req_addr_r)
                REG_MARKER: global_dout_r <= MARKER;
                default: global_dout_r <= 32'h55AA55AA;
            endcase
        end

        if (unmapped_stb && !unmapped_ack) begin
            //$display("unmapped read");
            unmapped_ack <= 1'b1;
            global_dout_r <= 32'h99BA_AD99;
        end
    end

endmodule