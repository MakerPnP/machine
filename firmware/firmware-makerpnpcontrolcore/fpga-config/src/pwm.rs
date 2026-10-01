//! PWM resolver for the FPGA `timer_pwm` peripheral.
//!
//! The peripheral (see `fpga/src/main/io/timer_pwm.v`) has 4 timers feeding 12 output channels
//! (PM1-4, OT1-8).  A timer's period is `G * P * N` sysclk cycles, where:
//! * `G` - global prescaler, shared by all 4 timers, 1..=64 (`TIM_SYNC.PRESCALER + 1`)
//! * `P` - the timer's own prescaler, 1..=256 (`TIMx_CTRL.PRESCALER + 1`)
//! * `N` - the timer's number of steps, 1..=256 (`TIMx_ARR + 1`), i.e. the duty resolution.
//!
//! [`try_resolve`] groups the requested outputs by frequency (one timer per distinct frequency),
//! then picks `G` and each timer's `P`/`N` so every timer has the maximum number of steps.
//!
//! Duty is the fraction of the period the output is *active*: HIGH for [`Polarity::Normal`], LOW
//! for [`Polarity::Inverted`].  With the FPGA's comparator (`pin = CNT < CMP ? !polarity :
//! polarity`) the active phase is the first `CMP` cycles of the period for both polarities
//! (conventional PWM mode-1 shape), so `CMP = active_steps` directly, regardless of polarity.

use alloc::vec::Vec;

use crate::SYSCLK;

pub const TIMER_COUNT: usize = 4;
pub const CHANNEL_COUNT: usize = 12;

pub const GLOBAL_PRESCALER_MAX: u32 = 64;
pub const TIMER_PRESCALER_MAX: u32 = 256;
pub const TIMER_STEPS_MAX: u32 = 256;
/// CMP is 8 bits, so `CMP = N` (output permanently active - CNT < N is true for every valid CNT)
/// is only possible when `N <= 255`. Timers that need a 100% maximum duty are limited to this
/// many steps; a 0% (fully idle) duty needs no such limit, since that's always `CMP = 0`.
pub const TIMER_STEPS_MAX_WITH_IDLE: u32 = 255;

/// Default permitted error between requested and actual frequency, in parts-per-million (0.1%).
///
/// The resolver maximises steps within the tolerance, so a looser tolerance trades frequency
/// accuracy for resolution, e.g. at 1% a 20kHz output resolves to 251 steps at 19.92kHz instead of
/// 250 steps at exactly 20kHz.
///
/// A period of the whole number of sysclk cycles nearest the requested frequency is always
/// accepted regardless of tolerance, since no closer frequency is possible, e.g. 192kHz (260.4
/// cycles) resolves to 260 cycles, 192.3kHz.
pub const DEFAULT_FREQUENCY_TOLERANCE_PPM: u32 = 1_000;

/// The FPGA shares one comparator round-robin between all 12 channels (one channel serviced per
/// sysclk cycle), so any one channel's pin is only revisited every `PWM_SCAN_ROTATION_CYCLES`
/// sysclk cycles, and can lag the true CNT/CMP crossing by anywhere up to that long.
pub const PWM_SCAN_ROTATION_CYCLES: u32 = 12;

/// Periods shorter than two full scan rotations can't be reproduced on the pin at all.  This is
/// necessary but NOT sufficient: see [`MIN_SAFE_PHASE_CYCLES`] for the per-phase (not just
/// per-period) bound that actually guarantees glitch-free output.
pub const MIN_PERIOD_CYCLES: u32 = 2 * PWM_SCAN_ROTATION_CYCLES;

/// Minimum safe length, in sysclk cycles, for a channel's shorter phase (its active time for a
/// near-0% duty, or its idle time for a near-100% duty) - two full scan rotations of margin, so
/// the round-robin scanner is guaranteed to catch the transition into AND out of that phase
/// before the next one is due. A phase shorter than this risks the scanner missing its window
/// entirely, carrying the previous level over into (or through) the next period - observed in
/// practice as pulses that are intermittently stretched to roughly double width, or pulses from
/// an adjacent channel in the scan order merging across what should have been two periods.
/// Exact 0% or exact 100% duty are always safe regardless (no transition to miss), since there's
/// no "short phase" to catch at all at that extreme.
pub const MIN_SAFE_PHASE_CYCLES: u32 = 2 * PWM_SCAN_ROTATION_CYCLES;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum PwmOutput {
    Pm1,
    Pm2,
    Pm3,
    Pm4,
    Ot1,
    Ot2,
    Ot3,
    Ot4,
    Ot5,
    Ot6,
    Ot7,
    Ot8,
}

