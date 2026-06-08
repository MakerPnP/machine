// Dedicated LED control module
module buzzer (
    input  wire        reset,
    input  wire        sys_clk,

    // Bus Slave Interface
    input  wire        bus_we,
    input  wire [5:0]  bus_addr,
    input  wire [31:0] bus_din,
    output reg  [31:0] bus_dout,

    output reg         buzzer,

    output reg [15:0]  debug
);

    reg [31:0] buzzer_ctrl;
    reg        strobe_update;

    // CDC (Clock Domain Crossing) Flag Catching
    // Because strobe_update /may/ originate from a diffent clock domain a simple pulse synchronizer
    // is used to clean it up for this clock domain.
    reg        strobe_sync_r1, strobe_sync_r2;
    reg        activity_flag;

    // --- 1. Synchronous Register Writes & Local Strobes ---
    always @(posedge sys_clk) begin
        if (reset) begin
            buzzer_ctrl       <= {24'd0, 8'b0000_0000};
            strobe_update  <= 1'b1;
        end else begin
            // Automatic self-clearing single-cycle strobe pulse
            strobe_update  <= 1'b0;

            if (bus_we) begin
                $display("led bus write. addr: %02x, value: %08h", bus_addr, bus_din);
                case (bus_addr)
                    6'h00: begin
                        buzzer_ctrl      <= bus_din;
                        strobe_update <= 1'b1;
                    end
                    default: begin end
                endcase
            end
        end
    end

    // --- 2. Instantaneous Combinational Readback ---
    always @(*) begin
        case (bus_addr)
            6'h00:   bus_dout = buzzer_ctrl;
            default: bus_dout = 32'hFFFFFFFF;
        endcase
    end

    // --- 3. Internal Business Logic / CDC Core ---
    always @(posedge sys_clk) begin
        if (reset) begin
            strobe_sync_r1 <= 1'b1;
            strobe_sync_r2 <= 1'b0;
            buzzer         <= 1'b0;
            activity_flag  <= 1'b0;
            debug          <= 16'd0;
        end else begin
            strobe_sync_r2 <= strobe_sync_r1;
            strobe_sync_r1 <= strobe_update;

            // Act on rising edge transition of our synchronized strobe signal
            if (strobe_sync_r1 && !strobe_sync_r2) begin
                // Bit 0 enables the buzzer
                buzzer <= buzzer_ctrl[0];

                $display("BUZZER_CTRL: 0x%02h", buzzer_ctrl);

            end

            activity_flag <= ~activity_flag;

            //debug <= 16'hffff;
            debug <= {
                buzzer_ctrl[7:0],
                reset,
                sys_clk,
                1'b0,
                buzzer,
                strobe_sync_r1,
                strobe_sync_r2,
                strobe_update,
                activity_flag
            };
        end
    end

endmodule
