`timescale 1ns / 1ps
`include "src/main/logging.svh"

// hx717_sim.v
//
// Behavioural model of the Avia Semiconductor HX717, built from the
// datasheet's Fig.2 timing diagram, Tables 3 and 4, and the "Reset and
// Power-Down Modes" section. Simulation only - never synthesised.
//
// The model is deliberately stricter than the part: every timing
// parameter the datasheet specifies (T1, T2, T3 min and max, T4, the
// 80us power-down hold) is checked, and violations bump err_count. A
// testbench that reads a plausible-looking value while err_count is
// non-zero has a driver that happens to work against this model and
// will not necessarily work against silicon.
//
// WHAT IS MODELLED
//   - DOUT high while idle, falling when a result is pending.
//   - 24 result bits shifted MSB first, bit N appearing T2 after the
//     Nth PD_SCK rising edge.
//   - The 25th rising edge returning DOUT high and starting the next
//     conversion.
//   - Total pulse count (25/26/27/28) selecting channel and gain for
//     the *next* conversion, per Table 4.
//   - PD_SCK held high >80us entering power-down, at the depth implied
//     by where the pulse train stopped (25-28 / 29 / 30).
//   - S1/S0 selecting the conversion period, per Table 3.
//
// ====================================================================
// CONVERSION CADENCE - where this model restarts its timer
// ====================================================================
//
// Observed data-ready events land ~299us apart even though
// LC_SIM_CONV_US_320HZ is 250. The extra ~49us is the pulse train:
//
//   data ready --+-- 0.27us --+-- 24 data pulses (47us) --+- pulse 25 -+---- 250us ----+-- data ready
//                |            |                           |            |               |
//             DOUT falls   train starts            conv_deadline set here        299us later
//
// This model sets conv_deadline on the 25th rising edge - the moment
// the host consumes the result - so the modelled period is really
// "250us after you finish reading".
//
// The datasheet describes it differently. Fig.2 labels "One conversion
// period" as spanning DOUT-fall to DOUT-fall, between "Current Output
// Data" and "Next Output Data", with the read shown as a short burst at
// the start and the rest of the period idle. That is a free-running
// converter: the modulator and decimator run off the oscillator, DOUT
// falls whenever a fresh result lands, and reading it does not
// reschedule anything. The remark that a pulse-count change must wait
// until "current conversion period is completed" points the same way -
// the part has a conversion period whether or not anyone is talking to
// it.
//
// This does not matter on hardware. At 320SPS the real period is
// 3125us against a ~60us train (<2% either way); at 10SPS it is 0.06%.
// It is only visible here because the periods are compressed 12500x
// while the *protocol* still runs at real speed, so the train grows
// from ~2% of a period to ~19% of one. Compress time on one side of a
// model and not the other and a ratio that was invisible becomes the
// dominant term.
//
// The driver is unaffected either way: loadcell.v is purely edge-driven
// off DOUT and assumes a period nowhere. The single period-shaped
// constant in it is LC_TIMEOUT_US, at 1s against a 400ms worst case.
//
// ====================================================================
// HOW THIS MODEL DIFFERS FROM SILICON
// ====================================================================
//
// Ordered by how much they matter. This model is a good authority on
// *protocol* - pulse counts, edge timing, channel/gain encoding,
// power-down entry - and that is what the testbench assertions cover.
// It says nothing about accuracy or convergence.
//
// 0. T1 IS MODELLED AS SILICON BEHAVES, NOT AS THE TABLE READS. The
//    timing table gives T1 (DOUT falling edge to first PD_SCK rising
//    edge) a 0.1us minimum. That is not sufficient on real parts: clock
//    much sooner than ~1us after DOUT falls and bit 23 of every
//    conversion is lost, so the host reads (true & 0x7FFFFF) - never
//    negative, bits 22:0 correct. The datasheet's own reference driver
//    reflects this, waiting "More than 1uS" before its first clock.
//    This model enforces LC_SIM_T1_SETUP_NS in addition to the table's
//    0.1us and reproduces the failure exactly - DOUT holds the
//    data-ready low level through pulse 1 and resumes on schedule at
//    pulse 2 - rather than only counting an error. The exact threshold
//    is not published; 1us is the figure the vendor's driver implies.
//
// 1. NO SETTLING TIME. The one with software consequences. The
//    datasheet specifies 4/fo settling "from power up, reset, input
//    channel change and gain change to valid stable output data" -
//    400ms at 10SPS, 12.5ms at 320SPS. This model serves a fully valid
//    sample on the very next conversion. On hardware the first ~4
//    samples after enabling, after any MODE change, and after every
//    wake are NOT trustworthy, and loadcell.v will raise READY for all
//    of them. Nothing in the current test suite catches this. Handle it
//    MCU-side (discard 4 samples after any channel/gain/power
//    transition) or add a settling counter to the peripheral that gates
//    READY for N conversions after mode_armed changes - the latter is
//    the better home for it.
//
// 2. UNREAD RESULTS ARE NEVER OVERWRITTEN. This model latches a result
//    and holds it until it is clocked out. Real silicon almost
//    certainly overwrites: the converter keeps running and DOUT simply
//    stays low, so a stale-but-newer value appears where this model
//    returns the older one. The datasheet is SILENT on this - it is
//    inferred from the free-running architecture and from HX711
//    behaviour. It does not affect loadcell.v, which reads promptly and
//    continuously, but do not treat this model as defining it.
//
// 3. THE INPUT SYNCHRONIZER IS EFFECTIVELY UNTESTED. DOUT is
//    asynchronous to sys_clk and changes within 100ns of an edge the
//    driver itself produces, which is why loadcell.v carries a 3-FF
//    synchronizer. Verilog transitions cleanly at simulator resolution
//    and never produces metastability, so simulation cannot exercise
//    that path at all. Only reasoning and static timing cover it.
//
// 4. NO ANALOG BEHAVIOUR. This model returns sample_value bit-exact.
//    Real: 18.2 noise-free bits at 10Hz but only 15.8 at 320Hz - the
//    bottom ~8 bits at the rate the testbench mostly runs are noise.
//    Plus +/-0.001% FS INL, 0.01mV input offset, +/-15nV/C offset drift
//    and +/-3ppm/C gain drift. Anything doing calibration or
//    thresholding needs those numbers, and no amount of green
//    simulation speaks to them.
//
// 5. NO OSCILLATOR TOLERANCE. The internal RC oscillator drifts, so the
//    real inter-sample interval wanders - which is exactly why the part
//    offers the XI external-clock input "if accurate output data rate
//    is needed". The driver is edge-triggered and immune, but absolute
//    sample timestamps on hardware will not be uniform. The watchdog is
//    deliberately loose for this reason.
//
// 6. COMPRESSED RATES, EXACT RATIOS. 8000/4000/1000/250us against the
//    real 100/50/12.5/3.125ms - the same 32:16:4:1. Fine for exercising
//    S1/S0, but any future logic carrying an absolute time constant
//    would need real values to be tested meaningfully.
//
// 7. IDEALISED T2. Modelled as a fixed 90ns against a 100ns maximum.
//    Pessimistic, which is the right direction, but real silicon is a
//    distribution rather than a constant.
//
// 8. PURELY DIGITAL. No supply ramp, no power-on-reset delay, no
//    current draw. pd_level records which power-down was commanded, not
//    that the regulator actually dropped out. Behaviour above 30 pulses
//    is flagged as an error rather than modelled, since the datasheet
//    does not define it.

