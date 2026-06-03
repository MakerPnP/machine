// encoders.v
// Example of an internal module managing registers in real time
module encoders (
    input  wire        sys_clk,
    input  wire        strobe_encoder_reset, // From memory.v write decoder
    // Dummy inputs representing pulse tracks
    input  wire [5:0]  encoder_hardware_pins, 
    
    // Read registers routed straight back to memory.v multiplexer
    output reg  [31:0] enc_1,
    output reg  [31:0] enc_2,
    output reg  [31:0] enc_3,
    output reg  [31:0] enc_4,
    output reg  [31:0] enc_5,
    output reg  [31:0] enc_6
);

    // Sync reset strobe across clock domain boundaries safely if needed
    // (For simplicity in this code block snippet, we assume short synchronous pulse alignment)
    always @(posedge sys_clk) begin
        if (strobe_encoder_reset) begin
            enc_1 <= 32'd0;
            enc_2 <= 32'd0;
            enc_3 <= 32'd0;
            enc_4 <= 32'd0;
            enc_5 <= 32'd0;
            enc_6 <= 32'd0;
        end else begin
            // Your optical hardware edge-counting logic runs completely uninhibited here!
            // Example increment trace:
            if (encoder_hardware_pins[0]) enc_1 <= enc_1 + 32'd1;
            if (encoder_hardware_pins[1]) enc_2 <= enc_2 + 32'd1;
        end
    end

endmodule
