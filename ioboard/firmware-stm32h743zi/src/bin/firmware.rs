#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use ioboard_main::TimeService;
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
    let time_service = EmbassyTimeService::default();

    ioboard_main::run(&mut stepper, main_delay, time_service);

    info!("halt");
}

#[derive(Default)]
struct EmbassyTimeService {}

impl TimeService for EmbassyTimeService {
    #[inline]
    fn now_micros(&self) -> u64 {
        embassy_time::Instant::now().as_micros()
    }

    fn delay_until_micros(&self, deadline: u64) {
        while self.now_micros() < deadline {
            // unsafe {
            //     core::arch::asm!("wfi");
            // }
        }
    }
}
