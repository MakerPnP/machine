// ====================================================================
// loadcell_shared.svh
//
// Constants shared between the production peripheral
// (src/main/io/loadcell.v) and the simulation environment
// (src/test/hx717_sim.v, src/test/loadcell_tb.v).
//
// Everything here is a `localparam`, so this file must be `include`d
// *inside* a module body (same convention as steppers_shared.svh).
// ====================================================================

// --------------------------------------------------------------------
// System clock
// --------------------------------------------------------------------
localparam LC_CLK_HZ  = 50_000_000;
localparam LC_CLK_MHZ = 50;             // sys_clk cycles per microsecond

// --------------------------------------------------------------------
// PD_SCK generation.
//
// HX717 Table (Fig.2):
//   T1  DOUT falling edge -> PD_SCK rising edge   min 0.1us
//   T2  PD_SCK rising edge -> DOUT data ready     max 0.1us
//   T3  PD_SCK high time            min 0.2us, typ 1us, MAX 50us
//   T4  PD_SCK low  time            min 0.2us, typ 1us, no max
//
// T3's 50us maximum is the hard constraint: holding PD_SCK high for
// longer than 80us is the power-down entry condition, and the 50us
// number is the chip's guard band around that. We use the datasheet
// typical of 1us for both halves, giving a 500 kHz PD_SCK - 40x margin
// below the T3 maximum and 5x above the T3/T4 minimums.
// --------------------------------------------------------------------
localparam LC_SCK_HALF_US     = 1;
localparam LC_SCK_HALF_CYCLES = LC_CLK_MHZ * LC_SCK_HALF_US;   // 50 cycles

// Guard delay between "DOUT observed low" and the first rising edge.
//
// The timing table gives T1 a 0.1us minimum. Silicon needs considerably
// more than that: clocked much before ~1us after DOUT falls, the part
// has not yet placed the MSB on DOUT, and bit 23 of the conversion is
// lost outright - DOUT holds the data-ready low level through pulse 1
// and resumes on schedule at pulse 2, so the host reads back
// (true & 0x7FFFFF): never negative, bits 22:0 intact.
//
// The datasheet's own reference C driver is the better guide here. It
// waits "More than 1uS" after `while (DOUT==1);` before its first
// clock, an order of magnitude beyond the table figure. 2us is used
// below so that requirement is met with 2x margin.
localparam LC_T1_GUARD_US = 2;
localparam LC_T1_CYCLES   = LC_CLK_MHZ * LC_T1_GUARD_US;       // 100 -> 2us

// Power-down entry: PD_SCK must stay high for longer than 80us. We hold
// it for 100us so the requirement is met with margin over any clock
// tolerance.
localparam LC_PD_HOLD_US = 100;

// --------------------------------------------------------------------
// Pulse counts (HX717 Table 4 + "Reset and Power-Down Modes")
//
//   25 pulses -> next conversion CH A, gain 128
//   26 pulses -> next conversion CH B, gain 64
//   27 pulses -> next conversion CH A, gain 64
//   28 pulses -> next conversion CH B, gain 8
//   29 pulses + hold high -> ADC and analog regulator powered down
//   30 pulses + hold high -> everything powered down
//
// Pulses 1..24 shift out the 24-bit result MSB first. The 25th rising
// edge returns DOUT high. So the pulse count is always 24 + at least 1,
// and LC_MODE_* is literally an offset onto 25.
// --------------------------------------------------------------------
localparam LC_DATA_PULSES   = 24;
localparam LC_PULSES_BASE   = 25;   // == LC_DATA_PULSES + 1
localparam LC_PULSES_PD_REG = 29;
localparam LC_PULSES_PD_ALL = 30;
localparam LC_PULSES_MAX    = 30;

// Channel / gain select for the *next* conversion. Value is the offset
// added to LC_PULSES_BASE, so LC_MODE_B8 == 3 == 28 total pulses.
localparam [1:0] LC_MODE_A128 = 2'd0;   // 25 pulses, CH A, gain 128
localparam [1:0] LC_MODE_B64  = 2'd1;   // 26 pulses, CH B, gain 64
localparam [1:0] LC_MODE_A64  = 2'd2;   // 27 pulses, CH A, gain 64
localparam [1:0] LC_MODE_B8   = 2'd3;   // 28 pulses, CH B, gain 8

