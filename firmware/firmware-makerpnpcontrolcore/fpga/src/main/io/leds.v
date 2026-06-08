// Dedicated LED control module
module leds (
    input  reset,
    input  wire        sys_clk,
    input  wire        strobe_update,
    input  wire [31:0] led_ctrl,
    output reg         mcu_act = 0,
    output reg         fpga_act = 0,
    output reg [15:0]  debug
);
    // CDC (Clock Domain Crossing) Flag Catching
    // Because strobe_update /may/ originate from a diffent clock domain a simple pulse synchronizer
    // is used to clean it up for this clock domain.
    reg strobe_sync_r1, strobe_sync_r2;
    reg [31:0] led_ctrl_sync;

    reg activity_flag = 1'b0;

    always @(posedge sys_clk) begin
        if (reset) begin
            strobe_sync_r1 = 1;
            strobe_sync_r2 = 0;

            led_ctrl_sync = 32'd0;
        end else begin
            strobe_sync_r2 = strobe_sync_r1;
            strobe_sync_r1 = strobe_update;

            if (strobe_sync_r1) begin
                led_ctrl_sync = led_ctrl;
            end
        end


        // Act on rising edge transition of our synchronized strobe signal
        if (strobe_sync_r1 && !strobe_sync_r2) begin
            // Bit 0 handles USER FPGA_ACT status
            fpga_act = led_ctrl_sync[0];
            // Bit 1 handles USER MCU_ACT status
            mcu_act = led_ctrl_sync[1];

            $display("LED out (sync): 0x%08h", led_ctrl_sync);
        end

        activity_flag = ~activity_flag;

        //debug = 16'hffff;
        debug = {
            led_ctrl[7:0],
            reset,
            sys_clk,
            fpga_act,
            mcu_act,
            strobe_sync_r1,
            strobe_sync_r2,
            strobe_update,
            activity_flag
        };
    end

endmodule
