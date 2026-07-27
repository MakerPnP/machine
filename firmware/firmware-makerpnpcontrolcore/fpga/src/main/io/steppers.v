`include "src/main/logging.svh"

// steppers.v
//
// 8-motor (2 banks x 4) step/dir pulse generator sharing one arithmetic
// datapath and one control FSM across all motors, to keep LC usage low on
// an iCE40HX8K. No multipliers/DSPs are used anywhere - only add/subtract/
// compare.
//
// Each motor has its own independent segment table and live motion state.
// Motor 3 can be loaded and started while motor 0 is mid-profile; all 8
// can be loaded with completely different sequences and started together
// (or staggered) with separate start strobes. Segment storage is
// addressed by {motor_instance, segment_index, word_select} - motor N's
// writes/reads only ever touch motor N's slice of the table. Nothing here
// shares live state (period/position/segment pointer/etc) across motors.
//
// ARCHITECTURE
// ---------------------------------------------------------------------
// - Per-motor segment table: up to MAX_SEG ramp segments per motor, held
//   in explicit SB_RAM40_4K primitives (see the memory subsystem comment
//   below for the full layout).
// - Per-motor live state (current period, delta, remaining step count,
//   dir, cmd, segment pointer, position, moving flag, countdown) is a
//   small 8-entry register file - unavoidable persistent state, but an
//   order of magnitude smaller than replicating a full ALU+sequencer per
//   motor.
// - One shared FSM (IDLE/RD0-RD4/COMMIT/FIRE_CALC) walks the 8 motors:
//     * start/stop strobes are serviced by a combinational lowest-set-bit
//       priority encoder, giving bounded response latency independent of
//       where the round-robin currently is.
//     * once per stepper_clk tick, the engine round-robins through all 8
//       motor slots (scan_idx), decrementing each active motor's
//       countdown, firing a step pulse when it reaches zero, and
//       reloading the next segment on segment/profile boundaries.
//
// SEGMENT READBACK: reading REG_STEP_SEG_CTST/SPDM back only returns
// correct data while the addressed motor has not yet been started. Once
// a motor is moving, the shared motion engine can be using the same
// physical memory read port to load that motor's own segment data at any
// time, and a concurrent host read is not synchronized against it - it
// will return whatever address happened to be on the port that cycle,
// which is only guaranteed to be the host's own address when the engine
// isn't using the port at all. There's no production need to read
// segment data back from a running motor; readback exists purely so a
// host can verify a just-uploaded profile before starting it.
//
// STEP PULSE WIDTH: REG_STEP_PLS_CONFIG holds an independent 4-bit
// preset per motor (bits [3:0]/[7:4]/.../[31:28] for motors 0-7).
// REG_STEP_PLS_PRESCALER holds an independent 6-bit prescaler per bank -
// bank 0 in bits [5:0] (the register's low 16-bit half) and bank 1 in
// bits [21:16] (the same position within the high 16-bit half), rather
// than packed back-to-back, so either field can widen later, or other
// per-bank config bits can be added, without touching the other bank's
// half. A motor's step pulse starts the instant that motor fires and
// stays high for exactly (prescaler+1)*(preset+1) sys_clk cycles - a
// per-bank divide value multiplied by a per-motor count value, the same
// shape as a UART baud-rate generator or an STM32 timer's PSC/ARR pair -
// reaching 474 distinct exact widths from 20ns up to 20.48us at 50 MHz.
// Both the pulse's start time and its width are exact, with no timing
// drift on either. See the "Step pulse width" comment further down for
// the exact mechanism (two small per-motor counters, no lookup table or
// adder needed at all). Reset default is preset 0, prescaler 0 on both
// banks - the shortest possible pulse (1 cycle, 20ns) - which is NOT
// safe for any real driver IC without explicit configuration first (see
// the NEMA17 driver minimums a few paragraphs down); this register pair
// works like many real MCU peripheral config registers (e.g. a UART's
// baud rate divisor) in that respect - the host is expected to program
// it before relying on it, not assume a production-safe value out of
// reset.
//
// STANDING DESIGN CHOICES
// ---------------------------------------------------------------------
// - bank_enable_pins is simply {2{global_motor_en}}.
// - CMD_MOVE_HALT and CMD_MOVE_HALT_WAIT are handled identically (both
//   pause and wait for a fresh start strobe).
// - A stop strobe is a full abort: moving<=0, segment pointer reset to 0,
//   remaining steps cleared.
// - REG_STEP_TX_CONFIG reconfigures a motor immediately, including
//   aborting anything in flight for that motor. If it lands in the exact
//   sys_clk cycle the shared engine's ST_COMMIT is finishing a load for
//   that same motor (only reachable by reconfiguring a motor within a few
//   cycles of having just started it), the commit's mot_moving<=1 can
//   overwrite the reconfiguration's mot_moving<=0 for that one cycle,
//   since the engine's case(state) block runs after the bus decode in
//   program order. Not a concern for normal usage (TX_CONFIG, then load
//   segments, then start), only for reconfiguring a motor within roughly
//   100ns of having just started that specific motor.
// ---------------------------------------------------------------------


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
    input  wire        global_motor_en
);

    `include "src/main/io/steppers_regs.svh"
    `include "src/main/io/steppers_shared.svh"

    localparam NUM_MOTORS = 8;

    // Segments per motor. SEG_IDX_BITS is however many bits are needed to
    // index MAX_SEG segments; the memory subsystem below scales its bank
    // count directly from it, so raising MAX_SEG raises EBR usage
    // (roughly doubling NUM_BANKS each time SEG_IDX_BITS grows by 1).
    localparam [7:0] MAX_SEG          = 8'd127;
    localparam [7:0] MAX_SEG_IDX_LAST = 8'd126; // MAX_SEG-1

    localparam SEG_IDX_BITS   = 7;                     // clog2(MAX_SEG)
    localparam WORD_ADDR_BITS = 3 + SEG_IDX_BITS + 1;  // motor+seg_idx+word_sel = 11
    localparam BANK_SEL_BITS  = WORD_ADDR_BITS - 8;    // bits selecting among banks
    localparam NUM_BANKS      = 32'd1 << BANK_SEL_BITS; // each SB_RAM40_4K bank is 256 words deep

    // Bus-facing config latches. Declared here (ahead of the memory
    // subsystem below, which references them in wire initializers)
    // because Icarus requires declaration-before-use in that context.
    reg [7:0]  tx_num_points;
    reg [2:0]  tx_motor_instance;
    reg [7:0]  tx_write_ptr;   // REG_STEP_SEG_CTST/SPDM write pointer
    reg [7:0]  tx_read_ptr;    // and an independent read pointer - both
                               // reset to 0 by REG_STEP_TX_CONFIG, but
                               // each only advances on its own kind of
                               // access, so a write-then-read-back
                               // sequence with no TX_CONFIG re-issue in
                               // between still starts the read at index 0.

    // Bus-protocol sub-FSM state for REG_STEP_SEG_CTST/SPDM only - every
    // other register acks in a single cycle. Writes ack at HI (3 cycles
    // total); reads wait one further cycle before acking at ACK (4 cycles
    // total).
    localparam BUS_STATE_IDLE = 2'd0;
    localparam BUS_STATE_LO   = 2'd1;
    localparam BUS_STATE_HI   = 2'd2;
    localparam BUS_STATE_ACK  = 2'd3;
    reg [1:0]  bus_state;

    // Shared engine FSM state constants and the state/target registers -
    // declared here (ahead of the memory subsystem below, which
    // references `state` in engine_owns_port's wire initializer) for the
    // same Icarus declaration-before-use reason as above.
    localparam ST_IDLE   = 3'd0;
    localparam ST_RD0    = 3'd1;
    localparam ST_RD1    = 3'd2;
    localparam ST_RD2    = 3'd3;
    localparam ST_RD3    = 3'd4;
    localparam ST_RD4    = 3'd5;
    localparam ST_COMMIT = 3'd6;
    localparam ST_FIRE_CALC = 3'd7;
    reg [2:0]  state;
    reg [2:0]  target;       // motor index currently being (re)loaded

    // ------------------------------------------------------------------
    // Segment table memory. One address space of NUM_MOTORS x MAX_SEG x 2
    // words, built from explicit SB_RAM40_4K primitives (256 x 16 each)
    // rather than an inferred array, so it maps to block RAM
    // unambiguously. A 32-bit word is split into two 16-bit halves (lo/
    // hi), each bank getting its own pair of RAM instances; the address
    // space is wider than one bank's 256-deep range, so it's split across
    // NUM_BANKS banks, selected by the top BANK_SEL_BITS address bits.
    //
    // There is exactly one physical read port, shared between the motion
    // engine (loading a segment while a motor runs) and the bus (a host
    // reading CTST/SPDM back). The engine takes priority whenever it's
    // actively using the port (states ST_RD0-ST_RD4); otherwise the bus's
    // own address is presented. See the "SEGMENT READBACK" note at the
    // top of the file for what this means for host reads.
    //
    // Address layout: {motor[2:0], seg_idx[SEG_IDX_BITS-1:0], word_sel}.
    // The top bit of motor selects which half of the bank space a motor's
    // data lives in along with the high segment-index bits; in practice
    // the low BANK_SEL_BITS bits of {motor, seg_idx} select the bank and
    // the remaining bits address within it - the generate loop below and
    // the address split (shared_raddr[WORD_ADDR_BITS-1:8] for bank,
    // [7:0] for in-bank) handle this uniformly regardless of exactly
    // where the bank/motor/segment boundaries fall.
    //
    // SB_RAM40_4K has a genuine 1-cycle registered-address read latency:
    // the address must be stable for a full clock cycle before RDATA
    // reflects it, not combinationally. ST_RD0-ST_RD4 below account for
    // this explicitly (one cycle to present each address, one more to
    // read the result).
    // ------------------------------------------------------------------

    // Write side (bus only - the engine never writes segment data).
    // Purely combinational, asserted for exactly the one cycle
    // (BUS_STATE_HI with bus_we) the write should commit.
    wire word_sel_bus = (bus_addr == REG_STEP_SEG_SPDM);
    wire [WORD_ADDR_BITS-1:0] wr_word_addr = {tx_motor_instance, tx_write_ptr[SEG_IDX_BITS-1:0], word_sel_bus};
    wire wr_en = bus_stb && (bus_state == BUS_STATE_HI) && bus_we &&
                 (bus_addr == REG_STEP_SEG_CTST || bus_addr == REG_STEP_SEG_SPDM);
    wire [7:0] wr_addr = wr_word_addr[7:0];
    wire [BANK_SEL_BITS-1:0] wr_bank = wr_word_addr[WORD_ADDR_BITS-1:8];
    wire [15:0] wr_data_lo = bus_din[15:0];
    wire [15:0] wr_data_hi = bus_din[31:16];

    // Engine-side read address (set by ST_RD0/ST_RD2 below, held stable
    // across the following state while the RAM captures it).
    reg [WORD_ADDR_BITS-1:0] mem_addr;

    // Bus-side read address - fully determined by (tx_motor_instance,
    // tx_read_ptr, which of CTST/SPDM is selected), stable for the whole
    // multi-cycle bus transaction.
    wire [WORD_ADDR_BITS-1:0] rd_word_addr = {tx_motor_instance, tx_read_ptr[SEG_IDX_BITS-1:0], word_sel_bus};

    // Shared read port arbitration: the engine wins whenever it's
    // actively presenting an address (ST_RD0-ST_RD4); the bus gets the
    // port at every other time, including while a motor is moving but the
    // engine isn't between reload cycles for it.
    wire engine_owns_port = (state == ST_RD0) || (state == ST_RD1) || (state == ST_RD2) ||
                            (state == ST_RD3) || (state == ST_RD4);
    wire [WORD_ADDR_BITS-1:0] shared_raddr = engine_owns_port ? mem_addr : rd_word_addr;
    wire [BANK_SEL_BITS-1:0]  rd_bank_sel  = shared_raddr[WORD_ADDR_BITS-1:8];

    wire [NUM_BANKS-1:0] bank_we;
    wire [15:0] bank_rdata_lo [0:NUM_BANKS-1];
    wire [15:0] bank_rdata_hi [0:NUM_BANKS-1];

    genvar gb;
    generate
        for (gb = 0; gb < NUM_BANKS; gb = gb + 1) begin : SEG_BANK
            assign bank_we[gb] = wr_en && (wr_bank == gb[BANK_SEL_BITS-1:0]);

            SB_RAM40_4K #(.WRITE_MODE(0), .READ_MODE(0)) ram_lo (
                .RCLK(sys_clk), .RCLKE(1'b1), .RE(1'b1),
                .RADDR({3'b000, shared_raddr[7:0]}), .RDATA(bank_rdata_lo[gb]),
                .WCLK(sys_clk), .WCLKE(1'b1), .WE(bank_we[gb]),
                .WADDR({3'b000, wr_addr}), .WDATA(wr_data_lo), .MASK(16'h0000)
            );
            SB_RAM40_4K #(.WRITE_MODE(0), .READ_MODE(0)) ram_hi (
                .RCLK(sys_clk), .RCLKE(1'b1), .RE(1'b1),
                .RADDR({3'b000, shared_raddr[7:0]}), .RDATA(bank_rdata_hi[gb]),
                .WCLK(sys_clk), .WCLKE(1'b1), .WE(bank_we[gb]),
                .WADDR({3'b000, wr_addr}), .WDATA(wr_data_hi), .MASK(16'h0000)
            );
        end
    endgenerate

    wire [31:0] shared_rdata = {bank_rdata_hi[rd_bank_sel], bank_rdata_lo[rd_bank_sel]};

    // CTST word layout (as written by the host):
    //   [31:28] reserved
    //   [27]    ramp              (RAMP_DOWN/RAMP_UP)
    //   [26]    dir               (DIR_NORMAL/DIR_REVERSE)
    //   [25:24] cmd               (CMD_MOVE/CMD_MOVE_HALT/CMD_MOVE_HALT_WAIT)
    //   [23:0]  n_steps           (steps in this segment)
    // SPDM word layout:
    //   [31:16] start_period (SP)
    //   [15:0]  delta_magnitude (DM)

    // ------------------------------------------------------------------
    // Per-motor live context (small register file, 8 entries) - fully
    // independent per motor.
    // ------------------------------------------------------------------
    reg [15:0] mot_period    [0:7];
    reg [15:0] mot_dm        [0:7];
    reg [23:0] mot_remaining [0:7];
    reg [7:0]  mot_segptr    [0:7];   // 0..MAX_SEG, MAX_SEG means "exhausted"
    reg [7:0]  mot_totalseg  [0:7];
    reg        mot_moving    [0:7];
    reg [31:0] mot_position  [0:7];
    reg [15:0] mot_countdown [0:7];
    reg        mot_dir       [0:7];
    reg [1:0]  mot_cmd       [0:7];
    reg        mot_incr      [0:7];

    // Start/stop strobe bitmaps (set by bus writes, cleared by the engine)
    reg [7:0]  pending_start;
    reg [7:0]  pending_stop;

    // ------------------------------------------------------------------
    // Step pulse width, UART-baud-rate-generator/STM32-timer style: a
    // per-bank prescaler (divide) and a per-motor preset (count), which
    // MULTIPLY rather than add. REG_STEP_PLS_CONFIG holds one 4-bit
    // preset per motor; REG_STEP_PLS_PRESCALER holds one 6-bit prescaler
    // per bank - bank 0 in bits[5:0], bank 1 in bits[21:16] (see the bus
    // decode below), so each bank gets a full 16-bit half of the
    // register to grow into later rather than the two fields packed
    // back-to-back (motor[2] selects bank - motors 0-3 are bank 0, 4-7
    // are bank 1, same split as the memory subsystem's bank addressing).
    // Register value V means "V+1" throughout (0 still means "1"), so:
    //   divide = pls_prescaler[bank_of(m)] + 1   (1-64 sys_clk cycles per tick)
    //   count  = pls_preset[m] + 1               (1-16 ticks)
    //   width  = divide * count sys_clk cycles    (1-1024 cycles, exactly)
    //
    // Deliberately asymmetric: prescaler is wider (6 bits) than preset (4
    // bits) because prescaler is per-BANK (only 2 instances) while preset
    // is per-MOTOR (8 instances) - widening prescaler buys more range for
    // less LC cost than widening preset would, since only prescaler's
    // config storage is cheap to widen (pulse_inner, the per-motor
    // counter that actually holds it while counting down, still costs
    // one bit per motor either way).
    //
    // An earlier version of this made preset and prescaler both act as
    // exponents (K = preset+prescaler, width = 2^K), which turned out to
    // be the wrong shape entirely: since the SUM saturated at a plain
    // 4-bit K, the 256 possible (preset, prescaler) combinations only
    // ever produced 16 distinct widths (the powers of 2 from 1 to
    // 32768) - most combinations were redundant with each other, and
    // plenty of practically useful widths (e.g. exactly 100 cycles, 2us
    // at 50MHz) were simply unreachable no matter what was configured.
    // Multiplying instead of adding fixes this: with the current 6-bit
    // prescaler and 4-bit preset, the 1024 possible combinations reach
    // 474 distinct widths, including awkward-looking targets like 100
    // (prescaler=9, preset=9) that no power of 2 is close to.
    //
    // Each motor has its own pair of small down-counters - pulse_inner
    // (6 bits, counts sys_clk cycles within one tick) and pulse_outer (4
    // bits, counts ticks) - plus a pulse_active flag. The instant a
    // motor fires: pulse_active is set, pulse_inner is loaded with that
    // motor's bank's prescaler, pulse_outer is loaded with that motor's
    // own preset, and step_pulse goes high immediately (exact start
    // time, same as before). Every cycle after that: pulse_inner
    // decrements; whenever it reaches zero, that's one tick - pulse_outer
    // decrements and pulse_inner reloads from the prescaler for the next
    // tick - until pulse_outer ALSO reaches zero on a tick boundary, at
    // which point the pulse ends (exact width, same guarantee as
    // before). Both counters are entirely private to each motor, freshly
    // reloaded at that motor's own fire event - never a shared,
    // continuously-running counter whose phase could drift relative to
    // any particular motor's fire schedule the way an earlier version's
    // shared prescale counter did.
    //
    // This is cheaper than the original additive version, not just more
    // capable: two small counters plus two cheap zero-checks per motor,
    // instead of one wide (15-bit) counter fed by a 16-way lookup table
    // and a saturating adder.
    //
    // Reset default is preset 0, prescaler 0 on both banks (divide=1,
    // count=1, width=1 cycle = 20ns at 50 MHz) - the shortest possible
    // pulse, not a safe one. Common NEMA17 driver ICs need considerably
    // more (A4988 ~1us, DRV8825 ~1.9us, TMC2209 ~100ns minimum/~1us
    // recommended), so unlike the segment table or motion parameters,
    // this is a register pair the host must explicitly configure before
    // relying on it - there's no single default width that would be
    // safe for every attached driver without knowing which one it is,
    // and defaulting to the shortest possible pulse means an
    // unconfigured system fails obviously (no movement, or a driver
    // that visibly rejects too-short pulses) rather than silently
    // working by chance on some drivers and not others.
    //
    // Configuring a pulse width wider than the interval between steps
    // (i.e. wider than the segment's SP allows) is still a configuration
    // error, same as it would be on real driver hardware - you cannot
    // physically step faster than your own pulse width permits - but it
    // no longer produces an ambiguous output. The previous pulse always
    // gets its full configured width (never cut short), and the next
    // one is deferred by exactly one cycle if it would otherwise start
    // before the previous one finished - guaranteeing at least one low
    // cycle between any two steps for the same motor no matter how the
    // configured width compares to the step interval, so anything
    // watching step_pins can always tell the steps apart. See
    // pulse_force_reload below for the mechanism.
    // ------------------------------------------------------------------
    reg [3:0] pls_preset    [0:7]; // per motor: count-1
    reg [5:0] pls_prescaler [0:1]; // per bank (index 0 = bank 0, 1 = bank 1): divide-1
    reg [5:0] pulse_inner   [0:7]; // per-motor: sys_clk cycles left in the current tick
    reg [3:0] pulse_outer   [0:7]; // per-motor: ticks left
    reg       pulse_active  [0:7]; // this motor's pulse is currently running
    reg       pulse_force_reload [0:7]; // previous pulse was still active
                                        // when this motor fired again -
                                        // forcing one low cycle before the
                                        // new pulse starts (see the FIRE
                                        // branch and the per-cycle pulse
                                        // output block further down)

    // A fire arriving while the previous pulse is still active (pulse
    // width configured wider than the step interval) doesn't corrupt
    // anything, but it used to mean the output never dropped low between
    // those steps at all - no falling edge, so no way for anything
    // watching step_pins to tell that two (or more) steps happened
    // rather than one. Now: the previous pulse runs its FULL interval
    // (never cut short), and the new one is deferred by exactly one
    // cycle - forced low for that one cycle, then started - so there's
    // always at least one low cycle between any two steps for the same
    // motor, regardless of configured width vs. interval. Internal
    // motion state (position, remaining count, segment advance) is
    // completely unaffected either way - this only changes how the
    // output pulse is shaped when misconfigured.

    // Step pulse outputs (1+ sys_clk cycles wide per fired step,
    // stretched to the configured width by pulse_inner/pulse_outer above)
    reg [7:0]  step_pulse;

    // ------------------------------------------------------------------
    // New-request detection. A request counts as new whenever (address,
    // we) differs from the last one this block started servicing, or
    // bus_stb was low last cycle - this correctly recognizes a fresh
    // request even if bus_stb toggles high/low/high entirely between two
    // posedges (zero simulation-time gap between transactions), which
    // gating on "!bus_ack" alone would miss.
    // ------------------------------------------------------------------
    reg [7:0]  last_bus_addr;
    reg        last_bus_we;
    reg        last_bus_valid;
    wire is_new_request = bus_stb && (!last_bus_valid || (bus_addr != last_bus_addr) || (bus_we != last_bus_we));

    // Round-robin / tick bookkeeping
    reg [2:0]  scan_idx;
    reg        tick_flag;
    reg [7:0]  tick_done;
    reg        stepper_clk_d;
    wire       tick_strobe = stepper_clk & ~stepper_clk_d;

    // ctst_word/spdm_word: captured segment data, used by ST_RD2/RD4/COMMIT.
    reg [31:0] ctst_word;
    reg [31:0] spdm_word;

    // Scratch combinational variables (blocking-assigned, used only
    // within the same always-block evaluation, always fully computed
    // before being written to any array).
    reg [23:0] new_remaining;
    reg [15:0] new_period;
    reg [15:0] new_countdown;
    reg [7:0]  new_segptr;
    reg [31:0] pos_delta;
    reg [31:0] new_position;
    reg [2:0]  pidx;
    reg [7:0]  segptr_rd; // scratch: mot_segptr[target] read out before use

    // FIRE-completion snapshot registers. The step pulse and position
    // update happen the same cycle the countdown reaches zero (ST_IDLE
    // below); the remaining/period/segment-boundary bookkeeping is
    // deferred one cycle (ST_FIRE_CALC) into these plain registers,
    // keeping the scan_idx-indexed array read and the arithmetic on
    // separate cycles for timing closure.
    reg [23:0] snap_remaining;
    reg [15:0] snap_period;
    reg [15:0] snap_dm;
    reg        snap_incr;
    reg [1:0]  snap_cmd;
    reg [7:0]  snap_segptr;
    reg [7:0]  snap_totalseg;

    wire any_stop  = |pending_stop;
    wire any_start = |pending_start;

    integer i;
    integer j;

    // ------------------------------------------------------------------
    // Lowest-set-bit priority encoder - gives start/stop strobes a
    // bounded response latency independent of where scan_idx happens to
    // be in its round-robin sweep.
    // ------------------------------------------------------------------
    function [2:0] priority_idx;
        input [7:0] bits;
        begin
            casez (bits)
                8'b???????1: priority_idx = 3'd0;
                8'b??????10: priority_idx = 3'd1;
                8'b?????100: priority_idx = 3'd2;
                8'b????1000: priority_idx = 3'd3;
                8'b???10000: priority_idx = 3'd4;
                8'b??100000: priority_idx = 3'd5;
                8'b?1000000: priority_idx = 3'd6;
                8'b10000000: priority_idx = 3'd7;
                default:     priority_idx = 3'd0;
            endcase
        end
    endfunction

    // ------------------------------------------------------------------
    // Shared period ALU: saturating add (period increasing) / floor-at-1
    // subtract (period decreasing).
    // ------------------------------------------------------------------
    function [15:0] calc_next_period;
        input [15:0] period;
        input [15:0] delta;
        input        incr;
        reg   [16:0] sum;
        begin
            if (incr) begin
                sum = {1'b0, period} + {1'b0, delta};
                calc_next_period = (sum > 17'd65535) ? 16'd65535 : sum[15:0];
            end else begin
                if (delta >= period)
                    calc_next_period = 16'd1;
                else
                    calc_next_period = period - delta;
            end
        end
    endfunction

    // ------------------------------------------------------------------
    // Output pin assigns
    // ------------------------------------------------------------------
    assign step_pins = step_pulse;
    assign dir_pins   = {mot_dir[7], mot_dir[6], mot_dir[5], mot_dir[4],
                         mot_dir[3], mot_dir[2], mot_dir[1], mot_dir[0]};
    assign bank_enable_pins = {2{global_motor_en}};

    always @(posedge sys_clk) begin
        if (reset) begin
            bus_dout          <= 32'h00000000;
            bus_ack           <= 1'b0;

            tx_num_points     <= 8'd0;
            tx_motor_instance <= 3'd0;
            tx_write_ptr      <= 8'd0;
            tx_read_ptr       <= 8'd0;
            bus_state         <= BUS_STATE_IDLE;
            last_bus_addr     <= 8'd0;
            last_bus_we       <= 1'b0;
            last_bus_valid    <= 1'b0;

            pending_start     <= 8'd0;
            pending_stop      <= 8'd0;
            step_pulse        <= 8'd0;
            pls_prescaler[0]  <= 6'd0; // default: shortest possible pulse (divide=1, count=1 -> 1 cycle = 20ns) -
            pls_prescaler[1]  <= 6'd0; // see the "Step pulse width" comment above for why

            scan_idx          <= 3'd0;
            tick_flag         <= 1'b0;
            tick_done         <= 8'd0;
            stepper_clk_d     <= 1'b0;

            state             <= ST_IDLE;
            target            <= 3'd0;
            snap_remaining    <= 24'd0;
            snap_period       <= 16'd0;
            snap_dm           <= 16'd0;
            snap_incr         <= 1'b0;
            snap_cmd          <= 2'd0;
            snap_segptr       <= 8'd0;
            snap_totalseg     <= 8'd0;
            mem_addr          <= {WORD_ADDR_BITS{1'b0}};

            for (i = 0; i < 8; i = i + 1) begin
                mot_period[i]    <= 16'd0;
                mot_dm[i]        <= 16'd0;
                mot_remaining[i] <= 24'd0;
                mot_segptr[i]    <= 8'd0;
                mot_totalseg[i]  <= 8'd0;
                mot_moving[i]    <= 1'b0;
                mot_position[i]  <= 32'd0;
                mot_countdown[i] <= 16'd0;
                mot_dir[i]       <= 1'b0;
                mot_cmd[i]       <= 2'd0;
                mot_incr[i]      <= 1'b0;
                pls_preset[i]   <= 4'd0; // default: shortest possible pulse (paired with prescaler=0)
                pulse_inner[i]  <= 6'd0;
                pulse_outer[i]  <= 4'd0;
                pulse_active[i] <= 1'b0;
                pulse_force_reload[i] <= 1'b0;
            end
        end else begin
            // Default: pulse outputs clear every cycle unless re-asserted
            // below.
            step_pulse    <= 8'd0;
            bus_ack       <= 1'b0;
            stepper_clk_d <= stepper_clk;

            // A new tick always (re)arms full service of all 8 motors,
            // independent of bus activity or FSM state below.
            if (tick_strobe) begin
                tick_flag <= 1'b1;
                tick_done <= 8'd0;
            end

            // ============================================================
            // Bus slave interface
            // ============================================================
            if (!bus_stb) begin
                bus_state      <= BUS_STATE_IDLE;
                last_bus_valid <= 1'b0;
            end else if (bus_addr == REG_STEP_SEG_CTST || bus_addr == REG_STEP_SEG_SPDM) begin
                // Segment table access goes through a multi-cycle
                // IDLE->LO->HI->(ACK on read) handshake. Writes commit
                // combinationally into the SB_RAM40_4K instances above
                // (bank_we/wr_addr/wr_data_lo/wr_data_hi are already
                // asserted this exact cycle whenever bus_state==HI and
                // bus_we) and ack at HI (3 cycles total); reads pull the
                // already-assembled shared_rdata but still wait for the
                // ACK cycle (4 cycles total). Only entry into LO is gated
                // on is_new_request - once latched, LO->HI->ACK continues
                // every cycle regardless of whether bus_stb ever reads
                // low, since the master is expected to hold (addr, we,
                // din) steady for the whole multi-cycle transaction.
                case (bus_state)
                    BUS_STATE_IDLE: begin
                        if (is_new_request) begin
                            last_bus_addr  <= bus_addr;
                            last_bus_we    <= bus_we;
                            last_bus_valid <= 1'b1;
                            bus_state      <= BUS_STATE_LO;
                        end
                    end
                    BUS_STATE_LO: begin
                        bus_state <= BUS_STATE_HI;
                    end
                    BUS_STATE_HI: begin
                        if (bus_we) begin
                            // Physical write already committed this cycle
                            // via the combinational bank_we/wr_addr/
                            // wr_data_lo/wr_data_hi feeding the RAM
                            // instances above - just handle the pointer
                            // advance and ack here.
                            if (bus_addr == REG_STEP_SEG_SPDM && tx_write_ptr < MAX_SEG_IDX_LAST)
                                tx_write_ptr <= tx_write_ptr + 8'd1;
                            bus_dout  <= 32'h00000000;
                            bus_ack   <= 1'b1;
                            bus_state <= BUS_STATE_IDLE;
                        end else begin
                            bus_state <= BUS_STATE_ACK;
                        end
                    end
                    BUS_STATE_ACK: begin
                        bus_dout  <= shared_rdata;
                        bus_ack   <= 1'b1;
                        bus_state <= BUS_STATE_IDLE;
                        if (bus_addr == REG_STEP_SEG_SPDM && tx_read_ptr < MAX_SEG_IDX_LAST)
                            tx_read_ptr <= tx_read_ptr + 8'd1;
                    end
                    default: bus_state <= BUS_STATE_IDLE;
                endcase
            end else if (is_new_request) begin
                    last_bus_addr  <= bus_addr;
                    last_bus_we    <= bus_we;
                    last_bus_valid <= 1'b1;
                    case (bus_addr)
                        REG_STEP_CTRL: begin
                            if (bus_we) begin
                                pending_start <= pending_start | {bus_din[19:16], bus_din[11:8]};
                                pending_stop  <= pending_stop  | {bus_din[23:20], bus_din[15:12]};
                                bus_dout <= 32'h00000000;
                            end else begin
                                bus_dout <= {24'd0,
                                             mot_moving[7], mot_moving[6], mot_moving[5], mot_moving[4],
                                             mot_moving[3], mot_moving[2], mot_moving[1], mot_moving[0]};
                            end
                            bus_ack <= 1'b1;
                        end

                        REG_STEP_TX_CONFIG: begin
                            if (bus_we) begin
                                tx_num_points     <= bus_din[7:0];
                                tx_motor_instance <= bus_din[10:8];
                                tx_write_ptr      <= 8'd0;
                                tx_read_ptr       <= 8'd0;
                                // Only this one motor's context is
                                // touched - every other motor's live
                                // state and table are completely
                                // untouched by this write. See "REG_STEP_
                                // TX_CONFIG reconfigures..." in the
                                // standing design choices above.
                                mot_totalseg[bus_din[10:8]]  <= (bus_din[7:0] > MAX_SEG) ? MAX_SEG : bus_din[7:0];
                                mot_segptr[bus_din[10:8]]    <= 8'd0;
                                mot_moving[bus_din[10:8]]    <= 1'b0;
                                mot_remaining[bus_din[10:8]] <= 24'd0;
                            end else begin
                                bus_dout <= {21'd0, tx_motor_instance, tx_num_points};
                            end
                            bus_ack <= 1'b1;
                        end

                        REG_STEP_PLS_CONFIG: begin
                            if (bus_we) begin
                                pls_preset[0] <= bus_din[3:0];
                                pls_preset[1] <= bus_din[7:4];
                                pls_preset[2] <= bus_din[11:8];
                                pls_preset[3] <= bus_din[15:12];
                                pls_preset[4] <= bus_din[19:16];
                                pls_preset[5] <= bus_din[23:20];
                                pls_preset[6] <= bus_din[27:24];
                                pls_preset[7] <= bus_din[31:28];
                            end else begin
                                bus_dout <= {pls_preset[7], pls_preset[6], pls_preset[5], pls_preset[4],
                                             pls_preset[3], pls_preset[2], pls_preset[1], pls_preset[0]};
                            end
                            bus_ack <= 1'b1;
                        end

                        REG_STEP_PLS_PRESCALER: begin
                            if (bus_we) begin
                                pls_prescaler[0] <= bus_din[5:0];
                                pls_prescaler[1] <= bus_din[21:16];
                            end else begin
                                bus_dout <= {10'd0, pls_prescaler[1], 10'd0, pls_prescaler[0]};
                            end
                            bus_ack <= 1'b1;
                        end

                        REG_STEP_STATUS_0: begin bus_dout <= {31'd0, mot_moving[0]}; bus_ack <= 1'b1; end
                        REG_STEP_STATUS_1: begin bus_dout <= {31'd0, mot_moving[1]}; bus_ack <= 1'b1; end
                        REG_STEP_STATUS_2: begin bus_dout <= {31'd0, mot_moving[2]}; bus_ack <= 1'b1; end
                        REG_STEP_STATUS_3: begin bus_dout <= {31'd0, mot_moving[3]}; bus_ack <= 1'b1; end
                        REG_STEP_STATUS_4: begin bus_dout <= {31'd0, mot_moving[4]}; bus_ack <= 1'b1; end
                        REG_STEP_STATUS_5: begin bus_dout <= {31'd0, mot_moving[5]}; bus_ack <= 1'b1; end
                        REG_STEP_STATUS_6: begin bus_dout <= {31'd0, mot_moving[6]}; bus_ack <= 1'b1; end
                        REG_STEP_STATUS_7: begin bus_dout <= {31'd0, mot_moving[7]}; bus_ack <= 1'b1; end

                        REG_STEP_POS_0: begin bus_dout <= mot_position[0]; bus_ack <= 1'b1; end
                        REG_STEP_POS_1: begin bus_dout <= mot_position[1]; bus_ack <= 1'b1; end
                        REG_STEP_POS_2: begin bus_dout <= mot_position[2]; bus_ack <= 1'b1; end
                        REG_STEP_POS_3: begin bus_dout <= mot_position[3]; bus_ack <= 1'b1; end
                        REG_STEP_POS_4: begin bus_dout <= mot_position[4]; bus_ack <= 1'b1; end
                        REG_STEP_POS_5: begin bus_dout <= mot_position[5]; bus_ack <= 1'b1; end
                        REG_STEP_POS_6: begin bus_dout <= mot_position[6]; bus_ack <= 1'b1; end
                        REG_STEP_POS_7: begin bus_dout <= mot_position[7]; bus_ack <= 1'b1; end

                        default: begin
                            bus_dout <= 32'hDEADBEEF;
                            bus_ack  <= 1'b1;
                        end
                    endcase
            end
            // else (bus_stb && !is_new_request && not CTST/SPDM): the
            // master is still holding a request already fully serviced
            // (ack already pulsed on the cycle it was accepted) - nothing
            // further to do until (addr,we) changes or stb drops.

            // ============================================================
            // Step pulse width. Runs every cycle for all 8 motors
            // independently of the round-robin below - a motor's pulse
            // can still be counting down long after scan_idx has moved
            // on to service other motors.
            //
            // If pulse_force_reload[j] is set (from the FIRE branch
            // below, on some earlier cycle - see its comment for when),
            // this cycle starts the deferred new pulse: reload
            // pulse_inner/pulse_outer straight from that motor's config
            // (pls_prescaler/pls_preset - stable registers, nothing else
            // could have changed them in the one cycle since they were
            // last read), set pulse_active, assert step_pulse. This is
            // exactly what a normal fire does, just one cycle later than
            // the tick that actually requested it.
            //
            // Otherwise, while pulse_active[j]: first check whether BOTH
            // pulse_inner[j] and pulse_outer[j] are already zero - if so,
            // this cycle is the pulse's end (pulse_active clears,
            // step_pulse is NOT asserted, going low via the default
            // clear). Otherwise step_pulse[j] stays high, and
            // pulse_inner[j] decrements; when IT reaches zero that's one
            // tick complete, so pulse_outer[j] decrements and
            // pulse_inner[j] reloads from that motor's bank's prescaler
            // for the next tick. Checking for exhaustion before asserting
            // (rather than asserting unconditionally and checking after)
            // is what keeps the width exact rather than one cycle too
            // long. The motion engine's FIRE branch below (which runs
            // later in program order) loads pulse_inner/pulse_outer
            // fresh and re-asserts step_pulse[scan_idx] on the cycle a
            // motor actually fires (or sets pulse_force_reload instead,
            // if the previous pulse hadn't finished), taking priority
            // over this for that one bit.
            // ============================================================
            for (j = 0; j < 8; j = j + 1) begin
                if (pulse_force_reload[j]) begin
                    pulse_force_reload[j] <= 1'b0;
                    pulse_active[j]       <= 1'b1;
                    step_pulse[j]         <= 1'b1;
                    pulse_inner[j]        <= pls_prescaler[j[2]];
                    pulse_outer[j]        <= pls_preset[j];
                end else if (pulse_active[j]) begin
                    if (pulse_inner[j] == 6'd0 && pulse_outer[j] == 4'd0) begin
                        // Both exhausted - this cycle is the pulse's end.
                        // step_pulse[j] is NOT asserted here, going low
                        // via the default clear at the top of this
                        // always block.
                        pulse_active[j] <= 1'b0;
                    end else begin
                        step_pulse[j] <= 1'b1;
                        if (pulse_inner[j] == 6'd0) begin
                            pulse_outer[j] <= pulse_outer[j] - 4'd1;
                            pulse_inner[j] <= pls_prescaler[j[2]];
                        end else begin
                            pulse_inner[j] <= pulse_inner[j] - 6'd1;
                        end
                    end
                end
            end

            // ============================================================
            // Shared motion engine
            // ============================================================
            case (state)

                ST_IDLE: begin
                    if (any_stop) begin
                        pidx = priority_idx(pending_stop);
                        mot_moving[pidx]    <= 1'b0;
                        mot_remaining[pidx] <= 24'd0;
                        mot_segptr[pidx]    <= 8'd0;
                        pending_stop[pidx]  <= 1'b0;
                        if (tick_flag) tick_done[pidx] <= 1'b1;
                    end

                    else if (any_start) begin
                        pidx = priority_idx(pending_start);
                        pending_start[pidx] <= 1'b0;
                        if (!mot_moving[pidx] && (mot_segptr[pidx] < mot_totalseg[pidx])) begin
                            target <= pidx;
                            state  <= ST_RD0;
                        end
                        // else: motor already running, or profile
                        // exhausted (needs a fresh TX_CONFIG) - strobe is
                        // a no-op.
                    end

                    else if (tick_flag && (tick_done != 8'hFF)) begin
                        if (tick_done[scan_idx]) begin
                            // Shouldn't normally happen (every index only
                            // gets visited once per tick), but rotate on
                            // if it does, rather than stalling.
                            scan_idx <= scan_idx + 3'd1;
                        end else if (!mot_moving[scan_idx]) begin
                            tick_done[scan_idx] <= 1'b1;
                            scan_idx <= scan_idx + 3'd1;
                        end else if (mot_countdown[scan_idx] > 16'd1) begin
                            new_countdown = mot_countdown[scan_idx] - 16'd1;
                            mot_countdown[scan_idx] <= new_countdown;
                            tick_done[scan_idx]     <= 1'b1;
                            scan_idx <= scan_idx + 3'd1;
                        end else begin
                            // FIRE: countdown has reached the end of its
                            // interval. Position update happens this
                            // exact cycle regardless; everything else
                            // (remaining count/period/segment bookkeeping)
                            // is snapshotted into plain registers and
                            // finished next cycle in ST_FIRE_CALC.
                            //
                            // The step pulse itself: if the previous
                            // pulse has already finished (the normal,
                            // well-configured case), start the new one
                            // immediately, this exact cycle - exact
                            // timing, same as always. If it's STILL
                            // active (pulse width configured wider than
                            // this step interval), force a clean gap
                            // instead of stepping on it: end the previous
                            // pulse now (letting it run its full
                            // interval rather than cutting it short) and
                            // set pulse_force_reload, which starts the
                            // new pulse exactly one cycle later - see the
                            // pulse output block above. Either way,
                            // pulse_outer/pulse_inner get loaded from
                            // this motor's config exactly once, either
                            // here or (one cycle later) there.
                            if (pulse_active[scan_idx]) begin
                                step_pulse[scan_idx]         <= 1'b0;
                                pulse_active[scan_idx]       <= 1'b0;
                                pulse_force_reload[scan_idx] <= 1'b1;
                            end else begin
                                step_pulse[scan_idx]         <= 1'b1;
                                pulse_active[scan_idx]       <= 1'b1;
                                pulse_force_reload[scan_idx] <= 1'b0;
                                pulse_inner[scan_idx]        <= pls_prescaler[scan_idx[2]];
                                pulse_outer[scan_idx]        <= pls_preset[scan_idx];
                            end
                            tick_done[scan_idx] <= 1'b1;

                            pos_delta    = (mot_dir[scan_idx] == DIR_NORMAL) ? 32'd1 : 32'hFFFFFFFF;
                            new_position = mot_position[scan_idx] + pos_delta;
                            mot_position[scan_idx] <= new_position;
                            `DBG_LOG(("[STEPPERS DEBUG] t=%0t FIRE motor=%0d dir=%b old_pos=%0d pos_delta=%0d new_pos=%0d remaining_before=%0d cmd=%b countdown_was=%0d",
                                      $time, scan_idx, mot_dir[scan_idx], mot_position[scan_idx], pos_delta,
                                      new_position, mot_remaining[scan_idx], mot_cmd[scan_idx], mot_countdown[scan_idx]));

                            target        <= scan_idx;
                            snap_remaining <= mot_remaining[scan_idx];
                            snap_period    <= mot_period[scan_idx];
                            snap_dm        <= mot_dm[scan_idx];
                            snap_incr      <= mot_incr[scan_idx];
                            snap_cmd       <= mot_cmd[scan_idx];
                            snap_segptr    <= mot_segptr[scan_idx];
                            snap_totalseg  <= mot_totalseg[scan_idx];
                            state          <= ST_FIRE_CALC;
                        end
                    end

                    else if (tick_flag && (tick_done == 8'hFF)) begin
                        tick_flag <= 1'b0;
                    end
                    // else: nothing pending, hold.
                end

                ST_FIRE_CALC: begin
                    // All inputs here are plain registers snapshotted
                    // last cycle - no scan_idx-indexed mux in this
                    // cycle's combinational path.
                    new_remaining = snap_remaining - 24'd1;
                    new_period    = calc_next_period(snap_period, snap_dm, snap_incr);

                    if (new_remaining == 24'd0) begin
                        if (snap_cmd == CMD_MOVE &&
                            ((snap_segptr + 8'd1) < snap_totalseg)) begin
                            // Seamless streaming continuation: fresh
                            // reload from the next segment's own SP,
                            // discarding this segment's evolved period.
                            new_segptr = snap_segptr + 8'd1;
                            mot_segptr[target]    <= new_segptr;
                            mot_remaining[target] <= 24'd0;
                            state <= ST_RD0; // target already set
                        end else begin
                            // CMD_MOVE_HALT / CMD_MOVE_HALT_WAIT, or a
                            // CMD_MOVE that ran off the end of the table
                            // (malformed profile - stop safely).
                            new_segptr = snap_segptr + 8'd1;
                            mot_moving[target]    <= 1'b0;
                            mot_segptr[target]    <= new_segptr;
                            mot_remaining[target] <= 24'd0;
                            mot_period[target]    <= new_period;
                            scan_idx <= target + 3'd1;
                            state    <= ST_IDLE;
                        end
                    end else begin
                        mot_remaining[target] <= new_remaining;
                        mot_period[target]    <= new_period;
                        mot_countdown[target] <= new_period;
                        scan_idx <= target + 3'd1;
                        state    <= ST_IDLE;
                    end
                end

                ST_RD0: begin
                    // Present the CTST address this cycle; the physical
                    // RAM needs it stable for a full cycle before RDATA
                    // reflects it (see ST_RD1).
                    segptr_rd = mot_segptr[target];
                    mem_addr <= {target, segptr_rd[SEG_IDX_BITS-1:0], 1'b0};
                    state    <= ST_RD1;
                end

                ST_RD1: begin
                    // CTST address now stable this whole cycle - the RAM
                    // is capturing it now; RDATA will be valid next
                    // cycle.
                    state <= ST_RD2;
                end

                ST_RD2: begin
                    // shared_rdata now reflects the CTST address set in
                    // ST_RD0. Capture it, then present the SPDM address.
                    ctst_word <= shared_rdata;
                    segptr_rd = mot_segptr[target];
                    mem_addr  <= {target, segptr_rd[SEG_IDX_BITS-1:0], 1'b1};
                    state     <= ST_RD3;
                end

                ST_RD3: begin
                    // SPDM address now stable this whole cycle.
                    state <= ST_RD4;
                end

                ST_RD4: begin
                    // shared_rdata now reflects the SPDM address set in
                    // ST_RD2.
                    spdm_word <= shared_rdata;
                    state     <= ST_COMMIT;
                end

                ST_COMMIT: begin
                    mot_period[target]    <= spdm_word[31:16];
                    mot_dm[target]        <= spdm_word[15:0];
                    mot_remaining[target] <= ctst_word[23:0];
                    mot_dir[target]       <= ctst_word[26];
                    mot_cmd[target]       <= ctst_word[25:24];
                    mot_incr[target]      <= ctst_word[27];
                    mot_countdown[target] <= spdm_word[31:16];
                    mot_moving[target]    <= 1'b1;
                    tick_done[target]     <= 1'b1;
                    scan_idx              <= target + 3'd1;
                    state                 <= ST_IDLE;
                    `DBG_LOG(("[STEPPERS DEBUG] t=%0t COMMIT motor=%0d ctst=%h spdm=%h dir=%b cmd=%b n_steps=%0d sp=%0d dm=%0d segptr=%0d",
                              $time, target, ctst_word, spdm_word, ctst_word[26], ctst_word[25:24],
                              ctst_word[23:0], spdm_word[31:16], spdm_word[15:0], mot_segptr[target]));
                end

                default: state <= ST_IDLE;
            endcase
        end
    end

`ifdef SIM_STEPPER_BUS_DEBUG
    // Pure observer - never drives anything, just reports every cycle
    // bus_stb is asserted plus the cycle immediately after it drops.
    reg dbg_stb_d;
    always @(posedge sys_clk) begin
        dbg_stb_d <= bus_stb;
        if (bus_stb || dbg_stb_d)
            $display("[BUS DEBUG] t=%0t stb=%b we=%b addr=%h din=%h ack=%b dout=%h bus_state=%0d",
                      $time, bus_stb, bus_we, bus_addr, bus_din, bus_ack, bus_dout, bus_state);
    end
`endif

endmodule
