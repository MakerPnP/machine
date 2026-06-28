use byteorder::{BigEndian, ByteOrder};
use embassy_stm32::mode::Blocking;
use embassy_stm32::ospi::{
    AddressSize, Instance, Ospi, OspiWidth,
    TransferConfig, 
};
use embassy_stm32::ospi::enums::DummyCycles;
use defmt::*;

mod commands {
    pub const CMD_READ_U32_BE: u8 = 0x10;
    pub const CMD_READ_U32_LE: u8 = 0x11;
    pub const CMD_WRITE_U32_BE: u8 = 0x90;
    pub const CMD_WRITE_U32_LE: u8 = 0x91;
}
pub use commands::*;


mod registers {
    pub const REG_LED_CTRL: u16 = 0x0040;
    pub const REG_IO_IN_1: u16 = 0x0084;
    pub const REG_BUZZER_CTRL: u16 = 0x00C0;
}
pub use registers::*;
use crate::fpga::adc::FpgaAdcMux;
use crate::fpga::ws2812::Ws2812LedControllerBuilder;

pub struct FpgaCore<I: Instance> {
    ospi: Ospi<'static, I, Blocking>,
    memory_mapped_mode_enabled: bool,
}

impl<I: Instance> FpgaCore<I> {
    pub async fn new(ospi: Ospi<'static, I, Blocking>) -> Self {
        let memory = Self {
            ospi,
            memory_mapped_mode_enabled: false,
        };

        memory
    }

    pub fn read_ident(&mut self) -> u32 {
        defmt::assert!(self.memory_mapped_mode_enabled);

        fpga_pac::SYSTEM0.ident().read().0
    }

    pub fn read_version(&mut self) -> FpgaVersion {
        defmt::assert!(self.memory_mapped_mode_enabled);

        FpgaVersion::from_u32(fpga_pac::SYSTEM0.version().read().0)
    }

    pub fn read_buttons(&mut self) -> u8 {
        defmt::assert!(!self.memory_mapped_mode_enabled);


        let value = self.read_u32(REG_IO_IN_1);

        value as u8 & 0x03
    }

    /// Returns a bitfield of the FPGA buttons.
    /// bit 0 = USER 0 button
    /// bit 1 = USER 1 button
    /// 1 indicates pressed
    pub fn read_buttons_mm(&mut self) -> u8 {
        defmt::assert!(self.memory_mapped_mode_enabled);

        let value = fpga_pac::IO.io_in_1().read();

        let buttons = (value.user0() as u8) | ((value.user1() as u8) << 1);
        defmt::debug!("FPGA value: 0x{:08x}, buttons: 0b{:02b}", value.0, buttons);

        buttons
    }

    /// Returns a bitfield of the FPGA IAK inputs.
    /// bit 0 = IAK1
    /// bit 1 = IAK2
    /// 1 indicates active-low (inverted)
    pub fn read_iak(&mut self) -> u8 {
        defmt::assert!(self.memory_mapped_mode_enabled);

        let value = fpga_pac::IO.io_in_1().read();

        let iak = (value.iak1() as u8) | ((value.iak2() as u8) << 1);
        defmt::debug!("FPGA value: 0x{:08x}, iak: 0b{:02b}", value.0, iak);

        iak
    }

    /// bits 0-7 = DIN1-8
    /// 1 indicates active-high (non-inverted)
    pub fn read_din(&mut self) -> u8 {
        defmt::assert!(self.memory_mapped_mode_enabled);

        let value = fpga_pac::IO.io_in_2().read();

        let din = value.din();
        defmt::debug!("FPGA value: 0x{:08x}, din: 0b{:08b}", value.0, din);

        din
    }

