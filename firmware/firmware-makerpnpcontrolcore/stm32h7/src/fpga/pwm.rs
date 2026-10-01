//! PWM outputs (PM1-4, OT1-8) using the FPGA `timer_pwm` peripheral.
//!
//! Usage:
//! 1. [`try_resolve`] the requested outputs into a [`PwmPlan`] - pure computation, also usable
//!    off-target via the `fpga-config` crate to validate a configuration before it is created.
//! 2. [`allocate`] the plan, which tears down and reprograms the whole peripheral and returns a
//!    [`PwmBank`] with one [`PwmChannel`] per request, in request order.  Every channel starts
//!    stopped, all timers are stopped, and all outputs are globally disabled.
//! 3. [`PwmBank::enable_outputs`], then set the duty and [`PwmChannel::start`] each channel.
//! 4. [`PwmBank::start_synced`] to start all timers on the same sysclk edge.
//!
//! Requires memory-mapped mode.
//!
//! FPGA limitation: while outputs are globally disabled (reset default, or
//! [`PwmBank::disable_outputs`]) the FPGA forces every pin LOW, which is the *active* level for an
//! inverted output.  OT1-8 are also isolated by their output buffer when globally disabled, PM1-4
//! are not.

use alloc::vec::Vec;
use core::ops::{Index, IndexMut};

use fpga_pac::common::{Reg, RW};
use fpga_pac::timer_pwm::{regs, vals};

pub use fpga_config::pwm::*;

/// Byte offset between consecutive PWM_CTRLn (and PWM_CMPn) registers.
const CHANNEL_STRIDE: usize = 0x08;
/// Byte offset between consecutive TIMx_CTRL (and TIMx_ARR) registers.
const TIMER_STRIDE: usize = 0x0c;

// The PAC has a distinct type per channel/timer register, but they all share the same layout, so
// the first one's type is used for all of them.

fn pwm_ctrl(channel: usize) -> Reg<regs::pwm_ctrl1, RW> {
    defmt::assert!(channel < CHANNEL_COUNT);
    let ptr = fpga_pac::TIMER_PWM.pwm_ctrl1().as_ptr() as *mut u8;
    unsafe { Reg::from_ptr(ptr.wrapping_add(channel * CHANNEL_STRIDE) as _) }
}

fn pwm_cmp(channel: usize) -> Reg<regs::pwm_cmp1, RW> {
    defmt::assert!(channel < CHANNEL_COUNT);
    let ptr = fpga_pac::TIMER_PWM.pwm_cmp1().as_ptr() as *mut u8;
    unsafe { Reg::from_ptr(ptr.wrapping_add(channel * CHANNEL_STRIDE) as _) }
}

fn tim_ctrl(timer: usize) -> Reg<regs::tim1_ctrl, RW> {
    defmt::assert!(timer < TIMER_COUNT);
    let ptr = fpga_pac::TIMER_PWM.tim1_ctrl().as_ptr() as *mut u8;
    unsafe { Reg::from_ptr(ptr.wrapping_add(timer * TIMER_STRIDE) as _) }
}

fn tim_arr(timer: usize) -> Reg<regs::tim1_arr, RW> {
    defmt::assert!(timer < TIMER_COUNT);
    let ptr = fpga_pac::TIMER_PWM.tim1_arr().as_ptr() as *mut u8;
    unsafe { Reg::from_ptr(ptr.wrapping_add(timer * TIMER_STRIDE) as _) }
}

