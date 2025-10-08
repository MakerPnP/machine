#![no_std]

extern crate alloc;

pub mod stepper;

use alloc::vec::Vec;

use defmt::info;
use embedded_alloc::LlffHeap as Heap;
use embedded_hal::delay::DelayNs;
use libm::round;
use rsruckig::prelude::*;

use crate::stepper::{Stepper, StepperDirection, StepperError};
use crate::tracepin::TracePins;

#[global_allocator]
static HEAP: Heap = Heap::empty();

pub fn run<DELAY: DelayNs, TIME: TimeService, #[cfg(feature = "tracepin")] TRACEPINS: TracePins>(
    stepper: &mut impl Stepper,
    mut delay: DELAY,
    mut time: TIME,
    #[cfg(feature = "tracepin")] mut trace_pins: TRACEPINS,
) {
    init_heap();

    #[cfg(feature = "tracepin")]
    {
        info!("Initializing trace pins");
        trace_pins.all_on();
        delay.delay_ms(500);
        trace_pins.all_off();

        tracepin::init(trace_pins);
    }

    let step_frequency_khz = 20_000;
    let step_period_us = 1_000_000 / step_frequency_khz;
    let step_pulse_width_us = 4;
    let step_pulse_delay_us = step_period_us - step_pulse_width_us;
    info!(
        "Step frequency: {} kHz, period: {} us, pulse width: {} us, pulse delay: {} us",
        step_frequency_khz, step_period_us, step_pulse_width_us, step_pulse_delay_us,
    );
    stepper.set_pulse_width_us(step_pulse_width_us);
    stepper.set_pulse_delay_us(step_pulse_delay_us);

    let default_motor_steps = 200;
    let micro_stepping_multiplier = 8;
    let motor_steps = default_motor_steps * micro_stepping_multiplier;

    info!(
        "Default motor steps: {}, micro stepping multiplier: {}",
        default_motor_steps, micro_stepping_multiplier
    );
    info!("Motor steps per revolution: {}", motor_steps);

    let move_steps = motor_steps;

    let trajectory_units: &[(f64, f64, f64, f64)] = &[
        // (degrees, max_jerk, max_acc, max_vel)
        (360.0, 10000.0, 10000.0, 10000.0),
        (0.0, 10000.0, 10000.0, 10000.0),
        (360.0, 20000.0, 20000.0, 20000.0),
        (0.0, 20000.0, 20000.0, 20000.0),
        (360.0, 40000.0, 40000.0, 40000.0),
        (0.0, 40000.0, 40000.0, 40000.0),
        (360.0, 80000.0, 80000.0, 80000.0),
        (0.0, 80000.0, 80000.0, 80000.0),
    ];

    let steps_per_unit = motor_steps as f64 / 360.0;

    loop {
        // for i in 0..2 {
        //     info!("Run simple loop {}", i);
        //     if run_simple_loop(&mut delay, stepper, &mut time, move_steps).is_err() {
        //         break
        //     }
        //     delay.delay_ms(1000);
        // }

        for i in 0..2 {
            info!("Run trajectory {}", i);
            if run_trajectory_loop(stepper, &mut time, trajectory_units, steps_per_unit).is_err() {
                break;
            }
            delay.delay_ms(1000);
        }
    }
}

fn run_simple_loop<DELAY: DelayNs, TIME: TimeService>(
    delay: &mut DELAY,
    stepper: &mut impl Stepper,
    time: &mut TIME,
    move_steps: i32,
) -> Result<(), StepperError> {
    let cycle_interval_micros = 200;
    let direction_change_delay_ms = 500;

    info!("Normal");
    stepper.direction(StepperDirection::Normal)?;

    delay.delay_ms(direction_change_delay_ms);

    let start_time = time.now_micros();
    let mut step_deadline = start_time;
    for _ in 0..move_steps {
        stepper.step_and_wait()?;
        // delay.delay_us(pulse_interval_us);
        step_deadline = step_deadline.wrapping_add(cycle_interval_micros);
        time.delay_until_micros(step_deadline);
    }

    info!("Reversed");
    stepper.direction(StepperDirection::Reversed)?;

    delay.delay_ms(direction_change_delay_ms);

    let start_time = time.now_micros();
    let mut step_deadline = start_time;
    for _ in 0..move_steps {
        stepper.step_and_wait()?;
        // delay.delay_us(pulse_interval_us);
        step_deadline = step_deadline.wrapping_add(cycle_interval_micros);
        time.delay_until_micros(step_deadline);
    }
    Ok::<(), StepperError>(())
}

