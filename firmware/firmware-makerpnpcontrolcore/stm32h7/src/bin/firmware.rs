//! BUILDING:
//! make sure the working directory is the `firmware-makerpnpcontrolcore` directory when running cargo.
//! if not, you'll likely get a compile error in embassy executor as the target from the cargo.toml
//! won't be picked up if you compile from the root directory

#![no_std]
#![no_main]
extern crate alloc;
extern crate firmware_makerpnpcontrolcore;

use core::ptr;

use cortex_m_rt::entry;
use defmt::*;
use embassy_embedded_hal::SetConfig;
use embassy_executor::SendSpawner;
use embassy_executor::{Executor, InterruptExecutor, Spawner};
use embassy_stm32::{dma, i2c, spi, Peri, Peripherals};
use embassy_stm32::eth::{PacketQueue, Sma };
use embassy_stm32::eth::{Ethernet, GenericPhy};
use embassy_stm32::gpio::{Level, Output, Speed, Input, Pull};
use embassy_stm32::interrupt::{InterruptExt, Priority};
use embassy_stm32::peripherals::{ETH, ETH_SMA, PA0_C, PA1_C, PC2_C, PC3_C, PC0, PH2, ADC3, PB8, PB9};
use embassy_stm32::rng::Rng;
use embassy_stm32::ospi::{
    ChipSelectHighTime, FIFOThresholdLevel, MemorySize, MemoryType, WrapSize,
};
use embassy_stm32::{bind_interrupts, eth, interrupt, peripherals, rng};
use embassy_stm32::adc::{Adc, SampleTime};
use embassy_stm32::i2c::{Error, I2c};
use embassy_stm32::mode::{Async, Blocking};
use embassy_stm32::spi::mode::Master;
use embassy_stm32::spi::Spi;
use embassy_stm32::time::mhz;
use embassy_embedded_hal::shared_bus::asynch::i2c::I2cDevice;
use embassy_time::{Delay, Duration, Ticker, Timer};
use embedded_alloc::LlffHeap as Heap;
use fpga_pac::loadcell0::vals::rate;
use ioboard_main::stepper::Stepper;
#[cfg(feature = "tracepin")]
use ioboard_trace::tracepin;
#[cfg(feature = "tracepin")]
use ioboard_trace::tracepin::TracePins;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};
use firmware_makerpnpcontrolcore::adc;
#[cfg(feature = "morse_startup")]
use morse_core::MorseSymbol;
use firmware_makerpnpcontrolcore::fpga::FpgaCore;
use firmware_makerpnpcontrolcore::fpga::loadcell::loadcell::FpgaLoadcell;
use firmware_makerpnpcontrolcore::fpga::ws2812::ws2812::ColorOrdering;
use firmware_makerpnpcontrolcore::fpga::pwm;
use firmware_makerpnpcontrolcore::rgb::rainbow_wave;
use firmware_makerpnpcontrolcore::stepper::bitbash::{GpioBitbashStepper, StepperEnableMode};
use firmware_makerpnpcontrolcore::stepper::fpgastepper::FpgaStepper;
use firmware_makerpnpcontrolcore::stepper::tmc5160::Tmc5160Stepper;
#[cfg(feature = "tracepin")]
use firmware_makerpnpcontrolcore::trace::TracePinsService;

use embassy_sync::mutex::Mutex;
use embassy_sync::blocking_mutex::raw::{RawMutex, NoopRawMutex};
use embedded_hal_async::i2c::I2c as AsyncI2c;
use embedded_hal::i2c::{Error as I2cErrorTrait, ErrorKind, ErrorType, Operation, SevenBitAddress};

//
// Heap/Allocator configuration
//

#[global_allocator]
static HEAP: Heap = Heap::empty();

//
// Embassy configuration
//

bind_interrupts!(struct Irqs {
    ETH => eth::InterruptHandler<ETH>;
    HASH_RNG => rng::InterruptHandler<peripherals::RNG>;
    I2C2_EV => i2c::EventInterruptHandler<peripherals::I2C2>;
    I2C2_ER => i2c::ErrorInterruptHandler<peripherals::I2C2>;
    I2C3_EV => i2c::EventInterruptHandler<peripherals::I2C3>;
    I2C3_ER => i2c::ErrorInterruptHandler<peripherals::I2C3>;
    DMA1_STREAM4 => dma::InterruptHandler<peripherals::DMA1_CH4>;
    DMA1_STREAM5 => dma::InterruptHandler<peripherals::DMA1_CH5>;
    DMA1_STREAM6 => dma::InterruptHandler<peripherals::DMA1_CH6>;
    DMA1_STREAM7 => dma::InterruptHandler<peripherals::DMA1_CH7>;
});

// I2C1 is unused, we will use it's spare interrupt.
#[interrupt]
unsafe fn I2C1_EV() {
    unsafe { EXECUTOR_HIGH.on_interrupt() }
}

static EXECUTOR_HIGH: InterruptExecutor = InterruptExecutor::new();
static EXECUTOR_LOW: StaticCell<Executor> = StaticCell::new();

