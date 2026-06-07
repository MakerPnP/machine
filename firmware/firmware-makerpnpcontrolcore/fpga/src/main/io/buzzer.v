module buzzer (
    input  reset,
    input  wire       sys_clk,
    input  wire       strobe_update,
    input  wire [7:0] buzzer_ctrl,
    output reg        buzzer = 0,
    output reg [15:0] debug
);
    // CDC (Clock Domain Crossing) Flag Catching
    // Because strobe /may/ originate from a diffent clock domain a simple pulse synchronizer
    // is used to clean it up for this clock domain.
    reg strobe_sync_r1, strobe_sync_r2;
    reg [7:0] buzzer_ctrl_sync;

    reg activity_flag = 1'b0;

    always @(posedge sys_clk) begin
        if (reset) begin
            strobe_sync_r1 = 1;
            strobe_sync_r2 = 0;

            buzzer_ctrl_sync = 8'b0000_0000;
        end else begin
            strobe_sync_r2 = strobe_sync_r1;
            strobe_sync_r1 = strobe_update;

            if (strobe_sync_r1) begin
                buzzer_ctrl_sync = buzzer_ctrl;
            end
        end


        // Act on rising edge transition of our synchronized strobe signal
        if (strobe_sync_r1 && !strobe_sync_r2) begin
            // Bit 0 enables the buzzer
            buzzer = buzzer_ctrl_sync[0];

            $display("Buzzer out (sync): 0x%02h", buzzer_ctrl_sync);
        end

        activity_flag = ~activity_flag;

        debug = {
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

endmodule
