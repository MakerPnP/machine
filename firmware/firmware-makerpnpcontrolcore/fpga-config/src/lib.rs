//! Configuration resolvers for the FPGA peripherals.
//!
//! Everything here is pure computation with no hardware access, so it can be used to validate a
//! machine configuration before it is created (e.g. by host-side tooling) and is unit-tested on
//! the host with `cargo test`.

#![no_std]

extern crate alloc;

/// FPGA system clock, in Hz.  All FPGA peripheral timing is derived from this.
pub const SYSCLK: u32 = 50_000_000;

pub mod pwm;
