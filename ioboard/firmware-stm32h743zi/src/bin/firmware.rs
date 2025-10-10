#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_net::tcp::client::TcpClient;
use embassy_net::tcp::client::TcpClientState;
use embassy_stm32::eth::PacketQueue;
use embassy_stm32::eth::{Ethernet, GenericPhy};
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::pac::rcc::vals::{Pllm, Plln, Pllsrc};
use embassy_stm32::peripherals::ETH;
use embassy_stm32::rcc::mux::{
    Fdcansel, Fmcsel, I2c4sel, I2c1235sel, Saisel, Sdmmcsel, Spi6sel, Spi45sel, Usart16910sel, Usart234578sel, Usbsel,
};
use embassy_stm32::rcc::{AHBPrescaler, APBPrescaler, LsConfig, PllDiv, Sysclk};
use embassy_stm32::rng::Rng;
use embassy_stm32::{Config, bind_interrupts, eth, peripherals, rcc, rng};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Ticker};
use embedded_alloc::LlffHeap as Heap;
use embedded_hal_async::delay::DelayNs;
use ioboard_trace::tracepin;
use ioboard_trace::tracepin::TracePins;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

use crate::stepper::bitbash::{GpioBitbashStepper, StepperEnableMode};
use crate::time::EmbassyTimeService;
use crate::trace::TracePinsService;
#[derive(Debug, Copy, Clone, PartialEq, Hash)]
#[allow(dead_code)]
enum CpuRevision {
    RevV,
    RevY,
    RevZ,
}
const CPU_REV: CpuRevision = CpuRevision::RevY;

mod stepper;
mod time;
#[cfg(feature = "tracepin")]
mod trace;

#[global_allocator]
static HEAP: Heap = Heap::empty();

bind_interrupts!(struct Irqs {
    ETH => eth::InterruptHandler;
    RNG => rng::InterruptHandler<peripherals::RNG>;
});

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let mut config = Config::default();
    config.rcc.hse = Some(rcc::Hse {
        freq: embassy_stm32::time::Hertz(8_000_000),
        mode: rcc::HseMode::Oscillator,
    });
    config.rcc.ls = LsConfig::off();
    config.rcc.hsi48 = Some(Default::default()); // needed for RNG
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

    init_heap();

    info!("Initializing LED");
    let led = Output::new(p.PB14, Level::Low, Speed::Low);
    {
        // scope required to release mutex guard
        *(LED.lock().await) = Some(led);
    }
    spawner.spawn(unwrap!(activity_indicator_task(&LED, Duration::from_millis(200))));

    let mut delay = embassy_time::Delay;
    let time_service = EmbassyTimeService::default();

    #[cfg(feature = "tracepin")]
    {
        info!("Initializing trace pins");
        let mut trace_pins = TracePinsService::new([p.PD2.into(), p.PD3.into(), p.PD4.into(), p.PD5.into()]);
        trace_pins.all_on();
        delay.delay_ms(500).await;
        trace_pins.all_off();

        tracepin::init(trace_pins);
    }

    // Generate random seed.
    info!("Initializing RNG");
    let mut rng = Rng::new(p.RNG, Irqs);
    let mut seed = [0; 8];
    rng.fill_bytes(&mut seed);
    let seed = u64::from_le_bytes(seed);

    info!("Initializing ETH");
    // TODO generate mac address from CPU ID
    //      potentially using this algorythm (C): https://github.com/zephyrproject-rtos/zephyr/issues/59993#issuecomment-1644030438
    let mac_addr = [0x00, 0x00, 0xDE, 0xAD, 0xBE, 0xEF];

    static PACKETS: StaticCell<PacketQueue<4, 4>> = StaticCell::new();
    // warning: Not all STM32H7 devices have the exact same pins here
    // for STM32H747XIH, replace p.PB13 for PG12
    let device = Ethernet::new(
        PACKETS.init(PacketQueue::<4, 4>::new()),
        p.ETH,
        Irqs,
        p.PA1,  // ref_clk
        p.PA2,  // mdio
        p.PC1,  // eth_mdc
        p.PA7,  // CRS_DV: Carrier Sense
        p.PC4,  // RX_D0: Received Bit 0
        p.PC5,  // RX_D1: Received Bit 1
        p.PG13, // TX_D0: Transmit Bit 0
        p.PB13, // TX_D1: Transmit Bit 1
        p.PG11, // TX_EN: Transmit Enable
        GenericPhy::new_auto(),
        mac_addr,
    );

    let (stack, runner) = ioboard_net_embassy::init(device, seed);

    // Launch network task
    spawner.spawn(unwrap!(net_task(runner)));
    spawner.spawn(unwrap!(comms_task(stack, time_service)));

    info!("Hardware address: {}", stack.hardware_address());

    info!("Initializing Stepper");
    let mut stepper = GpioBitbashStepper::new(
        // enable
        Output::new(p.PC8, Level::Low, Speed::Low),
        // step
        Output::new(p.PC9, Level::Low, Speed::Low),
        // direction
        Output::new(p.PC10, Level::Low, Speed::Low),
        StepperEnableMode::ActiveHigh,
        1000,
        1000,
    );
    stepper.initialize_io().unwrap();

    info!("Initialisation complete");
    ioboard_main::run(&mut stepper, delay, time_service).await;

    info!("halt");
}

type LedType = Mutex<ThreadModeRawMutex, Option<Output<'static>>>;
static LED: LedType = Mutex::new(None);

#[embassy_executor::task]
async fn activity_indicator_task(led: &'static LedType, delay: Duration) {
    let mut ticker = Ticker::every(delay);

    loop {
        {
            let mut led_unlocked = led.lock().await;
            if let Some(pin_ref) = led_unlocked.as_mut() {
                pin_ref.toggle();
            }
        }
        ticker.next().await;
    }
}

type Device = Ethernet<'static, ETH, GenericPhy>;

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, Device>) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn comms_task(stack: embassy_net::Stack<'static>, time_service: EmbassyTimeService) -> ! {
    // Ensure DHCP configuration is up before trying connect
    stack.wait_config_up().await;

    info!("Network task initialized");

    let state: TcpClientState<1, 1024, 1024> = TcpClientState::new();
    let client = TcpClient::new(stack, &state);

    let mut runner = ioboard_net::init(time_service, client);
    runner.run().await
}

#[allow(static_mut_refs)]
fn init_heap() {
    use core::mem::MaybeUninit;
    const HEAP_SIZE: usize = 1024;
    static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
    unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
}
