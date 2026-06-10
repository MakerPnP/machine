//! BUILDING:
//! make sure the working directory is the `firmware-makerpnpcontrolcore` directory when running cargo.
//! if not, you'll likely get a compile error in embassy executor as the target from the cargo.toml
//! won't be picked up if you compile from the root directory

#![no_std]
#![no_main]
extern crate alloc;

use core::ptr;

use cortex_m_rt::entry;
use defmt::*;
use embassy_executor::SendSpawner;
use embassy_executor::{Executor, InterruptExecutor, Spawner};
use embassy_stm32::{spi, Peripherals};
use embassy_stm32::eth::{PacketQueue, Sma, StationManagement};
use embassy_stm32::eth::{Ethernet, GenericPhy};
use embassy_stm32::gpio::{Level, Output, Speed, Input, Pull};
use embassy_stm32::interrupt::{InterruptExt, Priority};
use embassy_stm32::pac::rcc::vals::{Pllm, Plln, Pllsrc};
use embassy_stm32::peripherals::{ETH, ETH_SMA};
use embassy_stm32::rcc::mux::{
    Fdcansel, Fmcsel, I2c4sel, I2c1235sel, Saisel, Sdmmcsel, Spi6sel, Spi45sel, Usart16910sel, Usart234578sel, Usbsel,
};
use embassy_stm32::rcc::{AHBPrescaler, APBPrescaler, LsConfig, PllDiv, Sysclk};
use embassy_stm32::rng::Rng;
use embassy_stm32::ospi::{
    AddressSize, ChipSelectHighTime, FIFOThresholdLevel, Instance, MemorySize, MemoryType, Ospi, OspiWidth,
    TransferConfig, WrapSize,
};
use embassy_stm32::{Config, bind_interrupts, eth, interrupt, peripherals, rcc, rng};
use embassy_stm32::mode::Blocking;
use embassy_stm32::spi::mode::Master;
use embassy_stm32::spi::Spi;
use embassy_stm32::time::mhz;
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Delay, Duration, Ticker, Timer};
use embedded_alloc::LlffHeap as Heap;
use embedded_hal::spi::Operation::DelayNs;
use ioboard_main::stepper::Stepper;
#[cfg(feature = "tracepin")]
use ioboard_trace::tracepin;
#[cfg(feature = "tracepin")]
use ioboard_trace::tracepin::TracePins;
use static_cell::StaticCell;
use tmc5160::Tmc5160;
use {defmt_rtt as _, panic_probe as _};
#[cfg(feature = "morse_startup")]
use morse_core::{MorseCharacter, MorseSymbol};
use crate::fpga::FpgaCore;
use crate::stepper::bitbash::{GpioBitbashStepper, StepperEnableMode};
use crate::stepper::tmc5160::Tmc5160Stepper;
#[cfg(feature = "tracepin")]
use crate::trace::TracePinsService;

mod stepper;
#[cfg(feature = "tracepin")]
mod trace;

mod fpga;

//
// Heap/Allocator configuration
//

#[global_allocator]
static HEAP: Heap = Heap::empty();

//
// Embassy configuration
//

bind_interrupts!(struct Irqs {
    ETH => eth::InterruptHandler;
    HASH_RNG => rng::InterruptHandler<peripherals::RNG>;
});

#[interrupt]
unsafe fn I2C1_EV() {
    unsafe { EXECUTOR_HIGH.on_interrupt() }
}

static EXECUTOR_HIGH: InterruptExecutor = InterruptExecutor::new();
static EXECUTOR_LOW: StaticCell<Executor> = StaticCell::new();

#[entry]
fn main() -> ! {
    //trigger_stack_corruption();

    let p = rcc_setup::stm32h735g_init();
    info!("firmware-makerpnpcontrolcore");

    init_heap();

    // High-priority executor: using unused I2C1 interrupt, priority level 6
    interrupt::I2C1_EV.set_priority(Priority::P6);
    let hp_spawner = EXECUTOR_HIGH.start(interrupt::I2C1_EV);

    // Low priority executor: runs in thread mode, using WFE/SEV
    let executor = EXECUTOR_LOW.init(Executor::new());
    executor.run(|lp_spawner| {
        lp_spawner.spawn(unwrap!(init_task(lp_spawner, hp_spawner, p)));
    });
}

