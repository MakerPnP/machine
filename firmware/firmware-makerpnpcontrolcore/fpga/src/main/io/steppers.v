// Pipelined Interconnect Controller with Pre-Split 16-Bit Register Architecture
module steppers (
    input  wire        reset,
    input  wire        sys_clk,

    input  wire        stepper_clk,

    // Bus Slave Interface
    input  wire        bus_stb,
    input  wire        bus_we,
    input  wire [7:0]  bus_addr,
    input  wire [31:0] bus_din,
    output reg  [31:0] bus_dout,
    output reg         bus_ack,

    // Physical Hardware Interfaces
    output wire [7:0]  step_pins,
    output wire [7:0]  dir_pins,
    output wire [1:0]  bank_enable_pins,
    input wire         global_motor_en
);

    `include "src/main/io/steppers_regs.svh"

    // Multi-cycle Bus Phase State Encoder
    localparam STATE_IDLE = 2'd0;
    localparam STATE_LO   = 2'd1;
    localparam STATE_HI   = 2'd2;
    localparam STATE_ACK  = 2'd3;

    reg [1:0]  bus_state;
    reg [7:0]  tx_num_points;
    reg [1:0]  tx_motor_instance;
    reg        config_reset_pulse;

    // Control and status wires linked across core boundaries
    reg [3:0]  start_strobes;
    reg [3:0]  stop_strobes;
    wire [3:0] status_moving;

    reg  [15:0] selected_core_rdata;

    // Current position per motor (signed, in steps)
    wire signed [31:0] position_0;
    wire signed [31:0] position_1;
    wire signed [31:0] position_2;
    wire signed [31:0] position_3;

    // Driven back to each core by the shared run engine
    wire [3:0]  advance_read_pt;

    reg  [1:0]  current_reg_type;

    reg         core_phase;
    reg  [15:0] core_wdata;
    reg  [15:0] rdata_lo_hold;

    always @(*) begin
        case (bus_addr)
            REG_STEP_SEG_CTST:     current_reg_type = 2'b00;
            REG_STEP_SEG_SPDM:     current_reg_type = 2'b01;
            default:               current_reg_type = 2'b00;
        endcase
    end

    always @(posedge sys_clk) begin
        if (reset) begin
            tx_num_points      <= 8'd0;
            tx_motor_instance  <= 2'd0;
            bus_dout           <= 32'h00000000;
            bus_ack            <= 1'b0;
            config_reset_pulse <= 1'b0;
            bus_state          <= STATE_IDLE;
            rdata_lo_hold      <= 16'h0000;
            start_strobes      <= 4'b0000;
            stop_strobes       <= 4'b0000;
        end else begin
            config_reset_pulse <= 1'b0;
            start_strobes      <= 4'b0000;
            stop_strobes       <= 4'b0000;

            if (bus_stb) begin
                if (!bus_ack) begin
                    if (bus_addr == REG_STEP_TX_CONFIG) begin
                        if (bus_we) begin
                            tx_num_points      <= bus_din[7:0];
                            tx_motor_instance  <= bus_din[15:8];
                            config_reset_pulse <= 1'b1;
                        end else begin
                            bus_dout <= {16'd0, tx_motor_instance, tx_num_points};
                        end
                        bus_ack <= 1'b1;
                    end else if (bus_addr == REG_STEP_CTRL) begin
                        if (bus_we) begin
                            start_strobes <= bus_din[11:8];
                            stop_strobes  <= bus_din[15:12];
                        end
                        bus_dout <= 32'h00000000;
                        bus_ack  <= 1'b1;
                    end else if (bus_addr >= 8'h20 && bus_addr <= 8'h2c) begin
                        // Status registers 0x20, 0x24, 0x28, 0x2c mapping bit 0 to status_moving
                        if (!bus_we) begin
                            case (bus_addr)
                                8'h20: bus_dout <= {31'd0, status_moving[0]};
                                8'h24: bus_dout <= {31'd0, status_moving[1]};
                                8'h28: bus_dout <= {31'd0, status_moving[2]};
                                8'h2c: bus_dout <= {31'd0, status_moving[3]};
                                default: bus_dout <= 32'h00000000;
                            endcase
                        end else begin
                            bus_dout <= 32'h00000000;
                        end
                        bus_ack <= 1'b1;
                    end else if (bus_addr >= 8'h30 && bus_addr <= 8'h3c) begin
                        // Position registers 0x30, 0x34, 0x38, 0x3c - current
                        // signed step position, one 32-bit read, same single-
                        // cycle shape as the status registers above.
                        if (!bus_we) begin
                            case (bus_addr)
                                8'h30: bus_dout <= position_0;
                                8'h34: bus_dout <= position_1;
                                8'h38: bus_dout <= position_2;
                                8'h3c: bus_dout <= position_3;
                                default: bus_dout <= 32'h00000000;
                            endcase
                        end else begin
                            bus_dout <= 32'h00000000;
                        end
                        bus_ack <= 1'b1;
                    end else if (bus_addr >= REG_STEP_SEG_CTST && bus_addr <= REG_STEP_SEG_SPDM) begin
                        case (bus_state)
                            STATE_IDLE: begin
                                bus_state <= STATE_LO;
                            end
                            STATE_LO: begin
                                bus_state <= STATE_HI;
                            end
                            STATE_HI: begin
                                if (bus_we) begin
                                    bus_dout  <= 32'h00000000;
                                    bus_ack   <= 1'b1;
                                    bus_state <= STATE_IDLE;
                                end else begin
                                    rdata_lo_hold <= selected_core_rdata;
                                    bus_state     <= STATE_ACK;
                                end
                            end
                            STATE_ACK: begin
                                bus_dout  <= {selected_core_rdata, rdata_lo_hold};
                                bus_ack   <= 1'b1;
                                bus_state <= STATE_IDLE;
                            end
                        endcase
                    end else begin
                        bus_dout <= 32'hDEADBEEF;
                        bus_ack  <= 1'b1;
                    end
                end
            end else begin
                bus_ack   <= 1'b0;
                bus_state <= STATE_IDLE;
            end
        end
    end

endmodule
