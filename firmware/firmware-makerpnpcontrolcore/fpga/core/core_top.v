module core_top (
    input             TCXO,          // H16 Bank 1 50 MHz TCXO
    input  wire       USER_0,        // Button 0 (External Pull-up to 3V3)
    input  wire       USER_1,        // Button 1 (External Pull-up to 3V3)
    output wire       MCU_ACT,       // LED 1
    output wire       FPGA_ACT,      // LED 2

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
    inout  wire [3:0] QUADSPI1_IO
);

    wire clk_100;
    wire locked;
    wire reset;

    wire wake_1;

    // Interconnect Wires between QSPI Core and Memory Map Decoder
    wire [11:0] mem_addr;
    wire [7:0]  mem_din;
    wire [7:0]  mem_dout;
    wire        mem_we;

    // Interconnect Wires from Internal Modules to Memory Map Decoder
    wire [7:0]  reg_io_in_1;
    wire [31:0] enc_1, enc_2, enc_3, enc_4, enc_5, enc_6;

    wire [7:0]  led_out;

    wire        strobe_led_update;
    wire        strobe_encoder_reset;

    reg [7:0] la_src = 2;
    wire [15:0] led_debug;
    wire [15:0] la_in = led_debug;
    //wire [15:0] la_in = 16'h0F0F;

    assign reset = ~locked;

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
        .strobe_led_update(strobe_led_update),
        .led_out(led_out),
        .mcu_act(MCU_ACT),
        .fpga_act(FPGA_ACT),
        .debug(led_debug),
    );

    // ----------------------
    // Instantiate Central Address Decoder
    // ----------------------
    memory memory_map_inst (
        .reset(reset),
        .clk_a(clk_100),
        .addr_a(mem_addr),
        .we_a(mem_we),
        .din_a(mem_din),
        .dout_a(mem_dout),

        // Inputs from modules to read-mux
        .reg_io_in_1(reg_io_in_1),
        .enc_1(enc_1), .enc_2(enc_2), .enc_3(enc_3),
        .enc_4(enc_4), .enc_5(enc_5), .enc_6(enc_6),

        // Outputs from write-decoder to modules
        .strobe_led_update(strobe_led_update),
        .led_out(led_out),
        .strobe_encoder_reset(strobe_encoder_reset)
    );

    // ----------------------
    // Connect QUADSPI1 interface engine to Memory Port A
    // ----------------------
    quadspi qspi_inst (
        .clk_sys(clk_100),
        .sck(QUADSPI1_CLK),
        .cs_n(QUADSPI1_NCS),
        .io(QUADSPI1_IO),
        .mem_addr(mem_addr), // Connects to the 12-bit explicit wire
        .mem_din(mem_din),   // Connects to the 8-bit explicit wire
        .mem_dout(mem_dout), // Connects to the 8-bit explicit wire
        .mem_we(mem_we)      // Connects to the 1-bit explicit wire
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

    // Instantiate Encoders Module
    encoders encoder_inst (
        .sys_clk(clk_100),
        .strobe_encoder_reset(strobe_encoder_reset),
        .encoder_hardware_pins(6'b000000), // Map your actual encoder physical input pins here
        .enc_1(enc_1), .enc_2(enc_2), .enc_3(enc_3),
        .enc_4(enc_4), .enc_5(enc_5), .enc_6(enc_6)
    );

    // Button Capture / Debounce & Synchronizer Logic
    // Sync external asynchronous buttons into the sys_clk domain
    reg [1:0] btn_sync_m;
    reg [1:0] btn_sync_s;
    always @(posedge clk_100) begin
        btn_sync_m <= {USER_1, USER_0};
        btn_sync_s <= btn_sync_m;
    end

    // Map buttons to reg_io_in_1 (Bit 0 = USER 0, Bit 1 = USER 1)
    // Inverted (~btn) because external circuit pulls up to 3V3 (Pressed = 0)
    assign reg_io_in_1 = {6'b000000, ~btn_sync_s[1], ~btn_sync_s[0]};

endmodule