// --------------------------------------------------------------------
// Output data rate select, driven onto the S1/S0 pins (Table 3).
// Encoding is {S1, S0}.
// --------------------------------------------------------------------
localparam [1:0] LC_RATE_10HZ  = 2'b00;
localparam [1:0] LC_RATE_20HZ  = 2'b01;
localparam [1:0] LC_RATE_80HZ  = 2'b10;
localparam [1:0] LC_RATE_320HZ = 2'b11;

// --------------------------------------------------------------------
// Power-down depth.
//
// LC_PD_ADC keeps the pulse count at whatever the channel/gain select
// asks for (25..28), so the channel/gain setup is preserved across the
// power-down exactly as the datasheet describes. LC_PD_ADC_REG and
// LC_PD_ALL clock past the selection window to 29/30 pulses; the chip
// then restores "the set up conditions of the last change" on wake, so
// the channel/gain in effect after waking is the one that was latched
// by the *previous* conversion's pulse count, not this one.
// --------------------------------------------------------------------
localparam [1:0] LC_PD_ADC     = 2'd0;  // ADC only        (<360uA)
localparam [1:0] LC_PD_ADC_REG = 2'd1;  // ADC + regulator (<280uA), 29 pulses
localparam [1:0] LC_PD_ALL     = 2'd2;  // everything      (<1uA),   30 pulses

// --------------------------------------------------------------------
// REG_LC_CTRL bit layout (9 bits)
//
//   [0]   ENABLE   1 = run conversions back to back
//   [1]   PD       1 = enter power-down once the conversion in flight
//                      finishes (never mid-conversion - see datasheet:
//                      "power down should be executed after current
//                      conversion period is completed")
//   [3:2] RATE     {S1,S0}, LC_RATE_*
//   [5:4] MODE     channel/gain for the next conversion, LC_MODE_*
//   [7:6] PDMODE   power-down depth, LC_PD_*
//   [8]   SINGLE   write 1 to take exactly one sample; self-clearing,
//                  always reads back 0
// --------------------------------------------------------------------
localparam LC_CTRL_W          = 9;
localparam LC_CTRL_ENABLE_BIT = 0;
localparam LC_CTRL_PD_BIT     = 1;
localparam LC_CTRL_RATE_LSB   = 2;
localparam LC_CTRL_MODE_LSB   = 4;
localparam LC_CTRL_PDMODE_LSB = 6;
localparam LC_CTRL_SINGLE_BIT = 8;

// --------------------------------------------------------------------
// REG_LC_STATUS bit layout (5 bits, read-only)
//
//   [0] ENABLED  mirrors REG_LC_CTRL.ENABLE
//   [1] READY    a conversion has completed since the last write to
//                REG_LC_CTRL, so REG_LC_VALUE is valid for the *current*
//                configuration. Cleared by writing REG_LC_CTRL - which
//                is what makes it meaningful across a channel or gain
//                change, since the first samples after one are not
//                trustworthy anyway.
//   [2] BUSY     the FSM is mid-conversion (clocking PD_SCK)
//   [3] PD       the device is being held in power-down
//   [4] TIMEOUT  DOUT did not go low within LC_TIMEOUT_US. Sticky;
//                cleared by any write to REG_LC_CTRL.
//
// NOTHING HERE IS CLEARED BY A READ. See the read side-effect note on
// REG_LC_VALUE below - reads of this peripheral must be free of side
// effects, and that is a bus requirement, not a style preference.
// --------------------------------------------------------------------
localparam LC_STATUS_W           = 5;
localparam LC_STATUS_ENABLED_BIT = 0;
localparam LC_STATUS_READY_BIT   = 1;
localparam LC_STATUS_BUSY_BIT    = 2;
localparam LC_STATUS_PD_BIT      = 3;
localparam LC_STATUS_TIMEOUT_BIT = 4;