    pub fn read_block(&mut self, address: u16, buffer: &mut [u8]) {
        defmt::assert!(!self.memory_mapped_mode_enabled);

        let transaction: TransferConfig = TransferConfig {
            instruction: Some(CMD_READ_U32_BE as u32),
            isize: AddressSize::_8Bit,
            iwidth: OspiWidth::QUAD,

            address: Some(address as u32),
            adsize: AddressSize::_16Bit,
            adwidth: OspiWidth::QUAD,

            dummy: DummyCycles::_8,

            dwidth: OspiWidth::QUAD,
            ..Default::default()
        };
        self.ospi.blocking_read(buffer, transaction).unwrap();
        defmt::trace!("FPGA block read. address: 0x{:04x}, length: 0x{:04x} data: \n{:02x}", address, buffer.len(), buffer);
    }

    pub fn read_block_u32(&mut self, address: u16, buffer: &mut [u32]) {
        defmt::assert!(!self.memory_mapped_mode_enabled);

        let byte_len = buffer.len() * 4;

        let byte_buf: &mut [u8] = unsafe {
            core::slice::from_raw_parts_mut(buffer.as_mut_ptr() as *mut u8, byte_len)
        };

        self.read_block(address, byte_buf);

        for (out, chunk) in buffer.iter_mut().zip(byte_buf.chunks_exact(4)) {
            *out = BigEndian::read_u32(chunk);
        }
    }

    pub fn read_u32(&mut self, address: u16) -> u32 {
        defmt::assert!(!self.memory_mapped_mode_enabled);

        let mut buffer = [0; 4];
        let transaction: TransferConfig = TransferConfig {
            instruction: Some(CMD_READ_U32_BE as u32),
            isize: AddressSize::_8Bit,
            iwidth: OspiWidth::QUAD,

            address: Some(address as u32),
            adsize: AddressSize::_16Bit,
            adwidth: OspiWidth::QUAD,

            dummy: DummyCycles::_8,

            dwidth: OspiWidth::QUAD,
            ..Default::default()
        };
        self.ospi.blocking_read(&mut buffer, transaction).unwrap();
        let value = BigEndian::read_u32(&buffer);
        trace!("FPGA read_u32. address: 0x{:04x}, length: 0x{:04x} value: {:02x}", address, buffer.len(), buffer);

        value
    }


    /// Writes a sequence of u32s to the FPGA.
    /// The buffer must be aligned to a multiple of 4 bytes.
    /// The bytes are sent over the wire in big-endian order.
    pub fn write_block(&mut self, address: u16, buffer: &[u32]) {
        defmt::assert!(!self.memory_mapped_mode_enabled);

        trace!("FPGA block write. address: 0x{:04x}, length: 0x{:04x} data: \n{:02x}", address, buffer.len(), buffer);
        let transaction: TransferConfig = TransferConfig {
            instruction: Some(CMD_WRITE_U32_BE as u32),
            isize: AddressSize::_8Bit,
            iwidth: OspiWidth::QUAD,

            address: Some(address as u32),
            adsize: AddressSize::_16Bit,
            adwidth: OspiWidth::QUAD,

            dummy: DummyCycles::_0,

            dwidth: OspiWidth::QUAD,
            ..Default::default()
        };
        self.ospi.blocking_write(buffer, transaction).unwrap();
    }

    /// Writes a sequence of u32s to the FPGA.
    /// The buffer must be aligned to a multiple of 4 bytes.
    /// The bytes are sent in chunks of CHUNK_SIZE bytes, one transaction per chunk
    /// The bytes are sent over the wire in big-endian order.
    pub fn write_block_u32_chunked<const CHUNK_SIZE: usize>(&mut self, mut address: u16, buffer: &[u32]) {
        defmt::assert!(!self.memory_mapped_mode_enabled);

        let mut chunk_buf = [0u8; CHUNK_SIZE]; // tune to FIFO / DMA burst size

        let mut i = 0;

        while i < buffer.len() {
            let chunk_words = core::cmp::min(buffer.len() - i, chunk_buf.len() / 4);
            let byte_len = chunk_words * 4;

            // encode u32 -> BE bytes directly into chunk buffer
            for (out, &word) in chunk_buf[..byte_len]
                .chunks_exact_mut(4)
                .zip(&buffer[i..i + chunk_words])
            {
                BigEndian::write_u32(out, word);
            }

            let buffer = &chunk_buf[..byte_len];
            trace!("FPGA block write chunked ({}). address: 0x{:04x}, length: 0x{:04x} data: \n{:02x}", CHUNK_SIZE, address, buffer.len(), buffer);

            let transaction = TransferConfig {
                instruction: Some(CMD_WRITE_U32_BE as u32),
                isize: AddressSize::_8Bit,
                iwidth: OspiWidth::QUAD,

                address: Some(address as u32),
                adsize: AddressSize::_16Bit,
                adwidth: OspiWidth::QUAD,

                dummy: DummyCycles::_0,
                dwidth: OspiWidth::QUAD,
                ..Default::default()
            };

            self.ospi
                .blocking_write(buffer, transaction)
                .unwrap();

            i += chunk_words;
            address += byte_len as u16;
        }
    }

