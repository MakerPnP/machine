`include "src/main/logging.svh"

// timer_pwm.v
//
// 4 independent STM32-timer-style 8-bit counters (source clock -> 8-bit
// prescaler -> 8-bit auto-reload) feeding 12 flexible PWM output
// channels (PM1-4, OT1-8), each channel independently mapped to any of
// the 4 timers with its own 8-bit compare value and output polarity.
//
// INVARIANT: like every other peripheral behind memory.v, NO REGISTER
// HERE MAY HAVE A READ SIDE EFFECT - see memory.v's header comment.
// TIMx_CTRL's RESET bit is therefore never stored as persistent state:
// a write with that bit set produces a same-cycle reset pulse and the
// bit itself always reads back 0, exactly like REG_LC_CTRL.SINGLE in
// loadcell.v.
//
// ARCHITECTURE
// ---------------------------------------------------------------------
// - TIM_SYNC is a single global gate: a timer only counts while BOTH
//   its own TIMx_CTRL.ENABLE and TIM_SYNC.ENABLE are set. This lets a
//   host configure all 4 timers (prescaler/ARR/initial CNT) and each
//   timer's own ENABLE bit first, completely at rest, then start every
//   enabled timer on the same sys_clk edge by writing TIM_SYNC last -
//   the same "arm everything, then fire together" shape as steppers.v's
//   start-strobe bitmap, but for phase-aligning free-running counters
//   instead of one-shot motion profiles.
// - Each timer counts 0..ARR then wraps to 0 (STM32 upcounting mode).
//   TIMx_CNT is directly host-writable (while a timer is stopped, this
//   is how "different initial counter values" gets set up before the
//   first TIM_SYNC-gated start), and TIMx_CTRL.RESET forces it back to
//   0 immediately, independent of counting state.
// - PWM_CTRLx.TIMER_SRC selects, per channel, which timer's CNT that
//   channel's comparator watches - deliberately many-to-one (e.g. two
//   OT outputs can share one timer's frequency while each keeps its
//   own duty cycle), since PM1-4/OT1-8 may drive identical or entirely
//   different hardware (see the module header comment in core_top.v).
// - Per channel: pin = (CNT < CMP) ? initial_level : other_level, where
//   initial_level is HIGH for POLARITY=1 and LOW for POLARITY=0 - a
//   pure function of the channel's own timer's CNT, its own CMP, and
//   its own POLARITY, registered once per sys_clk cycle. This already
//   produces the right behaviour "when a timer resets" (natural ARR
//   wrap or an explicit TIMx_CTRL.RESET): the instant CNT reads 0 again
//   (assuming CMP != 0), the same comparison re-evaluates to
//   initial_level with no separate reset-handling logic needed.
// - PWM_CTRL (global, offset 0x00) is the "disable ALL outputs" master
//   switch required for OT1-8's electrical safety: OUTPUT_ENABLE=0 (the
//   reset default) forces every one of the 12 pins LOW regardless of
//   per-channel ENABLE/POLARITY, and drives OT_EN to its inactive
//   level, tri-stating the SN74HCT245 that isolates OT1-8 from the
//   FPGA. PM1-4 have no such buffer (their ADUM120N0 isolators are
//   always enabled per the hardware description), so forcing their
//   pins LOW is the only "disable" available to them - which is
//   sufficient, since LOW is the inactive/safe level either way.
// ---------------------------------------------------------------------

module timer_pwm (
    input  wire        reset,
    input  wire        sys_clk,

    // Bus Slave Interface
    input  wire        bus_stb,
    input  wire        bus_we,
    input  wire [7:0]  bus_addr,
    input  wire [31:0] bus_din,
    output reg  [31:0] bus_dout,
    output reg         bus_ack,

    // Physical Hardware Interfaces
    output wire [3:0]  pm_out,   // PM1-4
    output wire [7:0]  ot_out,   // OT1-8
    output wire        ot_en     // SN74HCT245 /OE, active low
);

    `include "src/main/io/timer_regs.svh"
    `include "src/main/io/timer_shared.svh"

    // ------------------------------------------------------------------
    // Global registers
    // ------------------------------------------------------------------
    reg pwm_output_enable;
    reg tim_sync_enable;

    // ------------------------------------------------------------------
    // Per-timer state (index 0-3 == TIM1-4)
    // ------------------------------------------------------------------
    reg        tim_enable    [0:3];
    reg [7:0]  tim_prescaler [0:3];
    reg [7:0]  tim_arr       [0:3];
    reg [7:0]  tim_cnt       [0:3];
    reg [7:0]  tim_presc_cnt [0:3]; // internal divide-down counter

    // ------------------------------------------------------------------
    // Per-PWM-channel state (index 0-3 == PM1-4, index 4-11 == OT1-8)
    // ------------------------------------------------------------------
    reg        pwm_enable    [0:11];
    reg        pwm_polarity  [0:11];
    reg [1:0]  pwm_timer_src [0:11];
    reg [7:0]  pwm_cmp       [0:11];
    reg        pwm_level     [0:11]; // registered comparator output

    integer i;

    // Scratch (blocking-assigned) variables for the per-channel level
    // computation - same pattern as steppers.v's scratch registers.
    reg [7:0] cmp_cnt;
    reg       cmp_active;
    reg       cmp_level;

    assign pm_out = {pwm_level[3], pwm_level[2], pwm_level[1], pwm_level[0]};
    assign ot_out = {pwm_level[11], pwm_level[10], pwm_level[9], pwm_level[8],
                      pwm_level[7], pwm_level[6], pwm_level[5], pwm_level[4]};
    // Active-low: OUTPUT_ENABLE=1 drives OT_EN low (SN74HCT245 outputs
    // active); OUTPUT_ENABLE=0 (reset default) drives it high (isolated).
    assign ot_en = !pwm_output_enable;

    always @(posedge sys_clk) begin
        if (reset) begin
            bus_dout          <= 32'h00000000;
            bus_ack           <= 1'b0;

            pwm_output_enable <= 1'b0; // safe default: outputs disabled, OT_EN isolated
            tim_sync_enable   <= 1'b0;

            for (i = 0; i < 4; i = i + 1) begin
                tim_enable[i]    <= 1'b0;
                tim_prescaler[i] <= 8'd0;
                tim_arr[i]       <= 8'd0;
                tim_cnt[i]       <= 8'd0;
                tim_presc_cnt[i] <= 8'd0;
            end

            for (i = 0; i < 12; i = i + 1) begin
                pwm_enable[i]    <= 1'b0;
                pwm_polarity[i]  <= PWM_POLARITY_NORMAL;
                pwm_timer_src[i] <= TIMER_SRC_TIM1;
                pwm_cmp[i]       <= 8'd0;
                pwm_level[i]     <= 1'b0;
            end
        end else begin
            // ========================================================
            // Free-running counters. Runs every cycle for all 4 timers;
            // a same-cycle bus write to a timer's own registers (below,
            // later in program order) overrides these nonblocking
            // assignments for that timer.
            // ========================================================
            for (i = 0; i < 4; i = i + 1) begin
                if (tim_sync_enable && tim_enable[i]) begin
                    if (tim_presc_cnt[i] == 8'd0) begin
                        tim_presc_cnt[i] <= tim_prescaler[i];
                        if (tim_cnt[i] == tim_arr[i])
                            tim_cnt[i] <= 8'd0;
                        else
                            tim_cnt[i] <= tim_cnt[i] + 8'd1;
                    end else begin
                        tim_presc_cnt[i] <= tim_presc_cnt[i] - 8'd1;
                    end
                end
            end

            // ========================================================
            // Bus slave interface - every register acks in one cycle,
            // no register here has a read side effect.
            // ========================================================
            if (bus_stb) begin
                if (!bus_ack) begin
                    bus_ack <= 1'b1;
                    if (bus_we) begin
                        `DBG_LOG(("timer_pwm bus write. addr: %02x, value: %08h", bus_addr, bus_din));
                        case (bus_addr)
                            REG_PWM_CTRL: begin
                                pwm_output_enable <= bus_din[PWM_GLOBAL_OUTPUT_ENABLE_BIT];
                            end

                            REG_TIM_SYNC: begin
                                tim_sync_enable <= bus_din[TIM_SYNC_ENABLE_BIT];
                            end

                            REG_TIM1_CTRL: begin
                                tim_enable[0]    <= bus_din[TIM_CTRL_ENABLE_BIT];
                                tim_prescaler[0] <= bus_din[TIM_CTRL_PRESCALER_LSB +: TIM_CTRL_PRESCALER_W];
                                if (bus_din[TIM_CTRL_RESET_BIT]) begin
                                    tim_cnt[0]       <= 8'd0;
                                    tim_presc_cnt[0] <= bus_din[TIM_CTRL_PRESCALER_LSB +: TIM_CTRL_PRESCALER_W];
                                end
                            end
                            REG_TIM1_ARR: tim_arr[0] <= bus_din[7:0];
                            REG_TIM1_CNT: tim_cnt[0] <= bus_din[7:0];

                            REG_TIM2_CTRL: begin
                                tim_enable[1]    <= bus_din[TIM_CTRL_ENABLE_BIT];
                                tim_prescaler[1] <= bus_din[TIM_CTRL_PRESCALER_LSB +: TIM_CTRL_PRESCALER_W];
                                if (bus_din[TIM_CTRL_RESET_BIT]) begin
                                    tim_cnt[1]       <= 8'd0;
                                    tim_presc_cnt[1] <= bus_din[TIM_CTRL_PRESCALER_LSB +: TIM_CTRL_PRESCALER_W];
                                end
                            end
                            REG_TIM2_ARR: tim_arr[1] <= bus_din[7:0];
                            REG_TIM2_CNT: tim_cnt[1] <= bus_din[7:0];

                            REG_TIM3_CTRL: begin
                                tim_enable[2]    <= bus_din[TIM_CTRL_ENABLE_BIT];
                                tim_prescaler[2] <= bus_din[TIM_CTRL_PRESCALER_LSB +: TIM_CTRL_PRESCALER_W];
                                if (bus_din[TIM_CTRL_RESET_BIT]) begin
                                    tim_cnt[2]       <= 8'd0;
                                    tim_presc_cnt[2] <= bus_din[TIM_CTRL_PRESCALER_LSB +: TIM_CTRL_PRESCALER_W];
                                end
                            end
                            REG_TIM3_ARR: tim_arr[2] <= bus_din[7:0];
                            REG_TIM3_CNT: tim_cnt[2] <= bus_din[7:0];

                            REG_TIM4_CTRL: begin
                                tim_enable[3]    <= bus_din[TIM_CTRL_ENABLE_BIT];
                                tim_prescaler[3] <= bus_din[TIM_CTRL_PRESCALER_LSB +: TIM_CTRL_PRESCALER_W];
                                if (bus_din[TIM_CTRL_RESET_BIT]) begin
                                    tim_cnt[3]       <= 8'd0;
                                    tim_presc_cnt[3] <= bus_din[TIM_CTRL_PRESCALER_LSB +: TIM_CTRL_PRESCALER_W];
                                end
                            end
                            REG_TIM4_ARR: tim_arr[3] <= bus_din[7:0];
                            REG_TIM4_CNT: tim_cnt[3] <= bus_din[7:0];

                            REG_PWM_CTRL1: begin
                                pwm_enable[0]    <= bus_din[PWM_CTRL_ENABLE_BIT];
                                pwm_polarity[0]  <= bus_din[PWM_CTRL_POLARITY_BIT];
                                pwm_timer_src[0] <= bus_din[PWM_CTRL_TIMER_SRC_LSB +: PWM_CTRL_TIMER_SRC_W];
                            end
                            REG_PWM_CMP1: pwm_cmp[0] <= bus_din[7:0];

                            REG_PWM_CTRL2: begin
                                pwm_enable[1]    <= bus_din[PWM_CTRL_ENABLE_BIT];
                                pwm_polarity[1]  <= bus_din[PWM_CTRL_POLARITY_BIT];
                                pwm_timer_src[1] <= bus_din[PWM_CTRL_TIMER_SRC_LSB +: PWM_CTRL_TIMER_SRC_W];
                            end
                            REG_PWM_CMP2: pwm_cmp[1] <= bus_din[7:0];

                            REG_PWM_CTRL3: begin
                                pwm_enable[2]    <= bus_din[PWM_CTRL_ENABLE_BIT];
                                pwm_polarity[2]  <= bus_din[PWM_CTRL_POLARITY_BIT];
                                pwm_timer_src[2] <= bus_din[PWM_CTRL_TIMER_SRC_LSB +: PWM_CTRL_TIMER_SRC_W];
                            end
                            REG_PWM_CMP3: pwm_cmp[2] <= bus_din[7:0];

                            REG_PWM_CTRL4: begin
                                pwm_enable[3]    <= bus_din[PWM_CTRL_ENABLE_BIT];
                                pwm_polarity[3]  <= bus_din[PWM_CTRL_POLARITY_BIT];
                                pwm_timer_src[3] <= bus_din[PWM_CTRL_TIMER_SRC_LSB +: PWM_CTRL_TIMER_SRC_W];
                            end
                            REG_PWM_CMP4: pwm_cmp[3] <= bus_din[7:0];

                            REG_PWM_CTRL5: begin
                                pwm_enable[4]    <= bus_din[PWM_CTRL_ENABLE_BIT];
                                pwm_polarity[4]  <= bus_din[PWM_CTRL_POLARITY_BIT];
                                pwm_timer_src[4] <= bus_din[PWM_CTRL_TIMER_SRC_LSB +: PWM_CTRL_TIMER_SRC_W];
                            end
                            REG_PWM_CMP5: pwm_cmp[4] <= bus_din[7:0];

                            REG_PWM_CTRL6: begin
                                pwm_enable[5]    <= bus_din[PWM_CTRL_ENABLE_BIT];
                                pwm_polarity[5]  <= bus_din[PWM_CTRL_POLARITY_BIT];
                                pwm_timer_src[5] <= bus_din[PWM_CTRL_TIMER_SRC_LSB +: PWM_CTRL_TIMER_SRC_W];
                            end
                            REG_PWM_CMP6: pwm_cmp[5] <= bus_din[7:0];

                            REG_PWM_CTRL7: begin
                                pwm_enable[6]    <= bus_din[PWM_CTRL_ENABLE_BIT];
                                pwm_polarity[6]  <= bus_din[PWM_CTRL_POLARITY_BIT];
                                pwm_timer_src[6] <= bus_din[PWM_CTRL_TIMER_SRC_LSB +: PWM_CTRL_TIMER_SRC_W];
                            end
                            REG_PWM_CMP7: pwm_cmp[6] <= bus_din[7:0];

                            REG_PWM_CTRL8: begin
                                pwm_enable[7]    <= bus_din[PWM_CTRL_ENABLE_BIT];
                                pwm_polarity[7]  <= bus_din[PWM_CTRL_POLARITY_BIT];
                                pwm_timer_src[7] <= bus_din[PWM_CTRL_TIMER_SRC_LSB +: PWM_CTRL_TIMER_SRC_W];
                            end
                            REG_PWM_CMP8: pwm_cmp[7] <= bus_din[7:0];

                            REG_PWM_CTRL9: begin
                                pwm_enable[8]    <= bus_din[PWM_CTRL_ENABLE_BIT];
                                pwm_polarity[8]  <= bus_din[PWM_CTRL_POLARITY_BIT];
                                pwm_timer_src[8] <= bus_din[PWM_CTRL_TIMER_SRC_LSB +: PWM_CTRL_TIMER_SRC_W];
                            end
                            REG_PWM_CMP9: pwm_cmp[8] <= bus_din[7:0];

                            REG_PWM_CTRL10: begin
                                pwm_enable[9]    <= bus_din[PWM_CTRL_ENABLE_BIT];
                                pwm_polarity[9]  <= bus_din[PWM_CTRL_POLARITY_BIT];
                                pwm_timer_src[9] <= bus_din[PWM_CTRL_TIMER_SRC_LSB +: PWM_CTRL_TIMER_SRC_W];
                            end
                            REG_PWM_CMP10: pwm_cmp[9] <= bus_din[7:0];

                            REG_PWM_CTRL11: begin
                                pwm_enable[10]    <= bus_din[PWM_CTRL_ENABLE_BIT];
                                pwm_polarity[10]  <= bus_din[PWM_CTRL_POLARITY_BIT];
                                pwm_timer_src[10] <= bus_din[PWM_CTRL_TIMER_SRC_LSB +: PWM_CTRL_TIMER_SRC_W];
                            end
                            REG_PWM_CMP11: pwm_cmp[10] <= bus_din[7:0];

                            REG_PWM_CTRL12: begin
                                pwm_enable[11]    <= bus_din[PWM_CTRL_ENABLE_BIT];
                                pwm_polarity[11]  <= bus_din[PWM_CTRL_POLARITY_BIT];
                                pwm_timer_src[11] <= bus_din[PWM_CTRL_TIMER_SRC_LSB +: PWM_CTRL_TIMER_SRC_W];
                            end
                            REG_PWM_CMP12: pwm_cmp[11] <= bus_din[7:0];

                            default: begin end
                        endcase
                    end else begin
                        case (bus_addr)
                            REG_PWM_CTRL: bus_dout <= {31'd0, pwm_output_enable};
                            REG_TIM_SYNC: bus_dout <= {31'd0, tim_sync_enable};

                            REG_TIM1_CTRL: bus_dout <= tim_ctrl_word(tim_enable[0], 1'b0, tim_prescaler[0]);
                            REG_TIM1_ARR:  bus_dout <= {24'd0, tim_arr[0]};
                            REG_TIM1_CNT:  bus_dout <= {24'd0, tim_cnt[0]};

                            REG_TIM2_CTRL: bus_dout <= tim_ctrl_word(tim_enable[1], 1'b0, tim_prescaler[1]);
                            REG_TIM2_ARR:  bus_dout <= {24'd0, tim_arr[1]};
                            REG_TIM2_CNT:  bus_dout <= {24'd0, tim_cnt[1]};

                            REG_TIM3_CTRL: bus_dout <= tim_ctrl_word(tim_enable[2], 1'b0, tim_prescaler[2]);
                            REG_TIM3_ARR:  bus_dout <= {24'd0, tim_arr[2]};
                            REG_TIM3_CNT:  bus_dout <= {24'd0, tim_cnt[2]};

                            REG_TIM4_CTRL: bus_dout <= tim_ctrl_word(tim_enable[3], 1'b0, tim_prescaler[3]);
                            REG_TIM4_ARR:  bus_dout <= {24'd0, tim_arr[3]};
                            REG_TIM4_CNT:  bus_dout <= {24'd0, tim_cnt[3]};

                            REG_PWM_CTRL1:  bus_dout <= pwm_ctrl_word(pwm_enable[0],  pwm_polarity[0],  pwm_timer_src[0]);
                            REG_PWM_CMP1:   bus_dout <= {24'd0, pwm_cmp[0]};
                            REG_PWM_CTRL2:  bus_dout <= pwm_ctrl_word(pwm_enable[1],  pwm_polarity[1],  pwm_timer_src[1]);
                            REG_PWM_CMP2:   bus_dout <= {24'd0, pwm_cmp[1]};
                            REG_PWM_CTRL3:  bus_dout <= pwm_ctrl_word(pwm_enable[2],  pwm_polarity[2],  pwm_timer_src[2]);
                            REG_PWM_CMP3:   bus_dout <= {24'd0, pwm_cmp[2]};
                            REG_PWM_CTRL4:  bus_dout <= pwm_ctrl_word(pwm_enable[3],  pwm_polarity[3],  pwm_timer_src[3]);
                            REG_PWM_CMP4:   bus_dout <= {24'd0, pwm_cmp[3]};
                            REG_PWM_CTRL5:  bus_dout <= pwm_ctrl_word(pwm_enable[4],  pwm_polarity[4],  pwm_timer_src[4]);
                            REG_PWM_CMP5:   bus_dout <= {24'd0, pwm_cmp[4]};
                            REG_PWM_CTRL6:  bus_dout <= pwm_ctrl_word(pwm_enable[5],  pwm_polarity[5],  pwm_timer_src[5]);
                            REG_PWM_CMP6:   bus_dout <= {24'd0, pwm_cmp[5]};
                            REG_PWM_CTRL7:  bus_dout <= pwm_ctrl_word(pwm_enable[6],  pwm_polarity[6],  pwm_timer_src[6]);
                            REG_PWM_CMP7:   bus_dout <= {24'd0, pwm_cmp[6]};
                            REG_PWM_CTRL8:  bus_dout <= pwm_ctrl_word(pwm_enable[7],  pwm_polarity[7],  pwm_timer_src[7]);
                            REG_PWM_CMP8:   bus_dout <= {24'd0, pwm_cmp[7]};
                            REG_PWM_CTRL9:  bus_dout <= pwm_ctrl_word(pwm_enable[8],  pwm_polarity[8],  pwm_timer_src[8]);
                            REG_PWM_CMP9:   bus_dout <= {24'd0, pwm_cmp[8]};
                            REG_PWM_CTRL10: bus_dout <= pwm_ctrl_word(pwm_enable[9],  pwm_polarity[9],  pwm_timer_src[9]);
                            REG_PWM_CMP10:  bus_dout <= {24'd0, pwm_cmp[9]};
                            REG_PWM_CTRL11: bus_dout <= pwm_ctrl_word(pwm_enable[10], pwm_polarity[10], pwm_timer_src[10]);
                            REG_PWM_CMP11:  bus_dout <= {24'd0, pwm_cmp[10]};
                            REG_PWM_CTRL12: bus_dout <= pwm_ctrl_word(pwm_enable[11], pwm_polarity[11], pwm_timer_src[11]);
                            REG_PWM_CMP12:  bus_dout <= {24'd0, pwm_cmp[11]};

                            default: bus_dout <= 32'hDEAD7000;
                        endcase
                    end
                end
            end else begin
                bus_ack <= 1'b0;
            end

            // ========================================================
            // Per-channel comparator output, registered once per cycle.
            // A disabled channel (its own PWM_CTRLx.ENABLE, or the
            // global PWM_CTRL.OUTPUT_ENABLE) is forced LOW regardless
            // of polarity, per spec.
            // ========================================================
            for (i = 0; i < 12; i = i + 1) begin
                cmp_cnt    = tim_cnt[pwm_timer_src[i]];
                cmp_active = pwm_output_enable && pwm_enable[i];
                cmp_level  = (cmp_cnt < pwm_cmp[i]) ? pwm_polarity[i] : !pwm_polarity[i];
                pwm_level[i] <= cmp_active ? cmp_level : 1'b0;
            end
        end
    end

endmodule
