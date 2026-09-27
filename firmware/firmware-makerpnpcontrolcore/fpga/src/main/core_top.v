module core_top (
    input             TCXO,          // H16 Bank 1 50 MHz TCXO

    input  wire [1:0] BTN,           // Buttons (External Pull-up to 3V3)
    input  wire [1:0] IAK,
    input  wire [7:0] DIN,
    output wire [1:0] OEC,
    output wire [1:0] ADC_MUX,
    input  wire       BASE_PRESENT,
    input  wire [3:0] PORT_PRESENT,

    output wire       MCU_ACT,       // LED 1
    output wire       FPGA_ACT,      // LED 2

    output wire       BUZZER,        // Buzzer
    output wire       RGB_PORTS,
    output wire       RGB_UP_CAM,

    output wire [15:0] LA_IO,

    output wire [3:0] XYZF_STEP,
    output wire [3:0] XYZF_DIR,

    output wire [3:0] BCDE_STEP,
    output wire [3:0] BCDE_DIR,

    output wire BCDE_EN,
    output wire XYZF_EN,

    output wire LC1_S0,
    output wire LC1_S1,
    output wire LC1_PD_SCK,
    input wire  LC1_DOUT,

    output wire [3:0] PM_OUT,
    output wire [7:0] OT_OUT,
    output wire       OT_EN,

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

    wire sys_clk;
    wire locked;
    wire reset;

    wire wake_1;

    // Interconnect Wires between QSPI Core and Memory Map Decoder
    wire [15:0] mem_addr;
    wire [31:0] mem_din;
    wire [31:0] mem_dout;
    wire        mem_stb;
    wire        mem_we;
    wire        mem_ack;

    wire [7:0]  led_addr;
    wire [31:0] led_din;
    wire [31:0] led_dout;
    wire        led_we;
    wire        led_ack;
    wire        led_stb;

    wire [7:0]  encoder_addr;
    wire [31:0] encoder_din;
    wire [31:0] encoder_dout;
    wire        encoder_we;
    wire        encoder_stb;
    wire        encoder_ack;

    wire [7:0]  io_addr;
    wire [31:0] io_din;
    wire [31:0] io_dout;
    wire        io_we;
    wire        io_stb;
    wire        io_ack;

    wire [7:0]  ws0_addr;
    wire [31:0] ws0_din;
    wire [31:0] ws0_dout;
    wire        ws0_we;
    wire        ws0_stb;
    wire        ws0_ack;

    wire [7:0]  ws1_addr;
    wire [31:0] ws1_din;
    wire [31:0] ws1_dout;
    wire        ws1_we;
    wire        ws1_stb;
    wire        ws1_ack;

    wire [7:0]  buzzer_addr;
    wire [31:0] buzzer_din;
    wire [31:0] buzzer_dout;
    wire        buzzer_we;
    wire        buzzer_stb;
    wire        buzzer_ack;

    wire [7:0]  steppers_addr;
    wire [31:0] steppers_din;
    wire [31:0] steppers_dout;
    wire        steppers_we;
    wire        steppers_stb;
    wire        steppers_ack;

    wire [7:0]  lc0_addr;
    wire [31:0] lc0_din;
    wire [31:0] lc0_dout;
    wire        lc0_we;
    wire        lc0_stb;
    wire        lc0_ack;

    wire [7:0]  timer_pwm_addr;
    wire [31:0] timer_pwm_din;
    wire [31:0] timer_pwm_dout;
    wire        timer_pwm_we;
    wire        timer_pwm_stb;
    wire        timer_pwm_ack;

    wire [15:0] led_debug;
    wire [15:0] buzzer_debug;
    wire [15:0] io_debug;
    wire [15:0] encoder_debug;

    reg [7:0] la_src = 2;
    //wire [15:0] la_in = buzzer_debug;
    wire [15:0] la_in = 16'h0F0F;


    wire global_motor_en;
    reg emergency_stop_r = 1'b0;
    assign global_motor_en = !emergency_stop_r;


    always @(posedge sys_clk) begin
        if (reset) begin
            emergency_stop_r = 1'b1;
        end else begin
            // TODO add e-stop hardware support
            //      probably via some system configuration registers.
            //      and use an output from the IO module.
            emergency_stop_r = 1'b0;
        end
    end

    reg [7:0] reset_cnt = 0;
    reg reset_r = 1;

    assign reset = reset_r;

    // once the PLL is locked, release reset after a short delay
    // to allow subsystems to process the reset signal while a clock is present
    always @(posedge sys_clk or negedge locked) begin
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
        .clock_out(sys_clk),
        .locked(locked)
    );

    la la_inst (
        .reset(reset),
        .sys_clk(sys_clk),
        .la_io(LA_IO),
        .la_src(la_src),
        .la_in(la_in)
    );

    // ----------------------
    // LEDs
    // ----------------------
    leds led_inst (
        .reset(reset),
        .sys_clk(sys_clk),

        .bus_stb(led_stb),
        .bus_we(led_we),
        .bus_addr(led_addr),
        .bus_din(led_din),
        .bus_dout(led_dout),
        .bus_ack(led_ack),

        .mcu_act(MCU_ACT),
        .fpga_act(FPGA_ACT),
        .debug(led_debug)
    );

    // ----------------------
    // Buzzer
    // ----------------------
    buzzer buzzer_inst (
        .reset(reset),
        .sys_clk(sys_clk),

        .bus_stb(buzzer_stb),
        .bus_we(buzzer_we),
        .bus_addr(buzzer_addr),
        .bus_din(buzzer_din),
        .bus_dout(buzzer_dout),
        .bus_ack(buzzer_ack),

        .buzzer(BUZZER),
        .debug(buzzer_debug)
    );

    // ----------------------
    // IO
    // ----------------------
    io io_inst (
        .reset(reset),
        .sys_clk(sys_clk),

        .bus_stb(io_stb),
        .bus_we(io_we),
        .bus_addr(io_addr),
        .bus_din(io_din),
        .bus_dout(io_dout),
        .bus_ack(io_ack),

        .btn(BTN),
        .iak(IAK),
        .din(DIN),
        .oec(OEC),
        .adc_mux(ADC_MUX),
        .base_present(BASE_PRESENT),
        .port_present(PORT_PRESENT),

        .debug(io_debug)
    );

    // ----------------------
    // Encoders
    // ----------------------
    encoders encoder_inst (
        .reset(reset),
        .sys_clk(sys_clk),

        .bus_stb(encoder_stb),
        .bus_we(encoder_we),
        .bus_addr(encoder_addr),
        .bus_din(encoder_din),
        .bus_dout(encoder_dout),
        .bus_ack(encoder_ack),

        .abz_a(ENCODER_A),
        .abz_b(ENCODER_B),
        .abz_c(ENCODER_C),
        .abz_x(ENCODER_X),
        .abz_y(ENCODER_Y),
        .abz_z(ENCODER_Z),

        .debug(encoder_debug)
    );

    // ----------------------
    // WS2812 - on-board LEDs
    // ----------------------
    ws2812 ws2812_0_inst (
        .sys_clk(sys_clk),
        .reset(reset),

        .bus_stb(ws0_stb),
        .bus_we(ws0_we),
        .bus_addr(ws0_addr),
        .bus_din(ws0_din),
        .bus_dout(ws0_dout),
        .bus_ack(ws0_ack),

        .ws_out(RGB_PORTS)
    );

    // ----------------------
    // WS2812 - Up-camera / Head / Work LEDs
    // ----------------------
    ws2812 ws2812_1_inst (
        .sys_clk(sys_clk),
        .reset(reset),

        .bus_stb(ws1_stb),
        .bus_we(ws1_we),
        .bus_addr(ws1_addr),
        .bus_din(ws1_din),
        .bus_dout(ws1_dout),
        .bus_ack(ws1_ack),

        .ws_out(RGB_UP_CAM)
    );

    // ----------------------
    // Steppers
    // ----------------------

    wire stepper_clk;

    stepper_clk stepper_clk_inst (
        .sys_clk(sys_clk),
        .reset(reset),
        .stepper_clk(stepper_clk)
    );

    steppers steppers_inst (
        .sys_clk(sys_clk),
        .reset(reset),

        .stepper_clk(stepper_clk),

        .bus_stb(steppers_stb),
        .bus_we(steppers_we),
        .bus_addr(steppers_addr),
        .bus_din(steppers_din),
        .bus_dout(steppers_dout),
        .bus_ack(steppers_ack),

        .step_pins({BCDE_STEP, XYZF_STEP}),
        .dir_pins({BCDE_DIR, XYZF_DIR}),
        .bank_enable_pins({BCDE_EN, XYZF_EN}),
        .global_motor_en(global_motor_en)
    );

    // ----------------------
    // Timer/PWM
    // ----------------------
    timer_pwm timer_pwm_inst (
        .reset(reset),
        .sys_clk(sys_clk),

        .bus_stb(timer_pwm_stb),
        .bus_we(timer_pwm_we),
        .bus_addr(timer_pwm_addr),
        .bus_din(timer_pwm_din),
        .bus_dout(timer_pwm_dout),
        .bus_ack(timer_pwm_ack),

        .pm_out(PM_OUT),
        .ot_out(OT_OUT),
        .ot_en(OT_EN)
    );

    // ----------------------
    // Instantiate Central Address Decoder
    // ----------------------
    memory memory_map_inst (
        .reset(reset),
        .clk_a(sys_clk),
        .we_a(mem_we),
        .stb_a(mem_stb),
        .addr_a(mem_addr),
        .din_a(mem_din),
        .dout_a(mem_dout),
        .ack_a(mem_ack),

        .led_stb(led_stb),
        .led_we(led_we),
        .led_addr(led_addr),
        .led_din(led_din),
        .led_dout(led_dout),
        .led_ack(led_ack),

        .encoder_stb(encoder_stb),
        .encoder_we(encoder_we),
        .encoder_addr(encoder_addr),
        .encoder_din(encoder_din),
        .encoder_dout(encoder_dout),
        .encoder_ack(encoder_ack),

        .io_stb(io_stb),
        .io_we(io_we),
        .io_addr(io_addr),
        .io_din(io_din),
        .io_dout(io_dout),
        .io_ack(io_ack),

        .ws0_stb(ws0_stb),
        .ws0_we(ws0_we),
        .ws0_addr(ws0_addr),
        .ws0_din(ws0_din),
        .ws0_dout(ws0_dout),
        .ws0_ack(ws0_ack),

        .ws1_stb(ws1_stb),
        .ws1_we(ws1_we),
        .ws1_addr(ws1_addr),
        .ws1_din(ws1_din),
        .ws1_dout(ws1_dout),
        .ws1_ack(ws1_ack),

        .steppers_stb(steppers_stb),
        .steppers_we(steppers_we),
        .steppers_addr(steppers_addr),
        .steppers_din(steppers_din),
        .steppers_dout(steppers_dout),
        .steppers_ack(steppers_ack),

        .timer_pwm_stb(timer_pwm_stb),
        .timer_pwm_we(timer_pwm_we),
        .timer_pwm_addr(timer_pwm_addr),
        .timer_pwm_din(timer_pwm_din),
        .timer_pwm_dout(timer_pwm_dout),
        .timer_pwm_ack(timer_pwm_ack),

        .lc0_stb(lc0_stb),
        .lc0_we(lc0_we),
        .lc0_addr(lc0_addr),
        .lc0_din(lc0_din),
        .lc0_dout(lc0_dout),
        .lc0_ack(lc0_ack),

        .buzzer_stb(buzzer_stb),
        .buzzer_we(buzzer_we),
        .buzzer_addr(buzzer_addr),
        .buzzer_din(buzzer_din),
        .buzzer_dout(buzzer_dout),
        .buzzer_ack(buzzer_ack)
    );

    // ----------------------
    // IO
    // ----------------------
    loadcell lc0_inst (
        .reset(reset),
        .sys_clk(sys_clk),

        .bus_stb(lc0_stb),
        .bus_we(lc0_we),
        .bus_addr(lc0_addr),
        .bus_din(lc0_din),
        .bus_dout(lc0_dout),
        .bus_ack(lc0_ack),

        .dout(LC1_DOUT),
        .s0(LC1_S0),
        .s1(LC1_S1),
        .pd_sck(LC1_PD_SCK)
    );

    // ----------------------
    // Connect QUADSPI1 interface engine to Memory Port A
    // ----------------------
    quadspi qspi_inst (
        .sys_clk(sys_clk),
        .sck(QUADSPI1_CLK),
        .cs_n(QUADSPI1_NCS),
        .io(QUADSPI1_IO),
        .mem_stb(mem_stb),
        .mem_addr(mem_addr),
        .mem_din(mem_din),
        .mem_dout(mem_dout),
        .mem_ack(mem_ack),
        .mem_we(mem_we)
    );

    // ----------------------
    // Application logic
    // ----------------------
//    blink u_blink (
//        .reset(reset),
//        .clk(sys_clk),
//        .led(FPGA_ACT)
//    );

    wake u_wake (
        .reset(reset),
        .sys_clk(sys_clk),
        .nwake_in(NWAKE_IN),
        .nwake_1(NWAKE_1),
        .nwake_2(NWAKE_2),
        .nwake_3(NWAKE_3),
        .nwake_4(NWAKE_4)
    );

    timer_mux u_timer_mux (
        .reset(reset),
        .sys_clk(sys_clk),
        .mux_sel1(MUX_SEL1),
        .mux_sel2(MUX_SEL2),
        .mux_sel3(MUX_SEL3),
        .mux_sel4(MUX_SEL4)
    );

    clock_out u_clock_out (
        .reset(reset),
        .sys_clk(sys_clk),
        .clock_out1(FPGA_CLK_1),
        .clock_out2(FPGA_CLK_2),
        .clock_out3(FPGA_CLK_3),
        .clock_out4(FPGA_CLK_4)
    );

endmodule