fn run_trajectory_loop<TIME: TimeService>(
    stepper: &mut impl Stepper,
    time: &mut TIME,
    trajectory_units: &[(f64, f64, f64, f64)],
    steps_per_unit: f64,
) -> Result<(), StepperError> {
    // -------- Configuration ---------
    let cycle_interval_micros = 1000; // 1 ms cycle (1000 Hz)
    let dt = 1.0_f64 / cycle_interval_micros as f64;

    info!("cycle_interval_micros: {}, dt: {}", cycle_interval_micros, dt);

    info!("Trajectory (units):");
    for (position, jerk, acc, velocity) in trajectory_units {
        info!(
            "position: {}, jerk: {}, acc: {}, velocity: {}",
            position, jerk, acc, velocity
        );
    }

    let trajectory_steps = trajectory_units
        .iter()
        .map(|(position, jerk, acc, velocity)| {
            (
                (position * steps_per_unit) as i64,
                jerk * steps_per_unit,
                acc * steps_per_unit,
                velocity * steps_per_unit,
            )
        })
        .collect::<Vec<(i64, f64, f64, f64)>>();

    info!("Trajectory (steps):");
    for (position, jerk, acc, velocity) in &trajectory_steps {
        info!(
            "position: {}, jerk: {}, acc: {}, velocity: {}",
            position, jerk, acc, velocity
        );
    }

    let mut ruckig = Ruckig::<1, ThrowErrorHandler>::new(None, dt);

    let mut input = InputParameter::<1>::new(None);
    let mut output = OutputParameter::<1>::new(None);
    let mut last_position_steps = 0i64;

    let mut segment_index = 0;

    let start_time = time.now_micros();
    let mut cycle_deadline = start_time;

    let mut prepare_next_segment = true;

    loop {
        if prepare_next_segment {
            info!("Preparing segment, index: {}", segment_index);

            let (target_steps, max_jerk, max_acc, max_vel) = trajectory_steps[segment_index];

            if target_steps as f64 > output.new_position[0] {
                info!("Direction: Normal");
                stepper.direction(StepperDirection::Normal)?;
            } else {
                info!("Direction: Reversed");
                stepper.direction(StepperDirection::Reversed)?;
            }

            input.target_position = daov_stack![target_steps as f64];
            input.target_velocity = daov_stack![0.0];
            input.target_acceleration = daov_stack![0.0];

            input.max_jerk = daov_stack![max_jerk];
            input.max_acceleration = daov_stack![max_acc];
            input.max_velocity = daov_stack![max_vel];

            output.time = 0.0;
            output.new_section = segment_index;

            ruckig.reset();
        }

        tracepin::on(0);

        // On an STM32H743ZI @ 400Mhz this takes ~758us when the segment is changed, and ~25us otherwise (including tracepin overheads)
        let result = ruckig
            .update(&input, &mut output)
            .unwrap();
        output.pass_to_input(&mut input);

        tracepin::off(0);

        if prepare_next_segment {
            prepare_next_segment = false;

            // When changing the segment, after the initial calculation is done, which takes longer then normal,
            // a the cycle deadline is reset to avoid first-step jitter on the rare case where there is actually
            // a step on the first cycle.
            cycle_deadline = time.now_micros();
        }

        if matches!(result, RuckigResult::Finished) {
            // prepare for new segment
            segment_index += 1;
            if segment_index >= trajectory_steps.len() {
                break;
            } else {
                prepare_next_segment = true;
            }
        }

        // Convert to steps with rounding - deterministic and safe because ruckig final position always includes target position.
        let new_position_steps = round(output.new_position[0]) as i64;
        let steps_this_cycle = (new_position_steps - last_position_steps).abs() as u32;

        if steps_this_cycle > 0 {
            let cycle_start_us = time.now_micros();
            let pulse_interval_us: u64 = cycle_interval_micros / steps_this_cycle as u64;

            let mut step_deadline = cycle_start_us;

            for _ in 0..steps_this_cycle {
                stepper.step_and_wait()?;

                step_deadline = step_deadline.wrapping_add(pulse_interval_us);
                time.delay_until_micros(step_deadline);
            }
        }

        // Prepare input for next cycle
        last_position_steps = new_position_steps;
        cycle_deadline = cycle_deadline.wrapping_add(cycle_interval_micros);

        // Sleep until next RT cycle
        time.delay_until_micros(cycle_deadline);
    }

    Ok::<(), StepperError>(())
}

#[allow(static_mut_refs)]
fn init_heap() {
    use core::mem::MaybeUninit;
    const HEAP_SIZE: usize = 1024;
    static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
    unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
}

pub trait TimeService {
    fn now_micros(&self) -> u64;
    fn delay_until_micros(&self, deadline: u64);
}

pub mod tracepin {
    use core::mem::MaybeUninit;

    /// Safety: for speed, any pins used are assumed to be initialized to the correct state.
    pub trait TracePins {
        fn set_pin_on(&mut self, pin: u8);
        fn set_pin_off(&mut self, pin: u8);

        fn all_off(&mut self);
        fn all_on(&mut self);
    }

    //
    // API to avoid having to pass around a mutable reference to the trace pins
    //

    #[inline(always)]
    pub fn on(pin: u8) {
        #[cfg(feature = "tracepin")]
        unsafe {
            (*TRACE_PINS.assume_init()).set_pin_on(pin);
        }
    }

    #[inline(always)]
    pub fn off(pin: u8) {
        #[cfg(feature = "tracepin")]
        unsafe {
            (*TRACE_PINS.assume_init()).set_pin_off(pin);
        }
    }

    #[cfg(feature = "tracepin")]
    static mut TRACE_PINS: MaybeUninit<*mut dyn TracePins> = MaybeUninit::uninit();

    pub fn init<TRACEPINS: TracePins>(trace_pins: TRACEPINS) {
        #[cfg(feature = "tracepin")]
        unsafe {
            // FUTURE find a no-alloc way to do this in a safe way, avoiding the need for suppressing errors or warnings

            // Leak the trace_pins to give it a true 'static lifetime
            // This is safe because we never try to drop it or access it from multiple places
            let trace_pins_box = alloc::boxed::Box::new(trace_pins);
            let trace_pins_leaked = alloc::boxed::Box::leak(trace_pins_box);
            let trace_pins_ptr: *mut dyn TracePins = trace_pins_leaked as *mut dyn TracePins as _;

            #[allow(static_mut_refs)]
            TRACE_PINS.write(trace_pins_ptr);
        }
    }
}
