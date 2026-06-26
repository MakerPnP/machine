reg [5:0]  addr;
reg [31:0] din;
reg [31:0] dout;
reg        we;
reg        stb;
reg        ack;

task sys_reset;
    begin
        $display("Resetting...");
        // reset pulse
        RESET = 1;
        #20;
        RESET = 0;

        // Run simulation for some time
        #50;
    end
endtask

task bus_init;
    begin
        addr <= 0;
        din  <= 0;
        we   <= 1'b0;
        stb  <= 1'b0;
    end
endtask


task bus_write(input [5:0] w_addr, input [31:0] w_data);
    begin
        @(posedge TCXO);
        addr <= w_addr;
        din  <= w_data;
        we   <= 1'b1;
        stb  <= 1'b1; // Raise strobe

        // Wait until the slave asserts acknowledgement
        while (!ack) begin
            @(posedge TCXO);
        end

        // Deassert strobe now that the transaction is acknowledged
        stb  <= 1'b0;
        we   <= 1'b0;
        while (ack) begin
            @(posedge TCXO);
        end
    end
endtask

task bus_read(input [5:0] r_addr, output [31:0] r_data);
    begin
        @(posedge TCXO);
        addr <= r_addr;
        we   <= 1'b0;
        stb  <= 1'b1; // Raise strobe

        // Wait until the slave asserts acknowledgement
        while (!ack) begin
            @(posedge TCXO);
        end

        r_data = dout; // Capture the stable data

        // Deassert strobe now that the transaction is acknowledged
        stb  <= 1'b0;
        while (ack) begin
            @(posedge TCXO);
        end
    end
endtask

