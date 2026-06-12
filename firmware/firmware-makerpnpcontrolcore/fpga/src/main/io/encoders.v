// encoders.v
// Example of an internal module managing registers in real time
module encoders(
    input  wire        sys_clk,
    input  wire        reset,
    
    // Bus Slave Interface
    input  wire        bus_we,
    input  wire [5:0]  bus_addr,
    input  wire [31:0] bus_din,
    output reg  [31:0] bus_dout,

    input  wire [2:0]  abz_a,
    input  wire [2:0]  abz_b,
    input  wire [2:0]  abz_c,
    input  wire [2:0]  abz_x,
    input  wire [2:0]  abz_y,
    input  wire [2:0]  abz_z,

    output reg [15:0]  debug
);

    reg        strobe_encoder_reset;

    wire [31:0] encoder_count [6];
    reg [31:0] encoder_set_value [6];

    reg [5:0] encoder_set;

    encoder encoder_a_inst (
        .sys_clk(sys_clk),
        .reset(strobe_encoder_reset),
        .count(encoder_count[0]),
        .set_value(encoder_set_value[0]),
        .set(encoder_set[0]),
        .abz(abz_a)
    );
    encoder encoder_b_inst (
        .sys_clk(sys_clk),
        .reset(strobe_encoder_reset),
        .count(encoder_count[1]),
        .set_value(encoder_set_value[1]),
        .set(encoder_set[1]),
        .abz(abz_b)
    );
    encoder encoder_c_inst (
        .sys_clk(sys_clk),
        .reset(strobe_encoder_reset),
        .count(encoder_count[2]),
        .set_value(encoder_set_value[2]),
        .set(encoder_set[2]),
        .abz(abz_c)
    );
    encoder encoder_x_inst (
        .sys_clk(sys_clk),
        .reset(strobe_encoder_reset),
        .count(encoder_count[3]),
        .set_value(encoder_set_value[3]),
        .set(encoder_set[3]),
        .abz(abz_x)
    );
    encoder encoder_y_inst (
        .sys_clk(sys_clk),
        .reset(strobe_encoder_reset),
        .count(encoder_count[4]),
        .set_value(encoder_set_value[4]),
        .set(encoder_set[4]),
        .abz(abz_y)
    );
    encoder encoder_z_inst (
        .sys_clk(sys_clk),
        .reset(strobe_encoder_reset),
        .count(encoder_count[5]),
        .set_value(encoder_set_value[5]),
        .set(encoder_set[5]),
        .abz(abz_z)
    );

    reg [31:0] enc_ctrl;

    reg [31:0] sync_reg;
    reg [5:0]  sync_addr;
    reg        strobe_update;

    // CDC (Clock Domain Crossing) Flag Catching
    // Because strobe_update /may/ originate from a diffent clock domain a simple pulse synchronizer
    // is used to clean it up for this clock domain.
    reg        strobe_sync_r1, strobe_sync_r2;
    reg        activity_flag;

    // --- 1. Local Write & Command Decoder ---
    always @(posedge sys_clk) begin
        if (reset) begin
            strobe_update  <= 1'b1;
        end else begin
            // Automatic self-clearing single-cycle strobe pulse
            strobe_update  <= 1'b0;

            if (bus_we) begin
                $display("encoder bus write. addr: %02x, value: %08h", bus_addr, bus_din);
                sync_addr = bus_addr;
                sync_reg = bus_din;
                strobe_update <= 1'b1;
            end
        end
    end

    // --- 2. Localized Combinational Read Multiplexer ---
    always @(*) begin
        case (bus_addr)
            6'h00: bus_dout = enc_ctrl;
            // 6'h04-18 - write only (set count)
            6'h20: bus_dout = encoder_count[0];
            6'h24: bus_dout = encoder_count[1];
            6'h28: bus_dout = encoder_count[2];
            6'h2c: bus_dout = encoder_count[3];
            6'h30: bus_dout = encoder_count[4];
            6'h34: bus_dout = encoder_count[5];
            default: bus_dout = 32'hFFFFFFFF;
        endcase
    end

    reg initialized = 0;

    // --- 3. Internal Business Logic / CDC Core ---
    always @(posedge sys_clk) begin
        if (reset) begin
            enc_ctrl       <= {24'd0, 8'b0000_0000};
            strobe_sync_r1 <= 1'b1;
            strobe_sync_r2 <= 1'b0;
            activity_flag  <= 1'b0;
            debug          <= 16'd0;
            strobe_encoder_reset <= 1'b1;
            encoder_set    <= 6'd0;
        end else begin
            strobe_sync_r2 <= strobe_sync_r1;
            strobe_sync_r1 <= strobe_update;

            // Act on rising edge transition of our synchronized strobe signal
            if (strobe_sync_r1 && !strobe_sync_r2) begin
                case (sync_addr)
                    6'h00: begin
                        $display("ENC_CTRL update");
                        // Bit 0 handles RESET
                        if (sync_reg[0] == 1'b1) begin
                            $display("Enable encoder reset strobe");
                            strobe_encoder_reset <= 1'b1;
                        end
                    end
                    6'h04: begin
                        $display("ENC_SET_COUNT_A update. value: 0x%08h", sync_reg);
                        encoder_set_value[0] <= sync_reg;
                        encoder_set <= encoder_set | 6'b000001;
                    end
                    6'h08: begin
                        $display("ENC_SET_COUNT_B update. value: 0x%08h", sync_reg);
                        encoder_set_value[1] <= sync_reg;
                        encoder_set <= encoder_set | 6'b000010;
                    end
                    6'h0C: begin
                        $display("ENC_SET_COUNT_C update. value: 0x%08h", sync_reg);
                        encoder_set_value[2] <= sync_reg;
                        encoder_set <= encoder_set | 6'b000100;
                    end
                    6'h10: begin
                        $display("ENC_SET_COUNT_X update. value: 0x%08h", sync_reg);
                        encoder_set_value[3] <= sync_reg;
                        encoder_set <= encoder_set | 6'b001000;
                    end
                    6'h14: begin
                        $display("ENC_SET_COUNT_Y update. value: 0x%08h", sync_reg);
                        encoder_set_value[4] <= sync_reg;
                        encoder_set <= encoder_set | 6'b010000;
                    end
                    6'h18: begin
                        $display("ENC_SET_COUNT_Z update. value: 0x%08h", sync_reg);
                        encoder_set_value[5] <= sync_reg;
                        encoder_set <= encoder_set | 6'b100000;
                    end
                endcase

                $display("ENC_CTRL: 0x%08h", enc_ctrl);
                $display("ENC_COUNT_A: 0x%08h", encoder_count[0]);
                $display("ENC_COUNT_B: 0x%08h", encoder_count[1]);
                $display("ENC_COUNT_C: 0x%08h", encoder_count[2]);
                $display("ENC_COUNT_X: 0x%08h", encoder_count[3]);
                $display("ENC_COUNT_Y: 0x%08h", encoder_count[4]);
                $display("ENC_COUNT_Z: 0x%08h", encoder_count[5]);
            end

            if (strobe_encoder_reset) begin
                strobe_encoder_reset <= 1'b0;
                // auto-clear the flag
                enc_ctrl[0] <= 0;
            end

            if (encoder_set > 6'd0) begin
                encoder_set <= 0;
            end

            activity_flag <= ~activity_flag;

            //debug <= 16'hffff;
            debug <= {
                enc_ctrl[7:0],
                reset,
                sys_clk,
                2'b00,
                strobe_sync_r1,
                strobe_sync_r2,
                strobe_update,
                activity_flag
            };
        end
    end

endmodule
