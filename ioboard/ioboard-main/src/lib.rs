#![no_std]

extern crate alloc;

pub mod stepper;

use defmt::info;
use embedded_alloc::LlffHeap as Heap;
use embedded_hal::delay::DelayNs;

use crate::stepper::{Stepper, StepperDirection, StepperError};

#[global_allocator]
static HEAP: Heap = Heap::empty();

pub fn run<DELAY: DelayNs>(stepper: &mut impl Stepper, mut delay: DELAY) {
    init_heap();

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
    let step_delay_us = 100;
    let direction_change_delay_ms = 500;
    let mut run_loop = || {
        info!("Normal");
        delay.delay_ms(direction_change_delay_ms);
        stepper.direction(StepperDirection::Normal)?;
        for _ in 0..move_steps {
            stepper.step_and_wait()?;
            delay.delay_us(step_delay_us);
        }

        info!("Reversed");
        delay.delay_ms(direction_change_delay_ms);
        stepper.direction(StepperDirection::Reversed)?;
        for _ in 0..move_steps {
            stepper.step_and_wait()?;
            delay.delay_us(step_delay_us);
        }
        Ok::<(), StepperError>(())
    };

    loop {
        if run_loop().is_err() {
            break;
        }
    }
}

#[allow(static_mut_refs)]
fn init_heap() {
    use core::mem::MaybeUninit;
    const HEAP_SIZE: usize = 1024;
    static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
    unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
}
