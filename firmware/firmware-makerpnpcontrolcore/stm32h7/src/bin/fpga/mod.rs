use embassy_stm32::mode::Blocking;
use embassy_stm32::ospi::{
    AddressSize, ChipSelectHighTime, FIFOThresholdLevel, Instance, MemorySize, MemoryType, Ospi, OspiWidth,
    TransferConfig, WrapSize,
};
use embassy_stm32::ospi::enums::DummyCycles;
use defmt::*;

mod commands {
    pub const CMD_READ_16: u8 = 0x10;
    pub const CMD_WRITE_16: u8 = 0x90;
}
pub use commands::*;


mod registers {
    pub const REG_LED_CTRL: u16 = 0x0020;
    pub const REG_IO_IN: u16 = 0x0024;
    pub const REG_BUZZER_CTRL: u16 = 0x0028;
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
        self.read_block(REG_IO_IN, &mut buffer);
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

    pub fn write_block(&mut self, address: u16, buffer: &[u8]) {
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
        let mut buffer: [u8; 1] = [0xff; 1];
        self.read_block(REG_LED_CTRL, &mut buffer);
        buffer[0] |= 0b0000_0001;
        self.write_block(REG_LED_CTRL, &buffer);
    }

    pub fn led_1_disable(&mut self) {
        let mut buffer: [u8; 1] = [0xff; 1];
        self.read_block(REG_LED_CTRL, &mut buffer);
        buffer[0] &= !0b0000_0001;
        self.write_block(REG_LED_CTRL, &buffer);
    }

    pub fn led_2_enable(&mut self) {
        let mut buffer: [u8; 1] = [0xff; 1];
        self.read_block(REG_LED_CTRL, &mut buffer);
        buffer[0] |= 0b0000_0010;
        self.write_block(REG_LED_CTRL, &buffer);
    }

    pub fn led_2_disable(&mut self) {
        let mut buffer: [u8; 1] = [0xff; 1];
        self.read_block(REG_LED_CTRL, &mut buffer);
        buffer[0] &= !0b0000_0010;
        self.write_block(REG_LED_CTRL, &buffer);
    }

    pub fn buzzer_enable(&mut self) {
        let mut buffer: [u8; 1] = [0xff; 1];
        self.read_block(REG_BUZZER_CTRL, &mut buffer);
        buffer[0] |= 0b0000_0001;
        self.write_block(REG_BUZZER_CTRL, &buffer);
    }

    pub fn buzzer_disable(&mut self) {
        let mut buffer: [u8; 1] = [0xff; 1];
        self.read_block(REG_BUZZER_CTRL, &mut buffer);
        buffer[0] &= !0b0000_0001;
        self.write_block(REG_BUZZER_CTRL, &buffer);
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
