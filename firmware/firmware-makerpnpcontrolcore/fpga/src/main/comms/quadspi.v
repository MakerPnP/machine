// QuadSPI handling engine
module quadspi (
    input  wire       sys_clk,
    input  wire       sck,
    (* noclk *)
    (* nomux *)
    input  wire       cs_n,
    inout  wire [3:0] io,

    // Memory Interface Port A
    output reg        mem_en    = 0,
    output reg [15:0] mem_addr,
    output reg [31:0] mem_din,
    input  wire [31:0] mem_dout,
    input  wire       mem_valid,
    output reg        mem_we    = 0
);

    // Named Localparam States
    localparam STATE_CMD     = 3'd0;
    localparam STATE_PROCESS = 3'd1;
    localparam STATE_ADDR    = 3'd2;
    localparam STATE_DUMMY   = 3'd3;
    localparam STATE_DATA_R  = 3'd4;
    localparam STATE_DATA_W  = 3'd5;
    localparam STATE_IGNORE  = 3'd6;

    localparam CMD_READ_U32_BE  = 8'h10;
    localparam CMD_READ_U32_LE  = 8'h11;
    localparam CMD_WRITE_U32_BE = 8'h90;
    localparam CMD_WRITE_U32_LE = 8'h91;

    reg [2:0]  state;
    reg [3:0]  phase_counter;
    reg [7:0]  cmd;
    reg [15:0] addr;

    wire       cmd_is_read =
        cmd == CMD_READ_U32_BE ||
        cmd == CMD_READ_U32_LE;
    wire       cmd_is_write =
        cmd == CMD_WRITE_U32_BE ||
        cmd == CMD_WRITE_U32_LE;
    wire       cmd_is_le =
        cmd == CMD_READ_U32_LE ||
        cmd == CMD_WRITE_U32_LE;

    // Tri-state buffer logic
    reg        io_out_en;
    reg [3:0]  io_out_reg;

    // -----------------------------------------------------------------
    // Structural Synchronizers to Prevent High Fanout Global Promotion
    // -----------------------------------------------------------------
    reg [1:0] cs_sync_ctrl = 2'b11;
    reg [1:0] cs_sync_data = 2'b11;
    reg [1:0] cs_sync_mem  = 2'b11;

    always @(posedge sys_clk) begin
        cs_sync_ctrl <= {cs_sync_ctrl[0], cs_n};
        cs_sync_data <= {cs_sync_data[0], cs_n};
        cs_sync_mem  <= {cs_sync_mem[0],  cs_n};
    end

    wire cs_ctrl = cs_sync_ctrl[1];
    wire cs_data = cs_sync_data[1];
    wire cs_mem  = cs_sync_mem[1];

    // -----------------------------------------------------------------
    // Synchronous Edge Detection for Bursty MCU SCK
    // -----------------------------------------------------------------
    reg [1:0] sck_sync = 2'b00;
    always @(posedge sys_clk) begin
        if (cs_ctrl) begin
            sck_sync <= 2'b00;
        end else begin
            sck_sync <= {sck_sync[0], sck};
        end
    end

    // High for exactly 1 sys_clk period when sck transitions
    wire sck_rising  = (sck_sync == 2'b01);
    wire sck_falling = (sck_sync == 2'b10);

    // Explicitly instantiate the 4 physical bidirectional I/O buffers
    wire [3:0] raw_io_in;
    reg  [3:0] io_in;

    always @(posedge sys_clk) begin
        io_in <= raw_io_in; // Unconditional sample forces IOLOGIC packing
    end

    genvar i;
    generate
        for (i = 0; i < 4; i = i + 1) begin : qspi_io_buffers
            SB_IO #(
                .PIN_TYPE(6'b1010_01),
                .PULLUP(1'b0)
            ) io_bit (
                .PACKAGE_PIN(io[i]),
                .OUTPUT_ENABLE(io_out_en),
                .D_OUT_0(io_out_reg[i]),
                .INPUT_CLK(sys_clk),
                .D_IN_0(raw_io_in[i]) // Route to the raw wire
            );
        end
    endgenerate
    reg [31:0] in_buf;
    reg [31:0] out_buf;
    reg [31:0] next_buf;
    reg        next_buf_valid = 1'b0;
    reg        pending_prefetch = 1'b0;

    reg word_complete_flag = 1'b0;
    reg commit_flag = 1'b0;

    // -----------------------------------------------------------------
    // Immediate disable of io_outputs (Synchronous)
    // -----------------------------------------------------------------
    always @(posedge sys_clk) begin
        if (cs_ctrl) begin
            if (io_out_en) begin
                $display("disabling quadspi outputs");
            end
            io_out_en <= 1'b0;
        end else begin
            if (sck_rising && state == STATE_DUMMY && phase_counter == 4'd3) begin
                $display("enabling quadspi outputs");
                io_out_en  <= 1'b1;
            end
        end
    end

    // -----------------------------------------------------------------
    // Block A: Protocol State Machine Control (Low Fanout Load)
    // -----------------------------------------------------------------
    always @(posedge sys_clk) begin
        if (cs_ctrl) begin
            state              <= STATE_CMD;
            phase_counter      <= 0;
            word_complete_flag <= 0;
        end else begin
            case (state)
                STATE_PROCESS: begin
                    if (cmd_is_read || cmd_is_write) begin
                        $display("command received: 0x%02h", cmd);
                        state <= STATE_ADDR;
                    end else begin
                        $display("unknown command ignored: 0x%02h", cmd);
                        state <= STATE_IGNORE;
                    end
                end
            endcase
            if (sck_rising) begin
                case (state)
                    STATE_CMD: begin
                        if (phase_counter == 4'd1) begin
                            phase_counter <= 0;
                            state <= STATE_PROCESS;
                        end else begin
                            phase_counter <= phase_counter + 4'd1;
                        end
                    end
                    STATE_ADDR: begin
                        if (phase_counter == 4'd3) begin
                            phase_counter <= 0;
                            if (cmd_is_write) begin
                                state <= STATE_DATA_W;
                            end else begin
                                state <= STATE_DUMMY;
                            end
                        end else begin
                            phase_counter <= phase_counter + 4'd1;
                        end
                    end

                    STATE_DUMMY: begin
                        if (phase_counter == 4'd7) begin
                            phase_counter <= 0;
                            state         <= STATE_DATA_R;
                        end else begin
                            phase_counter <= phase_counter + 4'd1;
                        end
                    end

                    STATE_DATA_R: begin
                        phase_counter <= phase_counter + 4'd1;
                        if (phase_counter == 4'd7) begin
                            phase_counter <= 4'd0;
                        end
                    end

                    STATE_DATA_W: begin
                        phase_counter <= phase_counter + 4'd1;
                        if (phase_counter == 4'd7) begin
                            word_complete_flag <= 1;
                            phase_counter      <= 4'd0;
                        end
                    end

                    STATE_IGNORE: begin
                    end
                endcase
            end else if (word_complete_flag) begin
                word_complete_flag <= 0;
            end
        end
    end

    // -----------------------------------------------------------------
    // Block B: Data Registers Shift and Collect Logic (Medium Fanout Load)
    // -----------------------------------------------------------------
    always @(posedge sys_clk) begin
        if (cs_data) begin
            cmd    <= 0;
            addr   <= 0;
            in_buf <= 0;
        end else begin
            if (sck_rising) begin
                case (state)
                    STATE_CMD: begin
                        cmd <= {cmd[3:0], io_in};
                    end

                    STATE_ADDR: begin
                        addr <= {addr[11:0], io_in};
                        `ifndef SYNTHESIS
                        if (phase_counter == 4'd3) begin
                            if (cmd_is_write) begin
                                $strobe("write address: 0x%04h", addr);
                            end else begin
                                $strobe("read address: 0x%04h", addr);
                            end
                        end
                        `endif
                    end

                    STATE_DATA_W: begin
                        if (phase_counter == 4'd0) begin
                            in_buf <= {28'd0, io_in};
                        end else begin
                            in_buf <= in_buf << 4 | io_in;
                            if (phase_counter == 4'd7) begin
                                $display("in_buf: 0x%08h", (in_buf << 4 | io_in));
                            end
                        end
                    end

                    STATE_IGNORE: begin
                        in_buf <= in_buf;
                    end
                endcase
            end
        end
    end

    // -----------------------------------------------------------------
    // Block C: Memory Port Interface and Driving Engine (Medium Fanout Load)
    // -----------------------------------------------------------------
    always @(posedge sys_clk) begin
        mem_en <= 1'b0;

        if (cs_mem) begin
            io_out_reg       <= 4'b0;
            mem_addr         <= 0;
            mem_din          <= 0;
            mem_we           <= 1'b0;
            out_buf          <= 0;
            next_buf         <= 0;
            next_buf_valid   <= 1'b0;
            pending_prefetch <= 1'b0;
            commit_flag      <= 1'b0;
        end else begin
            // Handle SPI Outputs on Falling Edge
            if (sck_falling) begin
                if (state == STATE_DATA_R) begin
                    io_out_reg <= out_buf[31:28];
                end else begin
                    io_out_reg <= 4'b0;
                end
            end

            // Process SCK Rising Data Events for Memory Address & Prefetches
            if (sck_rising) begin
                case (state)
                    STATE_ADDR: begin
                        if (phase_counter == 4'd3) begin
                            mem_addr <= {addr[11:0], io_in};
                            if (cmd_is_read) begin
                                pending_prefetch <= 1'b0;
                            end
                        end
                    end

                    STATE_DUMMY: begin
                        if (phase_counter == 4'd0) begin
                            mem_en <= 1'b1;
                            mem_we <= 1'b0;
                        end
                    end

                    STATE_DATA_R: begin
                        if (phase_counter == 4'd4) begin
                            if (mem_addr == 16'hFFFC)
                                mem_addr <= 16'h0000;
                            else
                                mem_addr <= mem_addr + 16'd4;

                            mem_en           <= 1'b1;
                            mem_we           <= 1'b0;
                            pending_prefetch <= 1'b1;
                        end

                        if (phase_counter == 4'd7) begin
                            if (next_buf_valid) begin
                                out_buf <= next_buf;

                                next_buf_valid <= 1'b0;
                            end else begin
                                io_out_reg <= 4'd0;
                            end
                        end else begin
                            out_buf <= out_buf << 4;
                        end
                    end
                endcase
            end

            // Internal FPGA Clock Domain Memory Transactions
            if (mem_valid) begin
                if (pending_prefetch) begin
                    if (cmd_is_le) begin
                        next_buf <= {
                            mem_dout[7:0],
                            mem_dout[15:8],
                            mem_dout[23:16],
                            mem_dout[31:24]
                        };
                    end else begin
                        next_buf <= mem_dout;
                    end

                    next_buf_valid <= 1'b1;
                end else begin
                    if (cmd_is_le) begin
                        out_buf <= {
                            mem_dout[7:0],
                            mem_dout[15:8],
                            mem_dout[23:16],
                            mem_dout[31:24]
                        };
                    end else begin
                        out_buf <= mem_dout;
                    end
                end

                pending_prefetch <= 1'b0;
            end

            if (word_complete_flag && state == STATE_DATA_W) begin
                if (cmd_is_le) begin
                    mem_din <= {
                        in_buf[7:0],
                        in_buf[15:8],
                        in_buf[23:16],
                        in_buf[31:24]
                    };
                end else begin
                    mem_din <= in_buf;
                end

                mem_en  <= 1'b1;
                mem_we  <= 1'b1;
            end

            if (mem_we) begin
                commit_flag <= 1'b1;
                mem_we      <= 1'b0;
            end

            if (commit_flag) begin
                $display("disabling mem_we flag");
                commit_flag   <= 1'b0;
                $display("incrementing address after write");
                if (mem_addr == 16'hFFFC)
                    mem_addr <= 16'h0000;
                else
                    mem_addr <= mem_addr + 16'd4;
            end
        end
    end

endmodule