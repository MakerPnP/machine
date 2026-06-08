// encoders.v
// Example of an internal module managing registers in real time
module encoders (
    input  wire        sys_clk,
    input  wire        reset,
    
    // Bus Slave Interface
    input  wire        bus_we,
    input  wire [5:0]  bus_addr,
    input  wire [31:0] bus_din,
    output reg  [31:0] bus_dout,

    input  wire [5:0]  encoder_hardware_pins,

    output reg [15:0]  debug
);

    reg [31:0] enc_ctrl;
    reg        strobe_update;

    reg [31:0] enc_1, enc_2, enc_3, enc_4, enc_5, enc_6;
    reg        strobe_encoder_reset;

    // CDC (Clock Domain Crossing) Flag Catching
    // Because strobe_update /may/ originate from a diffent clock domain a simple pulse synchronizer
    // is used to clean it up for this clock domain.
    reg        strobe_sync_r1, strobe_sync_r2;
    reg        activity_flag;

    // --- 1. Local Write & Command Decoder ---
    always @(posedge sys_clk) begin
        if (reset) begin
            enc_ctrl       <= {24'd0, 8'b0000_0000};
            strobe_update  <= 1'b1;
        end else begin
            // Automatic self-clearing single-cycle strobe pulse
            strobe_update  <= 1'b0;

            if (bus_we) begin
                $display("encoder bus write. addr: %02x, value: %08h", bus_addr, bus_din);
                case (bus_addr)
                    6'h00: begin
                        enc_ctrl      <= bus_din;
                        strobe_update <= 1'b1;
                    end
                    default: begin end
                endcase
            end

            if (strobe_encoder_reset) begin
                // auto-clear the flag
                enc_ctrl[0] <= 0;
            end
        end
    end

    // --- 2. Localized Combinational Read Multiplexer ---
    always @(*) begin
        case (bus_addr)
            6'h00: bus_dout = enc_ctrl;
            6'h20: bus_dout = enc_1;
            6'h24: bus_dout = enc_2;
            6'h28: bus_dout = enc_3;
            6'h2C: bus_dout = enc_4;
            6'h30: bus_dout = enc_5;
            6'h34: bus_dout = enc_6;
            default: bus_dout = 32'hFFFFFFFF;
        endcase
    end


    // --- 3. Internal Business Logic / CDC Core ---
    always @(posedge sys_clk) begin
        if (reset) begin
            strobe_sync_r1 <= 1'b1;
            strobe_sync_r2 <= 1'b0;
            activity_flag  <= 1'b0;
            debug          <= 16'd0;
            strobe_encoder_reset <= 1'b0;
        end else begin
            strobe_sync_r2 <= strobe_sync_r1;
            strobe_sync_r1 <= strobe_update;

            // Act on rising edge transition of our synchronized strobe signal
            if (strobe_sync_r1 && !strobe_sync_r2) begin
                // Bit 0 handles RESET
                if (enc_ctrl[0] == 1'b1) begin
                    $display("Enable encoder reset strobe");
                    strobe_encoder_reset <= 1'b1;
                end

                $display("ENC_CTRL: 0x%08h", enc_ctrl);
            end

            if (strobe_encoder_reset) begin
                strobe_encoder_reset <= 1'b0;
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


    // --- 4. Hardware Counting Logic ---
    always @(posedge sys_clk) begin
        if (reset || strobe_encoder_reset) begin
            enc_1 <= 32'd0;
            enc_2 <= 32'd0;
            enc_3 <= 32'd0;
            enc_4 <= 32'd0;
            enc_5 <= 32'd0;
            enc_6 <= 32'd0;
        end else begin
            // TODO implement something useful here
            if (encoder_hardware_pins[0]) enc_1 <= enc_1 + 32'd1;
            if (encoder_hardware_pins[1]) enc_2 <= enc_2 + 32'd1;
            if (encoder_hardware_pins[1]) enc_3 <= enc_3 + 32'd1;
            if (encoder_hardware_pins[1]) enc_4 <= enc_4 + 32'd1;
            if (encoder_hardware_pins[1]) enc_5 <= enc_5 + 32'd1;
            if (encoder_hardware_pins[1]) enc_6 <= enc_6 + 32'd1;
        end
    end

endmodule