static I2C2_BUS: StaticCell<Mutex<NoopRawMutex, I2c<'static, Async, i2c::Master>>> = StaticCell::new();
static I2C3_BUS: StaticCell<Mutex<NoopRawMutex, I2c<'static, Async, i2c::Master>>> = StaticCell::new();

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

    // wait for CDONE signal to be low from FPGA.
    loop {
        let level = fpga_cdone.get_level();

        // check checking, and give the FPGA some time to settle.
        Timer::after(Duration::from_millis(50)).await;

        match level {
            Level::Low => {
                info!("CDONE LOW");
                break
            }
            Level::High => {
                info!("Waiting for CDONE LOW");
            }
        }
    }

    info!("Enabling FPGA");
    fpga_creset_b.set_high();

    let mut core_peri = cortex_m::Peripherals::take().unwrap();

    let octospi_size = 256 * 1024 * 1024;

    {
        let mpu = core_peri.MPU;
        let scb = &mut core_peri.SCB;
        let size = octospi_size;
        // Refer to ARM®v7-M Architecture Reference Manual ARM DDI 0403
        // Version E.b Section B3.5
        const MEMFAULTENA: u32 = 1 << 16;

        unsafe {
            /* Make sure outstanding transfers are done */
            cortex_m::asm::dmb();

            scb.shcsr.modify(|r| r & !MEMFAULTENA);

            /* Disable the MPU and clear the control register*/
            mpu.ctrl.write(0);
        }

        const REGION_NUMBER0: u32 = 0x00;
        const REGION_BASE_ADDRESS: u32 = 0x9000_0000;

        const REGION_EXECUTE_NEVER: u32 = 0x01;
        const REGION_FULL_ACCESS: u32 = 0x03;
        const REGION_NON_CACHEABLE: u32 = 0x00;
        const REGION_NON_SHARABLE: u32 = 0x00;
        const REGION_NON_BUFFERABLE: u32 = 0x00;
        const REGION_ENABLE: u32 = 0x01;

        const TEX: u32 = 0x00;

        fn log2minus1(sz: u32) -> u32 {
            for i in 5..=31 {
                if sz == (1 << i) {
                    return i - 1;
                }
            }
            crate::panic!("Unknown memory region size!");
        }

        defmt::info!("OctoSPI region size value: 0x{:x}", log2minus1(size as u32));

        // Configure region 0
        unsafe {
            mpu.rnr.write(REGION_NUMBER0);
            mpu.rbar.write(REGION_BASE_ADDRESS);
            let rasr_value =
                0
                    | (TEX << 19)
                    | (REGION_NON_SHARABLE << 18)
                    | (REGION_NON_CACHEABLE << 17)
                    | (REGION_NON_BUFFERABLE << 16)
                    | (REGION_FULL_ACCESS << 24)
                    | (REGION_EXECUTE_NEVER << 28)
                    | (log2minus1(size as u32) << 1)
                    | REGION_ENABLE;
            defmt::info!("OctoSPI rasr value: 0x{:x}", rasr_value);
            mpu.rasr.write(rasr_value);
        }

        const MPU_ENABLE: u32 = 0x01;
        const MPU_DEFAULT_MMAP_FOR_PRIVILEGED: u32 = 0x04;

        // Enable
        unsafe {
            mpu.ctrl.modify(|r| r | MPU_DEFAULT_MMAP_FOR_PRIVILEGED | MPU_ENABLE);

            scb.shcsr.modify(|r| r | MEMFAULTENA);

            // Ensure MPU settings take effect
            cortex_m::asm::dsb();
            cortex_m::asm::isb();
        }
    }


    let ospi_config = embassy_stm32::ospi::Config {
        fifo_threshold: FIFOThresholdLevel::_16Bytes,
        memory_type: MemoryType::Standard,
        device_size: MemorySize::_2MiB,
        chip_select_high_time: ChipSelectHighTime::_1Cycle,
        free_running_clock: false,
        clock_mode: false,
        wrap_size: WrapSize::None,
        // TODO increase this speed as much as possible
        // clock_prescaler: 5, // 133.33Mhz / (5+1) = 22.22Mhz
        // clock_prescaler: 13, // 133.33Mhz / (13+1) = 9.5Mhz
        clock_prescaler: 132, // 133.33Mhz / (132+1) = 1.0Mhz
        // clock_prescaler: 254, // 133.33Mhz / (254+1) = 0.522Mhz
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

    // wait for CDONE signal to be low from FPGA.
    loop {
        let level = fpga_cdone.get_level();

        // check checking, and give the FPGA some time to settle.
        Timer::after(Duration::from_millis(50)).await;

        match level {
            Level::Low => {
                info!("Waiting for CDONE HIGH");
            }
            Level::High => {
                info!("CDONE HIGH");
                break
            }
        }
    }

    const EXPECTED_IDENT: u32 = 0xFACEB00B;

    let mut fpga = FpgaCore::new(ospi1).await;
    fpga.enable_memory_mapped_mode();

    {
        loop {
            Timer::after(Duration::from_millis(50)).await;

            let ident = fpga.read_ident();
            let version = fpga.read_version();
            info!("FPGA core. ident: {:02x}, version: {}", ident, version);

            if ident == 0xffffffff {
                defmt::error!("No response from FPGA");
                continue;
            }


            if ident == EXPECTED_IDENT {
                break
            }

            defmt::error!("Unexpected FPGA ident. received: {:02x}, expected: {:02x}", ident, EXPECTED_IDENT)
        }
    }

    Timer::after(Duration::from_millis(10)).await;

    fpga.disable_memory_mapped_mode();

    if true {
        let mut fpga_mem: [u8; 0x10000] = [0x00; 0x10000];
        fpga.read_block(0x0000, &mut fpga_mem);
        debug!("FPGA register map (u8):");
        dump_mem_u8_be(&fpga_mem);
    }

    if false {
        let mut fpga_mem: [u32; 0x10000 / 4] = [0x0000_0000; 0x10000 / 4];
        fpga.read_block_u32(0x0000, &mut fpga_mem);
        debug!("FPGA register map (u32):");
        dump_mem_u32(&fpga_mem);

        defmt::assert!(fpga_mem[0x0000] == EXPECTED_IDENT);
    }

    if false {
        let mut encoder_mem: [u32; 6] = [0xdead_beef; 6];

        for _ in 0..10 {
            fpga.read_block_u32(0x0c20, &mut encoder_mem);
            debug!("Encoder values (u32):\n{:08x}", encoder_mem);

            // set encoder values
            encoder_mem = [0x1100_0011, 0x2200_0022, 0x3300_0033, 0x4400_0044, 0x5500_0055, 0x6600_0066];
            fpga.write_block_u32_chunked::<16>(0x0c04, &mut encoder_mem);
            // read encoder values
            fpga.read_block_u32(0x0c20, &mut encoder_mem);
            debug!("Encoder values after explicit set (u32):\n{:08x}", encoder_mem);
        }
    }

    if false {
        for _ in 0..10 {
            let mut ctrl_and_tx_config = [0x0000_0000; 2];

            // 4x WS2812 leds with GRB color order.
            fpga.write_u32(0x0800, 0b00000000000000000000000000000101);
            fpga.write_u32(0x0804, 4);
            // Red, green, blue, white
            fpga.write_block_u32_chunked::<16>(0x0810, &[0x00FF0000, 0x0000FF00, 0x000000FF, 0x00FFFFFF]);
            fpga.read_block_u32(0x0800, &mut ctrl_and_tx_config);
            debug!("CTRL: 0x{:08x}, TX_CONFIG: 0x{:08x}", ctrl_and_tx_config[0], ctrl_and_tx_config[1]);

            // External LED strip - 32 leds
            fpga.write_u32(0x900, 0b00000000000000000000000000000101);
            fpga.write_u32(0x904, 32);
            fpga.write_block_u32_chunked::<16>(0x190, &[
                0x00FF0000, 0x0000FF00, 0x000000FF, 0x00FFFFFF,
                0x00FF0000, 0x0000FF00, 0x000000FF, 0x00FFFFFF,

                0x00FF0000, 0x0000FF00, 0x000000FF, 0x00FFFFFF,
                0x00FF0000, 0x0000FF00, 0x000000FF, 0x00FFFFFF,

                0x00FF0000, 0x0000FF00, 0x000000FF, 0x00FFFFFF,
                0x00FF0000, 0x0000FF00, 0x000000FF, 0x00FFFFFF,

                0x00FF0000, 0x0000FF00, 0x000000FF, 0x00FFFFFF,
                0x00FF0000, 0x0000FF00, 0x000000FF, 0x00FFFFFF,
            ]);
            fpga.read_block_u32(0x900, &mut ctrl_and_tx_config);
            debug!("CTRL: 0x{:08x}, TX_CONFIG: 0x{:08x}", ctrl_and_tx_config[0], ctrl_and_tx_config[1]);
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
            Timer::after(Duration::from_millis(100)).await;
        }
    }

    fpga.enable_memory_mapped_mode();

    if true {
        fpga.dump_registers();
    }
    if true {
        fpga.buzzer_enable();
        Timer::after(Duration::from_millis(250)).await;
        fpga.buzzer_disable();
        Timer::after(Duration::from_millis(250)).await;
        fpga.buzzer_enable();
        Timer::after(Duration::from_millis(250)).await;
        fpga.buzzer_disable();
        Timer::after(Duration::from_millis(250)).await;
    }

    //
    // Detection circuits
    //
    let base_present = fpga.base_present();
    if !base_present {
        // enable the buzzer output, however, if the base board is not present, the buzzer will not
        // be heard. if the base board is badly connected the user will hear the buzzer tone and
        // be able to identify and resolve the issue.
        fpga.buzzer_enable();
        Timer::after(Duration::from_millis(5000)).await;
        fpga.buzzer_disable();

        // FUTURE add some LED diagnostic blink codes too

        defmt::panic!("Base board not detected");
    } else {
        info!("Base board detected");
    }

    let port_present = fpga.port_present();
    for i in 0..4 {
        info!("Port {} present: {}", i, port_present & (1 << i));
    }

    if true {
        info!("Waiting for either button to be pressed.");
        let initial_buttons = fpga.read_buttons_mm();
        loop {
            let new_buttons = fpga.read_buttons_mm();

            if new_buttons != initial_buttons {
                break
            }
            Timer::after(Duration::from_millis(100)).await;
        }
    }

    if true {
        // estop is pulled to 3V3 when it is connected, but not activated.
        // when it is pressed it will be pulled to GND.  We pull to GND by default.
        // so that a valid signal can be read when the base board is not connected properly which
        // results in the same condition as if the ESTOP switch was pressed.
        let estop = Input::new(p.PG4, Pull::Down);

        info!("Waiting for ESTOP be be released (or re-connected).");
        loop {
            if estop.is_low() {
                break;
            }
            Timer::after(Duration::from_millis(100)).await;
        }
    }

    if true {
        let mut encoder_mem: [u16; 6] = [0xc0de; 6];

        fpga.reset_encoders();
        fpga.read_encoders(&mut encoder_mem);
        debug!("Encoder values (u32):\n{:08x}", encoder_mem);

        // set encoder values
        encoder_mem = [0x0011, 0x0022, 0x0033, 0x0044, 0x0055, 0x0066];
        fpga.set_encoders(&mut encoder_mem);
        // read encoder values
        fpga.read_encoders(&mut encoder_mem);
        debug!("Encoder values after explicit set (u32):\n{:08x}", encoder_mem);
    }

    let fpga_adc_mux = fpga.adc_mux();

    let fpga_stepper_bank_0 = fpga.stepper_bank(0);
    let _fpga_stepper_bank_1 = fpga.stepper_bank(1);

    let loadcell = fpga.loadcell();

    const PWM_FREQ: u32 = 25_000;
    const DUTY_PERCENT_MIN: u8 = 0;
    const DUTY_PERCENT_MAX: u8 = 100;

    let plan = pwm::try_resolve(&[
        // blow pump
        (pwm::PM_OUT1, PWM_FREQ, DUTY_PERCENT_MIN, DUTY_PERCENT_MAX, pwm::POLARITY_NORMAL),
        // vacuum pump
        (pwm::PM_OUT2, PWM_FREQ, DUTY_PERCENT_MIN, DUTY_PERCENT_MAX, pwm::POLARITY_NORMAL),
        // N/C
        (pwm::PM_OUT3, PWM_FREQ, DUTY_PERCENT_MIN, DUTY_PERCENT_MAX, pwm::POLARITY_NORMAL),
        // N/C
        (pwm::PM_OUT4, PWM_FREQ, DUTY_PERCENT_MIN, DUTY_PERCENT_MAX, pwm::POLARITY_NORMAL),
        // drag pin solenoid
        (pwm::OT_OUT1, PWM_FREQ, DUTY_PERCENT_MIN, DUTY_PERCENT_MAX, pwm::POLARITY_NORMAL),
        // head down-light (optional)
        (pwm::OT_OUT2, PWM_FREQ, DUTY_PERCENT_MIN, DUTY_PERCENT_MAX, pwm::POLARITY_NORMAL),
        // Vacuum switch 1
        (pwm::OT_OUT3, PWM_FREQ, DUTY_PERCENT_MIN, DUTY_PERCENT_MAX, pwm::POLARITY_NORMAL),
        // Vacuum switch 2
        (pwm::OT_OUT4, PWM_FREQ, DUTY_PERCENT_MIN, DUTY_PERCENT_MAX, pwm::POLARITY_NORMAL),
        // Work light (on gantry)
        (pwm::OT_OUT5, PWM_FREQ, DUTY_PERCENT_MIN, DUTY_PERCENT_MAX, pwm::POLARITY_NORMAL),
        // Up-light (around up-camera)
        (pwm::OT_OUT6, PWM_FREQ, DUTY_PERCENT_MIN, DUTY_PERCENT_MAX, pwm::POLARITY_NORMAL),
        // N/C
        (pwm::OT_OUT7, PWM_FREQ, DUTY_PERCENT_MIN, DUTY_PERCENT_MAX, pwm::POLARITY_NORMAL),
        // N/C
        (pwm::OT_OUT8, PWM_FREQ, DUTY_PERCENT_MIN, DUTY_PERCENT_MAX, pwm::POLARITY_NORMAL),
    ]).unwrap();

    let mut bank = pwm::allocate(&plan);

    bank.enable_outputs();

    bank[0].set_duty(0).unwrap();
    bank[0].start();
    bank[1].set_duty(0).unwrap();
    bank[1].start();
    bank[2].set_duty(0).unwrap();
    bank[2].start();
    bank[3].set_duty(0).unwrap();
    bank[3].start();

    bank[4].set_duty(0).unwrap();
    bank[4].start();
    bank[5].set_duty(100).unwrap();
    bank[5].start();
    bank[6].set_duty(0).unwrap();
    bank[6].start();
    bank[7].set_duty(0).unwrap();
    bank[7].start();
    bank[8].set_duty(100).unwrap();
    bank[8].start();
    bank[9].set_duty(100).unwrap();
    bank[9].start();
    bank[10].set_duty(0).unwrap();
    bank[10].start();
    bank[11].set_duty(0).unwrap();
    bank[11].start();

    bank.start_synced();

    lp_spawner.spawn(unwrap!(fpga_task(fpga)));

    let adc1 = Adc::new(p.ADC1);
    let adc3 = Adc::new(p.ADC3);


    let adc_mux = adc::Mux::new(
        // adc mux inputs
        fpga_adc_mux,
        adc1,
        p.PA0_C, // PA0_C MUX input 1, ADC1_INP0
        p.PA1_C, // PA1_C MUX input 2, ADC1_INP1
        embassy_stm32::adc::SampleTime::Cycles325,
    );

    lp_spawner.spawn(unwrap!(adc_task(
        adc_mux,

        // other adc inputs
        adc3,
        p.PC2_C, // PC2_C AI3, ADC3_INP0, EXT_SENSE_1
        p.PC3_C, // PC3_C AI4, ADC3_INP1, EXT_SENSE_2
        p.PC0, // PC0 AIN5, ADC3_INP10, VAC_SENSE_1 (5V tolerant)
        p.PH2, // PH2 AIN6, ADC3_INP13, VAC_SENSE_2 (5V tolerant)
    )));

    if true {
        lp_spawner.spawn(unwrap!(loadcell_task(loadcell)));
    }

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
    
    let serial_number = ioboard_net::SerialNumber(*embassy_stm32::uid::uid());
    info!("Serial number: {}", serial_number);

    info!("Initializing ETH");
    let mac_addr = mac_address_from_uid(embassy_stm32::uid::uid());
    info!("MAC address: {=[u8]:02X}", mac_addr);

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

    let runner = ioboard_net::init(device, seed, serial_number, lp_spawner.clone());

    // Launch network task
    lp_spawner.spawn(unwrap!(embassy_net_task(runner)));

    if false {
        info!("Initializing Port Stepper");

        // Setup spi i/o
        let p1_sck = p.PD3;
        let p1_mosi = p.PB15;
        let p1_miso = p.PB14;
        let p1_nss_1 = Output::new(p.PB12, Level::High, Speed::Low);
        let _p1_nss_2 = Output::new(p.PG3, Level::High, Speed::Low);
        // enable
        // Via PA8 to FPGA IOR_140_GBIN3, FPGA needs to route internally to the WAKE_1 output.
        // enable is ACTIVE_LOW.
        let p1_wake = Output::new(p.PA8, Level::High, Speed::Low);

        let mut spi_config = spi::Config::default();
        spi_config.frequency = mhz(1);

        let spi = spi::Spi::new_blocking(p.SPI2, p1_sck, p1_mosi, p1_miso, spi_config);

        let mut port_stepper = Tmc5160Stepper::new(
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
        port_stepper.initialize_io().unwrap();
        hp_spawner.spawn(unwrap!(port_stepper_task(StepperRunner::new(port_stepper))));
    }

    if false {
        info!("Initializing Bitbash Stepper");
        let mut bitbash_stepper = GpioBitbashStepper::new(
            // enable
            // TIM8_ETR
            Output::new(p.PG12, Level::High, Speed::Low),
            // step
            // TIM8_CH1 = P3_T16_CH1
            Output::new(p.PE14, Level::Low, Speed::Low),
            // direction
            // TIM8_CH2 = P3_T16_CH2
            Output::new(p.PE15, Level::Low, Speed::Low),
            StepperEnableMode::ActiveHigh,
            1000,
            1000,
        );
        bitbash_stepper.initialize_io().unwrap();

        hp_spawner.spawn(unwrap!(bitbash_stepper_task(StepperRunner::new(bitbash_stepper))));
    }

    if false {
        info!("Initializing Fpga Stepper");
        let fpga_stepper = FpgaStepper::new(
            fpga_stepper_bank_0,
            0,
            1000,
            1000,
        );

        hp_spawner.spawn(unwrap!(fpga_stepper_task(StepperRunner::new(fpga_stepper))));
    }

    const BUS_SWITCH_ADDRESS_BASE: u8 = 0x70;

    let i2c2 = I2c::new(p.I2C2, p.PH4, p.PH5, p.DMA1_CH4, p.DMA1_CH5, Irqs, Default::default());
    let i2c2_bus =Mutex::new(i2c2);
    let i2c2_bus = I2C2_BUS.init(i2c2_bus);

    let i2c2_dev1 = I2cDevice::new(i2c2_bus);
    let i2c2_bus_switch1 = BusSwitch::new(i2c2_bus, BUS_SWITCH_ADDRESS_BASE);

    let i2c3 = I2c::new(p.I2C3, p.PH7, p.PH8, p.DMA1_CH6, p.DMA1_CH7, Irqs, Default::default());
    let i2c3_bus = Mutex::new(i2c3);
    let i2c3_bus = I2C3_BUS.init(i2c3_bus);

    static I2C3_SWITCH1: StaticCell<BusSwitch<NoopRawMutex, I2c<'static, Async, i2c::Master>>> = StaticCell::new();

    let i2c3_bus_switch1 = BusSwitch::new(i2c3_bus, BUS_SWITCH_ADDRESS_BASE);
    let i2c3_bus_switch1 = I2C3_SWITCH1.init(i2c3_bus_switch1);
    
    let i2c3_switch_dev1 = i2c3_bus_switch1.add_device(0); // vacuum sensor
    let i2c3_switch_dev2 = i2c3_bus_switch1.add_device(1); // blow sensor

    // 65<P≤131 = K = 64
    const K_100KPA: f32 = 64.0;
    lp_spawner.spawn(unwrap!(pressure_task(i2c3_switch_dev1, "VAC", K_100KPA)));
    lp_spawner.spawn(unwrap!(pressure_task(i2c3_switch_dev2, "BLOW", K_100KPA)));

    info!("Initialisation complete");

    info!("running");

    let mut ticker = Ticker::every(Duration::from_secs(1));
    loop {
        info!("Tick");
        ticker.next().await;
    }
}

type PRESSURE_DEVICE = BusSwitchedDevice<'static, NoopRawMutex, I2c<'static, Async, i2c::Master>>;

#[embassy_executor::task(pool_size = 2)]
async fn pressure_task(mut i2c_dev: PRESSURE_DEVICE, label: &'static str, k: f32) {
    let mut ticker = Ticker::every(Duration::from_secs(1));
    loop {
        ticker.next().await;

        let outbuf = [0x30, 0x0a];
        if let Err(_e) = i2c_dev.write(0x6d, &outbuf).await {
            error!("[{}] Failed to send command to sensor. Retrying.", label);
            continue;
        }

        let mut ready = false;
        let mut attempts = 0;
        loop {
            let mut inbuf = [0u8; 1];
            let register_addr = [0x30; 1];
            if let Ok(_) = i2c_dev.write_read(0x6d, &register_addr, &mut inbuf).await {
                let sco = inbuf[0] & (1 << 3) != 0;
                if !sco {
                    ready = true;
                    break;
                }
            } else {
                error!("[{}] Failed to read register from sensor. Retrying ({} of 10).", label, attempts);
                attempts += 1;
                if attempts > 10 {
                    break;
                }
            }
            Timer::after(Duration::from_millis(20)).await;
        }

        if !ready {
            continue;
        }

        let mut inbuf = [0u8; 5];
        let outbuf = [0x06; 1];
        match i2c_dev.write_read(0x6d, &outbuf, &mut inbuf).await {
            Ok(_) => {
                // inbuf[0]=DATA_MSB(0x06), inbuf[1]=DATA_CSB(0x07), inbuf[2]=DATA_LSB(0x08)
                let praw: u32 = ((inbuf[0] as u32) << 16) | ((inbuf[1] as u32) << 8) | (inbuf[2] as u32);
                let pressure_adc: i32 = ((praw << 8) as i32) >> 8; // sign-extend 24-bit -> i32
                let pressure_pa: f32 = pressure_adc as f32 / k;

                // inbuf[3]=TEMP_MSB(0x09), inbuf[4]=TEMP_LSB(0x0A)
                let traw: u16 = ((inbuf[3] as u16) << 8) | (inbuf[4] as u16);
                let temp_adc: i16 = traw as i16;
                let temperature_c: f32 = temp_adc as f32 / 256.0;

                info!("[{}] inbuf (pressure/temp): {:?}", label, inbuf);
                info!("[{}] Pressure: {} Pa", label, pressure_pa);
                info!("[{}] Temperature: {} C", label, temperature_c);
            }
            Err(_e) => {
                error!("[{}] Failed to read register from sensor.", label);
            }
        }
    }
}

use core::sync::atomic::{AtomicU8, Ordering};

const NO_CHANNEL: u8 = 0xFF; // sentinel: valid channels are 0..=7 on an 8-way mux

pub struct BusSwitch<'a, M: RawMutex, BUS> {
    bus: &'a Mutex<M, BUS>,
    address: u8,
    last_channel: AtomicU8,
}

