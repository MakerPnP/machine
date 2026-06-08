// Dedicated IO control module
module io (
    input  wire        reset,
    input  wire        sys_clk,

    // Bus Slave Interface
    input  wire        bus_we,
    input  wire [5:0]  bus_addr,
    input  wire [31:0] bus_din,
    output reg  [31:0] bus_dout,

    input wire         user_0,
    input wire         user_1,

    output reg [15:0]  debug
);

    reg [31:0] io_ctrl;
    wire [31:0] io_in_1;
    reg        strobe_update;

    reg [1:0]  btn_sync_m;
    reg [1:0]  btn_sync_s;

    // CDC (Clock Domain Crossing) Flag Catching
    // Because strobe_update /may/ originate from a diffent clock domain a simple pulse synchronizer
    // is used to clean it up for this clock domain.
    reg        strobe_sync_r1, strobe_sync_r2;
    reg        activity_flag;

    // --- 1. Synchronous Register Writes & Local Strobes ---
    always @(posedge sys_clk) begin
        if (reset) begin
            io_ctrl       <= 32'd0;
            strobe_update  <= 1'b1;
        end else begin
            // Automatic self-clearing single-cycle strobe pulse
            strobe_update  <= 1'b0;

            if (bus_we) begin
                $display("io bus write. addr: %02x, value: %08h", bus_addr, bus_din);
                case (bus_addr)
                    6'h00: begin
                        io_ctrl       <= bus_din;
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
            6'h00:   bus_dout = io_ctrl;
            6'h04:   bus_dout = io_in_1;
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
        end else begin
            strobe_sync_r2 <= strobe_sync_r1;
            strobe_sync_r1 <= strobe_update;

            // Act on rising edge transition of our synchronized strobe signal
            if (strobe_sync_r1 && !strobe_sync_r2) begin
                // TODO use io_ctrl_sync as required
                $display("IO_CTRL: 0x%08h", io_ctrl);
            end

            btn_sync_m <= {30'd0, user_1, user_0};
            btn_sync_s <= btn_sync_m;

            activity_flag <= ~activity_flag;

            //debug <= 16'hffff;
            debug <= {
                io_in_1[7:0],
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

    // Map buttons to reg_io_in_1 (Bit 0 = USER 0, Bit 1 = USER 1)
    // Inverted (~btn) because external circuit pulls up to 3V3 (Pressed = 0)
    assign io_in_1 = {30'd0, ~btn_sync_s[1], ~btn_sync_s[0]};

endmodule