module hx717_sim (
    // Pins, as seen from the HX717
    input  wire        s0,
    input  wire        s1,
    input  wire        pd_sck,
    output reg         dout,

    // Testbench stimulus: the 24-bit two's-complement code the ADC
    // should produce. Sampled at the instant each conversion completes.
    input  wire [23:0] sample_value,

    // Testbench observation
    output reg  [23:0] latched_value,     // what this conversion served
    output reg  [31:0] conv_count,        // conversions completed
    output reg  [4:0]  last_pulse_count,  // pulses in the last train
    output reg         chan_b,            // 0 = channel A, 1 = channel B
    output reg  [7:0]  gain,              // 8 / 64 / 128
    output reg         powered_down,
    output reg  [1:0]  pd_level,          // LC_PD_ADC / _ADC_REG / _ALL
    output reg  [31:0] err_count
);

    `include "src/main/io/loadcell_shared.svh"

    // Sized copies of the pulse-count constants; the shared file states
    // them as plain integers, which cannot be part-selected.
    localparam [4:0] P_DATA   = LC_DATA_PULSES;
    localparam [4:0] P_BASE   = LC_PULSES_BASE;
    localparam [4:0] P_PD_REG = LC_PULSES_PD_REG;
    localparam [4:0] P_PD_ALL = LC_PULSES_PD_ALL;
    localparam [4:0] P_MAX    = LC_PULSES_MAX;

    reg [23:0] shifter;
    reg [4:0]  pulses;        // pulses seen since the current result
    reg        data_ready;
    // The first x->0 transition on PD_SCK as the DUT comes out of reset
    // is a negedge as far as Verilog is concerned, with no preceding
    // rising edge to measure against. Suppress T3 until PD_SCK has
    // genuinely been driven high at least once.
    reg        saw_rise;

    // Set when the host starts clocking before the part has finished
    // settling (T1 shorter than LC_SIM_T1_SETUP_NS). While set, the MSB
    // never reaches DOUT - see the note in the header.
    reg        msb_lost;

    real last_rise;
    real last_fall;
    real ready_time;
    real conv_deadline;

    // ------------------------------------------------------------------
    // Conversion period from the rate pins (Table 3).
    // ------------------------------------------------------------------
    function real conv_period_ns;
        input r1;
        input r0;
        begin
            case ({r1, r0})
                LC_RATE_10HZ:  conv_period_ns = LC_SIM_CONV_US_10HZ  * 1000.0;
                LC_RATE_20HZ:  conv_period_ns = LC_SIM_CONV_US_20HZ  * 1000.0;
                LC_RATE_80HZ:  conv_period_ns = LC_SIM_CONV_US_80HZ  * 1000.0;
                default:       conv_period_ns = LC_SIM_CONV_US_320HZ * 1000.0;
            endcase
        end
    endfunction

    task automatic protocol_error;
        input [639:0] what;   // up to 80 characters
        begin
            err_count = err_count + 1;
            `DBG_LOG(("[HX717] %0t PROTOCOL ERROR: %0s (pulses=%0d)", $time, what, pulses));
        end
    endtask

    // ------------------------------------------------------------------
    initial begin
        dout             <= 1'b1;
        shifter          = 24'd0;
        latched_value    = 24'd0;
        pulses           = 5'd0;
        data_ready       = 1'b0;
        powered_down     = 1'b0;
        pd_level         = LC_PD_ADC;
        chan_b           = 1'b0;
        gain             = 8'd128;      // power-on default: CH A, gain 128
        last_pulse_count = 5'd0;
        conv_count       = 32'd0;
        err_count        = 32'd0;
        saw_rise         = 1'b0;
        msb_lost         = 1'b0;
        last_rise        = 0.0;
        last_fall        = 0.0;
        ready_time       = 0.0;
        conv_deadline    = conv_period_ns(s1, s0);
    end

    // ------------------------------------------------------------------
    // Conversion timer and power-down hold detector.
    //
    // Polled rather than event-driven: the two things being measured
    // (conversion periods, an 80us hold) are both far coarser than the
    // poll interval, and polling sidesteps the ordering hazards of
    // racing a delayed process against pin edges.
    // ------------------------------------------------------------------
    always begin
        #(LC_SIM_POLL_NS);

        if (powered_down) begin
            // Nothing converts while asleep; keep the deadline pushed
            // out so waking starts a fresh period.
            conv_deadline = $realtime + conv_period_ns(s1, s0);
        end else if (saw_rise && pd_sck === 1'b1 &&
                     ($realtime - last_rise) >= LC_SIM_PD_HOLD_NS) begin
            // PD_SCK has been high past the 80us threshold. Depth comes
            // from where the pulse train stopped.
            powered_down = 1'b1;
            data_ready   = 1'b0;
            dout         <= 1'b1;
            if      (pulses >= P_PD_ALL) pd_level = LC_PD_ALL;
            else if (pulses >= P_PD_REG) pd_level = LC_PD_ADC_REG;
            else                         pd_level = LC_PD_ADC;
            `DBG_LOG(("[HX717] %0t power down, level=%0d after %0d pulses",
                      $time, pd_level, pulses));
        end else if (!data_ready && ($realtime >= conv_deadline)) begin
            // Conversion complete: latch the stimulus and pull DOUT low.
            // Note the !data_ready guard - an unread result blocks the
            // next conversion here, where silicon would overwrite it
            // (header, difference 2). No settling is applied: this
            // sample is valid even if the channel or gain just changed
            // (header, difference 1).
            shifter       = sample_value;
            latched_value = sample_value;
            data_ready    = 1'b1;
            msb_lost      = 1'b0;
            pulses        = 5'd0;
            dout          <= 1'b0;
            ready_time    = $realtime;
            conv_count    = conv_count + 1;
            `DBG_LOG(("[HX717] %0t data ready, value=%06h (ch%0s gain=%0d)",
                      $time, sample_value, chan_b ? "B" : "A", gain));
        end
    end

    // ------------------------------------------------------------------
    // PD_SCK rising edge: shift a bit out, or take a command pulse.
    // ------------------------------------------------------------------
    always @(posedge pd_sck) begin
        if (pulses == 5'd0) begin
            // T1: DOUT falling edge to first PD_SCK rising edge.
            if (data_ready && (($realtime - ready_time) < LC_SIM_T1_MIN_NS))
                protocol_error("T1 violated (DOUT fall to first PD_SCK rise < 0.1us)");

            // The part needs considerably longer than the table's 0.1us
            // before it can put the MSB on DOUT. Clock too early and
            // bit 23 is skipped outright - DOUT simply holds the
            // data-ready low level through pulse 1, and pulse 2 presents
            // bit 22 on schedule as if nothing happened.
            if (data_ready && (($realtime - ready_time) < LC_SIM_T1_SETUP_NS)) begin
                msb_lost = 1'b1;
                protocol_error("T1 too short for the part to present the MSB - bit 23 will be lost");
            end
        end else begin
            // T4: PD_SCK low time.
            if (($realtime - last_fall) < LC_SIM_T4_MIN_NS)
                protocol_error("T4 violated (PD_SCK low time < 0.2us)");
        end

        saw_rise  = 1'b1;
        last_rise = $realtime;
        pulses    = pulses + 5'd1;

        if (pulses <= P_DATA) begin
            if (!data_ready)
                protocol_error("data pulse issued while DOUT high (no result pending)");
            // Bit for pulse N is bit (24-N), MSB first, valid T2 later.
            if ((pulses == 5'd1) && msb_lost) begin
                // Bit 23 never appears. DOUT stays at the data-ready low
                // level for this pulse; every later pulse is on schedule.
                // The host captures {0, bit22..bit0} and reads back
                // (true & 0x7FFFFF) - always non-negative, bits 22:0
                // intact. Distinct from sampling on the wrong clock
                // edge, which loses bit alignment and yields
                // (true >> 1).
                dout <= #(LC_SIM_T2_NS) 1'b0;
            end else begin
                dout <= #(LC_SIM_T2_NS) shifter[P_DATA - pulses];
            end
        end else if (pulses == P_BASE) begin
            // 25th rising edge: DOUT returns high, this result is
            // consumed, and the next conversion begins. Restarting the
            // timer HERE rather than at the previous DOUT fall is the
            // cadence simplification described in the header - it adds
            // one train time to every period.
            dout          <= #(LC_SIM_T2_NS) 1'b1;
            data_ready    = 1'b0;
            conv_deadline = $realtime + conv_period_ns(s1, s0);
        end

        // Channel / gain selection is the pulse count itself (Table 4).
        if (pulses >= P_BASE) begin
            last_pulse_count = pulses;
            case (pulses)
                5'd25: begin chan_b = 1'b0; gain = 8'd128; end
                5'd26: begin chan_b = 1'b1; gain = 8'd64;  end
                5'd27: begin chan_b = 1'b0; gain = 8'd64;  end
                5'd28: begin chan_b = 1'b1; gain = 8'd8;   end
                default: ; // 29/30 are power-down depth, not selection:
                           // the chip keeps the last saved setup.
            endcase
        end

        if (pulses > P_MAX)
            protocol_error("more than 30 PD_SCK pulses in one train");
    end

    // ------------------------------------------------------------------
    // PD_SCK falling edge: T3 checks, or wake-up.
    // ------------------------------------------------------------------
    always @(negedge pd_sck) begin
        if (powered_down) begin
            powered_down  = 1'b0;
            pulses        = 5'd0;
            data_ready    = 1'b0;
            dout          <= 1'b1;
            // Datasheet: on wake the chip returns to the setup
            // conditions it had before the power-down, so chan_b/gain
            // are deliberately left alone here.
            conv_deadline = $realtime + conv_period_ns(s1, s0);
            `DBG_LOG(("[HX717] %0t wake (ch%0s gain=%0d)", $time,
                      chan_b ? "B" : "A", gain));
        end else if (saw_rise) begin
            if (($realtime - last_rise) < LC_SIM_T3_MIN_NS)
                protocol_error("T3 violated (PD_SCK high time < 0.2us)");
            if (($realtime - last_rise) > LC_SIM_T3_MAX_NS)
                protocol_error("T3 violated (PD_SCK high time > 50us)");
        end
        last_fall = $realtime;
    end

endmodule
