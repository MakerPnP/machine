use byteorder::{BigEndian, ByteOrder};
use embassy_stm32::mode::Blocking;
use embassy_stm32::ospi::{
    AddressSize, Instance, Ospi, OspiWidth,
    TransferConfig, 
};
use embassy_stm32::ospi::enums::DummyCycles;
use defmt::*;

mod commands {
    pub const CMD_READ_16: u8 = 0x10;
    pub const CMD_WRITE_16: u8 = 0x90;
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
}

impl<I: Instance> FpgaCore<I> {
    pub async fn new(ospi: Ospi<'static, I, Blocking>) -> Self {
        let memory = Self { ospi };

        memory
    }

    pub fn read_ident(&mut self) -> [u8; 4] {
        let mut buffer = [0; 4];
        let transaction: TransferConfig = TransferConfig {
            instruction: Some(CMD_READ_16 as u32),
            isize: AddressSize::_8Bit,
            iwidth: OspiWidth::QUAD,

            address: Some(0x0000),
            adsize: AddressSize::_16Bit,
            adwidth: OspiWidth::QUAD,

            dummy: DummyCycles::_8,

            dwidth: OspiWidth::QUAD,
            ..Default::default()
        };
        self.ospi.blocking_read(&mut buffer, transaction).unwrap();
        buffer
    }

    pub fn read_version(&mut self) -> FpgaVersion {
        let mut buffer = [0; 4];
        let transaction: TransferConfig = TransferConfig {
            instruction: Some(CMD_READ_16 as u32),
            iwidth: OspiWidth::QUAD,
            isize: AddressSize::_8Bit,

            address: Some(0x0004),
            adsize: AddressSize::_16Bit,
            adwidth: OspiWidth::QUAD,

            dummy: DummyCycles::_8,

            dwidth: OspiWidth::QUAD,
            ..Default::default()
        };
        self.ospi.blocking_read(&mut buffer, transaction).unwrap();

        FpgaVersion::from_bytes(buffer)
    }

    pub fn read_buttons(&mut self) -> u8 {
        let mut buffer = [0; 1];
        self.read_block(REG_IO_IN_1, &mut buffer);
        buffer[0]
    }

    pub fn read_block(&mut self, address: u16, buffer: &mut [u8]) {
        let transaction: TransferConfig = TransferConfig {
            instruction: Some(CMD_READ_16 as u32),
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
        trace!("FPGA block read. address: 0x{:04x}, length: 0x{:04x} data: \n{:02x}", address, buffer.len(), buffer);
    }

    pub fn read_block_u32(&mut self, address: u16, buffer: &mut [u32]) {
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
        let mut buffer = [0; 4];
        let transaction: TransferConfig = TransferConfig {
            instruction: Some(CMD_READ_16 as u32),
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
        trace!("FPGA read_u32. address: 0x{:04x}, length: 0x{:04x} value: {:04x}", address, buffer.len(), buffer);

        value
    }


    /// Writes a sequence of u32s to the FPGA.
    /// The buffer must be aligned to a multiple of 4 bytes.
    /// The bytes are sent over the wire in big-endian order.
    pub fn write_block(&mut self, address: u16, buffer: &[u32]) {
        trace!("FPGA block write. address: 0x{:04x}, length: 0x{:04x} data: \n{:02x}", address, buffer.len(), buffer);
        let transaction: TransferConfig = TransferConfig {
            instruction: Some(CMD_WRITE_16 as u32),
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

            let transaction = TransferConfig {
                instruction: Some(CMD_WRITE_16 as u32),
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
                .blocking_write(&chunk_buf[..byte_len], transaction)
                .unwrap();

            i += chunk_words;
            address += byte_len as u16;
        }
    }

    pub fn write_u32(&mut self, address: u16, value: u32) {
        let buffer = &mut [0; 4];
        <BigEndian as ByteOrder>::write_u32(buffer, value);
        trace!("FPGA block write. address: 0x{:04x}, length: 0x{:04x} data: \n{:02x}", address, buffer.len(), buffer);
        let transaction: TransferConfig = TransferConfig {
            instruction: Some(CMD_WRITE_16 as u32),
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

    pub fn led_1_enable(&mut self) {
        let mut buffer = self.read_u32(REG_LED_CTRL);
        buffer |= 0b0000_0001;
        self.write_u32(REG_LED_CTRL, buffer);
    }

    pub fn led_1_disable(&mut self) {
        let mut buffer = self.read_u32(REG_LED_CTRL);
        buffer &= !0b0000_0001;
        self.write_u32(REG_LED_CTRL, buffer);
    }

    pub fn led_2_enable(&mut self) {
        let mut buffer = self.read_u32(REG_LED_CTRL);
        buffer |= 0b0000_0010;
        self.write_u32(REG_LED_CTRL, buffer);
    }

    pub fn led_2_disable(&mut self) {
        let mut buffer = self.read_u32(REG_LED_CTRL);
        buffer &= !0b0000_0010;
        self.write_u32(REG_LED_CTRL, buffer);
    }

    pub fn buzzer_enable(&mut self) {
        let mut buffer = self.read_u32(REG_BUZZER_CTRL);
        buffer |= 0b0000_0001;
        self.write_u32(REG_BUZZER_CTRL, buffer);
    }

    pub fn buzzer_disable(&mut self) {
        let mut buffer = self.read_u32(REG_BUZZER_CTRL);
        buffer &= !0b0000_0001;
        self.write_u32(REG_BUZZER_CTRL, buffer);
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
}