    pub fn write_u32(&mut self, address: u16, value: u32) {
        defmt::assert!(!self.memory_mapped_mode_enabled);

        let buffer = &mut [0; 4];
        <BigEndian as ByteOrder>::write_u32(buffer, value);
        trace!("FPGA block write. address: 0x{:04x}, length: 0x{:04x} data: \n{:02x}", address, buffer.len(), buffer);
        let transaction: TransferConfig = TransferConfig {
            instruction: Some(CMD_WRITE_U32_BE as u32),
            isize: AddressSize::_8Bit,
            iwidth: OspiWidth::QUAD,

            address: Some(address as u32),
            adsize: AddressSize::_16Bit,
            adwidth: OspiWidth::QUAD,

            dummy: DummyCycles::_0,

            dwidth: OspiWidth::QUAD,
            ..Default::default()
        };
        self.ospi.blocking_write(buffer, transaction).unwrap();
    }

    pub fn enable_memory_mapped_mode(&mut self) {
        defmt::assert!(!self.memory_mapped_mode_enabled);

        let read_config: TransferConfig = TransferConfig {
            instruction: Some(CMD_READ_U32_LE as u32),
            isize: AddressSize::_8Bit,
            iwidth: OspiWidth::QUAD,

            adsize: AddressSize::_16Bit,
            adwidth: OspiWidth::QUAD,

            dummy: DummyCycles::_8,

            dwidth: OspiWidth::QUAD,
            sioo: false,
            ..Default::default()
        };

        let write_config: TransferConfig = TransferConfig {
            instruction: Some(CMD_WRITE_U32_LE as u32),
            isize: AddressSize::_8Bit,
            iwidth: OspiWidth::QUAD,

            adsize: AddressSize::_16Bit,
            adwidth: OspiWidth::QUAD,

            dummy: DummyCycles::_0,

            dwidth: OspiWidth::QUAD,
            sioo: false,
            ..Default::default()
        };

        self.ospi.enable_memory_mapped_mode(read_config, write_config).unwrap();
        self.memory_mapped_mode_enabled = true;
    }

    pub fn disable_memory_mapped_mode(&mut self) {
        defmt::assert!(self.memory_mapped_mode_enabled);

        self.ospi.disable_memory_mapped_mode();
        self.memory_mapped_mode_enabled = false;
    }

    pub fn led_1_enable(&mut self) {
        defmt::assert!(self.memory_mapped_mode_enabled);

        fpga_pac::LED.led_ctrl().modify(|w| {
            w.set_mcu_led(true);
        });
    }

    pub fn led_1_disable(&mut self) {
        defmt::assert!(self.memory_mapped_mode_enabled);

        fpga_pac::LED.led_ctrl().modify(|w| {
            w.set_mcu_led(false);
        });
    }

    pub fn led_2_enable(&mut self) {
        defmt::assert!(self.memory_mapped_mode_enabled);

        fpga_pac::LED.led_ctrl().modify(|w| {
            w.set_fpga_led(true);
        });
    }

    pub fn led_2_disable(&mut self) {
        defmt::assert!(self.memory_mapped_mode_enabled);

        fpga_pac::LED.led_ctrl().modify(|w| {
            w.set_fpga_led(false);
        });
    }

