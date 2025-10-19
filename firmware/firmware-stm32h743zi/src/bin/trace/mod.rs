use embassy_stm32::Peri;
use embassy_stm32::gpio::{AnyPin, Level, Output, Speed};
use ioboard_trace::tracepin::TracePins;

pub struct TracePinsService {
    pins: [Output<'static>; 4],
}

impl TracePinsService {
    pub fn new(pins: [Peri<'static, AnyPin>; 4]) -> Self {
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
