#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::Peri;
use embassy_stm32::gpio::{AnyPin, Level, Output, Speed};
use ioboard_main::TimeService;
use ioboard_main::tracepin::TracePins;
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
    #[cfg(feature = "tracepin")]
    let trace_pins_service = TracePinsService::new([p.PD2.into(), p.PD3.into(), p.PD4.into(), p.PD5.into()]);

    ioboard_main::run(
        &mut stepper,
        main_delay,
        time_service,
        #[cfg(feature = "tracepin")]
        trace_pins_service,
    );

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

struct TracePinsService {
    pins: [Output<'static>; 4],
}

impl TracePinsService {
    fn new(pins: [Peri<'static, AnyPin>; 4]) -> Self {
        Self {
            pins: pins.map(|pin| Output::new(pin, Level::Low, Speed::Low)),
        }
    }
}

impl TracePins for TracePinsService {
    #[inline(always)]
    fn set_pin_on(&mut self, pin: u8) {
        unsafe {
            self.pins
                .get_unchecked_mut(pin as usize)
                .set_high();
        }
    }

    #[inline(always)]
    fn set_pin_off(&mut self, pin: u8) {
        unsafe {
            self.pins
                .get_unchecked_mut(pin as usize)
                .set_low();
        }
    }

    fn all_off(&mut self) {
        for pin in &mut self.pins {
            pin.set_low();
        }
    }

    fn all_on(&mut self) {
        for pin in &mut self.pins {
            pin.set_high();
        }
    }
}