    pub fn buzzer_enable(&mut self) {
        defmt::assert!(self.memory_mapped_mode_enabled);

        fpga_pac::BUZZER.buzzer_ctrl().modify(|w| {
            w.set_buzzer(true);
        });    }

    pub fn buzzer_disable(&mut self) {
        defmt::assert!(self.memory_mapped_mode_enabled);

        fpga_pac::BUZZER.buzzer_ctrl().modify(|w| {
            w.set_buzzer(false);
        });
    }

    pub fn dump_registers(&mut self) {
        defmt::assert!(self.memory_mapped_mode_enabled);

        defmt::debug!("FPGA register map memory mapped (u32):");
        let base = 0x9000_0000 as *const u32;

        const FPGA_REG_SIZE: usize = 0x10000 / 4;

        for row in (0..FPGA_REG_SIZE).step_by(0x10) {
            let mut row_values: [u32; 0x10] = [0xff; 0x10];
            for col in 0..0x10_usize {
                row_values[col] = unsafe { core::ptr::read_volatile(base.add(row + col)) };
            }
            defmt::info!("{:04x}: {:08x}", row * 4, row_values);
        }
    }

    pub fn reset_encoders(&mut self) {
        fpga_pac::ENCODERS.enc_ctrl().modify(|w| {
            w.set_reset(true);
        });
    }

    /// ordering: a,b,c,x,y,z
    pub fn read_encoders(&self, values: &mut [u16; 6]) {
        values[0] = fpga_pac::ENCODERS.enc_count_a().read().value();
        values[1] = fpga_pac::ENCODERS.enc_count_b().read().value();
        values[2] = fpga_pac::ENCODERS.enc_count_c().read().value();
        values[3] = fpga_pac::ENCODERS.enc_count_x().read().value();
        values[4] = fpga_pac::ENCODERS.enc_count_y().read().value();
        values[5] = fpga_pac::ENCODERS.enc_count_z().read().value();
    }

    pub fn set_encoders(&self, values: &[u16; 6]) {
        fpga_pac::ENCODERS.enc_set_count_a().write(|w| { w.set_value(values[0])});
        fpga_pac::ENCODERS.enc_set_count_b().write(|w| { w.set_value(values[1])});
        fpga_pac::ENCODERS.enc_set_count_c().write(|w| { w.set_value(values[2])});
        fpga_pac::ENCODERS.enc_set_count_x().write(|w| { w.set_value(values[3])});
        fpga_pac::ENCODERS.enc_set_count_y().write(|w| { w.set_value(values[4])});
        fpga_pac::ENCODERS.enc_set_count_z().write(|w| { w.set_value(values[5])});
    }

    pub fn led_controller_0(&self) -> Ws2812LedControllerBuilder {
        Ws2812LedControllerBuilder::new(0)
    }

    pub fn led_controller_1(&self) -> Ws2812LedControllerBuilder {
        Ws2812LedControllerBuilder::new(1)
    }

    pub fn oec1_enable(&mut self) {
        defmt::assert!(self.memory_mapped_mode_enabled);

        fpga_pac::IO.io_out_1().modify(|w| {
            w.set_oec1(true);
        });
    }

    pub fn oec1_disable(&mut self) {
        defmt::assert!(self.memory_mapped_mode_enabled);

        fpga_pac::IO.io_out_1().modify(|w| {
            w.set_oec1(false);
        });
    }

    pub fn oec2_enable(&mut self) {
        defmt::assert!(self.memory_mapped_mode_enabled);

        fpga_pac::IO.io_out_1().modify(|w| {
            w.set_oec2(true);
        });
    }

    pub fn oec2_disable(&mut self) {
        defmt::assert!(self.memory_mapped_mode_enabled);

        fpga_pac::IO.io_out_1().modify(|w| {
            w.set_oec2(false);
        });
    }

    pub fn adc_mux(&self) -> FpgaAdcMux {
        FpgaAdcMux::new()
    }


    /// Indicates if the base board is present.
    pub fn base_present(&mut self) -> bool {
        defmt::assert!(self.memory_mapped_mode_enabled);

        let value = fpga_pac::IO.io_in_1().read();

        let present = value.base_present();
        defmt::debug!("FPGA value: 0x{:08x}, base_present: 0b{:01b}", value.0, present);

        present
    }

