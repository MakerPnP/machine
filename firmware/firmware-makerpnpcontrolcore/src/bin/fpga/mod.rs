use embassy_stm32::mode::Blocking;
use embassy_stm32::ospi::{
    AddressSize, ChipSelectHighTime, FIFOThresholdLevel, Instance, MemorySize, MemoryType, Ospi, OspiWidth,
    TransferConfig, WrapSize,
};
use embassy_stm32::ospi::enums::DummyCycles;

mod commands {
    pub const CMD_READ_16: u8 = 0x10;
    pub const CMD_WRITE_16: u8 = 0x90;
}

pub use commands::*;

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

    pub fn read_version(&mut self) -> [u8; 4] {
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
        buffer
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
    }

}
