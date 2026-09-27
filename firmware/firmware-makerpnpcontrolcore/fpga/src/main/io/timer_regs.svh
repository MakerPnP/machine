// ====================================================================
// timer_regs.svh - register offsets for the timer_pwm peripheral
// (src/main/io/timer_pwm.v)
//
// One global output-enable register, one global timer-sync register,
// 4 independent 8-bit timers (CTRL/ARR/CNT each), and 12 PWM channels
// (CTRL/CMP each) - see timer_shared.svh for bit-layout constants and
// the PM/OT channel-to-pin mapping.
// ====================================================================

// --------------------------------------------------------------------
// Global registers
// --------------------------------------------------------------------
localparam REG_PWM_CTRL   = 8'h00; // master output enable / OT_EN control
localparam REG_TIM_SYNC   = 8'h04; // timer counting sync/enable gate

// --------------------------------------------------------------------
// Timer 1-4 registers
// --------------------------------------------------------------------
localparam REG_TIM1_CTRL  = 8'h08;
localparam REG_TIM1_ARR   = 8'h0c;
localparam REG_TIM1_CNT   = 8'h10;

localparam REG_TIM2_CTRL  = 8'h14;
localparam REG_TIM2_ARR   = 8'h18;
localparam REG_TIM2_CNT   = 8'h1c;

localparam REG_TIM3_CTRL  = 8'h20;
localparam REG_TIM3_ARR   = 8'h24;
localparam REG_TIM3_CNT   = 8'h28;

localparam REG_TIM4_CTRL  = 8'h2c;
localparam REG_TIM4_ARR   = 8'h30;
localparam REG_TIM4_CNT   = 8'h34;

// --------------------------------------------------------------------
// PWM channel 1-12 registers.
// Channel 1-4 = PM1-4, channel 5-12 = OT1-8 (see timer_shared.svh).
// --------------------------------------------------------------------
localparam REG_PWM_CTRL1  = 8'h38;
localparam REG_PWM_CMP1   = 8'h3c;

localparam REG_PWM_CTRL2  = 8'h40;
localparam REG_PWM_CMP2   = 8'h44;

localparam REG_PWM_CTRL3  = 8'h48;
localparam REG_PWM_CMP3   = 8'h4c;

localparam REG_PWM_CTRL4  = 8'h50;
localparam REG_PWM_CMP4   = 8'h54;

localparam REG_PWM_CTRL5  = 8'h58;
localparam REG_PWM_CMP5   = 8'h5c;

localparam REG_PWM_CTRL6  = 8'h60;
localparam REG_PWM_CMP6   = 8'h64;

localparam REG_PWM_CTRL7  = 8'h68;
localparam REG_PWM_CMP7   = 8'h6c;

localparam REG_PWM_CTRL8  = 8'h70;
localparam REG_PWM_CMP8   = 8'h74;

localparam REG_PWM_CTRL9  = 8'h78;
localparam REG_PWM_CMP9   = 8'h7c;

localparam REG_PWM_CTRL10 = 8'h80;
localparam REG_PWM_CMP10  = 8'h84;

localparam REG_PWM_CTRL11 = 8'h88;
localparam REG_PWM_CMP11  = 8'h8c;

localparam REG_PWM_CTRL12 = 8'h90;
localparam REG_PWM_CMP12  = 8'h94;
