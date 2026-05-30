// 100Mhz input clock
// 1/100,000,000 = 0.000_000_01 = 10ns
// 0.000_000_01 * 100 = 0.000001 = 1us
// 0.000001 (1us) * 1000 = 0.001 = 1ms
// 0.001 (1ms) * 250 = .25 = 250ms
// recap: 1/100,000,000 * 100 * 1000 * 250 = 0.25
// 100,000,000 / 0.25 = 25_000_000
// 1 / 0.25 = 4
// 100,000,000 / 4 = 25_000_000

// 4 edge transitions per second = 2 low->high transitions/second = led flashes twice per second

module blink
    #(parameter SPEED = 25_000_000)
    (
        input reset,
        input clk,
        output reg led = 0
    );

reg [31:0] counter = 0;

always @(posedge clk) begin
    if (reset) begin
        counter <= 0;
        led <= 0;
    end else begin
        if (counter == SPEED) begin
            led <= ~led;
            // reset to 1 not zero here to keep timing exact.
            counter <= 1;
        end else begin
            counter <= counter + 1;
        end
    end
end

endmodule