// --------------------------------------------------------------------
// REG_LC_VALUE layout, and why reads must have no side effects.
//
// The MCU reaches these registers through the STM32H735 OctoSPI in
// memory-mapped mode, which prefetches a full 16-byte FIFO line aligned
// to 16 bytes. REG_LC_CTRL (0x00), REG_LC_STATUS (0x04) and
// REG_LC_VALUE (0x08) all live in the SAME line, so a read of any one
// of them makes the controller issue reads of all four words in the
// line. A read side effect on REG_LC_VALUE would therefore fire when
// the host merely polled REG_LC_STATUS - the flag would be destroyed by
// the act of reading it. Reads here are strictly free of side effects.
//
// That removes the read-to-consume handshake, so "is this sample new?"
// is answered by a sequence number instead:
//
//   [23:0]  the raw 24-bit two's complement result
//   [31:24] an 8-bit counter, incremented once per completed conversion
//
// The counter shares the word with the value on purpose. A single
// 32-bit read gets both, so they cannot skew - whereas putting the
// counter in REG_LC_STATUS would make correctness depend on the
// prefetch issuing 0x04 before 0x08, and would break outright for a
// host that read the value on its own. The host keeps the last counter
// it saw; a difference means new data, and a jump of more than one
// means samples were missed. It wraps every 256 conversions - 0.8s at
// 320SPS, 25.6s at 10SPS.
// --------------------------------------------------------------------
localparam LC_SEQ_W       = 8;
localparam LC_VALUE_W     = 24;
localparam LC_VALUE_SEQ_LSB = 24;

// --------------------------------------------------------------------
// Data-ready watchdog.
//
// Worst case in production is the 10Hz rate: 100ms conversion period,
// and a 4/fo = 400ms settling time after power-up / channel change /
// gain change. 1s therefore clears the slowest legitimate case with
// margin. Shortened under SIM so the testbench does not have to burn
// 50M clock edges to exercise the timeout path.
// --------------------------------------------------------------------
`ifdef SIM
localparam LC_TIMEOUT_US = 2_000;        // 2ms
`else
localparam LC_TIMEOUT_US = 1_000_000;    // 1s
`endif

// --------------------------------------------------------------------
// Simulation-only: HX717 model conversion periods, per rate code.
//
// Real periods are 100ms / 50ms / 12.5ms / 3.125ms. These are the same
// 32:16:4:1 ratios scaled by 1/12500, so S1/S0 genuinely changes the
// model's behaviour rather than only its pin state, while a full test
// run stays inside ~15ms of simulated time.
//
// The floor here is not arbitrary: a 30-pulse train at 500 kHz PD_SCK
// takes ~60us, so the *fastest* period must stay comfortably above that
// or the model would complete a new conversion in the middle of a train
// still being clocked out - something the real part never does, since
// even 320SPS gives 3.125ms against the same 60us train.
// --------------------------------------------------------------------
`ifdef SIM
localparam LC_SIM_CONV_US_10HZ  = 8000;
localparam LC_SIM_CONV_US_20HZ  = 4000;
localparam LC_SIM_CONV_US_80HZ  = 1000;
localparam LC_SIM_CONV_US_320HZ = 250;

// Datasheet limits the model checks against.
localparam LC_SIM_T1_MIN_NS   = 100;      // 0.1us, the Table value

// The setup time silicon requires between DOUT falling and the first
// PD_SCK rising edge, as distinct from the 0.1us the timing table
// claims. Clocked sooner than this, the part loses bit 23 of the
// conversion (see the msb_lost handling in hx717_sim.v). The exact
// threshold is not published; 1us is the figure implied by the
// datasheet's reference driver, which waits "More than 1uS" after
// `while (DOUT==1);` before clocking.
localparam LC_SIM_T1_SETUP_NS = 1000;
localparam LC_SIM_T3_MIN_NS   = 200;      // 0.2us
localparam LC_SIM_T3_MAX_NS   = 50_000;   // 50us
localparam LC_SIM_T4_MIN_NS   = 200;      // 0.2us
localparam LC_SIM_PD_HOLD_NS  = 80_000;   // 80us -> power down

// T2: PD_SCK rising edge -> DOUT valid, 0.1us max. The model presents
// the bit 90ns after the edge rather than instantly, so the DUT's
// sampling point is actually exercised against the limit.
localparam LC_SIM_T2_NS = 90;

// How often the model re-evaluates its conversion timer and the
// power-down hold. Coarse on purpose - a 1us poll is far finer than
// the conversion periods and the 80us hold it is measuring.
localparam LC_SIM_POLL_NS = 1_000;
`endif

// --------------------------------------------------------------------
// Helper for assembling a REG_LC_CTRL word. Shared so the testbench
// and any future firmware-side generator agree on the packing.
// --------------------------------------------------------------------
function [LC_CTRL_W-1:0] lc_ctrl_word;
    input       en;
    input       pd;
    input [1:0] rate;
    input [1:0] mode;
    input [1:0] pdmode;
    input       single;
    begin
        lc_ctrl_word = { single, pdmode, mode, rate, pd, en };
    end
endfunction