impl PwmOutput {
    /// FPGA PWM channel index, 0..=11 (PWM_CTRL1-12).
    pub const fn channel(self) -> usize {
        self as usize
    }
}

pub const PM_OUT1: PwmOutput = PwmOutput::Pm1;
pub const PM_OUT2: PwmOutput = PwmOutput::Pm2;
pub const PM_OUT3: PwmOutput = PwmOutput::Pm3;
pub const PM_OUT4: PwmOutput = PwmOutput::Pm4;
pub const OT_OUT1: PwmOutput = PwmOutput::Ot1;
pub const OT_OUT2: PwmOutput = PwmOutput::Ot2;
pub const OT_OUT3: PwmOutput = PwmOutput::Ot3;
pub const OT_OUT4: PwmOutput = PwmOutput::Ot4;
pub const OT_OUT5: PwmOutput = PwmOutput::Ot5;
pub const OT_OUT6: PwmOutput = PwmOutput::Ot6;
pub const OT_OUT7: PwmOutput = PwmOutput::Ot7;
pub const OT_OUT8: PwmOutput = PwmOutput::Ot8;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Polarity {
    /// Active HIGH, idle LOW.
    Normal,
    /// Active LOW, idle HIGH.
    Inverted,
}

impl Polarity {
    /// Value of the FPGA's `PWM_CTRLn.POLARITY` bit.
    pub const fn to_bit(self) -> bool {
        matches!(self, Polarity::Inverted)
    }
}

pub const POLARITY_NORMAL: Polarity = Polarity::Normal;
pub const POLARITY_INVERTED: Polarity = Polarity::Inverted;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct PwmRequest {
    pub output: PwmOutput,
    pub frequency_hz: u32,
    pub min_duty_percent: u8,
    pub max_duty_percent: u8,
    pub polarity: Polarity,
}