impl<'a, M: RawMutex, BUS> BusSwitch<'a, M, BUS>
where
    BUS: AsyncI2c,
{
    pub fn new(bus: &'a Mutex<M, BUS>, address: u8) -> Self {
        Self { bus, address, last_channel: AtomicU8::new(NO_CHANNEL) }
    }

    /// Get a device handle bound to a specific channel on this switch.
    /// Multiple devices can be created from the same `BusSwitch` and will
    /// correctly coordinate which channel is currently selected.
    pub fn add_device(&'a self, channel: u8) -> BusSwitchedDevice<'a, M, BUS> {
        BusSwitchedDevice::new(self, channel)
    }

    async fn with_channel<F, R>(
        &self,
        channel: u8,
        f: F,
    ) -> Result<R, BusSwitchError<BUS::Error>>
    where
        F: AsyncFnOnce(&mut BUS) -> R,
    {
        let mut bus = self.bus.lock().await;

        let should_switch = self.last_channel.load(Ordering::SeqCst) != channel;

        if should_switch {
            let mut out = [0u8; 1];
            let mut verify = [0u8; 1];
            out[0] = 1 << channel;

            info!("Switching. Channel: {}", channel);

            bus.write(self.address, &out).await.map_err(BusSwitchError::DeviceError)?;
            bus.read(self.address, &mut verify).await.map_err(BusSwitchError::DeviceError)?;

            if out[0] != verify[0] {
                error!(
                    "Bus switch verification failed. control: {:08b}, expected: {:08b}",
                    verify[0],
                    out[0]
                );
                return Err(BusSwitchError::UnableToSwitch)
            }

            debug!("Bus switch verified. control: {:08b}", verify[0]);
            self.last_channel.store(channel, Ordering::SeqCst);
        }

        Ok(f(&mut bus).await)
    }
}

#[derive(Debug)]
pub enum BusSwitchError<E> {
    UnableToSwitch,
    DeviceError(E),
}

impl<E: I2cErrorTrait> I2cErrorTrait for BusSwitchError<E> {
    fn kind(&self) -> ErrorKind {
        match self {
            BusSwitchError::UnableToSwitch => ErrorKind::Other,
            BusSwitchError::DeviceError(e) => e.kind(),
        }
    }
}

pub struct BusSwitchedDevice<'a, M: RawMutex, BUS> {
    switch: &'a BusSwitch<'a, M, BUS>,
    channel: u8,
}