    /// Returns a bitmask of the port present status
    /// one bit per port, 4 port.
    pub fn port_present(&mut self) -> u8 {
        defmt::assert!(self.memory_mapped_mode_enabled);

        let value = fpga_pac::IO.io_in_1().read();

        let present = value.port_present() & 0b1111;

        defmt::debug!("FPGA value: 0x{:08x}, port_present: 0b{:04b}", value.0, present);

        present
    }

}

#[derive(defmt::Format)]
#[repr(C)]
pub struct FpgaVersion {
    pub major: u8,
    pub minor: u8,
    pub patch: u8,
    pub build: u8,
}

impl FpgaVersion {
    pub fn from_bytes(bytes: [u8; 4]) -> Self {
        Self {
            major: bytes[0],
            minor: bytes[1],
            patch: bytes[2],
            build: bytes[3],
        }
    }

    pub fn from_u32(value: u32) -> Self {
        let bytes: [u8; 4] = value.to_le_bytes();
        Self {
            major: bytes[0],
            minor: bytes[1],
            patch: bytes[2],
            build: bytes[3],
        }
    }
}

pub mod ws2812 {
    pub struct Ws2812LedController {
        instance: fpga_pac::ws2812_0::ws2812_0,
    }

    impl Ws2812LedController {
        pub fn update_leds(&mut self, wrgb: &[u32]) {
            for wrgb in wrgb.iter() {
                self.instance.ws_data_0().write(|w| {
                    w.0 = *wrgb;
                });
            }
        }
    }

    impl Ws2812LedController {

        /// use the builder to create a configured instance
        fn new(instance: fpga_pac::ws2812_0::ws2812_0) -> Self {
            Self {
                instance
            }
        }
    }

    pub struct Ws2812LedControllerBuilder {
        instance: usize,
        led_count: u8,
        color_ordering: ColorOrdering,
    }

    impl Ws2812LedControllerBuilder {
        pub fn new(instance: usize) -> Self {
            Self {
                instance,
                led_count: 0,
                color_ordering: ColorOrdering::RGB,
            }
        }

        pub fn with_led_count(mut self, led_count: u8) -> Self {
            self.led_count = led_count;
            self
        }

        pub fn with_mode(mut self, color_ordering: ColorOrdering) -> Self {
            self.color_ordering = color_ordering;
            self
        }

        pub fn enable(self) -> Ws2812LedController {
            let instance = match self.instance {
                0 => fpga_pac::WS2812_0,
                1 => fpga_pac::WS2812_1,
                _ => panic!("Invalid instance"),
            };

            instance.ws_ctrl().modify(|w| {
                w.set_enabled(true);
                w.set_mode(self.color_ordering.into());
            });
            instance.ws_tx_config().write(|w| {
                w.set_leds_count(self.led_count);
            });

            Ws2812LedController::new(instance)
        }
    }

    #[repr(u8)]
    pub enum ColorOrdering {
        RGB,
        RGBW,
        GRB,
        GRBW,
    }

    impl Into<fpga_pac::ws2812_0::vals::mode> for ColorOrdering {
        fn into(self) -> fpga_pac::ws2812_0::vals::mode {
            match self {
                ColorOrdering::RGB => fpga_pac::ws2812_0::vals::mode::RGB,
                ColorOrdering::RGBW => fpga_pac::ws2812_0::vals::mode::RGBW,
                ColorOrdering::GRB => fpga_pac::ws2812_0::vals::mode::GRB,
                ColorOrdering::GRBW => fpga_pac::ws2812_0::vals::mode::GRBW,
            }
        }
    }
}

pub mod adc {
    pub struct FpgaAdcMux {}

    impl FpgaAdcMux {
        pub fn new() -> Self {
            Self {}
        }

        pub fn select_port(&mut self, port: usize) {
            assert!(port < 4);

            fpga_pac::IO.io_out_1().modify(|w| {
                w.set_adc_mux_sel(port as u8);
            })
        }
    }
}