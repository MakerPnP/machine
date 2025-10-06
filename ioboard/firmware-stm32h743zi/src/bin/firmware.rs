#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());
    info!("firmware-stm32h743zi");

    let mut led: Output = Output::new(p.PB14, Level::High, Speed::Low);

    ioboard_main::run();

    info!("halt");
}
