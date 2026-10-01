#![no_std]
#![no_main]

extern crate alloc;

pub mod stepper;
#[cfg(feature = "tracepin")]
pub mod trace;

pub mod fpga;

pub mod rgb;

pub mod adc;