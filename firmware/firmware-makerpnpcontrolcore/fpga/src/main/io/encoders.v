// encoders.v
// Example of an internal module managing registers in real time
module encoders(
    input  wire        sys_clk,
    input  wire        reset,
    
    // Bus Slave Interface
    input  wire        bus_stb,
    input  wire        bus_we,
    input  wire [7:0]  bus_addr,
    input  wire [31:0] bus_din,
    output reg  [31:0] bus_dout,
    output reg         bus_ack,

    input  wire [2:0]  abz_a,
    input  wire [2:0]  abz_b,
    input  wire [2:0]  abz_c,
    input  wire [2:0]  abz_x,
    input  wire [2:0]  abz_y,
    input  wire [2:0]  abz_z,

    output reg [15:0]  debug
);

    `include "src/main/io/encoders_regs.svh"

    reg        strobe_encoder_reset;

    wire [15:0] encoder_count [6];
    reg [15:0] encoder_set_value_a;
    reg [15:0] encoder_set_value_b;
    reg [15:0] encoder_set_value_c;
    reg [15:0] encoder_set_value_x;
    reg [15:0] encoder_set_value_y;
    reg [15:0] encoder_set_value_z;

    // one flag for each encoder
    reg [5:0] encoder_set;

    encoder encoder_a_inst (
        .sys_clk(sys_clk),
        .reset(strobe_encoder_reset),
        .count(encoder_count[0]),
        .set_value(encoder_set_value_a),
        .set(encoder_set[0]),
        .abz(abz_a)
    );
    encoder encoder_b_inst (
        .sys_clk(sys_clk),
        .reset(strobe_encoder_reset),
        .count(encoder_count[1]),
        .set_value(encoder_set_value_b),
        .set(encoder_set[1]),
        .abz(abz_b)
    );
    encoder encoder_c_inst (
        .sys_clk(sys_clk),
        .reset(strobe_encoder_reset),
        .count(encoder_count[2]),
        .set_value(encoder_set_value_c),
        .set(encoder_set[2]),
        .abz(abz_c)
    );
    encoder encoder_x_inst (
        .sys_clk(sys_clk),
        .reset(strobe_encoder_reset),
        .count(encoder_count[3]),
        .set_value(encoder_set_value_x),
        .set(encoder_set[3]),
        .abz(abz_x)
    );
    encoder encoder_y_inst (
        .sys_clk(sys_clk),
        .reset(strobe_encoder_reset),
        .count(encoder_count[4]),
        .set_value(encoder_set_value_y),
        .set(encoder_set[4]),
        .abz(abz_y)
    );
    encoder encoder_z_inst (
        .sys_clk(sys_clk),
        .reset(strobe_encoder_reset),
        .count(encoder_count[5]),
        .set_value(encoder_set_value_z),
        .set(encoder_set[5]),
        .abz(abz_z)
    );

    reg [31:0] enc_ctrl;

    reg [31:0] sync_reg;
    reg [7:0]  sync_addr;
    reg        strobe_update;

    // CDC (Clock Domain Crossing) Flag Catching
    // Because strobe_update /may/ originate from a diffent clock domain a simple pulse synchronizer
    // is used to clean it up for this clock domain.
    reg        strobe_sync_r1, strobe_sync_r2;
    reg        activity_flag;

    // --- Local Read/Write & Command Decoder ---
    always @(posedge sys_clk) begin
        if (reset) begin
            strobe_update  <= 1'b1;
            bus_dout        <= 32'h00000000;
            bus_ack         <= 1'b0;
        end else begin
            // Automatic self-clearing single-cycle strobe pulse
            strobe_update  <= 1'b0;

            if (bus_stb) begin
                if (!bus_ack) begin
                    bus_ack <= 1'b1;
                    if (bus_we) begin
                        $display("encoder bus write. addr: %02x, value: %08h", bus_addr, bus_din);
                        sync_addr = bus_addr;
                        sync_reg = bus_din;
                        strobe_update <= 1'b1;
                    end else begin
                        case (bus_addr)
                            REG_ENC_CTRL: bus_dout <= enc_ctrl;
                            // 6'h04-18 - write only (set count)
                            REG_ENC_COUNT_A: bus_dout <= {16'd0, encoder_count[0]};
                            REG_ENC_COUNT_B: bus_dout <= {16'd0, encoder_count[1]};
                            REG_ENC_COUNT_C: bus_dout <= {16'd0, encoder_count[2]};
                            REG_ENC_COUNT_X: bus_dout <= {16'd0, encoder_count[3]};
                            REG_ENC_COUNT_Y: bus_dout <= {16'd0, encoder_count[4]};
                            REG_ENC_COUNT_Z: bus_dout <= {16'd0, encoder_count[5]};
                            default: bus_dout <= 32'h22222222;
                        endcase
                    end
                end
            end else begin
                bus_ack <= 1'b0;
            end
        end
    end

    reg initialized = 0;

    // --- Internal Business Logic / CDC Core ---
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
                $display("ENC strobe");
                case (sync_addr)
                    REG_ENC_CTRL: begin
                        $display("ENC_CTRL update");
                        // Bit 0 handles RESET
                        if (sync_reg[0] == 1'b1) begin
                            $display("Enable encoder reset strobe");
                            strobe_encoder_reset <= 1'b1;
                        end
                    end
                    REG_ENC_SET_COUNT_A: begin
                        $display("ENC_SET_COUNT_A update. value: 0x%08h", sync_reg);
                        encoder_set_value_a <= sync_reg;
                        encoder_set <= encoder_set | 6'b000001;
                    end
                    REG_ENC_SET_COUNT_B: begin
                        $display("ENC_SET_COUNT_B update. value: 0x%08h", sync_reg);
                        encoder_set_value_b <= sync_reg;
                        encoder_set <= encoder_set | 6'b000010;
                    end
                    REG_ENC_SET_COUNT_C: begin
                        $display("ENC_SET_COUNT_C update. value: 0x%08h", sync_reg);
                        encoder_set_value_c <= sync_reg;
                        encoder_set <= encoder_set | 6'b000100;
                    end
                    REG_ENC_SET_COUNT_X: begin
                        $display("ENC_SET_COUNT_X update. value: 0x%08h", sync_reg);
                        encoder_set_value_x <= sync_reg;
                        encoder_set <= encoder_set | 6'b001000;
                    end
                    REG_ENC_SET_COUNT_Y: begin
                        $display("ENC_SET_COUNT_Y update. value: 0x%08h", sync_reg);
                        encoder_set_value_y <= sync_reg;
                        encoder_set <= encoder_set | 6'b010000;
                    end
                    REG_ENC_SET_COUNT_Z: begin
                        $display("ENC_SET_COUNT_Z update. value: 0x%08h", sync_reg);
                        encoder_set_value_z <= sync_reg;
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
