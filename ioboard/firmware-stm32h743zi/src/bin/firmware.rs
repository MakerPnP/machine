#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use {defmt_rtt as _, panic_probe as _};

use crate::stepper::bitbash::{GpioBitbashStepper, StepperEnableMode};

mod stepper;
#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    info!("firmware-stm32h743zi");

    let stepper_delay = embassy_time::Delay;
    let mut stepper = GpioBitbashStepper::new(
        // enable
        Output::new(p.PC8, Level::Low, Speed::Low),
        // step
        Output::new(p.PC9, Level::Low, Speed::Low),
        // direction
        Output::new(p.PC10, Level::Low, Speed::Low),
        StepperEnableMode::ActiveHigh,
        stepper_delay,
        1000,
        1000,
    );
    stepper.initialize_io().unwrap();

    let main_delay = embassy_time::Delay;

    ioboard_main::run(&mut stepper, main_delay);

    info!("halt");
}
