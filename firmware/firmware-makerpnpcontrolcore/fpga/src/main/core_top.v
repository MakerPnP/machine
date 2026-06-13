module core_top (
    input             TCXO,          // H16 Bank 1 50 MHz TCXO

    input  wire       USER_0,        // Button 0 (External Pull-up to 3V3)
    input  wire       USER_1,        // Button 1 (External Pull-up to 3V3)

    output wire       MCU_ACT,       // LED 1
    output wire       FPGA_ACT,      // LED 2

    output wire       BUZZER,        // Buzzer

    output  wire [15:0] LA_IO,

    (* PULLUP = 1 *)
    input NWAKE_IN,
    output NWAKE_1,
    output NWAKE_2,
    output NWAKE_3,
    output NWAKE_4,
    output MUX_SEL1,
    output MUX_SEL2,
    output MUX_SEL3,
    output MUX_SEL4,
    output FPGA_CLK_1,
    output FPGA_CLK_2,
    output FPGA_CLK_3,
    output FPGA_CLK_4,

    input  wire       QUADSPI1_CLK,
    input  wire       QUADSPI1_NCS,
    // Maps to QUADSPI1_IO0, QUADSPI1_IO1,...
    inout  wire [3:0] QUADSPI1_IO,

    input  wire [2:0] ENCODER_A,
    input  wire [2:0] ENCODER_B,
    input  wire [2:0] ENCODER_C,
    input  wire [2:0] ENCODER_X,
    input  wire [2:0] ENCODER_Y,
    input  wire [2:0] ENCODER_Z
);

    wire clk_100;
    wire locked;
    wire reset;

    wire wake_1;

    // Interconnect Wires between QSPI Core and Memory Map Decoder
    wire [15:0] mem_addr;
    wire [31:0] mem_din;
    wire [31:0] mem_dout;
    wire        mem_en;
    wire        mem_we;
    wire        mem_valid;

    wire [5:0]  led_addr;
    wire [31:0] led_din;
    wire [31:0] led_dout;
    wire        led_we;

    wire [5:0]  encoder_addr;
    wire [31:0] encoder_din;
    wire [31:0] encoder_dout;
    wire        encoder_we;

    wire [5:0]  io_addr;
    wire [31:0] io_din;
    wire [31:0] io_dout;
    wire        io_we;

    wire [5:0]  buzzer_addr;
    wire [31:0] buzzer_din;
    wire [31:0] buzzer_dout;
    wire        buzzer_we;

    wire [15:0] led_debug;
    wire [15:0] buzzer_debug;
    wire [15:0] io_debug;
    wire [15:0] encoder_debug;

    reg [7:0] la_src = 2;
    wire [15:0] la_in = buzzer_debug;
    //wire [15:0] la_in = 16'h0F0F;

    reg [7:0] reset_cnt = 0;
    reg reset_r = 1;

    assign reset = reset_r;

    // once the PLL is locked, release reset after a short delay
    // to allow subsystems to process the reset signal while a clock is present
    always @(posedge clk_100 or negedge locked) begin
        if (!locked) begin
            reset_cnt <= 0;
            reset_r   <= 1;
        end else begin
            if (reset_cnt < 8'd10) begin
                reset_cnt <= reset_cnt + 1;
                reset_r   <= 1;
            end else begin
                reset_r   <= 0;
            end
        end
    end

    // ----------------------
    // PLL
    // ----------------------
    pll u_pll (
        .clock_in(TCXO),
        .clock_out(clk_100),
        .locked(locked)
    );

    la la_inst (
        .reset(reset),
        .sys_clk(clk_100),
        .la_io(LA_IO),
        .la_src_in(la_src),
        .la_in(la_in)
    );

    // ----------------------
    // LEDs
    // ----------------------
    leds led_inst (
        .reset(reset),
        .sys_clk(clk_100),

        .bus_we(led_we),
        .bus_addr(led_addr),
        .bus_din(led_din),
        .bus_dout(led_dout),

        .mcu_act(MCU_ACT),
        .fpga_act(FPGA_ACT),
        .debug(led_debug)
    );

    // ----------------------
    // Buzzer
    // ----------------------
    buzzer buzzer_inst (
        .reset(reset),
        .sys_clk(clk_100),

        .bus_we(buzzer_we),
        .bus_addr(buzzer_addr),
        .bus_din(buzzer_din),
        .bus_dout(buzzer_dout),

        .buzzer(BUZZER),
        .debug(buzzer_debug)
    );

    // ----------------------
    // IO
    // ----------------------
    io io_inst (
        .reset(reset),
        .sys_clk(clk_100),

        .bus_we(io_we),
        .bus_addr(io_addr),
        .bus_din(io_din),
        .bus_dout(io_dout),
        .user_0(USER_0),
        .user_1(USER_1),
        .debug(io_debug)
    );

    // ----------------------
    // Encoders
    // ----------------------
    encoders encoder_inst (
        .sys_clk(clk_100),
        .reset(reset),

        .bus_we(encoder_we),
        .bus_addr(encoder_addr),
        .bus_din(encoder_din),
        .bus_dout(encoder_dout),

        .abz_a(ENCODER_A),
        .abz_b(ENCODER_B),
        .abz_c(ENCODER_C),
        .abz_x(ENCODER_X),
        .abz_y(ENCODER_Y),
        .abz_z(ENCODER_Z),

        .debug(encoder_debug)
    );

    // ----------------------
    // Instantiate Central Address Decoder
    // ----------------------
    memory memory_map_inst (
        .reset(reset),
        .clk_a(clk_100),
        .we_a(mem_we),
        .en_a(mem_en),
        .addr_a(mem_addr),
        .din_a(mem_din),
        .dout_a(mem_dout),
        .valid_a(mem_valid),

        .led_we(led_we),
        .led_addr(led_addr),
        .led_din(led_din),
        .led_dout(led_dout),

        .encoder_we(encoder_we),
        .encoder_addr(encoder_addr),
        .encoder_din(encoder_din),
        .encoder_dout(encoder_dout),

        .io_we(io_we),
        .io_addr(io_addr),
        .io_din(io_din),
        .io_dout(io_dout),

        .buzzer_we(buzzer_we),
        .buzzer_addr(buzzer_addr),
        .buzzer_din(buzzer_din),
        .buzzer_dout(buzzer_dout)
    );

    // ----------------------
    // Connect QUADSPI1 interface engine to Memory Port A
    // ----------------------
    quadspi qspi_inst (
        .clk_sys(clk_100),
        .sck(QUADSPI1_CLK),
        .cs_n(QUADSPI1_NCS),
        .io(QUADSPI1_IO),
        .mem_en(mem_en),
        .mem_addr(mem_addr),
        .mem_din(mem_din),
        .mem_dout(mem_dout),
        .mem_valid(mem_valid),
        .mem_we(mem_we)
    );

    // ----------------------
    // Application logic
    // ----------------------
//    blink u_blink (
//        .reset(reset),
//        .clk(clk_100),
//        .led(FPGA_ACT)
//    );

    wake u_wake (
        .reset(reset),
        .nwake_in(NWAKE_IN),
        .nwake_1(NWAKE_1),
        .nwake_2(NWAKE_2),
        .nwake_3(NWAKE_3),
        .nwake_4(NWAKE_4)
    );

    timer_mux u_timer_mux (
        .reset(reset),
        .mux_sel1(MUX_SEL1),
        .mux_sel2(MUX_SEL2),
        .mux_sel3(MUX_SEL3),
        .mux_sel4(MUX_SEL4)
    );

    clock_out u_clock_out (
        .reset(reset),
        .clock_out1(FPGA_CLK_1),
        .clock_out2(FPGA_CLK_2),
        .clock_out3(FPGA_CLK_3),
        .clock_out4(FPGA_CLK_4)
    );

endmodule
