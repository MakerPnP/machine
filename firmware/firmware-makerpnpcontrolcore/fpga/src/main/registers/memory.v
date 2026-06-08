// memory.v
// Streamlined architecture with unified synchronous state registers and
// an instantaneous combinational read-back multiplexer.
module memory (
    input              reset,
    input  wire        clk_a,       // Driven by System Clock (TCXO / clk_sys)
    input  wire [11:0] addr_a,      // Address space from QSPI
    input  wire        we_a,        // Write Enable from QSPI
    input  wire [31:0] din_a,       // Data from MCU
    output reg  [31:0] dout_a,      // Data out to MCU (Combinational Read Routing)

    // Read Connections FROM internal modules
    input  wire [31:0] reg_io_in_1, // From Buttons
    input  wire [31:0] enc_1,       // From Encoder Module 1
    input  wire [31:0] enc_2,       // From Encoder Module 2
    input  wire [31:0] enc_3,       // From Encoder Module 3
    input  wire [31:0] enc_4,       // From Encoder Module 4
    input  wire [31:0] enc_5,       // From Encoder Module 5
    input  wire [31:0] enc_6,       // From Encoder Module 6

    // Write Connections TO internal modules
    output reg         strobe_led_update,
    output reg [31:0]  led_ctrl,
    output reg         strobe_buzzer_update,
    output reg [31:0]  buzzer_ctrl,
    output reg         strobe_encoder_reset // Triggers an encoder reset
);

    // Hardcoded Read-Only Constants
    localparam [31:0] IDENT   = 32'hFA_CE_B0_0B;
    localparam [31:0] VERSION = 32'h01_02_03_04; // Major, Minor, Patch, Build

    localparam [31:0] MARKER   = 32'hDE_AD_C0_DE;

    reg reset_flag = 1'b0;

    // -----------------------------------------------------------------
    // 1. INSTANTANEOUS COMBINATIONAL READ MULTIPLEXER
    // -----------------------------------------------------------------
    // Keeps read paths clean and free of multi-clock synchronization lag
    always @(*) begin
        case (addr_a)
            // IDENT read out
            12'h000: dout_a = IDENT;

            // VERSION read out
            12'h004: dout_a = VERSION;

            // LED outputs readback
            12'h020: dout_a = led_ctrl;

            // IO INPUTS (Buttons)
            12'h024: dout_a = reg_io_in_1;

            // Buzzer control readback
            12'h028: dout_a = buzzer_ctrl;

            // ENCODER 1 (32-bit layout sliced into bytes)
            12'h040: dout_a = enc_1;

            // ENCODER 2
            12'h044: dout_a = enc_2;

            // ENCODER 3
            12'h048: dout_a = enc_3;

            // ENCODER 4
            12'h04C: dout_a = enc_4;

            // ENCODER 5
            12'h050: dout_a = enc_5;

            // ENCODER 6
            12'h054: dout_a = enc_6;

            // END OF MEMORY (0x1FC)
            12'h1FC: dout_a = MARKER;

            default: dout_a = 32'hFFFFFFFF; // Reserved/unmapped space default
        endcase
    end

    // -----------------------------------------------------------------
    // 2. UNIFIED SYNCHRONOUS WRITE TARGET AND STROBE ENGINE
    // -----------------------------------------------------------------
    // Storage flip-flops and command pulses belong together on the clock edge
    always @(posedge clk_a) begin
        if (reset) begin
            reset_flag <= 1'b1;

            // Configure register defaults and set strobes
            led_ctrl <= {24'd0, 8'b0000_0011};
            strobe_led_update    <= 1'b1;

            buzzer_ctrl <= {24'd0, 8'b0000_0000};
            strobe_buzzer_update    <= 1'b1;

            strobe_encoder_reset <= 1'b1;
        end else begin
            // Strobes default to 0; they automatically de-assert on the next system clock edge
            strobe_led_update    <= 1'b0;
            strobe_buzzer_update <= 1'b0;
            strobe_encoder_reset <= 1'b0;
            
            reset_flag <= 1'b0;

            if (we_a) begin
                // Sim-only logging using non-blocking friendly formats
                $strobe("(strobe)writing to address: 0x%04h, data: 0x%08h", addr_a, din_a);
                $display("(display)writing to address: 0x%04h, data: 0x%08h", addr_a, din_a);

                case (addr_a)
                    12'h010: begin
                        if (din_a[0] == 1'b1) begin
                            strobe_encoder_reset <= 1'b1; // Registered clean 1-cycle pulse
                            $display("resetting encoders");
                        end
                    end
                    12'h020: begin
                        led_ctrl           <= din_a;        // Saves stable configuration state
                        strobe_led_update <= 1'b1;         // Registered clean 1-cycle pulse
                    end
                    12'h028: begin
                        buzzer_ctrl       <= din_a;        // Saves stable configuration state
                        strobe_buzzer_update <= 1'b1;         // Registered clean 1-cycle pulse
                    end
                    default: begin end
                endcase
            end
        end
    end

endmodule