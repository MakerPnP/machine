// Dedicated IO control module
module io (
    input  wire        reset,
    input  wire        sys_clk,

    input  wire        bus_stb,
    input  wire        bus_we,
    input  wire [7:0]  bus_addr,
    input  wire [31:0] bus_din,
    output reg  [31:0] bus_dout,
    output reg         bus_ack,

    input  wire [1:0]  btn,
    input  wire [1:0]  iak,
    input  wire [7:0]  din,
    output wire [1:0]  oec,
    output wire [1:0]  adc_mux,
    input  wire        base_present,
    input  wire [3:0]  port_present,

    output reg [15:0]  debug
);

    `include "src/main/io/io_regs.svh"

    reg [31:0] io_ctrl;

    // port_present (4), base_present (1), iak (2), btn (2) = 9 bits
    wire [8:0] io_in_1;
    wire [7:0] io_in_2;

    // Duplicated registers to isolate internal vs external logic paths
    (* keep *)
    reg [3:0] io_out_1_r;
    reg [1:0] adc_mux_r;
    reg [1:0] oec_r;

    reg        strobe_update;

    reg [8:0]  io_sync_m;
    reg [8:0]  io_sync_s;

    reg [7:0]  din_sync_m;
    reg [7:0]  din_sync_s;

    SB_IO #(.PIN_TYPE(6'b0101_00)) io_adc_mux1 (
        .PACKAGE_PIN(adc_mux[0]),
        .OUTPUT_CLK(sys_clk),
        .D_OUT_0(reset ? 1'b0 : adc_mux_r[0])
    );

    SB_IO #(.PIN_TYPE(6'b0101_00)) io_adc_mux2 (
        .PACKAGE_PIN(adc_mux[1]),
        .OUTPUT_CLK(sys_clk),
        .D_OUT_0(reset ? 1'b0 : adc_mux_r[1])
    );

    SB_IO #(.PIN_TYPE(6'b0101_00)) io_oec1 (
        .PACKAGE_PIN(oec[0]),
        .OUTPUT_CLK(sys_clk),
        .D_OUT_0(reset ? 1'b0 : oec_r[0])
    );

    SB_IO #(.PIN_TYPE(6'b0101_00)) io_oec2 (
        .PACKAGE_PIN(oec[1]),
        .OUTPUT_CLK(sys_clk),
        .D_OUT_0(reset ? 1'b0 : oec_r[1])
    );

    // CDC (Clock Domain Crossing) Flag Catching
    // Because strobe_update /may/ originate from a diffent clock domain a simple pulse synchronizer
    // is used to clean it up for this clock domain.
    reg        strobe_sync_r1, strobe_sync_r2;
    reg        activity_flag;

    // --- 1. Synchronous Register Writes & Local Strobes ---
    always @(posedge sys_clk) begin
        if (reset) begin
            io_ctrl         <= 32'd0;
            strobe_update   <= 1'b1;
            io_out_1_r      <= 4'b0000;
            oec_r             <= 2'b00;
            adc_mux_r         <= 2'b00;
            bus_dout        <= 32'h00000000;
            bus_ack         <= 1'b0;
        end else begin
            // Automatic self-clearing single-cycle strobe pulse
            strobe_update  <= 1'b0;

            if (bus_stb) begin
                if (!bus_ack) begin
                    // Process writes only when a cycle is valid, a write is asserted, and we haven't acknowledged yet
                    bus_ack <= 1'b1;
                    if (bus_we) begin
                        $display("io bus write. addr: %02x, value: %08h", bus_addr, bus_din);
                        case (bus_addr)
                            REG_IO_CTRL: begin
                                io_ctrl       <= bus_din;
                                strobe_update <= 1'b1;
                            end
                            REG_IO_OUT_1: begin
                                // Update internal readback register
                                io_out_1_r  <= {bus_din[9:8], bus_din[1:0]};
                                // Update isolated top-level registers (clean IOB packing)
                                oec_r       <= bus_din[1:0];
                                adc_mux_r   <= bus_din[9:8];
                            end
                            default: begin end
                        endcase
                    end else begin
                        // Process reads cleanly from the fabric-only copy
                        case (bus_addr)
                            REG_IO_CTRL:   bus_dout <= io_ctrl;
                            REG_IO_IN_1:   bus_dout <= {16'd0, io_in_1[8:5], 3'b000, io_in_1[4:4], 4'b0000, io_in_1[3:0]};
                            REG_IO_IN_2:   bus_dout <= {24'd0, io_in_2};
                            REG_IO_OUT_1:   bus_dout <= {22'd0, io_out_1_r[3:2], 6'd0, io_out_1_r[1:0]};
                            default: bus_dout <= 32'h33333333;
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
            io_sync_m      <= 9'd0;
            io_sync_s      <= 9'd0;
            din_sync_m     <= 8'd0;
            din_sync_s     <= 8'd0;
        end else begin
            strobe_sync_r2 <= strobe_sync_r1;
            strobe_sync_r1 <= strobe_update;

            // Act on rising edge transition of our synchronized strobe signal
            if (strobe_sync_r1 && !strobe_sync_r2) begin
                // TODO use io_ctrl_sync as required
                $display("IO_CTRL: 0x%08h", io_ctrl);
            end

            io_sync_m  <= {port_present, base_present, iak, btn};
            io_sync_s  <= io_sync_m;

            din_sync_m <= din;
            din_sync_s <= din_sync_m;

            activity_flag <= ~activity_flag;

            //debug <= 16'hffff;
            debug <= {
//                io_ctrl[7:0],
                8'd0,
                reset,
                sys_clk,
                io_in_1[1:0],
                strobe_sync_r1,
                strobe_sync_r2,
                strobe_update,
                activity_flag
            };
        end
    end

    // Map buttons to io_in_1 (Bit 0 = USER 0, Bit 1 = USER 1)
    // Inverted (~btn) because external circuit pulls up to 3V3 (Pressed = 0)
    //
    // Map IAK to io_in_1 (Bit 2 = IAK1, Bit 1 = IAK2)
    // Inverted, as inputs are via optical isolators, active-low.
    //
    // Map present signals, non-inverted
    assign io_in_1  = {io_sync_s[8:4], ~io_sync_s[3:0]};

    // Map DIN to io_in_2
    // Inverted (~btn) because external circuit pulls up to 5V5 though a octal bus tranceiver.
    assign io_in_2  = ~din_sync_s[7:0];

endmodule
