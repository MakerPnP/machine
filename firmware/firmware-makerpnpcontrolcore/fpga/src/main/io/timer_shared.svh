// ====================================================================
// timer_shared.svh
//
// Constants and helpers shared between the production peripheral
// (src/main/io/timer_pwm.v) and the simulation environment
// (src/test/io/timer_tb.v, src/test/io/timer_pwm_tb.v).
//
// Everything here is a `localparam`/`function`, so this file must be
// `include`d *inside* a module body (same convention as
// steppers_shared.svh / loadcell_shared.svh), and it must be included
// AFTER timer_regs.svh - the register-lookup functions below reference
// the REG_* localparams defined there.
// ====================================================================

localparam NUM_TIMERS       = 4;
localparam NUM_PWM_CHANNELS = 12;

// --------------------------------------------------------------------
// Channel-to-pin mapping (fixed in hardware, see pins.pcf):
//   channel 0-3  (PWM_CTRL1-4)  -> PM_OUT[0-3]   (PM1-4)
//   channel 4-11 (PWM_CTRL5-12) -> OT_OUT[0-7]   (OT1-8)
// --------------------------------------------------------------------
localparam PWM_CH_PM_FIRST = 0;
localparam PWM_CH_PM_LAST  = 3;
localparam PWM_CH_OT_FIRST = 4;
localparam PWM_CH_OT_LAST  = 11;

// --------------------------------------------------------------------
// TIMx_CTRL bit layout
//   [0]     ENABLE     1 = this timer counts (subject to TIM_SYNC below)
//   [1]     RESET      write 1 to reset this timer's counter to 0;
//                       self-clearing, always reads back 0
//   [7:2]   reserved
//   [15:8]  PRESCALER  divide-1 (0 = tick every sys_clk cycle, matching
//                       the steppers.v pls_prescaler convention)
// --------------------------------------------------------------------
localparam TIM_CTRL_ENABLE_BIT   = 0;
localparam TIM_CTRL_RESET_BIT    = 1;
localparam TIM_CTRL_PRESCALER_LSB = 8;
localparam TIM_CTRL_PRESCALER_W   = 8;

// --------------------------------------------------------------------
// TIM_SYNC bit layout
//   [0]    ENABLE      1 = every timer with its own CTRL.ENABLE set
//                       counts; 0 = no timer counts, regardless of its
//                       own CTRL.ENABLE
//   [1]    reserved
//   [7:2]  PRESCALER   divide-1, shared by all 4 timers, applied ahead
//                       of each timer's own TIMx_CTRL.PRESCALER: a
//                       timer only advances on sys_clk cycles this
//                       shared stage "ticks" on, so the effective divide
//                       to one timer TICK is
//                       (TIM_SYNC.PRESCALER+1) * (TIMx_CTRL.PRESCALER+1)
//                       sys_clk cycles - see timer_pwm.v's module header
//                       for why this sits ahead of, not instead of, each
//                       timer's own prescaler.
// --------------------------------------------------------------------
localparam TIM_SYNC_ENABLE_BIT     = 0;
localparam TIM_SYNC_PRESCALER_LSB  = 2;
localparam TIM_SYNC_PRESCALER_W    = 6;

// --------------------------------------------------------------------
// PWM_CTRL (global) bit layout
//   [0] OUTPUT_ENABLE  0 (reset default) = every PWM output pin forced
//                       LOW and OT_EN driven to its isolate/inactive
//                       state; 1 = OT_EN active and each channel's own
//                       PWM_CTRLx.ENABLE/polarity/timer_src takes effect
// --------------------------------------------------------------------
localparam PWM_GLOBAL_OUTPUT_ENABLE_BIT = 0;

// --------------------------------------------------------------------
// PWM_CTRLx bit layout
//   [0]   ENABLE     1 = this channel drives its pin from its timer's
//                     comparator; 0 = pin forced LOW regardless of
//                     polarity
//   [1]   POLARITY   1 = pin HIGH while CNT < CMP, LOW once CNT >= CMP
//                     0 = pin LOW while CNT < CMP, HIGH once CNT >= CMP
//                     Either way the pin returns to its "before compare"
//                     level the instant the timer's counter resets to 0
//                     (natural ARR wrap or an explicit TIMx_CTRL.RESET).
//   [3:2] reserved
//   [5:4] TIMER_SRC  which of the 4 timers this channel compares
//                     against - TIMER_SRC_TIM1..TIMER_SRC_TIM4 below
//   [7:6] reserved (room to grow TIMER_SRC past 4 timers later)
// --------------------------------------------------------------------
localparam PWM_CTRL_ENABLE_BIT     = 0;
localparam PWM_CTRL_POLARITY_BIT   = 1;
localparam PWM_CTRL_TIMER_SRC_LSB  = 4;
localparam PWM_CTRL_TIMER_SRC_W    = 2;

localparam PWM_POLARITY_NORMAL   = 1'b0;
localparam PWM_POLARITY_INVERTED = 1'b1;

