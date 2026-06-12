module ws2812 #(
    parameter MAX_LEDS = 256
)(
    input  wire        sys_clk,
    input  wire        reset,

    // =========================
    // BUS INTERFACE
    // =========================
    input  wire        bus_we,
    input  wire [5:0]  bus_addr,
    input  wire [31:0] bus_din,
    output reg  [31:0] bus_dout,

    // =========================
    // WS OUTPUT
    // =========================
    output reg         ws_out
);

    // ============================================================
    // ADDRESS MAP
    // ============================================================
    localparam WS_CTRL        = 6'h00;
    localparam WS_TX_CONFIG      = 6'h04;

    localparam WS_DATA_1      = 6'h10;

    // ============================================================
    // CONTROL REGISTERS
    // ============================================================
    reg        enabled;
    reg [1:0]  mode;
    reg [7:0]  num_leds;

    // ============================================================
    // STREAMING STATE
    // ============================================================
    reg [7:0] write_ptr;
    reg [7:0] read_ptr;

    reg        streaming;
    reg        frame_ready;

    localparam ADDR_WIDTH = 8;   // up to 256 entries
    localparam DATA_WIDTH = 32;

    wire [15:0] ram_rdata_lo;
    wire [15:0] ram_rdata_hi;

    reg write_en_r;

    reg  [7:0]  wr_addr;
    reg  [7:0]  rd_addr;

    reg  [31:0] wr_data;
    wire [31:0] rd_data;

    assign rd_data = {ram_rdata_hi, ram_rdata_lo};

    SB_RAM40_4K #(
        .WRITE_MODE(0),
        .READ_MODE(0)
    ) ram_lo (
        .RCLK(sys_clk),
        .RCLKE(1'b1),
        .RE(1'b1),
        .RADDR({3'b000, rd_addr}),
        .RDATA(ram_rdata_lo),

        .WCLK(sys_clk),
        .WCLKE(1'b1),
        .WE(write_en_r),
        .WADDR({3'b000, wr_addr}),
        .WDATA(wr_data[15:0]),
        .MASK(16'h0000)
    );

    SB_RAM40_4K #(
        .WRITE_MODE(0),
        .READ_MODE(0)
    ) ram_hi (
        .RCLK(sys_clk),
        .RCLKE(1'b1),
        .RE(1'b1),
        .RADDR({3'b000, rd_addr}),
        .RDATA(ram_rdata_hi),

        .WCLK(sys_clk),
        .WCLKE(1'b1),
        .WE(write_en_r),
        .WADDR({3'b000, wr_addr}),
        .WDATA(wr_data[31:16]),
        .MASK(16'h0000)
    );

    // ============================================================
    // BUS WRITE CAPTURE (encoder-style)
    // ============================================================
    reg [31:0] sync_reg;
    reg [5:0]  sync_addr;
    reg        strobe_update;

    reg strobe_r1, strobe_r2;

    always @(posedge sys_clk) begin
        if (reset) begin
            strobe_update <= 0;
        end else begin
            strobe_update <= 0;

            if (bus_we) begin
                sync_addr <= bus_addr;
                sync_reg  <= bus_din;
                strobe_update <= 1;
            end
        end
    end

    // ============================================================
    // BUS READ
    // ============================================================
    always @(*) begin
        case (bus_addr)
            WS_CTRL:   bus_dout = {29'b0, mode, enabled};
            WS_TX_CONFIG: bus_dout = {23'b0, num_leds};

            default:   bus_dout = 32'h00000000;
        endcase
    end

    // ============================================================
    // COLOR PACKING FUNCTION
    // ============================================================
    function [31:0] pack_pixel;
        input [31:0] in;
        input [1:0] mode;
        reg [7:0] r, g, b, w;
        begin
            r = in[23:16];
            g = in[15:8];
            b = in[7:0];
            w = in[31:24];

            case (mode)
                2'b00: pack_pixel = {8'd0, r, g, b}; // RGB
                2'b01: pack_pixel = {8'd0, r, g, b}; // RGB1 (same for now)
                2'b10: pack_pixel = {8'd0, g, r, b}; // GRB
                2'b11: pack_pixel = {w,    r, g, b}; // GRBW
            endcase
        end
    endfunction

    // ============================================================
    // STREAMING WRITE LOGIC
    // ============================================================
    integer i;

    wire data_write_region =
        (sync_addr >= WS_DATA_1 && sync_addr <= 6'h2C);

    always @(posedge sys_clk) begin
        write_en_r <= strobe_update && data_write_region;
    end

    always @(posedge sys_clk) begin
        if (reset) begin
            write_ptr  <= 0;
            streaming  <= 0;
            num_leds   <= 0;
            frame_ready <= 0;

        end else begin

            if (strobe_update && sync_addr == WS_CTRL) begin
                mode  <= sync_reg[2:1];
                enabled <= sync_reg[0];
                frame_ready <= 0;
                $display("enabled flag: %1d, mode: 0b%02b", sync_reg[0], sync_reg[2:1]);
            end

            if (strobe_update && sync_addr == WS_TX_CONFIG) begin
                num_leds <= sync_reg[7:0];
                write_ptr <= 0;
                frame_ready <= 0;
            end

            // DATA writes (all map to same behavior)
            if (strobe_update &&
                (sync_addr >= WS_DATA_1 && sync_addr <= 6'h2C)) begin

                frame_ready <= 0;
                
                if (!streaming) begin
                    streaming <= 1;
                end

                if (write_ptr < MAX_LEDS) begin
                    wr_addr <= write_ptr;
                    wr_data <= pack_pixel(sync_reg, mode);
                    write_ptr <= write_ptr + 1;
                end
            end

            // End of stream detection
            if (streaming && write_ptr >= num_leds) begin
                streaming <= 0;
                frame_ready <= 1;
                write_ptr <= 0;
            end
        end
    end

    // ============================================================
    // WS2812 TRANSMITTER FSM
    // ============================================================
    reg [7:0]  led_index;
    reg [5:0]  bit_index;

    reg [23:0] shift_reg;

    localparam T0H = 40;  // ~0.4us @ 100MHz (adjust as needed)
    localparam T1H = 80;  // ~0.8us
    localparam T_TOTAL = 120;
    localparam T_RESET = 8000; // 80us (50us min)


    reg [7:0]  tcount;

    localparam PHASE_RESET     = 2'd0;
    localparam PHASE_FETCH     = 2'd1;
    localparam PHASE_PREPARE   = 2'd2;
    localparam PHASE_TRANSMIT  = 2'd3;

    reg [1:0]  phase;

    reg [31:0] reset_counter;

    always @(posedge sys_clk) begin
        if (reset) begin
            ws_out    <= 0;
            reset_counter <= 0;
            phase     <= PHASE_RESET;
        end else if (enabled && frame_ready) begin

            case (phase)
                PHASE_RESET: begin
                    reset_counter <= reset_counter + 1;
                    if (reset_counter == T_RESET) begin
                        reset_counter <= 0;

                        led_index <= 0;

                        phase <= PHASE_FETCH;
                    end
                end
                PHASE_FETCH: begin
                    $display("fetch");
                    // BRAM is one-cycle delayed
                    rd_addr <= led_index;   // request read
                    phase <= PHASE_PREPARE; // wait 1 cycle
                end
                PHASE_PREPARE: begin
                    $display("prepare");
                    shift_reg <= rd_data[23:0];
                    tcount    <= 0;
                    bit_index <= 23;
                    ws_out <= 1;
                    phase <= PHASE_TRANSMIT;
                end
                PHASE_TRANSMIT: begin
                    // Loaded new LED
                    if (bit_index == 23 && tcount == 0) begin
                        $display("transmit. index: %d, shift_reg: 0x%08h", led_index, shift_reg);
                    end

                    // Timing engine
                    if (tcount < T_TOTAL) begin
                        tcount <= tcount + 1;

                        if (shift_reg[bit_index]) begin
                            ws_out <= (tcount < T1H);
                        end else begin
                            ws_out <= (tcount < T0H);
                        end
                    end else begin
                        ws_out <= 0;

                        tcount <= 0;

                        // next bit
                        if (bit_index == 0) begin
                            bit_index <= 23;

                            // next LED
                            if (led_index == num_leds - 1) begin
                                $display("finished leds");
                                phase <= PHASE_RESET;
                            end else begin
                                $display("next led. led_index: %d", led_index);
                                led_index <= led_index + 1;
                                phase <= PHASE_FETCH;
                            end
                        end else begin
                            bit_index <= bit_index - 1;
                        end
                    end

                end
            endcase

        end else begin
            ws_out <= 0;
        end
    end

endmodule