/// Tears down the whole `timer_pwm` peripheral and programs it according to the plan.
///
/// Any previously allocated [`PwmBank`] must not be used afterwards.
pub fn allocate(plan: &PwmPlan) -> PwmBank {
    let timer_pwm = fpga_pac::TIMER_PWM;

    // tear down: outputs off, timers stopped and reset, channels disabled.
    timer_pwm
        .pwm_ctrl()
        .write(|w| w.set_output_enable(false));
    timer_pwm.tim_sync().write(|w| {
        w.set_enable(false);
        w.set_prescaler(0);
    });
    for channel in 0..CHANNEL_COUNT {
        pwm_ctrl(channel).write(|w| w.set_enable(false));
        pwm_cmp(channel).write(|w| w.set_value(0));
    }
    for timer in 0..TIMER_COUNT {
        tim_ctrl(timer).write(|w| w.set_reset(true));
        tim_arr(timer).write(|w| w.set_value(0));
    }

    // timers: reset with prescaler, ARR, then enable; they only count once TIM_SYNC is enabled.
    for (timer, timer_plan) in plan.timers.iter().enumerate() {
        let Some(timer_plan) = timer_plan else {
            continue;
        };
        defmt::debug!(
            "PWM TIM{}: frequency: {}Hz, prescaler: {}, steps: {}",
            timer + 1,
            timer_plan.frequency_hz,
            timer_plan.prescaler,
            timer_plan.steps
        );

        tim_ctrl(timer).write(|w| {
            w.set_reset(true);
            w.set_prescaler(timer_plan.prescaler_reg());
        });
        tim_arr(timer).write(|w| w.set_value(timer_plan.arr_reg()));
        tim_ctrl(timer).write(|w| {
            w.set_enable(true);
            w.set_prescaler(timer_plan.prescaler_reg());
        });
    }

    let channels = plan
        .channels
        .iter()
        .map(|channel_plan| {
            let channel = PwmChannel::new(*channel_plan);
            channel.write_idle();
            channel
        })
        .collect();

    // the timers stay stopped until `PwmBank::start_synced`.
    defmt::debug!("PWM global prescaler: {}", plan.global_prescaler);
    timer_pwm.tim_sync().write(|w| {
        w.set_enable(false);
        w.set_prescaler(plan.global_prescaler_reg());
    });

    PwmBank {
        channels,
        global_prescaler_reg: plan.global_prescaler_reg(),
    }
}

pub struct PwmBank {
    channels: Vec<PwmChannel>,
    global_prescaler_reg: u8,
}

impl PwmBank {
    /// Starts all allocated timers counting on the same sysclk edge (TIM_SYNC.ENABLE).
    ///
    /// Channels can be configured and started before or after this; a started channel's output
    /// follows its timer's counter, which stays at 0 (the start of the period) until then.
    pub fn start_synced(&mut self) {
        self.write_sync(true);
    }

    /// Freezes all timers' counters (TIM_SYNC.ENABLE).  Started channels hold whatever level they
    /// were at when the counters stopped; [`PwmChannel::stop`] a channel to hold its idle level.
    /// [`Self::start_synced`] resumes counting from where the timers stopped.
    pub fn stop_synced(&mut self) {
        self.write_sync(false);
    }

    fn write_sync(&self, enable: bool) {
        fpga_pac::TIMER_PWM.tim_sync().write(|w| {
            w.set_enable(enable);
            w.set_prescaler(self.global_prescaler_reg);
        });
    }

    /// Master output enable for all 12 outputs, also enables the OT1-8 output buffer.
    pub fn enable_outputs(&mut self) {
        fpga_pac::TIMER_PWM
            .pwm_ctrl()
            .write(|w| {
                w.set_output_enable(true);
            });
    }

    /// Forces all 12 outputs LOW (see the module documentation) and isolates OT1-8.
    pub fn disable_outputs(&mut self) {
        fpga_pac::TIMER_PWM
            .pwm_ctrl()
            .write(|w| w.set_output_enable(false));
    }

    pub fn len(&self) -> usize {
        self.channels.len()
    }

    pub fn is_empty(&self) -> bool {
        self.channels.is_empty()
    }

    pub fn iter(&self) -> core::slice::Iter<'_, PwmChannel> {
        self.channels.iter()
    }

    pub fn iter_mut(&mut self) -> core::slice::IterMut<'_, PwmChannel> {
        self.channels.iter_mut()
    }

    pub fn channel(&mut self, output: PwmOutput) -> Option<&mut PwmChannel> {
        self.channels
            .iter_mut()
            .find(|channel| channel.output() == output)
    }
}