localparam [1:0] TIMER_SRC_TIM1 = 2'd0;
localparam [1:0] TIMER_SRC_TIM2 = 2'd1;
localparam [1:0] TIMER_SRC_TIM3 = 2'd2;
localparam [1:0] TIMER_SRC_TIM4 = 2'd3;

// --------------------------------------------------------------------
// Register lookup helpers - one function per register kind, indexed
// 0-3 for timers / 0-11 for PWM channels, same shape as steppers_
// motion_tb.v's status_reg_for_motor/pos_reg_for_motor. Centralized
// here (rather than duplicated per-testbench) since both timer_tb.v
// and timer_pwm_tb.v need them.
// --------------------------------------------------------------------
function automatic [7:0] tim_ctrl_reg;
    input [1:0] idx;
    begin
        case (idx)
            2'd0: tim_ctrl_reg = REG_TIM1_CTRL;
            2'd1: tim_ctrl_reg = REG_TIM2_CTRL;
            2'd2: tim_ctrl_reg = REG_TIM3_CTRL;
            default: tim_ctrl_reg = REG_TIM4_CTRL;
        endcase
    end
endfunction

function automatic [7:0] tim_arr_reg;
    input [1:0] idx;
    begin
        case (idx)
            2'd0: tim_arr_reg = REG_TIM1_ARR;
            2'd1: tim_arr_reg = REG_TIM2_ARR;
            2'd2: tim_arr_reg = REG_TIM3_ARR;
            default: tim_arr_reg = REG_TIM4_ARR;
        endcase
    end
endfunction

function automatic [7:0] tim_cnt_reg;
    input [1:0] idx;
    begin
        case (idx)
            2'd0: tim_cnt_reg = REG_TIM1_CNT;
            2'd1: tim_cnt_reg = REG_TIM2_CNT;
            2'd2: tim_cnt_reg = REG_TIM3_CNT;
            default: tim_cnt_reg = REG_TIM4_CNT;
        endcase
    end
endfunction

function automatic [7:0] pwm_ctrl_reg;
    input [3:0] idx;
    begin
        case (idx)
            4'd0:  pwm_ctrl_reg = REG_PWM_CTRL1;
            4'd1:  pwm_ctrl_reg = REG_PWM_CTRL2;
            4'd2:  pwm_ctrl_reg = REG_PWM_CTRL3;
            4'd3:  pwm_ctrl_reg = REG_PWM_CTRL4;
            4'd4:  pwm_ctrl_reg = REG_PWM_CTRL5;
            4'd5:  pwm_ctrl_reg = REG_PWM_CTRL6;
            4'd6:  pwm_ctrl_reg = REG_PWM_CTRL7;
            4'd7:  pwm_ctrl_reg = REG_PWM_CTRL8;
            4'd8:  pwm_ctrl_reg = REG_PWM_CTRL9;
            4'd9:  pwm_ctrl_reg = REG_PWM_CTRL10;
            4'd10: pwm_ctrl_reg = REG_PWM_CTRL11;
            default: pwm_ctrl_reg = REG_PWM_CTRL12;
        endcase
    end
endfunction

function automatic [7:0] pwm_cmp_reg;
    input [3:0] idx;
    begin
        case (idx)
            4'd0:  pwm_cmp_reg = REG_PWM_CMP1;
            4'd1:  pwm_cmp_reg = REG_PWM_CMP2;
            4'd2:  pwm_cmp_reg = REG_PWM_CMP3;
            4'd3:  pwm_cmp_reg = REG_PWM_CMP4;
            4'd4:  pwm_cmp_reg = REG_PWM_CMP5;
            4'd5:  pwm_cmp_reg = REG_PWM_CMP6;
            4'd6:  pwm_cmp_reg = REG_PWM_CMP7;
            4'd7:  pwm_cmp_reg = REG_PWM_CMP8;
            4'd8:  pwm_cmp_reg = REG_PWM_CMP9;
            4'd9:  pwm_cmp_reg = REG_PWM_CMP10;
            4'd10: pwm_cmp_reg = REG_PWM_CMP11;
            default: pwm_cmp_reg = REG_PWM_CMP12;
        endcase
    end
endfunction

// --------------------------------------------------------------------
// Word-assembly helpers, mirroring loadcell_shared.svh's lc_ctrl_word -
// so the testbenches and the RTL's bit layout can never drift apart.
// --------------------------------------------------------------------
function automatic [31:0] tim_ctrl_word;
    input        enable;
    input        rst;
    input [7:0]  prescaler;
    begin
        tim_ctrl_word = {16'd0, prescaler, 6'd0, rst, enable};
    end
endfunction

function automatic [31:0] tim_sync_word;
    input        enable;
    input [5:0]  prescaler;
    begin
        tim_sync_word = {24'd0, prescaler, 1'd0, enable};
    end
endfunction

function automatic [31:0] pwm_ctrl_word;
    input        enable;
    input        polarity;
    input [1:0]  timer_src;
    begin
        pwm_ctrl_word = {24'd0, 2'd0, timer_src, 2'd0, polarity, enable};
    end
endfunction
