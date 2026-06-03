// leds.v
// Dedicated LED control module (Verilog-2001 compliant)
module leds (
    input  wire       sys_clk,
    input  wire       strobe_led_update, // High for 1 SCK cycle when address 0x20 is written
    input  wire [7:0] led_out,   // Parallel byte written by MCU
    output reg        mcu_act,
    output reg        fpga_act
);

    // Initial hardware boot configuration
    initial begin
        mcu_act  = 1'b0;
        fpga_act = 1'b1; // Default ON to confirm FPGA fabric is running
    end

    // CDC (Clock Domain Crossing) Flag Catching
    // Because strobe_led_update originates from the MCU's QSPI clock domain,
    // we use a simple pulse synchronizer to clean it up for our sys_clk domain.
    reg strobe_sync_r1, strobe_sync_r2;
    reg [7:0] led_out_sync;

    always @(posedge sys_clk) begin
        strobe_sync_r1 <= strobe_led_update;
        strobe_sync_r2 <= strobe_sync_r1;
        
        if (strobe_sync_r1) begin
            led_out_sync <= led_out;
        end

        // Act on rising edge transition of our synchronized strobe signal
        if (strobe_sync_r1 && !strobe_sync_r2) begin
            mcu_act <= led_out_sync[0]; // Bit 0 handles USER MCU_ACT status
        end
    end

endmodule