impl Index<usize> for PwmBank {
    type Output = PwmChannel;

    fn index(&self, index: usize) -> &Self::Output {
        &self.channels[index]
    }
}

impl IndexMut<usize> for PwmBank {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.channels[index]
    }
}

pub struct PwmChannel {
    plan: ChannelPlan,
    active_steps: u16,
    running: bool,
}

impl PwmChannel {
    fn new(plan: ChannelPlan) -> Self {
        let (min_active_steps, _) = plan.active_steps_range();
        Self {
            plan,
            active_steps: min_active_steps,
            running: false,
        }
    }

    pub fn plan(&self) -> &ChannelPlan {
        &self.plan
    }

    pub fn output(&self) -> PwmOutput {
        self.plan.output()
    }

    pub fn polarity(&self) -> Polarity {
        self.plan.polarity()
    }

    /// Steps per period; [`Self::set_reload`] takes `0..=steps()`.
    pub fn steps(&self) -> u16 {
        self.plan.steps()
    }

    pub fn actual_frequency_hz(&self) -> f32 {
        self.plan.actual_frequency_hz()
    }

    /// Usable duty range in percent (inclusive); at least as wide as the requested range.
    pub fn duty_range_percent(&self) -> (f32, f32) {
        self.plan.duty_range_percent()
    }

    /// Usable range for [`Self::set_reload`] (inclusive); at least as wide as the requested range.
    pub fn reload_range(&self) -> (u16, u16) {
        self.plan.active_steps_range()
    }

    pub fn duty_percent(&self) -> f32 {
        self.plan
            .duty_percent_for_active_steps(self.active_steps)
    }

    /// Active steps per period, see [`Self::set_reload`].
    pub fn reload(&self) -> u16 {
        self.active_steps
    }

    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Sets the duty, as the percentage of each period the output is active, rounded to the
    /// nearest step.  Takes effect immediately if running, otherwise on [`Self::start`].
    pub fn set_duty(&mut self, duty_percent: u8) -> Result<(), DutyError> {
        let active_steps = self.plan.active_steps_for_duty(duty_percent)?;
        self.apply(active_steps);
        Ok(())
    }

    /// Sets the duty as the number of steps per period the output is active,
    /// `0..=steps()` (0 = always idle, `steps()` = always active).  Takes effect immediately if
    /// running, otherwise on [`Self::start`].
    pub fn set_reload(&mut self, active_steps: u16) -> Result<(), DutyError> {
        let active_steps = self.plan.check_active_steps(active_steps)?;
        self.apply(active_steps);
        Ok(())
    }

    pub fn start(&mut self) {
        pwm_cmp(self.channel()).write(|w| {
            w.set_value(
                self.plan
                    .compare_for_active_steps(self.active_steps),
            )
        });
        self.write_ctrl(true);
        self.running = true;
    }

    /// Holds the output at its idle level: LOW for normal, HIGH for inverted.
    pub fn stop(&mut self) {
        self.write_idle();
        self.running = false;
    }

    fn apply(&mut self, active_steps: u16) {
        self.active_steps = active_steps;
        if self.running {
            pwm_cmp(self.channel()).write(|w| w.set_value(self.plan.compare_for_active_steps(active_steps)));
        }
    }

    fn write_idle(&self) {
        match self.polarity() {
            // the FPGA forces a disabled channel LOW
            Polarity::Normal => self.write_ctrl(false),
            // LOW is active, so instead keep it enabled with CNT < CMP for the whole period.
            Polarity::Inverted => {
                pwm_cmp(self.channel()).write(|w| w.set_value(self.plan.idle_compare()));
                self.write_ctrl(true);
            }
        }
    }

    fn write_ctrl(&self, enable: bool) {
        pwm_ctrl(self.channel()).write(|w| {
            w.set_enable(enable);
            w.set_polarity(self.polarity().to_bit());
            w.set_timer_src(vals::pwm_ctrl1_timer_src::from_bits(self.plan.timer));
        });
    }

    fn channel(&self) -> usize {
        self.output().channel()
    }
}
