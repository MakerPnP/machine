// Dedicated LED control module
module buzzer (
    input  wire        reset,
    input  wire        sys_clk,

    // Bus Slave Interface
    input  wire        bus_stb,
    input  wire        bus_we,
    input  wire [7:0]  bus_addr,
    input  wire [31:0] bus_din,
    output reg  [31:0] bus_dout,
    output reg         bus_ack,

    output wire        buzzer,

    output reg [15:0]  debug
);

    `include "src/main/io/buzzer_regs.svh"

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
            bus_dout       <= 32'h00000000;
            bus_ack        <= 1'b0;
        end else begin
            // Automatic self-clearing single-cycle strobe pulse
            strobe_update  <= 1'b0;

            if (bus_stb) begin
                if (!bus_ack) begin
                    bus_ack <= 1'b1;
                    if (bus_we) begin
                        $display("led bus write. addr: %02x, value: %08h", bus_addr, bus_din);
                        case (bus_addr)
                            REG_BUZZER_CTRL: begin
                                buzzer_ctrl     <= bus_din;
                                strobe_update   <= 1'b1;
                            end
                            default: begin end
                        endcase
                    end else begin
                        case (bus_addr)
                            REG_BUZZER_CTRL:   bus_dout <= buzzer_ctrl;
                            default: bus_dout <= 32'h11111111;
                        endcase
                    end
                end
            end else begin
                bus_ack <= 1'b0;
            end
        end
    end

    // --- 3. Internal Business Logic / CDC Core ---
    always @(posedge sys_clk) begin
        if (reset) begin
            strobe_sync_r1 <= 1'b1;
            strobe_sync_r2 <= 1'b0;
            activity_flag  <= 1'b0;
            debug          <= 16'd0;
        end else begin
            strobe_sync_r2 <= strobe_sync_r1;
            strobe_sync_r1 <= strobe_update;

            // Act on rising edge transition of our synchronized strobe signal
            if (strobe_sync_r1 && !strobe_sync_r2) begin
                // Bit 0 enables the buzzer

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

    assign buzzer = buzzer_ctrl[0];

endmodule