impl<'a, M: RawMutex, BUS> BusSwitchedDevice<'a, M, BUS> {
    fn new(switch: &'a BusSwitch<'a, M, BUS>, channel: u8) -> Self {
        Self { switch, channel }
    }
}

impl<'a, M: RawMutex, BUS> ErrorType for BusSwitchedDevice<'a, M, BUS>
where
    BUS: AsyncI2c,
{
    type Error = BusSwitchError<BUS::Error>;
}

impl<'a, M: RawMutex, BUS> AsyncI2c for BusSwitchedDevice<'a, M, BUS>
where
    BUS: AsyncI2c,
{
    async fn read(&mut self, addr: u8, buf: &mut [u8]) -> Result<(), Self::Error> {
        self.switch
            .with_channel(self.channel, async |bus| bus.read(addr, buf).await)
            .await?
            .map_err(BusSwitchError::DeviceError)
    }

    async fn write(&mut self, addr: u8, bytes: &[u8]) -> Result<(), Self::Error> {
        self.switch
            .with_channel(self.channel, async |bus| bus.write(addr, bytes).await)
            .await?
            .map_err(BusSwitchError::DeviceError)
    }

    async fn write_read(
        &mut self,
        addr: u8,
        bytes: &[u8],
        buf: &mut [u8],
    ) -> Result<(), Self::Error> {
        self.switch
            .with_channel(self.channel, async |bus| bus.write_read(addr, bytes, buf).await)
            .await?
            .map_err(BusSwitchError::DeviceError)
    }

    async fn transaction(
        &mut self,
        address: SevenBitAddress,
        operations: &mut [Operation<'_>],
    ) -> Result<(), Self::Error> {
        self.switch
            .with_channel(self.channel, async |bus| bus.transaction(address, operations).await)
            .await?
            .map_err(BusSwitchError::DeviceError)
    }
}

#[embassy_executor::task]
async fn loadcell_task(
    mut loadcell: FpgaLoadcell,
) -> ! {
    let mut ticker = Ticker::every(Duration::from_millis(50));
    loadcell.set_rate(rate::RATE_20HZ);

    loadcell.start_continuous();
    loop {
        // wait before reading to allow conversion to complete
        ticker.next().await;
        match loadcell.read() {
            Ok((value, sequence)) => {
                info!("Loadcell value: {}, sequence: {}", value, sequence);
            }
            Err(e) => {
                error!("Loadcell read failed. error: {:?}", e);
            }
        }
    }
}

type AdcMuxInstance = adc::Mux<
    'static,
    embassy_stm32::peripherals::ADC1,
    Peri<'static, PA0_C>,
    Peri<'static, PA1_C>
>;

#[embassy_executor::task]
async fn adc_task(
    mut adc_mux: AdcMuxInstance,
    mut adc: Adc<'static, ADC3>,
    mut ext1_in: Peri<'static, PC2_C>,
    mut ext2_in: Peri<'static, PC3_C>,
    mut vac1_in: Peri<'static, PC0>,
    mut vac2_in: Peri<'static, PH2>,
) -> ! {
    let mut ticker = Ticker::every(Duration::from_secs(1));
    loop {
        for port in 0..4 {
            adc_mux.select_port(port);

            // allow some time for settling before reading
            Timer::after(Duration::from_millis(10)).await;

            let values = adc_mux.read_pair();
            defmt::info!("ADC inputs. port: {}, values: {:?})", port, values);
        }

        let ext = (
            adc.blocking_read(&mut ext1_in, SampleTime::Cycles325),
            adc.blocking_read(&mut ext2_in, SampleTime::Cycles325),
        );
        defmt::info!("ADC ext inputs. values: {:?})", ext);

        let vac = (
            adc.blocking_read(&mut vac1_in, SampleTime::Cycles325),
            adc.blocking_read(&mut vac2_in, SampleTime::Cycles325),
        );
        defmt::info!("ADC vac inputs. values: {:?})", vac);

        ticker.next().await;
    }
}


type FpgaInstance = FpgaCore<embassy_stm32::peripherals::OCTOSPI1>;

#[embassy_executor::task]
async fn fpga_task(mut fpga: FpgaInstance) -> ! {

    let mut encoders: [u16; 6] = [0x0000; 6];

    startup_beeps(&mut fpga).await;

    Timer::after(Duration::from_millis(500)).await;

    fpga.reset_encoders();

    // 4x WS2812 leds with GRB color order.
    let mut led_controller_0 = fpga.led_controller_0()
        .with_led_count(4)
        .with_mode(ColorOrdering::GRB)
        .enable();

    let mut port_rgb_leds = [0x00FF0000, 0x0000FF00, 0x000000FF, 0x00FFFFFF];
    led_controller_0.update_leds(&port_rgb_leds);

    // External LED strip - 32 leds
    let mut led_controller_1 = fpga.led_controller_1()
        .with_led_count(32)
        .with_mode(ColorOrdering::GRB)
        .enable();

    let mut external_leds = [
        0x00FF0000, 0x0000FF00, 0x000000FF, 0x00FFFFFF,
        0x00FF0000, 0x0000FF00, 0x000000FF, 0x00FFFFFF,

        0x00FF0000, 0x0000FF00, 0x000000FF, 0x00FFFFFF,
        0x00FF0000, 0x0000FF00, 0x000000FF, 0x00FFFFFF,

        0x00FF0000, 0x0000FF00, 0x000000FF, 0x00FFFFFF,
        0x00FF0000, 0x0000FF00, 0x000000FF, 0x00FFFFFF,

        0x00FF0000, 0x0000FF00, 0x000000FF, 0x00FFFFFF,
        0x00FF0000, 0x0000FF00, 0x000000FF, 0x00FFFFFF,
    ];
    led_controller_1.update_leds(&external_leds);

    let mut led_frame_counter = 0;
    let mut toggle = true;
    loop {
        if led_frame_counter % 50 == 0 {
            if toggle {
                fpga.led_1_disable();
                fpga.led_2_enable();
                fpga.oec1_disable();
                fpga.oec2_enable();
            } else {
                fpga.led_1_enable();
                fpga.led_2_disable();
                fpga.oec1_enable();
                fpga.oec2_disable();
            }
            toggle = !toggle;
        }

        if led_frame_counter % 100 == 0 {
            fpga.read_encoders(&mut encoders);
            debug!("Encoder values (u16):\n{:04x}", encoders);

            let iak = fpga.read_iak();
            debug!("IAK: 0b{:02b}", iak);

            let din = fpga.read_din();
            debug!("DIN: 0b{:08b}", din);
        }

        rainbow_wave(&mut port_rgb_leds, led_frame_counter);
        led_controller_0.update_leds(&port_rgb_leds);

        rainbow_wave(&mut external_leds, led_frame_counter);
        led_controller_1.update_leds(&external_leds);

        led_frame_counter = led_frame_counter.wrapping_add(5);
        Timer::after(Duration::from_millis(100)).await;
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

    for symbol in morse.iter() {
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

/// Derive a stable, locally-administered, unicast MAC address from the MCU's unique device ID, so that multiple boards
/// can be on the same network.
fn mac_address_from_uid(uid: &[u8; 12]) -> [u8; 6] {
    // FNV-1a, 64-bit
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in uid {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    let hash = hash.to_le_bytes();

    // first octet: locally administered (bit 1 set), unicast (bit 0 clear)
    [0x02, hash[0], hash[1], hash[2], hash[3], hash[4]]
}

type Device = Ethernet<'static, ETH, GenericPhy<Sma<'static, ETH_SMA>>>;

#[embassy_executor::task]
async fn embassy_net_task(mut runner: embassy_net::Runner<'static, Device>) -> ! {
    runner.run().await
}

type PortStepperInstance = Tmc5160Stepper<Spi<'static, Blocking, Master>, Output<'static>, Output<'static>, Delay, Output<'static>, Output<'static>>;
#[embassy_executor::task]
async fn port_stepper_task(runner: StepperRunner<PortStepperInstance>) {
    runner.run().await
}

type BitBashStepperInstance = GpioBitbashStepper<Output<'static>, Output<'static>, Output<'static>>;
#[embassy_executor::task]
async fn bitbash_stepper_task(runner: StepperRunner<BitBashStepperInstance>) {
    runner.run().await
}

type FpgaStepperInstance = FpgaStepper;
#[embassy_executor::task]
async fn fpga_stepper_task(runner: StepperRunner<FpgaStepperInstance>) {
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

        ioboard_main::tasks::stepper_test::run(stepper).await;
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

    use embassy_stm32::rcc::mux::{Adcsel, Fmcsel, Rngsel, Usbsel};
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
        config.rcc.mux.adcsel = Adcsel::Pll2P; // 80Mhz

        embassy_stm32::init(config)
    }
}

pub fn dump_mem_u8_be(mem: &[u8]) {
    for row in (0..mem.len()).step_by(0x10 * 4) {
        let mut row_values: [u32; 0x10] = [0x00000000; 0x10];
        for col in 0..0x10 {
            let offset: usize = row + (col * 4);
            if offset < mem.len() {
                row_values[col] =
                    (mem[offset] as u32) << 24 |
                    (mem[offset+1] as u32) << 16 |
                    (mem[offset+2] as u32) << 8 |
                    (mem[offset+3] as u32) << 0;
            }
        }
        defmt::info!("{:04x}: {:08x}", row, row_values);
    }
}

pub fn dump_mem_u32(mem: &[u32]) {
    for row in (0..mem.len()).step_by(0x10) {
        let mut row_values: [u32; 0x10] = [0x00; 0x10];
        for col in 0..0x10_usize {
            let offset: usize = row * 0x0F + col;
            if offset < mem.len() {
                row_values[col] = mem[offset];
            }
        }
        defmt::info!("{:04x}: {:08x}", row * 4, row_values);
    }
}
