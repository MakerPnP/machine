#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{AnyPin, Level, Output, Speed};
use embassy_stm32::pac::rcc::vals::{Pllm, Plln, Pllsrc};
use embassy_stm32::rcc::mux::{
    Fdcansel, Fmcsel, I2c4sel, I2c1235sel, Saisel, Sdmmcsel, Spi6sel, Spi45sel, Usart16910sel, Usart234578sel, Usbsel,
};
use embassy_stm32::rcc::{AHBPrescaler, APBPrescaler, LsConfig, PllDiv, Sysclk};
use embassy_stm32::{Config, Peri, rcc};
use ioboard_main::TimeService;
use ioboard_main::tracepin::TracePins;
use {defmt_rtt as _, panic_probe as _};

use crate::stepper::bitbash::{GpioBitbashStepper, StepperEnableMode};

#[derive(Debug, Copy, Clone, PartialEq, Hash)]
#[allow(dead_code)]
enum CpuRevision {
    RevV,
    RevY,
    RevZ,
}
const CPU_REV: CpuRevision = CpuRevision::RevY;

mod stepper;
#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let mut config = Config::default();
    config.rcc.hse = Some(rcc::Hse {
        freq: embassy_stm32::time::Hertz(8_000_000),
        mode: rcc::HseMode::Oscillator,
    });
    config.rcc.ls = LsConfig::off();

    config.rcc.sys = Sysclk::PLL1_P;
    config.rcc.d1c_pre = AHBPrescaler::DIV1;
    config.rcc.ahb_pre = AHBPrescaler::DIV2;
    config.rcc.apb1_pre = APBPrescaler::DIV2;
    config.rcc.apb2_pre = APBPrescaler::DIV2;
    config.rcc.apb3_pre = APBPrescaler::DIV2;
    config.rcc.apb4_pre = APBPrescaler::DIV2;
    config.rcc.timer_prescaler = rcc::TimerPrescaler::DefaultX2;

    if CPU_REV == CpuRevision::RevV {
        config.rcc.voltage_scale = rcc::VoltageScale::Scale0;
        config.rcc.pll1 = Some(rcc::Pll {
            source: Pllsrc::HSE,
            prediv: Pllm::DIV1,
            mul: Plln::MUL120,
            // 480Mhz
            divp: Some(PllDiv::DIV2),
            // 160Mhz
            divq: Some(PllDiv::DIV6),
            divr: None,
        });
    } else {
        config.rcc.voltage_scale = rcc::VoltageScale::Scale1;
        config.rcc.pll1 = Some(rcc::Pll {
            source: Pllsrc::HSE,
            prediv: Pllm::DIV1,
            mul: Plln::MUL100,
            // 400Mhz
            divp: Some(PllDiv::DIV2),
            // 160Mhz
            divq: Some(PllDiv::DIV5),
            divr: None,
        });
    }
    config.rcc.pll2 = Some(rcc::Pll {
        source: Pllsrc::HSE,

        prediv: Pllm::DIV4,
        mul: Plln::MUL200,
        // 200Mhz
        divp: Some(PllDiv::DIV2),
        // 100Mhz
        divq: Some(PllDiv::DIV4),
        // 200Mhz
        divr: Some(PllDiv::DIV2),
    });
    config.rcc.pll3 = Some(rcc::Pll {
        source: Pllsrc::HSE,
        prediv: Pllm::DIV4,
        mul: Plln::MUL192,
        // 192Mhz
        divp: Some(PllDiv::DIV2),
        // 48Mhz
        divq: Some(PllDiv::DIV8),
        // 92Mhz
        divr: Some(PllDiv::DIV4),
    });

    // 200mhz
    config.rcc.mux.quadspisel = Fmcsel::PLL2_R;
    // 200mhz
    config.rcc.mux.sdmmcsel = Sdmmcsel::PLL2_R;
    // 100mhz
    config.rcc.mux.fdcansel = Fdcansel::PLL2_Q;
    // 48mhz from crystal (not RC48)
    config.rcc.mux.usbsel = Usbsel::PLL3_Q;
    // 100/120mhz
    config.rcc.mux.usart234578sel = Usart234578sel::PCLK1;
    // 100/120mhz
    config.rcc.mux.usart16910sel = Usart16910sel::PCLK2;
    // 100/120mhz
    config.rcc.mux.i2c1235sel = I2c1235sel::PCLK1;
    // 100mhz
    config.rcc.mux.i2c4sel = I2c4sel::PCLK4;
    // 200mhz
    config.rcc.mux.spi123sel = Saisel::PLL2_P;
    // 100mhz
    config.rcc.mux.spi45sel = Spi45sel::PLL2_Q;
    // 100mhz
    config.rcc.mux.spi6sel = Spi6sel::PLL2_Q;

    let p = embassy_stm32::init(config);
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
