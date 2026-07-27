module stepper_clk (
    input  wire         reset,
    input  wire         sys_clk,
    output wire         stepper_clk
);

    reg [8:0] base_clk_div;

    assign stepper_clk = (base_clk_div == 9'd499);

    always @(posedge sys_clk) begin
        if (reset) begin
            base_clk_div <= 9'd0;
        end else begin
            if (stepper_clk)
                base_clk_div <= 9'd0;
            else
                base_clk_div <= base_clk_div + 1'b1;
        end
    end

endmodule
