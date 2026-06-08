// QuadSPI handling engine
module quadspi (
    input  wire       clk_sys,
    input  wire       sck,
    input  wire       cs_n,
    inout  wire [3:0] io,

    // Memory Interface Port A
    output reg [11:0] mem_addr,
    output reg [31:0]  mem_din,
    input  wire [31:0] mem_dout,
    output reg        mem_we    = 0
);

    // Named Localparam States
    localparam STATE_CMD     = 3'd0;
    localparam STATE_ADDR    = 3'd1;
    localparam STATE_DUMMY   = 3'd2;
    localparam STATE_DATA_R  = 3'd3;
    localparam STATE_DATA_W  = 3'd4;

    reg [2:0]  state;
    reg [3:0]  phase_counter;
    reg [7:0]  cmd;
    reg [15:0] addr;

    // Tri-state buffer logic
    reg        io_out_en;
    reg [3:0]  io_out_reg;
    wire [3:0] io_in;

    // -----------------------------------------------------------------
    // Synchronous Edge Detection for Bursty MCU SCK
    // -----------------------------------------------------------------
    reg [1:0] sck_sync = 2'b00;
    always @(posedge clk_sys or posedge cs_n) begin
        if (cs_n) begin
            sck_sync <= 2'b00;
        end else begin
            sck_sync <= {sck_sync[0], sck};
        end
    end

    // High for exactly 1 clk_sys period when sck transitions
    wire sck_rising  = (sck_sync == 2'b01);
    wire sck_falling = (sck_sync == 2'b10); // FIXED: Corrected from 2'b00 to 2'b10

    // this generates a warning: "Yosys has only limited support for tri-state logic at the moment."
    // ```
    // reg [3:0]  io_out;
    // assign io = io_out_en ? io_out : 4'bz;
    // assign io_in = io;
    // ```
    // instead:
    // Explicitly instantiate the 4 physical bidirectional I/O buffers
    // This makes Yosys happy and forces precise routing.

    genvar i;
    generate
        for (i = 0; i < 4; i = i + 1) begin : qspi_io_buffers
            SB_IO #(
                .PIN_TYPE(6'b1010_01),
                .PULLUP(1'b0)
            ) io_bit (
                .PACKAGE_PIN(io[i]),
                .OUTPUT_ENABLE(io_out_en),
                .D_OUT_0(io_out_reg[i]), // Driven directly by the glitch-free register
                .D_IN_0(io_in[i])
            );
        end
    endgenerate

    reg [31:0] in_buf;
    reg [31:0] out_buf;

    reg word_complete_flag = 1'b0;
    reg commit_flag = 1'b0;

    // -----------------------------------------------------------------
    // Synchronous Output Driver Logic (Prepares data on SCK Falling Edge)
    // -----------------------------------------------------------------
    always @(posedge clk_sys or posedge cs_n) begin
        if (cs_n) begin
            io_out_reg <= 4'b0;
        end else if (sck_falling) begin
            if (state == STATE_DATA_R) begin
                io_out_reg <= out_buf[31:28];
            end else begin
                io_out_reg <= 4'b0;
            end
        end
    end

    // -----------------------------------------------------------------
    // Immediate disable of io_outputs
    // -----------------------------------------------------------------
    always @(posedge clk_sys or posedge cs_n) begin
        if (cs_n) begin
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
    // Main SPI Protocol State Machine (Processes on SCK Rising Edge)
    // -----------------------------------------------------------------
    always @(posedge clk_sys) begin
        if (word_complete_flag) begin
            word_complete_flag <= 0;
            case (state)
                STATE_DATA_R: begin
                    out_buf       <= mem_dout;
                end
                STATE_DATA_W: begin
                    // capture the completed input buffer
                    mem_din       <= in_buf;
                    mem_we        <= 1'b1;
                    $display("in_buf: 0x%08h", in_buf);
                end
            endcase
        end

        if (mem_we) begin
            // if CS goes high during a commit, we need to clear mem_we on the next clock cycle.
            // if mem_we is reset here the commit is lost.
            commit_flag <= 1'b1;
        end

        if (commit_flag) begin
            $display("disabling mem_we flag");
            mem_we        <= 1'b0;
            commit_flag   <= 1'b0;
            // increment the address /after/ the write
            $display("incrementing address after write");
            if (mem_addr == 12'h1FC)
                mem_addr <= 12'h000;
            else
                mem_addr <= mem_addr + 12'd4;
        end

        if (cs_n) begin
            state         <= STATE_CMD;
            phase_counter <= 0;
            cmd           <= 0;
            addr          <= 0;
            in_buf        <= 0;
            word_complete_flag <= 0;
        end else begin
            // Only process state modifications on valid rising edges of sck
            if (sck_rising) begin
                case (state)
                    STATE_CMD: begin
                        cmd <= {cmd[3:0], io_in};
                        if (phase_counter == 4'd1) begin
                            // 2 nibbles = 1 byte Command
                            phase_counter <= 0;
                            state         <= STATE_ADDR;
                            $strobe("command received: 0x%02h", cmd);
                        end else begin
                            phase_counter <= phase_counter + 4'd1;
                        end
                    end

                    STATE_ADDR: begin
                        if (phase_counter == 4'd3) begin
                            phase_counter <= 0;
                            mem_addr <= {addr[7:0], io_in};

                            if (cmd[7] == 1'b1) begin
                                // High bit in MSB indicates a write operation
                                state    <= STATE_DATA_W;
                                $strobe("write address: 0x%04h", mem_addr);
                            end else begin
                                state    <= STATE_DUMMY;
                                $strobe("read address: 0x%04h", mem_addr);
                            end
                        end else begin
                            phase_counter <= phase_counter + 4'd1;
                            addr          <= {addr[11:0], io_in};
                        end
                    end

                    STATE_DUMMY: begin
                        if (phase_counter == 4'd7) begin
                            // 8 full clock periods finished
                            phase_counter <= 0;
                            out_buf       <= mem_dout;
                            state         <= STATE_DATA_R;
                        end else begin
                            phase_counter <= phase_counter + 4'd1;
                        end
                    end

                    STATE_DATA_R: begin
                        phase_counter <= phase_counter + 4'd1;
                        out_buf <= out_buf << 4;

                        // Advance address space after the last nibble phase finishes
                        if (phase_counter == 4'd7) begin

                            phase_counter <= 4'd0;

                            word_complete_flag <= 1;
                            if (mem_addr == 12'h1FC)
                                mem_addr <= 12'h000;
                            else
                                mem_addr <= mem_addr + 12'd4;
                        end
                    end

                    STATE_DATA_W: begin
                        phase_counter <= phase_counter + 4'd1;

                        if (phase_counter == 4'd0) begin
                            in_buf        <= {28'd0, io_in};
                        end else begin
                            in_buf       <= {in_buf << 4, io_in};

                            if (phase_counter == 4'd7) begin
                                word_complete_flag <= 1;
                                phase_counter <= 4'd0;
                            end
                        end
                    end
                endcase
            end
        end
    end

endmodule