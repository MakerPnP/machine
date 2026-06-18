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

        fpga_pac::SYSTEM1.ident().read().0
    }

    pub fn read_version(&mut self) -> FpgaVersion {
        defmt::assert!(self.memory_mapped_mode_enabled);

        FpgaVersion::from_u32(fpga_pac::SYSTEM1.version().read().0)
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
        // do a write, to force a re-read next time it's accessed
        unsafe { core::ptr::write(0x9000_0088 as * mut u32, 0x00000000); }

        let buttons = (value.user0() as u8) | ((value.user1() as u8) << 1);
        defmt::debug!("FPGA value: 0x{:08x}, buttons: 0b{:02b}", value.0, buttons);

        buttons
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

        defmt::debug!("FPGA register map (u32):");
        let base = 0x9000_0000 as *const u32;

        const FPGA_REG_SIZE: usize = 0x80;

        for i in 0..FPGA_REG_SIZE {
            let val = unsafe { core::ptr::read_volatile(base.add(i)) };
            defmt::info!("{:03x}: {:08x}", i * 4, val);
        }
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