/// `(output, frequency_hz, min_duty_percent, max_duty_percent, polarity)`
impl From<(PwmOutput, u32, u8, u8, Polarity)> for PwmRequest {
    fn from((output, frequency_hz, min_duty_percent, max_duty_percent, polarity): (PwmOutput, u32, u8, u8, Polarity)) -> Self {
        Self {
            output,
            frequency_hz,
            min_duty_percent,
            max_duty_percent,
            polarity,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ResolveError {
    /// The same output was requested more than once.
    DuplicateOutput(PwmOutput),
    /// `min_duty_percent > max_duty_percent`, or `max_duty_percent > 100`.
    InvalidDutyRange(PwmOutput),
    /// The frequency is zero, or cannot be generated within tolerance by any prescaler/steps.
    FrequencyOutOfRange(PwmOutput),
    /// More distinct frequencies than there are timers.
    TooManyFrequencies { count: usize },
    /// Each frequency can be generated on its own, but not all together with one shared global
    /// prescaler.
    NoCommonPrescaler,
    /// At this frequency, some duty in the requested (possibly widened) range would leave this
    /// channel's active or idle phase shorter than [`MIN_SAFE_PHASE_CYCLES`] - the shared
    /// round-robin comparator isn't guaranteed to catch the transition in time, risking
    /// intermittently stretched or merged pulses. Narrow the duty range away from that extreme,
    /// or lower the frequency.
    DutyPhaseTooShort(PwmOutput),
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DutyError {
    /// The duty is outside the channel's usable (widened) range.
    OutOfRange,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct TimerPlan {
    pub frequency_hz: u32,
    /// Timer prescaler divide, 1..=256.
    pub prescaler: u16,
    /// Steps per period (`ARR + 1`), 1..=256.
    pub steps: u16,
}

impl TimerPlan {
    /// Value for `TIMx_CTRL.PRESCALER`.
    pub const fn prescaler_reg(&self) -> u8 {
        (self.prescaler - 1) as u8
    }

    /// Value for `TIMx_ARR`.
    pub const fn arr_reg(&self) -> u8 {
        (self.steps - 1) as u8
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct ChannelPlan {
    pub request: PwmRequest,
    /// Timer index, 0..=3 (TIM1-4).
    pub timer: u8,
    steps: u16,
    period_cycles: u32,
    min_active_steps: u16,
    max_active_steps: u16,
}

impl ChannelPlan {
    fn new(request: PwmRequest, timer: u8, global_prescaler: u8, timer_plan: &TimerPlan) -> Self {
        let steps = timer_plan.steps as u32;
        // widen, never narrow: round the minimum down and the maximum up.
        let min_active_steps = (request.min_duty_percent as u32 * steps) / 100;
        let max_active_steps = (request.max_duty_percent as u32 * steps).div_ceil(100);

        Self {
            request,
            timer,
            steps: timer_plan.steps,
            period_cycles: global_prescaler as u32 * timer_plan.prescaler as u32 * steps,
            min_active_steps: min_active_steps as u16,
            max_active_steps: max_active_steps as u16,
        }
    }

    pub const fn output(&self) -> PwmOutput {
        self.request.output
    }

    pub const fn polarity(&self) -> Polarity {
        self.request.polarity
    }

    /// Steps per period, i.e. the duty resolution.  Active steps are `0..=steps()`.
    pub const fn steps(&self) -> u16 {
        self.steps
    }

    pub const fn period_cycles(&self) -> u32 {
        self.period_cycles
    }

    pub fn actual_frequency_hz(&self) -> f32 {
        SYSCLK as f32 / self.period_cycles as f32
    }

    /// Sysclk cycles per tick (one CNT increment) - `period_cycles() / steps()`.
    const fn divide_per_tick(&self) -> u32 {
        self.period_cycles / self.steps as u32
    }

    /// `Some(output)` if this channel's requested (possibly widened) duty range would allow an
    /// active or idle phase shorter than [`MIN_SAFE_PHASE_CYCLES`] - see that constant's docs.
    /// Exact 0%/100% duty is exempt (nothing for the scanner to catch at that extreme).
    fn unsafe_phase_output(&self) -> Option<PwmOutput> {
        let divide_per_tick = self.divide_per_tick();
        let min_active_cycles = self.min_active_steps as u32 * divide_per_tick;
        let max_idle_cycles = (self.steps as u32 - self.max_active_steps as u32) * divide_per_tick;
        let active_unsafe = self.min_active_steps > 0 && min_active_cycles < MIN_SAFE_PHASE_CYCLES;
        let idle_unsafe = (self.max_active_steps as u32) < self.steps as u32 && max_idle_cycles < MIN_SAFE_PHASE_CYCLES;
        (active_unsafe || idle_unsafe).then_some(self.request.output)
    }

    /// Usable range of active steps (inclusive), always at least as wide as the requested duty range.
    pub const fn active_steps_range(&self) -> (u16, u16) {
        (self.min_active_steps, self.max_active_steps)
    }

    /// Usable duty range in percent (inclusive), always at least as wide as the requested duty range.
    pub fn duty_range_percent(&self) -> (f32, f32) {
        (
            self.duty_percent_for_active_steps(self.min_active_steps),
            self.duty_percent_for_active_steps(self.max_active_steps),
        )
    }

    pub fn duty_percent_for_active_steps(&self, active_steps: u16) -> f32 {
        active_steps as f32 * 100.0 / self.steps as f32
    }

    /// Converts a duty percentage to active steps (rounded to nearest), checked against the usable range.
    pub fn active_steps_for_duty(&self, duty_percent: u8) -> Result<u16, DutyError> {
        if duty_percent > 100 {
            return Err(DutyError::OutOfRange);
        }
        let active_steps = (duty_percent as u32 * self.steps as u32 + 50) / 100;
        self.check_active_steps(active_steps as u16)
    }

    pub fn check_active_steps(&self, active_steps: u16) -> Result<u16, DutyError> {
        if active_steps < self.min_active_steps || active_steps > self.max_active_steps {
            return Err(DutyError::OutOfRange);
        }
        Ok(active_steps)
    }

    /// Value for `PWM_CMPn` producing `active_steps`, which must already have been checked.
    pub const fn compare_for_active_steps(&self, active_steps: u16) -> u8 {
        active_steps as u8
    }

    /// Value for `PWM_CMPn` holding the output permanently at its idle (inactive) level.
    ///
    /// Always `0` (`CNT < 0` is never true), so unlike [`Self::compare_for_active_steps`] at
    /// `active_steps = steps()`, this needs no [`TIMER_STEPS_MAX_WITH_IDLE`] headroom - it works
    /// for any step count.
    pub const fn idle_compare(&self) -> u8 {
        0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct PwmPlan {
    /// Global prescaler divide, 1..=64.
    pub global_prescaler: u8,
    /// TIM1-4, `None` when unused.
    pub timers: [Option<TimerPlan>; TIMER_COUNT],
    /// One per request, in request order.
    pub channels: Vec<ChannelPlan>,
}

impl PwmPlan {
    /// Value for `TIM_SYNC.PRESCALER`.
    pub const fn global_prescaler_reg(&self) -> u8 {
        self.global_prescaler - 1
    }
}

/// Resolves using [`DEFAULT_FREQUENCY_TOLERANCE_PPM`].
pub fn try_resolve<R: Into<PwmRequest> + Copy>(requests: &[R]) -> Result<PwmPlan, ResolveError> {
    try_resolve_with_tolerance(requests, DEFAULT_FREQUENCY_TOLERANCE_PPM)
}

pub fn try_resolve_with_tolerance<R: Into<PwmRequest> + Copy>(
    requests: &[R],
    tolerance_ppm: u32,
) -> Result<PwmPlan, ResolveError> {
    let requests: Vec<PwmRequest> = requests
        .iter()
        .map(|&request| request.into())
        .collect();

    // validate
    let mut seen = [false; CHANNEL_COUNT];
    for request in requests.iter() {
        let channel = request.output.channel();
        if seen[channel] {
            return Err(ResolveError::DuplicateOutput(request.output));
        }
        seen[channel] = true;

        if request.min_duty_percent > request.max_duty_percent || request.max_duty_percent > 100 {
            return Err(ResolveError::InvalidDutyRange(request.output));
        }
        if request.frequency_hz == 0 || nearest_period_cycles(request.frequency_hz) < MIN_PERIOD_CYCLES {
            return Err(ResolveError::FrequencyOutOfRange(request.output));
        }
    }

    // one timer per distinct frequency
    let mut groups: Vec<Group> = Vec::new();
    let mut request_groups: Vec<usize> = Vec::with_capacity(requests.len());
    for request in requests.iter() {
        let needs_full_duty = request.max_duty_percent == 100;
        let max_steps = if needs_full_duty {
            TIMER_STEPS_MAX_WITH_IDLE
        } else {
            TIMER_STEPS_MAX
        };

        let index = match groups
            .iter()
            .position(|group| group.frequency_hz == request.frequency_hz)
        {
            Some(index) => index,
            None => {
                groups.push(Group {
                    frequency_hz: request.frequency_hz,
                    max_steps,
                    first_output: request.output,
                });
                groups.len() - 1
            }
        };
        groups[index].max_steps = groups[index].max_steps.min(max_steps);
        request_groups.push(index);
    }

    if groups.len() > TIMER_COUNT {
        return Err(ResolveError::TooManyFrequencies {
            count: groups.len(),
        });
    }

    // report frequencies that are unreachable on their own before trying combinations.
    for group in groups.iter() {
        let reachable = (1..=GLOBAL_PRESCALER_MAX)
            .any(|global_prescaler| best_candidate(group, global_prescaler, tolerance_ppm).is_some());
        if !reachable {
            return Err(ResolveError::FrequencyOutOfRange(group.first_output));
        }
    }

    let mut best: Option<(Score, u32, [Option<Candidate>; TIMER_COUNT])> = None;
    'global: for global_prescaler in 1..=GLOBAL_PRESCALER_MAX {
        let mut candidates = [None; TIMER_COUNT];
        for (index, group) in groups.iter().enumerate() {
            match best_candidate(group, global_prescaler, tolerance_ppm) {
                Some(candidate) => candidates[index] = Some(candidate),
                None => continue 'global,
            }
        }

        let score = Score::new(&candidates);
        // strictly better only, so ties keep the smallest global prescaler.
        if best
            .as_ref()
            .map_or(true, |(best_score, _, _)| score > *best_score)
        {
            best = Some((score, global_prescaler, candidates));
        }
    }

    let Some((_, global_prescaler, candidates)) = best else {
        return Err(ResolveError::NoCommonPrescaler);
    };
    let global_prescaler = global_prescaler as u8;

    let mut timers = [None; TIMER_COUNT];
    for (index, (group, candidate)) in groups.iter().zip(candidates.iter()).enumerate() {
        let candidate = candidate.unwrap();
        timers[index] = Some(TimerPlan {
            frequency_hz: group.frequency_hz,
            prescaler: candidate.prescaler as u16,
            steps: candidate.steps as u16,
        });
    }

    let channels: Vec<ChannelPlan> = requests
        .iter()
        .zip(request_groups.iter())
        .map(|(request, &timer)| ChannelPlan::new(*request, timer as u8, global_prescaler, timers[timer].as_ref().unwrap()))
        .collect();

    for channel in channels.iter() {
        if let Some(output) = channel.unsafe_phase_output() {
            return Err(ResolveError::DutyPhaseTooShort(output));
        }
    }

    Ok(PwmPlan {
        global_prescaler,
        timers,
        channels,
    })
}

struct Group {
    frequency_hz: u32,
    max_steps: u32,
    first_output: PwmOutput,
}

#[derive(Debug, Copy, Clone)]
struct Candidate {
    prescaler: u32,
    steps: u32,
    error_ppm: u32,
}

/// Ordered so that a greater score is a better solution: maximise the worst timer's resolution
/// first, then the total resolution, then minimise the total frequency error.
#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Score {
    min_steps: u32,
    total_steps: u32,
    negated_total_error_ppm: i64,
}

impl Score {
    fn new(candidates: &[Option<Candidate>]) -> Self {
        let candidates = candidates.iter().flatten();
        Self {
            min_steps: candidates
                .clone()
                .map(|candidate| candidate.steps)
                .min()
                .unwrap_or(0),
            total_steps: candidates
                .clone()
                .map(|candidate| candidate.steps)
                .sum(),
            negated_total_error_ppm: -candidates
                .map(|candidate| candidate.error_ppm as i64)
                .sum::<i64>(),
        }
    }
}

/// The candidate with the most steps (then least error) for the group at the given global
/// prescaler, if any is within tolerance (or has the nearest possible period).
fn best_candidate(group: &Group, global_prescaler: u32, tolerance_ppm: u32) -> Option<Candidate> {
    let mut best: Option<Candidate> = None;
    let nearest_cycles = nearest_period_cycles(group.frequency_hz) as u64;

    for prescaler in 1..=TIMER_PRESCALER_MAX {
        let cycles_per_step = (global_prescaler * prescaler) as u64;
        let denominator = group.frequency_hz as u64 * cycles_per_step;
        let steps_floor = SYSCLK as u64 / denominator;

        // the ideal (fractional) step count lies between these two
        for steps in [steps_floor, steps_floor + 1] {
            if steps < 1 || steps > group.max_steps as u64 {
                continue;
            }

            let period_cycles = cycles_per_step * steps;
            let Some(error_ppm) = frequency_error_ppm(group.frequency_hz, period_cycles as u32) else {
                continue;
            };
            if error_ppm > tolerance_ppm && period_cycles != nearest_cycles {
                continue;
            }

            let candidate = Candidate {
                prescaler,
                steps: steps as u32,
                error_ppm,
            };
            let better = match best {
                None => true,
                Some(best) => {
                    candidate.steps > best.steps || (candidate.steps == best.steps && candidate.error_ppm < best.error_ppm)
                }
            };
            if better {
                best = Some(candidate);
            }
        }
    }

    best
}

/// The whole number of sysclk cycles closest to the requested frequency's period.
fn nearest_period_cycles(frequency_hz: u32) -> u32 {
    ((SYSCLK as u64 + frequency_hz as u64 / 2) / frequency_hz as u64) as u32
}

/// `|actual - requested| / requested`, in ppm, where `actual = SYSCLK / period_cycles`.
/// `None` if it doesn't fit in a u32 (far outside any sensible tolerance).
fn frequency_error_ppm(frequency_hz: u32, period_cycles: u32) -> Option<u32> {
    let requested_cycles_hz = period_cycles as u128 * frequency_hz as u128;
    let difference = requested_cycles_hz.abs_diff(SYSCLK as u128);
    u32::try_from(difference * 1_000_000 / requested_cycles_hz).ok()
}

#[cfg(test)]
mod tests {
    extern crate std;

    use super::*;

    fn plan_ok(requests: &[(PwmOutput, u32, u8, u8, Polarity)]) -> PwmPlan {
        try_resolve(requests).unwrap()
    }

    #[test]
    fn example_config() {
        let plan = plan_ok(&[
            (PM_OUT1, 200, 0, 100, POLARITY_NORMAL),
            (OT_OUT2, 200, 25, 75, POLARITY_INVERTED),
            (PM_OUT2, 20_000, 0, 100, POLARITY_NORMAL),
            (OT_OUT8, 40_000, 50, 75, POLARITY_NORMAL),
        ]);

        // 20kHz/40kHz are exact with G=5 and 250 steps; 200Hz gets the full 255 steps
        // (5 * 196 * 255 = 249900 cycles, 200.08Hz, 400ppm) - 256 isn't allowed, as PM_OUT1
        // requests a 100% maximum duty.
        assert_eq!(plan.global_prescaler, 5);
        assert_eq!(plan.timers[0], Some(TimerPlan { frequency_hz: 200, prescaler: 196, steps: 255 }));
        assert_eq!(plan.timers[1], Some(TimerPlan { frequency_hz: 20_000, prescaler: 2, steps: 250 }));
        assert_eq!(plan.timers[2], Some(TimerPlan { frequency_hz: 40_000, prescaler: 1, steps: 250 }));
        assert_eq!(plan.timers[3], None);
        assert_eq!(plan.global_prescaler_reg(), 4);
        assert_eq!(plan.timers[0].unwrap().prescaler_reg(), 195);
        assert_eq!(plan.timers[0].unwrap().arr_reg(), 254);

        let timers: Vec<u8> = plan.channels.iter().map(|channel| channel.timer).collect();
        assert_eq!(timers, [0, 0, 1, 2]);

        for (channel, frequency) in plan.channels.iter().zip([200.0f32, 200.0, 20_000.0, 40_000.0]) {
            let error = (channel.actual_frequency_hz() - frequency).abs() / frequency;
            assert!(error <= 0.001, "error: {}", error);
        }
        assert_eq!(plan.channels[2].actual_frequency_hz(), 20_000.0);
        assert_eq!(plan.channels[3].actual_frequency_hz(), 40_000.0);

        // 25..75% of 255 steps = 63.75..191.25 -> widened to 63..192
        assert_eq!(plan.channels[1].active_steps_range(), (63, 192));
        assert_eq!(plan.channels[0].active_steps_range(), (0, 255));
        // 50..75% of 250 steps = 125..187.5 -> 125..188
        assert_eq!(plan.channels[3].active_steps_range(), (125, 188));
    }

    #[test]
    fn exact_full_resolution() {
        // 50MHz / (G * P * 256): G=1, P=1 -> 195312.5Hz, so use a frequency that divides exactly.
        // min=10/max=90, not 0/100: a 100% maximum duty needs CMP=256 (unrepresentable in 8
        // bits, capping at 255 steps), and either extreme would also leave a too-short phase for
        // MIN_SAFE_PHASE_CYCLES - neither is what this test (full 256-step resolution) is about.
        let plan = plan_ok(&[(PM_OUT1, 50_000_000 / (256 * 4), 10, 90, POLARITY_NORMAL)]);
        let timer = plan.timers[0].unwrap();
        assert_eq!(timer.steps, 256);
        assert_eq!(plan.global_prescaler as u16 * timer.prescaler, 4);
        assert_eq!(timer.arr_reg(), 255);
    }

    #[test]
    fn full_max_duty_limits_steps_regardless_of_polarity() {
        let frequency = 50_000_000 / (256 * 4);
        for polarity in [POLARITY_NORMAL, POLARITY_INVERTED] {
            let plan = plan_ok(&[(PM_OUT1, frequency, 0, 100, polarity)]);
            let timer = plan.timers[0].unwrap();
            assert!(timer.steps <= 255);
        }
    }

    #[test]
    fn zero_min_duty_needs_no_step_limit() {
        // A 0% minimum (but comfortably < 100% maximum) duty is always CMP = 0, so it doesn't
        // need the TIMER_STEPS_MAX_WITH_IDLE headroom a 100% maximum does - full 256 steps stays
        // available. max=90, not 99: 99% would leave only a 2-step (8-cycle) idle phase, well
        // under MIN_SAFE_PHASE_CYCLES, which isn't what this test is about.
        let frequency = 50_000_000 / (256 * 4);
        for polarity in [POLARITY_NORMAL, POLARITY_INVERTED] {
            let plan = plan_ok(&[(PM_OUT1, frequency, 0, 90, polarity)]);
            let timer = plan.timers[0].unwrap();
            assert_eq!(timer.steps, 256);
            assert_eq!(plan.channels[0].idle_compare(), 0);
        }
    }

    #[test]
    fn idle_limit_applies_to_whole_timer() {
        let frequency = 50_000_000 / (256 * 4);
        let plan = plan_ok(&[
            (PM_OUT1, frequency, 10, 100, POLARITY_NORMAL),
            (PM_OUT2, frequency, 10, 100, POLARITY_INVERTED),
        ]);
        assert!(plan.timers[0].unwrap().steps <= 255);
        assert_eq!(plan.channels[0].steps(), plan.channels[1].steps());
    }

    #[test]
    fn identical_frequencies_share_a_timer() {
        let plan = plan_ok(&[
            (OT_OUT1, 1_000, 0, 100, POLARITY_NORMAL),
            (OT_OUT2, 2_000, 0, 100, POLARITY_NORMAL),
            (OT_OUT3, 1_000, 0, 100, POLARITY_NORMAL),
        ]);
        let timers: Vec<u8> = plan.channels.iter().map(|channel| channel.timer).collect();
        assert_eq!(timers, [0, 1, 0]);
        assert!(plan.timers[2].is_none());
    }

    #[test]
    fn four_frequencies_ok_five_rejected() {
        let mut requests = std::vec![
            (OT_OUT1, 1_000, 0, 100, POLARITY_NORMAL),
            (OT_OUT2, 2_000, 0, 100, POLARITY_NORMAL),
            (OT_OUT3, 5_000, 0, 100, POLARITY_NORMAL),
            (OT_OUT4, 10_000, 0, 100, POLARITY_NORMAL),
        ];
        let plan = plan_ok(&requests);
        assert!(plan.timers.iter().all(Option::is_some));

        requests.push((OT_OUT5, 20_000, 0, 100, POLARITY_NORMAL));
        assert_eq!(try_resolve(&requests), Err(ResolveError::TooManyFrequencies { count: 5 }));
    }

    #[test]
    fn duplicate_output_rejected() {
        let result = try_resolve(&[
            (OT_OUT1, 1_000, 0, 100, POLARITY_NORMAL),
            (OT_OUT1, 1_000, 0, 100, POLARITY_NORMAL),
        ]);
        assert_eq!(result, Err(ResolveError::DuplicateOutput(OT_OUT1)));
    }

    #[test]
    fn invalid_duty_range_rejected() {
        assert_eq!(
            try_resolve(&[(OT_OUT1, 1_000, 60, 40, POLARITY_NORMAL)]),
            Err(ResolveError::InvalidDutyRange(OT_OUT1))
        );
        assert_eq!(
            try_resolve(&[(OT_OUT1, 1_000, 0, 101, POLARITY_NORMAL)]),
            Err(ResolveError::InvalidDutyRange(OT_OUT1))
        );
    }

    #[test]
    fn frequency_limits() {
        // floor is 50MHz / (64 * 256 * 256) ~= 11.92Hz
        assert!(try_resolve(&[(OT_OUT1, 13, 1, 100, POLARITY_NORMAL)]).is_ok());
        assert_eq!(
            try_resolve(&[(OT_OUT1, 10, 1, 100, POLARITY_NORMAL)]),
            Err(ResolveError::FrequencyOutOfRange(OT_OUT1))
        );
        assert_eq!(
            try_resolve(&[(OT_OUT1, 0, 1, 100, POLARITY_NORMAL)]),
            Err(ResolveError::FrequencyOutOfRange(OT_OUT1))
        );
        // ceiling is a period of MIN_PERIOD_CYCLES (24) cycles, ~2.08MHz
        let plan = plan_ok(&[(OT_OUT1, 2_000_000, 0, 100, POLARITY_NORMAL)]);
        assert_eq!(plan.channels[0].period_cycles(), 25);
        assert!(try_resolve(&[(OT_OUT1, 2_083_333, 0, 100, POLARITY_NORMAL)]).is_ok());
        assert_eq!(
            try_resolve(&[(OT_OUT1, 2_200_000, 0, 100, POLARITY_NORMAL)]),
            Err(ResolveError::FrequencyOutOfRange(OT_OUT1))
        );
        assert_eq!(
            try_resolve(&[(OT_OUT1, 60_000_000, 0, 100, POLARITY_NORMAL)]),
            Err(ResolveError::FrequencyOutOfRange(OT_OUT1))
        );
    }

    #[test]
    fn nearest_period_accepted_outside_tolerance() {
        // 260.4 cycles: the nearest possible period, 260 cycles (192.3kHz), is 0.16% off.
        let plan = plan_ok(&[(OT_OUT1, 192_000, 0, 100, POLARITY_NORMAL)]);
        assert_eq!(plan.channels[0].period_cycles(), 260);
        assert_eq!(plan.channels[0].steps(), 130);
    }

    #[test]
    fn no_common_prescaler() {
        // 13Hz needs G>=59, 2MHz (25 cycles) needs G<=25.
        let result = try_resolve(&[
            (OT_OUT1, 13, 1, 100, POLARITY_NORMAL),
            (OT_OUT2, 2_000_000, 0, 100, POLARITY_NORMAL),
        ]);
        assert_eq!(result, Err(ResolveError::NoCommonPrescaler));
    }

    #[test]
    fn inexact_frequency_within_tolerance() {
        let plan = plan_ok(&[(OT_OUT1, 30_000, 0, 100, POLARITY_NORMAL)]);
        let error = (plan.channels[0].actual_frequency_hz() - 30_000.0).abs() / 30_000.0;
        assert!(error <= 0.001, "error: {}", error);
        // 1666.67 cycles: G*P=7 -> 238 steps (30012Hz); 239 steps would be 0.38% off.
        assert_eq!(plan.channels[0].steps(), 238);
        assert_eq!(
            try_resolve_with_tolerance(&[(OT_OUT1, 30_000, 0, 100, POLARITY_NORMAL)], 10_000)
                .unwrap()
                .channels[0]
                .steps(),
            239
        );

        // 3846153.8 cycles: 13Hz is fine at 0.1%, but the nearest period can't be made from G * P * N.
        assert!(try_resolve(&[(OT_OUT1, 13, 1, 100, POLARITY_NORMAL)]).is_ok());
        assert_eq!(
            try_resolve_with_tolerance(&[(OT_OUT1, 13, 1, 100, POLARITY_NORMAL)], 0),
            Err(ResolveError::FrequencyOutOfRange(OT_OUT1))
        );
    }

    #[test]
    fn widened_duty_range_is_never_narrower() {
        // Some (frequency, min, max) combinations here are now correctly rejected with
        // DutyPhaseTooShort (e.g. 1MHz with a duty extreme close to 0% or 100%) - this test is
        // about widening behaviour for configs that DO resolve, so skip those rather than assert
        // on them; full_max_duty_limits_steps_regardless_of_polarity and friends cover the
        // rejection path itself.
        for frequency in [13, 200, 1_234, 20_000, 40_000, 192_000, 1_000_000] {
            for min in (0..=100u8).step_by(7) {
                for max in (min..=100u8).step_by(5) {
                    let plan = match try_resolve(&[(OT_OUT1, frequency, min, max, POLARITY_INVERTED)]) {
                        Ok(plan) => plan,
                        Err(ResolveError::DutyPhaseTooShort(_)) => continue,
                        Err(e) => panic!("{} {}..{} -> unexpected error {:?}", frequency, min, max, e),
                    };
                    let channel = &plan.channels[0];
                    let (lo, hi) = channel.duty_range_percent();
                    assert!(lo <= min as f32 && hi >= max as f32, "{} {}..{} -> {}..{}", frequency, min, max, lo, hi);
                    assert!(channel.active_steps_for_duty(min).is_ok());
                    assert!(channel.active_steps_for_duty(max).is_ok());
                }
            }
        }
    }

    #[test]
    fn set_duty_conversion_and_range_check() {
        let plan = plan_ok(&[(OT_OUT2, 20_000, 25, 75, POLARITY_INVERTED)]);
        let channel = &plan.channels[0];
        assert_eq!(channel.steps(), 250);

        assert_eq!(channel.active_steps_for_duty(50), Ok(125));
        assert_eq!(channel.active_steps_for_duty(25), Ok(63)); // 62.5 rounds up
        assert_eq!(channel.active_steps_for_duty(75), Ok(188)); // 187.5 rounds up
        assert_eq!(channel.active_steps_for_duty(24), Err(DutyError::OutOfRange));
        assert_eq!(channel.active_steps_for_duty(76), Err(DutyError::OutOfRange));
        assert_eq!(channel.active_steps_for_duty(101), Err(DutyError::OutOfRange));

        // widened bounds are usable via raw steps
        assert_eq!(channel.check_active_steps(62), Ok(62));
        assert_eq!(channel.check_active_steps(188), Ok(188));
        assert_eq!(channel.check_active_steps(61), Err(DutyError::OutOfRange));
        assert_eq!(channel.check_active_steps(189), Err(DutyError::OutOfRange));

        assert_eq!(channel.compare_for_active_steps(125), 125);
        assert_eq!(channel.compare_for_active_steps(188), 188);
    }

    #[test]
    fn full_and_zero_duty_compare_values() {
        let plan = plan_ok(&[(PM_OUT1, 200, 0, 100, POLARITY_NORMAL)]);
        let channel = &plan.channels[0];
        let steps = channel.steps();
        assert_eq!(channel.compare_for_active_steps(steps) as u16, steps); // 100%: CNT < N always true -> always active
        assert_eq!(channel.compare_for_active_steps(0), 0); // 0%: CNT < 0 never true -> always idle
    }

    #[test]
    fn too_short_phase_rejected_at_1mhz() {
        // Real-world regression case: two 1MHz outputs on one timer resolve to only steps=10
        // (period=50 sysclk cycles). 25%/75% duty each land within MIN_SAFE_PHASE_CYCLES of one
        // extreme (OT7's active phase, OT8's idle phase), which in practice produced
        // intermittent stretched/merged pulses on real hardware - this must now be rejected
        // rather than silently accepted.
        let result = try_resolve(&[
            (OT_OUT7, 1_000_000, 25, 25, POLARITY_NORMAL),
            (OT_OUT8, 1_000_000, 75, 75, POLARITY_NORMAL),
        ]);
        assert!(
            matches!(result, Err(ResolveError::DutyPhaseTooShort(_))),
            "{:?}",
            result
        );

        // The same frequency with a comfortably central duty (and no other channel sharing the
        // timer, so the resolver isn't forced to as few steps) is fine.
        let plan = try_resolve(&[(OT_OUT7, 1_000_000, 50, 50, POLARITY_NORMAL)]);
        assert!(plan.is_ok(), "{:?}", plan);
    }
}