#[embassy_executor::task]
async fn init_task(lp_spawner: Spawner, hp_spawner: SendSpawner, p: Peripherals) {
    let mut fpga_creset_b = Output::new(p.PF15, Level::Low, Speed::Low);
    let fpga_cdone = Input::new(p.PC15, Pull::None);

    info!("Enabling FPGA");
    fpga_creset_b.set_high();

    let ospi_config = embassy_stm32::ospi::Config {
        fifo_threshold: FIFOThresholdLevel::_16Bytes,
        memory_type: MemoryType::Standard,
        device_size: MemorySize::_2MiB,
        chip_select_high_time: ChipSelectHighTime::_1Cycle,
        free_running_clock: false,
        clock_mode: false,
        wrap_size: WrapSize::None,
        // TODO increase this speed as much as possible
        clock_prescaler: 5, // 133.33Mhz / (5+1) = 22.22Mhz
        //clock_prescaler: 132, // 133.33Mhz / (132+1) = 9.5Mhz
        sample_shifting: true,
        delay_hold_quarter_cycle: false,
        chip_select_boundary: 0,
        delay_block_bypass: true,
        max_transfer: 0,
        refresh: 0,
    };

    #[allow(unused_variables)]
    let ospi1 = embassy_stm32::ospi::Ospi::new_blocking_quadspi(
        p.OCTOSPI1,
        p.PF10, // P1_CLK
        p.PD11, // P1_IO0
        p.PD12, // P1_IO1
        p.PC2,  // P1_IO2
        p.PD13, // P1_IO3
        p.PG6,  // P1_NCS
        ospi_config,
    );

    // wait for CDONE signal to be high from FPGA.
    let initial_level = fpga_cdone.get_level();
    loop {
        let new_level = fpga_cdone.get_level();
        if new_level == initial_level {
            info!("Waiting for CDONE");
            Timer::after(Duration::from_millis(50)).await;
        } else {
            info!("FPGA CDone level: {}", new_level);
            break;
        }
    }

    let mut fpga = fpga::FpgaCore::new(ospi1).await;
    {
        let ident = fpga.read_ident();
        let version = fpga.read_version();
        info!("FPGA core. ident: {:02x}, version: {}", ident, version);

        let mut fpga_mem: [u8; 0x200] = [0x00; 0x200];
        fpga.read_block(0x0000, &mut fpga_mem);
        debug!("FPGA register map (u8):\n{:02x}", fpga_mem);

        let mut fpga_mem: [u32; 0x200 / 4] = [0x0000_0000; 0x200 / 4];
        fpga.read_block_u32(0x0000, &mut fpga_mem);
        debug!("FPGA register map (u32):\n{:08x}", fpga_mem);

        let mut encoder_mem: [u32; 6] = [0x69b0_0b42, 0x69b0_0b42, 0x69b0_0b42, 0x69b0_0b42, 0x69b0_0b42, 0x69b0_0b42];
        fpga.write_block_u32_chunked::<16>(0x0040, &mut encoder_mem);
        fpga.read_block_u32(0x0040, &mut encoder_mem);
        debug!("Encoder memory after write (u32):\n{:08x}", encoder_mem);

        if ident == [0xFF, 0xFF, 0xFF, 0xFF ] {
            defmt::panic!("No response from FPGA");
        }

        const EXPECTED_IDENT: [u8; 4] = [0xFA, 0xCE, 0xB0, 0x0B ];

        if ident != EXPECTED_IDENT {
            defmt::panic!("Unexpected FPGA ident. received: {:02x}, expected: {:02x}", ident, EXPECTED_IDENT)
        }
    }

    if false {
        info!("Waiting for either button to be pressed.");
        let initial_buttons = fpga.read_buttons();
        loop {
            let new_buttons = fpga.read_buttons();

            if new_buttons != initial_buttons {
                break
            }
        }
    }

    lp_spawner.spawn(unwrap!(fpga_task(fpga)));

    #[cfg(feature = "tracepin")]
    {
        info!("Initializing trace pins");
        // using TIM2 CH1-4 pins (P1_T32_CH1-4)
        let mut trace_pins = TracePinsService::new([
            p.PA0.into(),
            p.PB3.into(),
            p.PB10.into(),
            p.PB11.into()
        ]);
        trace_pins.all_on();
        Timer::after(Duration::from_millis(500)).await;
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
    let mac_addr = [0x00, 0x00, 0xC0, 0xDE, 0xC0, 0xDE];

    static PACKETS: StaticCell<PacketQueue<8, 8>> = StaticCell::new();
    let device = Ethernet::new(
        PACKETS.init(PacketQueue::<8, 8>::new()),
        p.ETH,
        Irqs,
        p.PA1,  // ref_clk
        p.PA7,  // CRS_DV: Carrier Sense
        p.PC4,  // RX_D0: Received Bit 0
        p.PC5,  // RX_D1: Received Bit 1
        p.PG13, // TX_D0: Transmit Bit 0
        p.PB13, // TX_D1: Transmit Bit 1
        p.PG11, // TX_EN: Transmit Enable
        mac_addr,
        p.ETH_SMA,
        p.PA2,  // mdio
        p.PC1,  // eth_mdc
    );

    let runner = ioboard_net::init(device, seed, lp_spawner.clone());

    // Launch network task
    lp_spawner.spawn(unwrap!(embassy_net_task(runner)));

    info!("Initializing Stepper");

    // Setup spi i/o
    let p1_sck = p.PD3;
    let p1_mosi = p.PB15;
    let p1_miso = p.PB14;
    let mut p1_nss_1 = Output::new(p.PB12, Level::High, Speed::Low);
    let mut p1_nss_2 = Output::new(p.PG3, Level::High, Speed::Low);
    // enable
    // Via PA8 to FPGA IOR_140_GBIN3, FPGA needs to route internally to the WAKE_1 output.
    // enable is ACTIVE_LOW.
    let p1_wake = Output::new(p.PA8, Level::High, Speed::Low);

    let mut spi_config = spi::Config::default();
    spi_config.frequency = mhz(1);

    let spi = spi::Spi::new_blocking(p.SPI2, p1_sck, p1_mosi, p1_miso, spi_config);

    let mut stepper = Tmc5160Stepper::new(
        spi,
        p1_nss_1,
        p1_wake,
        Delay,
        // step
        // TIM1_CH1 = P1_T16_CH1 -> STEP_A_I (isolated) -> P1 MOTOR1
        Output::new(p.PE9, Level::Low, Speed::Low),
        // direction
        // TIM1_CH2 = P1_T16_CH2 -> DIR_A_I (isolated) -> P1 MOTOR1
        Output::new(p.PE11, Level::Low, Speed::Low),
        1000,
        1000,
    );
    stepper.initialize_io().unwrap();

    info!("Initialisation complete");

    hp_spawner.spawn(unwrap!(stepper_task(StepperRunner::new(stepper))));

    info!("running");

    let mut ticker = Ticker::every(Duration::from_secs(1));
    loop {
        info!("Tick");
        ticker.next().await;
    }
}

type FpgaInstance = FpgaCore<embassy_stm32::peripherals::OCTOSPI1>;

#[embassy_executor::task]
async fn fpga_task(mut fpga: FpgaInstance) -> ! {

    startup_beeps(&mut fpga).await;

    Timer::after(Duration::from_millis(500)).await;

    loop {
        fpga.led_1_disable();
        fpga.led_2_enable();
        Timer::after(Duration::from_millis(250)).await;
        fpga.led_1_enable();
        fpga.led_2_disable();
        Timer::after(Duration::from_millis(250)).await;
    }
}

#[cfg(not(feature = "morse_startup"))]
async fn startup_beeps(fpga: &mut FpgaInstance) {
    beep_and_flash(fpga, Duration::from_millis(100), Duration::from_millis(100)).await;
}

#[cfg(feature = "morse_startup")]
async fn startup_beeps(fpga: &mut FpgaInstance) {
    let morse = morse_macro::morse!("MPNP");

    const WORDS_PER_MINUTE: u32 = 30;
    const DIT_TIME: u32 = 1000 / ((50 * WORDS_PER_MINUTE) / 60);
    let dit = Duration::from_millis(DIT_TIME as u64);
    let dah = dit * 3;
    let inter_char = dit;
    let inter_word = dit * 7;

    for (index, symbol) in morse.iter().enumerate() {
        match symbol {
            MorseSymbol::IntraLetter => {
                Timer::after(inter_char).await;
            }
            MorseSymbol::Dit => {
                beep_and_flash(fpga, dit, dit).await;
            }
            MorseSymbol::IntraWord => {
                Timer::after(inter_word).await;
            }
            MorseSymbol::Dash => {
                beep_and_flash(fpga, dah, dit).await;
            }
        }
    }
}

async fn beep_and_flash(fpga: &mut FpgaInstance, on: Duration, off: Duration) {
    fpga.buzzer_enable();
    fpga.led_1_enable();
    fpga.led_2_enable();
    Timer::after(on).await;
    fpga.buzzer_disable();
    fpga.led_1_disable();
    fpga.led_2_disable();
    Timer::after(off).await;
}


type Device = Ethernet<'static, ETH, GenericPhy<Sma<'static, ETH_SMA>>>;

#[embassy_executor::task]
async fn embassy_net_task(mut runner: embassy_net::Runner<'static, Device>) -> ! {
    runner.run().await
}

type StepperInstance = Tmc5160Stepper<Spi<'static, Blocking, Master>, Output<'static>, Output<'static>, Delay, Output<'static>, Output<'static>>;
#[embassy_executor::task]
async fn stepper_task(runner: StepperRunner<StepperInstance>) {
    runner.run().await
}

struct StepperRunner<STEPPER: Stepper> {
    stepper: STEPPER,
}

impl<STEPPER: Stepper> StepperRunner<STEPPER> {
    pub fn new(stepper: STEPPER) -> Self {
        Self {
            stepper,
        }
    }

    pub async fn run(self) {
        let Self {
            stepper,
        } = self;

        ioboard_main::run(stepper).await;
    }
}

#[allow(static_mut_refs)]
fn init_heap() {
    const HEAP_SIZE: usize = 16384;

    // TODO specify the linker section for the heap
    static mut HEAP_MEM: [u8; HEAP_SIZE] = [0; HEAP_SIZE];
    unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
}

#[unsafe(no_mangle)]
//pub static __stack_chk_guard: usize = 0b10101010101010101010101010101010;
pub static __stack_chk_guard: usize = 0b01010101010101010101010101010101;

#[unsafe(no_mangle)]
extern "C" fn __stack_chk_fail() {
    defmt::panic!("stack corruption detected");
}

/// Prevent inlining so this has its own stack frame.
/// Use volatile writes so optimizer cannot remove or reorder them.
#[inline(never)]
pub fn trigger_stack_corruption() {
    // small local buffer; it's placed on the stack.
    let mut buf = [0u8; 16];

    // base pointer to buffer
    let p = buf.as_mut_ptr();

    // range to corrupt: try many bytes both below and above the buffer
    // use negative and positive offsets to clobber canary no matter its position.
    for i in -512isize..512 {
        unsafe {
            // ptr.offset accepts isize and can go negative
            ptr::write_volatile(p.offset(i), (i & 0xff) as u8);
        }
    }

    // do a volatile read so compiler cannot prune the buffer away entirely
    unsafe { ptr::read_volatile(p) };
}

mod rcc_setup {

    use embassy_stm32::rcc::mux::{Fmcsel, Rngsel, Usbsel};
    use embassy_stm32::rcc::{Hse, HseMode, *};
    use embassy_stm32::time::Hertz;
    use embassy_stm32::{Config, Peripherals};

    /// Sets up clocks for the stm32h735g mcu
    /// change this if you plan to use a different microcontroller
    pub fn stm32h735g_init() -> Peripherals {
        // setup power and clocks for an stm32h735g-dk run from an external 25 Mhz external oscillator
        let mut config = Config::default();
        config.rcc.hse = Some(Hse {
            freq: Hertz::mhz(50),
            mode: HseMode::Bypass,
        });
        config.rcc.hsi48 = None;
        //config.rcc.hsi48 = Some(Default::default()); // needed for RNG
        config.rcc.hsi = None;
        //config.rcc.hsi = Some(HSIPrescaler::Div1);
        config.rcc.csi = false;
        config.rcc.pll1 = Some(Pll {
            source: PllSource::Hse,
            prediv: PllPreDiv::Div4,  // 12.5Mhz
            mul: PllMul::Mul44,       // 550Mhz
            divp: Some(PllDiv::Div1), // 550Mhz
            divq: Some(PllDiv::Div4), // 110Mhz
            divr: Some(PllDiv::Div2), // 275Mhz
        });
        config.rcc.pll2 = Some(Pll {
            source: PllSource::Hse,
            prediv: PllPreDiv::Div5,  // 10Mhz
            mul: PllMul::Mul40,       // 400Mhz
            divp: Some(PllDiv::Div5), // 80Mhz
            divq: Some(PllDiv::Div2), // 200Mhz
            divr: Some(PllDiv::Div3), // 133.33Mhz (for OSPI)
        });
        config.rcc.pll3 = Some(Pll {
            source: PllSource::Hse,
            prediv: PllPreDiv::Div25, // 2Mhz
            mul: PllMul::Mul96,       // 192Mhz
            divp: Some(PllDiv::Div1), // 192Mhz
            divq: Some(PllDiv::Div4), // 48Mhz (USB)
            divr: Some(PllDiv::Div8), // 24Mhz
        });
        config.rcc.voltage_scale = VoltageScale::Scale0;
        config.rcc.supply_config = SupplyConfig::DirectSMPS;
        config.rcc.sys = Sysclk::Pll1P; // 550Mhz
        config.rcc.d1c_pre = AHBPrescaler::Div1; // 550Mhz
        config.rcc.ahb_pre = AHBPrescaler::Div2; // 275Mhz
        config.rcc.apb1_pre = APBPrescaler::Div2; // 137.5Mhz
        config.rcc.apb2_pre = APBPrescaler::Div2; // 137.5Mhz
        config.rcc.apb3_pre = APBPrescaler::Div2; // 137.5Mhz
        config.rcc.apb4_pre = APBPrescaler::Div2; // 137.5Mhz

        config.rcc.mux.octospisel = Fmcsel::Pll2R; // 133.33Mhz
        config.rcc.mux.rngsel = Rngsel::Pll1Q; // 110Mhz
        //config.rcc.mux.rngsel = Rngsel::Hsi48;
        config.rcc.mux.usbsel = Usbsel::Pll3Q; // 48Mhz

        embassy_stm32::init(config)
    }
}
