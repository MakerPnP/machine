#![no_std]

extern crate alloc;

pub mod stepper;

use embedded_alloc::LlffHeap as Heap;
use embedded_hal::delay::DelayNs;

use crate::stepper::{Stepper, StepperDirection, StepperError};

#[global_allocator]
static HEAP: Heap = Heap::empty();

pub fn run<DELAY: DelayNs>(stepper: &mut impl Stepper, mut delay: DELAY) {
    init_heap();

    let mut run_loop = || {
        delay.delay_ms(500);
        stepper.direction(StepperDirection::Normal)?;
        for _ in 0..100 {
            stepper.step_and_wait()?;
            delay.delay_ms(1);
        }

        delay.delay_ms(500);
        stepper.direction(StepperDirection::Reversed)?;
        for _ in 0..100 {
            stepper.step_and_wait()?;
            delay.delay_ms(1);
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
