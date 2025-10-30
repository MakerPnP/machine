#![no_std]

extern crate alloc;

pub mod stepper;

use alloc::vec::Vec;

use defmt::info;
use embassy_time::{Duration, Instant, Ticker, Timer};
use ioboard_trace::tracepin;
use libm::round;
use rsruckig::prelude::*;

use crate::stepper::{Stepper, StepperDirection, StepperError};

pub async fn run<STEPPER: Stepper>(mut stepper: STEPPER) {
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
        (90.0, 10000.0, 10000.0, 10000.0),
        (0.0, 10000.0, 10000.0, 10000.0),
        (180.0, 50000.0, 50000.0, 50000.0),
        (0.0, 50000.0, 50000.0, 50000.0),
        (360.0, 100000.0, 100000.0, 100000.0),
        (0.0, 100000.0, 100000.0, 100000.0),
    ];

    let steps_per_unit = motor_steps as f64 / 360.0;

    loop {
        for i in 0..2 {
            info!("Run simple loop {}", i);
            if run_simple_loop(&mut stepper, move_steps)
                .await
                .is_err()
            {
                break;
            }
            Timer::after(Duration::from_millis(1000)).await;
        }

        for i in 0..2 {
            info!("Run trajectory {}", i);
            if run_trajectory_loop(&mut stepper, trajectory_units, steps_per_unit)
                .await
                .is_err()
            {
                break;
            }
            Timer::after(Duration::from_millis(1000)).await;
        }
    }
}

async fn run_simple_loop(stepper: &mut impl Stepper, move_steps: i32) -> Result<(), StepperError> {
    let cycle_interval_micros = 175;
    let direction_change_delay_ms = 250;

    info!("Normal");
    stepper.direction(StepperDirection::Normal)?;

    Timer::after(Duration::from_millis(direction_change_delay_ms)).await;

    let mut step_ticker = Ticker::every(Duration::from_micros(cycle_interval_micros));

    for _ in 0..move_steps {
        stepper.step_and_wait().await?;
        step_ticker.next().await;
    }

    info!("Reversed");
    stepper.direction(StepperDirection::Reversed)?;

    Timer::after(Duration::from_millis(direction_change_delay_ms)).await;

    step_ticker.reset();
    for _ in 0..move_steps {
        stepper.step_and_wait().await?;
        step_ticker.next().await;
    }
    Ok::<(), StepperError>(())
}

async fn run_trajectory_loop(
    stepper: &mut impl Stepper,
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

    let mut prepare_next_segment = true;

    let mut cycle_ticker = Ticker::every(Duration::from_micros(cycle_interval_micros));

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
            cycle_ticker.reset();
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

        // FUTURE improve step spacing (e.g. by using a hardware timer to control the step pulse width and frequency
        //        or by using a hardware driven DMA stream

        if steps_this_cycle > 0 {
            let cycle_start_us = Instant::now().as_micros();
            let pulse_interval_us: u64 = cycle_interval_micros / steps_this_cycle as u64;

            let mut step_deadline = cycle_start_us;

            for _ in 0..steps_this_cycle {
                let pulse_delay = stepper.step().await?;

                // wait until next step pulse or the pulse delay has elapsed
                step_deadline = step_deadline.wrapping_add(pulse_interval_us.max(pulse_delay as u64));
                Timer::at(Instant::from_micros(step_deadline)).await
            }
        }

        // Prepare input for next cycle
        last_position_steps = new_position_steps;

        // Sleep until next RT cycle
        cycle_ticker.next().await;
    }

    Ok::<(), StepperError>(())
}
