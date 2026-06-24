#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![doc = "Peripheral access API (generated using chiptool v0.1.0 (bcf538a 2026-05-18))"]
#![no_std]
#[doc = "system block 1"]
pub const SYSTEM1: system1::system1 = unsafe { system1::system1::from_ptr(0x9000_0000usize as _) };
#[doc = "led control block"]
pub const LED: led::led = unsafe { led::led::from_ptr(0x9000_0040usize as _) };
#[doc = "io control block"]
pub const IO: io::io = unsafe { io::io::from_ptr(0x9000_0080usize as _) };
#[doc = "buzzer control block"]
pub const BUZZER: buzzer::buzzer = unsafe { buzzer::buzzer::from_ptr(0x9000_00c0usize as _) };
#[doc = "encoders control block"]
pub const ENCODERS: encoders::encoders =
    unsafe { encoders::encoders::from_ptr(0x9000_0100usize as _) };
#[doc = "ws2812 RGB LED block"]
pub const WS2812_1: ws2812_1::ws2812_1 =
    unsafe { ws2812_1::ws2812_1::from_ptr(0x9000_0140usize as _) };
#[doc = "ws2812 RGB LED block"]
pub const WS2812_2: ws2812_1::ws2812_1 =
    unsafe { ws2812_1::ws2812_1::from_ptr(0x9000_0180usize as _) };
#[doc = "system block 2"]
pub const SYSTEM2: system2::system2 = unsafe { system2::system2::from_ptr(0x9000_01c0usize as _) };
#[cfg(feature = "rt")]
pub use cortex_m_rt::interrupt;
#[cfg(feature = "rt")]
pub use Interrupt as interrupt;
pub mod buzzer {
    #[doc = "buzzer control block."]
    #[derive(Copy, Clone, Eq, PartialEq)]
    pub struct buzzer {
        ptr: *mut u8,
    }
    unsafe impl Send for buzzer {}
    unsafe impl Sync for buzzer {}
    impl buzzer {
        #[inline(always)]
        pub const unsafe fn from_ptr(ptr: *mut ()) -> Self {
            Self { ptr: ptr as _ }
        }
        #[inline(always)]
        pub const fn as_ptr(&self) -> *mut () {
            self.ptr as _
        }
        #[doc = "buzzer control register."]
        #[inline(always)]
        pub const fn buzzer_ctrl(self) -> crate::common::Reg<regs::buzzer_ctrl, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x0usize) as _) }
        }
    }
    pub mod regs {
        #[doc = "buzzer control register."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct buzzer_ctrl(pub u32);
        impl buzzer_ctrl {
            #[doc = "buzzer control (0 = off)."]
            #[must_use]
            #[inline(always)]
            pub const fn buzzer(&self) -> bool {
                let val = (self.0 >> 0usize) & 0x01;
                val != 0
            }
            #[doc = "buzzer control (0 = off)."]
            #[inline(always)]
            pub const fn set_buzzer(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 0usize)) | (((val as u32) & 0x01) << 0usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved(&self) -> u32 {
                let val = (self.0 >> 1usize) & 0x7fff_ffff;
                val as u32
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0x7fff_ffff << 1usize)) | (((val as u32) & 0x7fff_ffff) << 1usize);
            }
        }
        impl Default for buzzer_ctrl {
            #[inline(always)]
            fn default() -> buzzer_ctrl {
                buzzer_ctrl(0)
            }
        }
        impl core::fmt::Debug for buzzer_ctrl {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("buzzer_ctrl")
                    .field("buzzer", &self.buzzer())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for buzzer_ctrl {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "buzzer_ctrl {{ buzzer: {=bool:?}, reserved: {=u32:?} }}",
                    self.buzzer(),
                    self.reserved()
                )
            }
        }
    }
}
pub mod common {

    // The QuadSPI peripheral has a FIFO that cannot be turned off and is always used.
    //
    // The FIFO is a block of 0x20 bytes.
    // * On the first read from the block, say at 0x84, the hardware issues a block read start at
    //   address 0x84 and fills up-to the length of the FIFI.
    // * A second read from an address in the block will NOT trigger a new octospi transaction,
    //   but will instead read the data from the FIFO.
    //
    // This means data in the FIFO will be stale, polling registers in the same block will not work.
    //
    // To workaround this, we must check if the second read is in the same block as the first read,
    // and if IS in the same block, then we need to issue a dummy read OUTSIDE of the block, then
    // issue the actual read afterwards, this cases the FIFO to be fulled by the data from the dummy
    // read so that when the actual read is requested the FIFO will be filled again.
    //
    // Safety:
    //
    // An AtomicUsize is used to keep track of the last block read, which it makes it thread-safe.
    // FPGA register must not have side-effects on reads.

    static LAST_BLOCK: AtomicUsize = AtomicUsize::new(usize::MAX);
    const QUAD_SPI_FIFO_DEPTH: usize = 0x20;
    const FPGA_MEMORY_SIZE: usize = 0x0000_0200;

    // Note: This assumes OCTOSPI1 is used
    const DUMMY_READ_ADDRESS: usize = 0x9000_0000 + FPGA_MEMORY_SIZE;

    #[inline(always)]
    fn compute_block(addr: usize) -> usize {
        addr & !(QUAD_SPI_FIFO_DEPTH - 1)
    }

    #[inline(always)]
    fn dummy_read() {
        unsafe { core::ptr::read_volatile(DUMMY_READ_ADDRESS as *mut u8); }
    }

    use core::marker::PhantomData;
    use core::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Copy, Clone, PartialEq, Eq)]
    pub struct RW;
    #[derive(Copy, Clone, PartialEq, Eq)]
    pub struct R;
    #[derive(Copy, Clone, PartialEq, Eq)]
    pub struct W;
    mod sealed {
        use super::*;
        pub trait Access {}
        impl Access for R {}
        impl Access for W {}
        impl Access for RW {}
    }
    pub trait Access: sealed::Access + Copy {}
    impl Access for R {}
    impl Access for W {}
    impl Access for RW {}
    pub trait Read: Access {}
    impl Read for RW {}
    impl Read for R {}
    pub trait Write: Access {}
    impl Write for RW {}
    impl Write for W {}
    #[derive(Copy, Clone, PartialEq, Eq)]
    pub struct Reg<T: Copy, A: Access> {
        ptr: *mut u8,
        phantom: PhantomData<*mut (T, A)>,
    }
    unsafe impl<T: Copy, A: Access> Send for Reg<T, A> {}
    unsafe impl<T: Copy, A: Access> Sync for Reg<T, A> {}
    impl<T: Copy, A: Access> Reg<T, A> {
        #[allow(clippy::missing_safety_doc)]
        #[inline(always)]
        pub const unsafe fn from_ptr(ptr: *mut T) -> Self {
            Self {
                ptr: ptr as _,
                phantom: PhantomData,
            }
        }
        #[inline(always)]
        pub const fn as_ptr(&self) -> *mut T {
            self.ptr as _
        }
    }

    /// OctoSPI-safe read implementation, with OctoSPI FIFO bypass.
    impl<T: Copy, A: Read> Reg<T, A> {
        #[inline(always)]
        pub fn read(&self) -> T {
            let addr = self.ptr as usize;
            let block = compute_block(addr);

            let last = LAST_BLOCK.load(Ordering::Relaxed);

            if last == block {
                // Same FIFO block → force flush
                dummy_read();
            }

            // Update block AFTER dummy logic
            LAST_BLOCK.store(block, Ordering::Relaxed);

            unsafe { (self.ptr as *mut T).read_volatile() }
        }
    }

    impl<T: Copy, A: Write> Reg<T, A> {
        #[inline(always)]
        pub fn write_value(&self, val: T) {
            unsafe { (self.ptr as *mut T).write_volatile(val) }
        }
    }
    impl<T: Default + Copy, A: Write> Reg<T, A> {
        #[inline(always)]
        pub fn write(&self, f: impl FnOnce(&mut T)) {
            let mut val = Default::default();
            f(&mut val);
            self.write_value(val);
        }
    }
    impl<T: Copy, A: Read + Write> Reg<T, A> {
        #[inline(always)]
        pub fn modify(&self, f: impl FnOnce(&mut T)) {
            let mut val = self.read();
            f(&mut val);
            self.write_value(val);
        }
    }
}
pub mod encoders {
    #[doc = "encoders control block."]
    #[derive(Copy, Clone, Eq, PartialEq)]
    pub struct encoders {
        ptr: *mut u8,
    }
    unsafe impl Send for encoders {}
    unsafe impl Sync for encoders {}
    impl encoders {
        #[inline(always)]
        pub const unsafe fn from_ptr(ptr: *mut ()) -> Self {
            Self { ptr: ptr as _ }
        }
        #[inline(always)]
        pub const fn as_ptr(&self) -> *mut () {
            self.ptr as _
        }
        #[doc = "encoders control register."]
        #[inline(always)]
        pub const fn enc_ctrl(self) -> crate::common::Reg<regs::enc_ctrl, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x0usize) as _) }
        }
        #[doc = "set encoder a counter."]
        #[inline(always)]
        pub const fn enc_set_count_a(
            self,
        ) -> crate::common::Reg<regs::enc_set_count_a, crate::common::W> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x04usize) as _) }
        }
        #[doc = "set encoder b counter."]
        #[inline(always)]
        pub const fn enc_set_count_b(
            self,
        ) -> crate::common::Reg<regs::enc_set_count_b, crate::common::W> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x08usize) as _) }
        }
        #[doc = "set encoder c counter."]
        #[inline(always)]
        pub const fn enc_set_count_c(
            self,
        ) -> crate::common::Reg<regs::enc_set_count_c, crate::common::W> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x0cusize) as _) }
        }
        #[doc = "set encoder x counter."]
        #[inline(always)]
        pub const fn enc_set_count_x(
            self,
        ) -> crate::common::Reg<regs::enc_set_count_x, crate::common::W> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x10usize) as _) }
        }
        #[doc = "set encoder y counter."]
        #[inline(always)]
        pub const fn enc_set_count_y(
            self,
        ) -> crate::common::Reg<regs::enc_set_count_y, crate::common::W> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x14usize) as _) }
        }
        #[doc = "set encoder z counter."]
        #[inline(always)]
        pub const fn enc_set_count_z(
            self,
        ) -> crate::common::Reg<regs::enc_set_count_z, crate::common::W> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x18usize) as _) }
        }
        #[doc = "encoder a counter."]
        #[inline(always)]
        pub const fn enc_count_a(self) -> crate::common::Reg<regs::enc_count_a, crate::common::R> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x20usize) as _) }
        }
        #[doc = "encoder b counter."]
        #[inline(always)]
        pub const fn enc_count_b(self) -> crate::common::Reg<regs::enc_count_b, crate::common::R> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x24usize) as _) }
        }
        #[doc = "encoder c counter."]
        #[inline(always)]
        pub const fn enc_count_c(self) -> crate::common::Reg<regs::enc_count_c, crate::common::R> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x28usize) as _) }
        }
        #[doc = "encoder x counter."]
        #[inline(always)]
        pub const fn enc_count_x(self) -> crate::common::Reg<regs::enc_count_x, crate::common::R> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x2cusize) as _) }
        }
        #[doc = "encoder y counter."]
        #[inline(always)]
        pub const fn enc_count_y(self) -> crate::common::Reg<regs::enc_count_y, crate::common::R> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x30usize) as _) }
        }
        #[doc = "encoder z counter."]
        #[inline(always)]
        pub const fn enc_count_z(self) -> crate::common::Reg<regs::enc_count_z, crate::common::R> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x34usize) as _) }
        }
    }
    pub mod regs {
        #[doc = "encoder a counter."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct enc_count_a(pub u32);
        impl enc_count_a {
            #[doc = "encoder counter value."]
            #[must_use]
            #[inline(always)]
            pub const fn value(&self) -> u16 {
                let val = (self.0 >> 0usize) & 0xffff;
                val as u16
            }
            #[doc = "encoder counter value."]
            #[inline(always)]
            pub const fn set_value(&mut self, val: u16) {
                self.0 = (self.0 & !(0xffff << 0usize)) | (((val as u32) & 0xffff) << 0usize);
            }
            #[doc = "reserved, ignored."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved(&self) -> u16 {
                let val = (self.0 >> 16usize) & 0xffff;
                val as u16
            }
            #[doc = "reserved, ignored."]
            #[inline(always)]
            pub const fn set_reserved(&mut self, val: u16) {
                self.0 = (self.0 & !(0xffff << 16usize)) | (((val as u32) & 0xffff) << 16usize);
            }
        }
        impl Default for enc_count_a {
            #[inline(always)]
            fn default() -> enc_count_a {
                enc_count_a(0)
            }
        }
        impl core::fmt::Debug for enc_count_a {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("enc_count_a")
                    .field("value", &self.value())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for enc_count_a {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "enc_count_a {{ value: {=u16:?}, reserved: {=u16:?} }}",
                    self.value(),
                    self.reserved()
                )
            }
        }
        #[doc = "encoder b counter."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct enc_count_b(pub u32);
        impl enc_count_b {
            #[doc = "encoder counter value."]
            #[must_use]
            #[inline(always)]
            pub const fn value(&self) -> u16 {
                let val = (self.0 >> 0usize) & 0xffff;
                val as u16
            }
            #[doc = "encoder counter value."]
            #[inline(always)]
            pub const fn set_value(&mut self, val: u16) {
                self.0 = (self.0 & !(0xffff << 0usize)) | (((val as u32) & 0xffff) << 0usize);
            }
            #[doc = "reserved, ignored."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved(&self) -> u16 {
                let val = (self.0 >> 16usize) & 0xffff;
                val as u16
            }
            #[doc = "reserved, ignored."]
            #[inline(always)]
            pub const fn set_reserved(&mut self, val: u16) {
                self.0 = (self.0 & !(0xffff << 16usize)) | (((val as u32) & 0xffff) << 16usize);
            }
        }
        impl Default for enc_count_b {
            #[inline(always)]
            fn default() -> enc_count_b {
                enc_count_b(0)
            }
        }
        impl core::fmt::Debug for enc_count_b {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("enc_count_b")
                    .field("value", &self.value())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for enc_count_b {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "enc_count_b {{ value: {=u16:?}, reserved: {=u16:?} }}",
                    self.value(),
                    self.reserved()
                )
            }
        }
        #[doc = "encoder c counter."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct enc_count_c(pub u32);
        impl enc_count_c {
            #[doc = "encoder counter value."]
            #[must_use]
            #[inline(always)]
            pub const fn value(&self) -> u16 {
                let val = (self.0 >> 0usize) & 0xffff;
                val as u16
            }
            #[doc = "encoder counter value."]
            #[inline(always)]
            pub const fn set_value(&mut self, val: u16) {
                self.0 = (self.0 & !(0xffff << 0usize)) | (((val as u32) & 0xffff) << 0usize);
            }
            #[doc = "reserved, ignored."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved(&self) -> u16 {
                let val = (self.0 >> 16usize) & 0xffff;
                val as u16
            }
            #[doc = "reserved, ignored."]
            #[inline(always)]
            pub const fn set_reserved(&mut self, val: u16) {
                self.0 = (self.0 & !(0xffff << 16usize)) | (((val as u32) & 0xffff) << 16usize);
            }
        }
        impl Default for enc_count_c {
            #[inline(always)]
            fn default() -> enc_count_c {
                enc_count_c(0)
            }
        }
        impl core::fmt::Debug for enc_count_c {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("enc_count_c")
                    .field("value", &self.value())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for enc_count_c {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "enc_count_c {{ value: {=u16:?}, reserved: {=u16:?} }}",
                    self.value(),
                    self.reserved()
                )
            }
        }
        #[doc = "encoder x counter."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct enc_count_x(pub u32);
        impl enc_count_x {
            #[doc = "encoder counter value."]
            #[must_use]
            #[inline(always)]
            pub const fn value(&self) -> u16 {
                let val = (self.0 >> 0usize) & 0xffff;
                val as u16
            }
            #[doc = "encoder counter value."]
            #[inline(always)]
            pub const fn set_value(&mut self, val: u16) {
                self.0 = (self.0 & !(0xffff << 0usize)) | (((val as u32) & 0xffff) << 0usize);
            }
            #[doc = "reserved, ignored."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved(&self) -> u16 {
                let val = (self.0 >> 16usize) & 0xffff;
                val as u16
            }
            #[doc = "reserved, ignored."]
            #[inline(always)]
            pub const fn set_reserved(&mut self, val: u16) {
                self.0 = (self.0 & !(0xffff << 16usize)) | (((val as u32) & 0xffff) << 16usize);
            }
        }
        impl Default for enc_count_x {
            #[inline(always)]
            fn default() -> enc_count_x {
                enc_count_x(0)
            }
        }
        impl core::fmt::Debug for enc_count_x {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("enc_count_x")
                    .field("value", &self.value())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for enc_count_x {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "enc_count_x {{ value: {=u16:?}, reserved: {=u16:?} }}",
                    self.value(),
                    self.reserved()
                )
            }
        }
        #[doc = "encoder y counter."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct enc_count_y(pub u32);
        impl enc_count_y {
            #[doc = "encoder counter value."]
            #[must_use]
            #[inline(always)]
            pub const fn value(&self) -> u16 {
                let val = (self.0 >> 0usize) & 0xffff;
                val as u16
            }
            #[doc = "encoder counter value."]
            #[inline(always)]
            pub const fn set_value(&mut self, val: u16) {
                self.0 = (self.0 & !(0xffff << 0usize)) | (((val as u32) & 0xffff) << 0usize);
            }
            #[doc = "reserved, ignored."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved(&self) -> u16 {
                let val = (self.0 >> 16usize) & 0xffff;
                val as u16
            }
            #[doc = "reserved, ignored."]
            #[inline(always)]
            pub const fn set_reserved(&mut self, val: u16) {
                self.0 = (self.0 & !(0xffff << 16usize)) | (((val as u32) & 0xffff) << 16usize);
            }
        }
        impl Default for enc_count_y {
            #[inline(always)]
            fn default() -> enc_count_y {
                enc_count_y(0)
            }
        }
        impl core::fmt::Debug for enc_count_y {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("enc_count_y")
                    .field("value", &self.value())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for enc_count_y {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "enc_count_y {{ value: {=u16:?}, reserved: {=u16:?} }}",
                    self.value(),
                    self.reserved()
                )
            }
        }
        #[doc = "encoder z counter."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct enc_count_z(pub u32);
        impl enc_count_z {
            #[doc = "encoder counter value."]
            #[must_use]
            #[inline(always)]
            pub const fn value(&self) -> u16 {
                let val = (self.0 >> 0usize) & 0xffff;
                val as u16
            }
            #[doc = "encoder counter value."]
            #[inline(always)]
            pub const fn set_value(&mut self, val: u16) {
                self.0 = (self.0 & !(0xffff << 0usize)) | (((val as u32) & 0xffff) << 0usize);
            }
            #[doc = "reserved, ignored."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved(&self) -> u16 {
                let val = (self.0 >> 16usize) & 0xffff;
                val as u16
            }
            #[doc = "reserved, ignored."]
            #[inline(always)]
            pub const fn set_reserved(&mut self, val: u16) {
                self.0 = (self.0 & !(0xffff << 16usize)) | (((val as u32) & 0xffff) << 16usize);
            }
        }
        impl Default for enc_count_z {
            #[inline(always)]
            fn default() -> enc_count_z {
                enc_count_z(0)
            }
        }
        impl core::fmt::Debug for enc_count_z {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("enc_count_z")
                    .field("value", &self.value())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for enc_count_z {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "enc_count_z {{ value: {=u16:?}, reserved: {=u16:?} }}",
                    self.value(),
                    self.reserved()
                )
            }
        }
        #[doc = "encoders control register."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct enc_ctrl(pub u32);
        impl enc_ctrl {
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved(&self) -> u32 {
                let val = (self.0 >> 0usize) & 0x7fff_ffff;
                val as u32
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0x7fff_ffff << 0usize)) | (((val as u32) & 0x7fff_ffff) << 0usize);
            }
            #[doc = "reset encoders (1 = reset)."]
            #[must_use]
            #[inline(always)]
            pub const fn reset(&self) -> bool {
                let val = (self.0 >> 0usize) & 0x01;
                val != 0
            }
            #[doc = "reset encoders (1 = reset)."]
            #[inline(always)]
            pub const fn set_reset(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 0usize)) | (((val as u32) & 0x01) << 0usize);
            }
        }
        impl Default for enc_ctrl {
            #[inline(always)]
            fn default() -> enc_ctrl {
                enc_ctrl(0)
            }
        }
        impl core::fmt::Debug for enc_ctrl {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("enc_ctrl")
                    .field("reserved", &self.reserved())
                    .field("reset", &self.reset())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for enc_ctrl {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "enc_ctrl {{ reserved: {=u32:?}, reset: {=bool:?} }}",
                    self.reserved(),
                    self.reset()
                )
            }
        }
        #[doc = "set encoder a counter."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct enc_set_count_a(pub u32);
        impl enc_set_count_a {
            #[doc = "encoder counter value."]
            #[must_use]
            #[inline(always)]
            pub const fn value(&self) -> u16 {
                let val = (self.0 >> 0usize) & 0xffff;
                val as u16
            }
            #[doc = "encoder counter value."]
            #[inline(always)]
            pub const fn set_value(&mut self, val: u16) {
                self.0 = (self.0 & !(0xffff << 0usize)) | (((val as u32) & 0xffff) << 0usize);
            }
            #[doc = "reserved, ignored."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved(&self) -> u16 {
                let val = (self.0 >> 16usize) & 0xffff;
                val as u16
            }
            #[doc = "reserved, ignored."]
            #[inline(always)]
            pub const fn set_reserved(&mut self, val: u16) {
                self.0 = (self.0 & !(0xffff << 16usize)) | (((val as u32) & 0xffff) << 16usize);
            }
        }
        impl Default for enc_set_count_a {
            #[inline(always)]
            fn default() -> enc_set_count_a {
                enc_set_count_a(0)
            }
        }
        impl core::fmt::Debug for enc_set_count_a {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("enc_set_count_a")
                    .field("value", &self.value())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for enc_set_count_a {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "enc_set_count_a {{ value: {=u16:?}, reserved: {=u16:?} }}",
                    self.value(),
                    self.reserved()
                )
            }
        }
        #[doc = "set encoder b counter."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct enc_set_count_b(pub u32);
        impl enc_set_count_b {
            #[doc = "encoder counter value."]
            #[must_use]
            #[inline(always)]
            pub const fn value(&self) -> u16 {
                let val = (self.0 >> 0usize) & 0xffff;
                val as u16
            }
            #[doc = "encoder counter value."]
            #[inline(always)]
            pub const fn set_value(&mut self, val: u16) {
                self.0 = (self.0 & !(0xffff << 0usize)) | (((val as u32) & 0xffff) << 0usize);
            }
            #[doc = "reserved, ignored."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved(&self) -> u16 {
                let val = (self.0 >> 16usize) & 0xffff;
                val as u16
            }
            #[doc = "reserved, ignored."]
            #[inline(always)]
            pub const fn set_reserved(&mut self, val: u16) {
                self.0 = (self.0 & !(0xffff << 16usize)) | (((val as u32) & 0xffff) << 16usize);
            }
        }
        impl Default for enc_set_count_b {
            #[inline(always)]
            fn default() -> enc_set_count_b {
                enc_set_count_b(0)
            }
        }
        impl core::fmt::Debug for enc_set_count_b {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("enc_set_count_b")
                    .field("value", &self.value())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for enc_set_count_b {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "enc_set_count_b {{ value: {=u16:?}, reserved: {=u16:?} }}",
                    self.value(),
                    self.reserved()
                )
            }
        }
        #[doc = "set encoder c counter."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct enc_set_count_c(pub u32);
        impl enc_set_count_c {
            #[doc = "encoder counter value."]
            #[must_use]
            #[inline(always)]
            pub const fn value(&self) -> u16 {
                let val = (self.0 >> 0usize) & 0xffff;
                val as u16
            }
            #[doc = "encoder counter value."]
            #[inline(always)]
            pub const fn set_value(&mut self, val: u16) {
                self.0 = (self.0 & !(0xffff << 0usize)) | (((val as u32) & 0xffff) << 0usize);
            }
            #[doc = "reserved, ignored."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved(&self) -> u16 {
                let val = (self.0 >> 16usize) & 0xffff;
                val as u16
            }
            #[doc = "reserved, ignored."]
            #[inline(always)]
            pub const fn set_reserved(&mut self, val: u16) {
                self.0 = (self.0 & !(0xffff << 16usize)) | (((val as u32) & 0xffff) << 16usize);
            }
        }
        impl Default for enc_set_count_c {
            #[inline(always)]
            fn default() -> enc_set_count_c {
                enc_set_count_c(0)
            }
        }
        impl core::fmt::Debug for enc_set_count_c {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("enc_set_count_c")
                    .field("value", &self.value())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for enc_set_count_c {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "enc_set_count_c {{ value: {=u16:?}, reserved: {=u16:?} }}",
                    self.value(),
                    self.reserved()
                )
            }
        }
        #[doc = "set encoder x counter."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct enc_set_count_x(pub u32);
        impl enc_set_count_x {
            #[doc = "encoder counter value."]
            #[must_use]
            #[inline(always)]
            pub const fn value(&self) -> u16 {
                let val = (self.0 >> 0usize) & 0xffff;
                val as u16
            }
            #[doc = "encoder counter value."]
            #[inline(always)]
            pub const fn set_value(&mut self, val: u16) {
                self.0 = (self.0 & !(0xffff << 0usize)) | (((val as u32) & 0xffff) << 0usize);
            }
            #[doc = "reserved, ignored."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved(&self) -> u16 {
                let val = (self.0 >> 16usize) & 0xffff;
                val as u16
            }
            #[doc = "reserved, ignored."]
            #[inline(always)]
            pub const fn set_reserved(&mut self, val: u16) {
                self.0 = (self.0 & !(0xffff << 16usize)) | (((val as u32) & 0xffff) << 16usize);
            }
        }
        impl Default for enc_set_count_x {
            #[inline(always)]
            fn default() -> enc_set_count_x {
                enc_set_count_x(0)
            }
        }
        impl core::fmt::Debug for enc_set_count_x {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("enc_set_count_x")
                    .field("value", &self.value())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for enc_set_count_x {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "enc_set_count_x {{ value: {=u16:?}, reserved: {=u16:?} }}",
                    self.value(),
                    self.reserved()
                )
            }
        }
        #[doc = "set encoder y counter."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct enc_set_count_y(pub u32);
        impl enc_set_count_y {
            #[doc = "encoder counter value."]
            #[must_use]
            #[inline(always)]
            pub const fn value(&self) -> u16 {
                let val = (self.0 >> 0usize) & 0xffff;
                val as u16
            }
            #[doc = "encoder counter value."]
            #[inline(always)]
            pub const fn set_value(&mut self, val: u16) {
                self.0 = (self.0 & !(0xffff << 0usize)) | (((val as u32) & 0xffff) << 0usize);
            }
            #[doc = "reserved, ignored."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved(&self) -> u16 {
                let val = (self.0 >> 16usize) & 0xffff;
                val as u16
            }
            #[doc = "reserved, ignored."]
            #[inline(always)]
            pub const fn set_reserved(&mut self, val: u16) {
                self.0 = (self.0 & !(0xffff << 16usize)) | (((val as u32) & 0xffff) << 16usize);
            }
        }
        impl Default for enc_set_count_y {
            #[inline(always)]
            fn default() -> enc_set_count_y {
                enc_set_count_y(0)
            }
        }
        impl core::fmt::Debug for enc_set_count_y {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("enc_set_count_y")
                    .field("value", &self.value())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for enc_set_count_y {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "enc_set_count_y {{ value: {=u16:?}, reserved: {=u16:?} }}",
                    self.value(),
                    self.reserved()
                )
            }
        }
        #[doc = "set encoder z counter."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct enc_set_count_z(pub u32);
        impl enc_set_count_z {
            #[doc = "encoder counter value."]
            #[must_use]
            #[inline(always)]
            pub const fn value(&self) -> u16 {
                let val = (self.0 >> 0usize) & 0xffff;
                val as u16
            }
            #[doc = "encoder counter value."]
            #[inline(always)]
            pub const fn set_value(&mut self, val: u16) {
                self.0 = (self.0 & !(0xffff << 0usize)) | (((val as u32) & 0xffff) << 0usize);
            }
            #[doc = "reserved, ignored."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved(&self) -> u16 {
                let val = (self.0 >> 16usize) & 0xffff;
                val as u16
            }
            #[doc = "reserved, ignored."]
            #[inline(always)]
            pub const fn set_reserved(&mut self, val: u16) {
                self.0 = (self.0 & !(0xffff << 16usize)) | (((val as u32) & 0xffff) << 16usize);
            }
        }
        impl Default for enc_set_count_z {
            #[inline(always)]
            fn default() -> enc_set_count_z {
                enc_set_count_z(0)
            }
        }
        impl core::fmt::Debug for enc_set_count_z {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("enc_set_count_z")
                    .field("value", &self.value())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for enc_set_count_z {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "enc_set_count_z {{ value: {=u16:?}, reserved: {=u16:?} }}",
                    self.value(),
                    self.reserved()
                )
            }
        }
    }
}
pub mod io {
    #[doc = "io control block."]
    #[derive(Copy, Clone, Eq, PartialEq)]
    pub struct io {
        ptr: *mut u8,
    }
    unsafe impl Send for io {}
    unsafe impl Sync for io {}
    impl io {
        #[inline(always)]
        pub const unsafe fn from_ptr(ptr: *mut ()) -> Self {
            Self { ptr: ptr as _ }
        }
        #[inline(always)]
        pub const fn as_ptr(&self) -> *mut () {
            self.ptr as _
        }
        #[doc = "io control register."]
        #[inline(always)]
        pub const fn io_ctrl(self) -> crate::common::Reg<regs::io_ctrl, crate::common::R> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x0usize) as _) }
        }
        #[doc = "io in 1 register."]
        #[inline(always)]
        pub const fn io_in_1(self) -> crate::common::Reg<regs::io_in_1, crate::common::R> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x04usize) as _) }
        }
        #[doc = "io in 2 register."]
        #[inline(always)]
        pub const fn io_in_2(self) -> crate::common::Reg<regs::io_in_2, crate::common::R> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x08usize) as _) }
        }
        #[doc = "io out 1 register."]
        #[inline(always)]
        pub const fn io_out_1(self) -> crate::common::Reg<regs::io_out_1, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x10usize) as _) }
        }
    }
    pub mod regs {
        #[doc = "io control register."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct io_ctrl(pub u32);
        impl io_ctrl {
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved(&self) -> u32 {
                let val = (self.0 >> 0usize) & 0xffff_ffff;
                val as u32
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0xffff_ffff << 0usize)) | (((val as u32) & 0xffff_ffff) << 0usize);
            }
        }
        impl Default for io_ctrl {
            #[inline(always)]
            fn default() -> io_ctrl {
                io_ctrl(0)
            }
        }
        impl core::fmt::Debug for io_ctrl {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("io_ctrl")
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for io_ctrl {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(f, "io_ctrl {{ reserved: {=u32:?} }}", self.reserved())
            }
        }
        #[doc = "io in 1 register."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct io_in_1(pub u32);
        impl io_in_1 {
            #[doc = "user 0 button (1 = pressed)."]
            #[must_use]
            #[inline(always)]
            pub const fn user0(&self) -> bool {
                let val = (self.0 >> 0usize) & 0x01;
                val != 0
            }
            #[doc = "user 0 button (1 = pressed)."]
            #[inline(always)]
            pub const fn set_user0(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 0usize)) | (((val as u32) & 0x01) << 0usize);
            }
            #[doc = "user 1 button (1 = pressed)."]
            #[must_use]
            #[inline(always)]
            pub const fn user1(&self) -> bool {
                let val = (self.0 >> 1usize) & 0x01;
                val != 0
            }
            #[doc = "user 1 button (1 = pressed)."]
            #[inline(always)]
            pub const fn set_user1(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 1usize)) | (((val as u32) & 0x01) << 1usize);
            }
            #[doc = "optical input 1 (optical isolator anode cathode)."]
            #[must_use]
            #[inline(always)]
            pub const fn iak1(&self) -> bool {
                let val = (self.0 >> 2usize) & 0x01;
                val != 0
            }
            #[doc = "optical input 1 (optical isolator anode cathode)."]
            #[inline(always)]
            pub const fn set_iak1(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 2usize)) | (((val as u32) & 0x01) << 2usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved(&self) -> u32 {
                let val = (self.0 >> 2usize) & 0x3fff_ffff;
                val as u32
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0x3fff_ffff << 2usize)) | (((val as u32) & 0x3fff_ffff) << 2usize);
            }
            #[doc = "optical input 2 (optical isolator anode cathode)."]
            #[must_use]
            #[inline(always)]
            pub const fn iak2(&self) -> bool {
                let val = (self.0 >> 3usize) & 0x01;
                val != 0
            }
            #[doc = "optical input 2 (optical isolator anode cathode)."]
            #[inline(always)]
            pub const fn set_iak2(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 3usize)) | (((val as u32) & 0x01) << 3usize);
            }
        }
        impl Default for io_in_1 {
            #[inline(always)]
            fn default() -> io_in_1 {
                io_in_1(0)
            }
        }
        impl core::fmt::Debug for io_in_1 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("io_in_1")
                    .field("user0", &self.user0())
                    .field("user1", &self.user1())
                    .field("iak1", &self.iak1())
                    .field("reserved", &self.reserved())
                    .field("iak2", &self.iak2())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for io_in_1 {
            fn format(&self, f: defmt::Formatter) {
                defmt :: write ! (f , "io_in_1 {{ user0: {=bool:?}, user1: {=bool:?}, iak1: {=bool:?}, reserved: {=u32:?}, iak2: {=bool:?} }}" , self . user0 () , self . user1 () , self . iak1 () , self . reserved () , self . iak2 ())
            }
        }
        #[doc = "io in 2 register."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct io_in_2(pub u32);
        impl io_in_2 {
            #[doc = "digital in 8 to 1 (ordering = 7:0)."]
            #[must_use]
            #[inline(always)]
            pub const fn din(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0xff;
                val as u8
            }
            #[doc = "digital in 8 to 1 (ordering = 7:0)."]
            #[inline(always)]
            pub const fn set_din(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 0usize)) | (((val as u32) & 0xff) << 0usize);
            }
            #[doc = "reserved."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved(&self) -> u32 {
                let val = (self.0 >> 8usize) & 0x00ff_ffff;
                val as u32
            }
            #[doc = "reserved."]
            #[inline(always)]
            pub const fn set_reserved(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0x00ff_ffff << 8usize)) | (((val as u32) & 0x00ff_ffff) << 8usize);
            }
        }
        impl Default for io_in_2 {
            #[inline(always)]
            fn default() -> io_in_2 {
                io_in_2(0)
            }
        }
        impl core::fmt::Debug for io_in_2 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("io_in_2")
                    .field("din", &self.din())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for io_in_2 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "io_in_2 {{ din: {=u8:?}, reserved: {=u32:?} }}",
                    self.din(),
                    self.reserved()
                )
            }
        }
        #[doc = "io out 1 register."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct io_out_1(pub u32);
        impl io_out_1 {
            #[doc = "optical output 1 (optical isolator emitter collector)."]
            #[must_use]
            #[inline(always)]
            pub const fn oec1(&self) -> bool {
                let val = (self.0 >> 0usize) & 0x01;
                val != 0
            }
            #[doc = "optical output 1 (optical isolator emitter collector)."]
            #[inline(always)]
            pub const fn set_oec1(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 0usize)) | (((val as u32) & 0x01) << 0usize);
            }
            #[doc = "optical output 2 (optical isolator emitter collector)."]
            #[must_use]
            #[inline(always)]
            pub const fn oec2(&self) -> bool {
                let val = (self.0 >> 1usize) & 0x01;
                val != 0
            }
            #[doc = "optical output 2 (optical isolator emitter collector)."]
            #[inline(always)]
            pub const fn set_oec2(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 1usize)) | (((val as u32) & 0x01) << 1usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved1(&self) -> u8 {
                let val = (self.0 >> 2usize) & 0x3f;
                val as u8
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved1(&mut self, val: u8) {
                self.0 = (self.0 & !(0x3f << 2usize)) | (((val as u32) & 0x3f) << 2usize);
            }
            #[doc = "ADC Mux input select."]
            #[must_use]
            #[inline(always)]
            pub const fn adc_mux_sel(&self) -> u8 {
                let val = (self.0 >> 8usize) & 0x03;
                val as u8
            }
            #[doc = "ADC Mux input select."]
            #[inline(always)]
            pub const fn set_adc_mux_sel(&mut self, val: u8) {
                self.0 = (self.0 & !(0x03 << 8usize)) | (((val as u32) & 0x03) << 8usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved2(&self) -> u32 {
                let val = (self.0 >> 10usize) & 0x003f_ffff;
                val as u32
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved2(&mut self, val: u32) {
                self.0 = (self.0 & !(0x003f_ffff << 10usize))
                    | (((val as u32) & 0x003f_ffff) << 10usize);
            }
        }
        impl Default for io_out_1 {
            #[inline(always)]
            fn default() -> io_out_1 {
                io_out_1(0)
            }
        }
        impl core::fmt::Debug for io_out_1 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("io_out_1")
                    .field("oec1", &self.oec1())
                    .field("oec2", &self.oec2())
                    .field("reserved1", &self.reserved1())
                    .field("adc_mux_sel", &self.adc_mux_sel())
                    .field("reserved2", &self.reserved2())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for io_out_1 {
            fn format(&self, f: defmt::Formatter) {
                defmt :: write ! (f , "io_out_1 {{ oec1: {=bool:?}, oec2: {=bool:?}, reserved1: {=u8:?}, adc_mux_sel: {=u8:?}, reserved2: {=u32:?} }}" , self . oec1 () , self . oec2 () , self . reserved1 () , self . adc_mux_sel () , self . reserved2 ())
            }
        }
    }
}
pub mod led {
    #[doc = "led control block."]
    #[derive(Copy, Clone, Eq, PartialEq)]
    pub struct led {
        ptr: *mut u8,
    }
    unsafe impl Send for led {}
    unsafe impl Sync for led {}
    impl led {
        #[inline(always)]
        pub const unsafe fn from_ptr(ptr: *mut ()) -> Self {
            Self { ptr: ptr as _ }
        }
        #[inline(always)]
        pub const fn as_ptr(&self) -> *mut () {
            self.ptr as _
        }
        #[doc = "led control register."]
        #[inline(always)]
        pub const fn led_ctrl(self) -> crate::common::Reg<regs::led_ctrl, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x0usize) as _) }
        }
    }
    pub mod regs {
        #[doc = "led control register."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct led_ctrl(pub u32);
        impl led_ctrl {
            #[doc = "fpga activity led (0 = off)."]
            #[must_use]
            #[inline(always)]
            pub const fn fpga_led(&self) -> bool {
                let val = (self.0 >> 0usize) & 0x01;
                val != 0
            }
            #[doc = "fpga activity led (0 = off)."]
            #[inline(always)]
            pub const fn set_fpga_led(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 0usize)) | (((val as u32) & 0x01) << 0usize);
            }
            #[doc = "mcu activity led (0 = off)."]
            #[must_use]
            #[inline(always)]
            pub const fn mcu_led(&self) -> bool {
                let val = (self.0 >> 1usize) & 0x01;
                val != 0
            }
            #[doc = "mcu activity led (0 = off)."]
            #[inline(always)]
            pub const fn set_mcu_led(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 1usize)) | (((val as u32) & 0x01) << 1usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved(&self) -> u32 {
                let val = (self.0 >> 2usize) & 0x3fff_ffff;
                val as u32
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0x3fff_ffff << 2usize)) | (((val as u32) & 0x3fff_ffff) << 2usize);
            }
        }
        impl Default for led_ctrl {
            #[inline(always)]
            fn default() -> led_ctrl {
                led_ctrl(0)
            }
        }
        impl core::fmt::Debug for led_ctrl {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("led_ctrl")
                    .field("fpga_led", &self.fpga_led())
                    .field("mcu_led", &self.mcu_led())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for led_ctrl {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "led_ctrl {{ fpga_led: {=bool:?}, mcu_led: {=bool:?}, reserved: {=u32:?} }}",
                    self.fpga_led(),
                    self.mcu_led(),
                    self.reserved()
                )
            }
        }
    }
}
pub mod system1 {
    #[doc = "system block 1."]
    #[derive(Copy, Clone, Eq, PartialEq)]
    pub struct system1 {
        ptr: *mut u8,
    }
    unsafe impl Send for system1 {}
    unsafe impl Sync for system1 {}
    impl system1 {
        #[inline(always)]
        pub const unsafe fn from_ptr(ptr: *mut ()) -> Self {
            Self { ptr: ptr as _ }
        }
        #[inline(always)]
        pub const fn as_ptr(&self) -> *mut () {
            self.ptr as _
        }
        #[doc = "device identifier."]
        #[inline(always)]
        pub const fn ident(self) -> crate::common::Reg<regs::ident, crate::common::R> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x0usize) as _) }
        }
        #[doc = "version information."]
        #[inline(always)]
        pub const fn version(self) -> crate::common::Reg<regs::version, crate::common::R> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x04usize) as _) }
        }
    }
    pub mod regs {
        #[doc = "device identifier."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct ident(pub u32);
        impl ident {
            #[doc = "device identifier."]
            #[must_use]
            #[inline(always)]
            pub const fn ident(&self) -> u32 {
                let val = (self.0 >> 0usize) & 0xffff_ffff;
                val as u32
            }
            #[doc = "device identifier."]
            #[inline(always)]
            pub const fn set_ident(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0xffff_ffff << 0usize)) | (((val as u32) & 0xffff_ffff) << 0usize);
            }
        }
        impl Default for ident {
            #[inline(always)]
            fn default() -> ident {
                ident(0)
            }
        }
        impl core::fmt::Debug for ident {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("ident")
                    .field("ident", &self.ident())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for ident {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(f, "ident {{ ident: {=u32:?} }}", self.ident())
            }
        }
        #[doc = "version information."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct version(pub u32);
        impl version {
            #[must_use]
            #[inline(always)]
            pub const fn build(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_build(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 0usize)) | (((val as u32) & 0xff) << 0usize);
            }
            #[must_use]
            #[inline(always)]
            pub const fn patch(&self) -> u8 {
                let val = (self.0 >> 8usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_patch(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 8usize)) | (((val as u32) & 0xff) << 8usize);
            }
            #[must_use]
            #[inline(always)]
            pub const fn minor(&self) -> u8 {
                let val = (self.0 >> 16usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_minor(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 16usize)) | (((val as u32) & 0xff) << 16usize);
            }
            #[must_use]
            #[inline(always)]
            pub const fn major(&self) -> u8 {
                let val = (self.0 >> 24usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_major(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 24usize)) | (((val as u32) & 0xff) << 24usize);
            }
        }
        impl Default for version {
            #[inline(always)]
            fn default() -> version {
                version(0)
            }
        }
        impl core::fmt::Debug for version {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("version")
                    .field("build", &self.build())
                    .field("patch", &self.patch())
                    .field("minor", &self.minor())
                    .field("major", &self.major())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for version {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "version {{ build: {=u8:?}, patch: {=u8:?}, minor: {=u8:?}, major: {=u8:?} }}",
                    self.build(),
                    self.patch(),
                    self.minor(),
                    self.major()
                )
            }
        }
    }
}
pub mod system2 {
    #[doc = "system block 2."]
    #[derive(Copy, Clone, Eq, PartialEq)]
    pub struct system2 {
        ptr: *mut u8,
    }
    unsafe impl Send for system2 {}
    unsafe impl Sync for system2 {}
    impl system2 {
        #[inline(always)]
        pub const unsafe fn from_ptr(ptr: *mut ()) -> Self {
            Self { ptr: ptr as _ }
        }
        #[inline(always)]
        pub const fn as_ptr(&self) -> *mut () {
            self.ptr as _
        }
        #[doc = "fixed marker value."]
        #[inline(always)]
        pub const fn marker(self) -> crate::common::Reg<regs::marker, crate::common::R> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x3cusize) as _) }
        }
    }
    pub mod regs {
        #[doc = "fixed marker value."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct marker(pub u32);
        impl marker {
            #[doc = "fixed marker."]
            #[must_use]
            #[inline(always)]
            pub const fn marker(&self) -> u32 {
                let val = (self.0 >> 0usize) & 0xffff_ffff;
                val as u32
            }
            #[doc = "fixed marker."]
            #[inline(always)]
            pub const fn set_marker(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0xffff_ffff << 0usize)) | (((val as u32) & 0xffff_ffff) << 0usize);
            }
        }
        impl Default for marker {
            #[inline(always)]
            fn default() -> marker {
                marker(0)
            }
        }
        impl core::fmt::Debug for marker {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("marker")
                    .field("marker", &self.marker())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for marker {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(f, "marker {{ marker: {=u32:?} }}", self.marker())
            }
        }
    }
}
pub mod ws2812_1 {
    #[doc = "ws2812 RGB LED block."]
    #[derive(Copy, Clone, Eq, PartialEq)]
    pub struct ws2812_1 {
        ptr: *mut u8,
    }
    unsafe impl Send for ws2812_1 {}
    unsafe impl Sync for ws2812_1 {}
    impl ws2812_1 {
        #[inline(always)]
        pub const unsafe fn from_ptr(ptr: *mut ()) -> Self {
            Self { ptr: ptr as _ }
        }
        #[inline(always)]
        pub const fn as_ptr(&self) -> *mut () {
            self.ptr as _
        }
        #[doc = "led control register."]
        #[inline(always)]
        pub const fn ws_ctrl(self) -> crate::common::Reg<regs::ws_ctrl, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x0usize) as _) }
        }
        #[doc = "ws2812 transmit configuration."]
        #[inline(always)]
        pub const fn ws_tx_config(
            self,
        ) -> crate::common::Reg<regs::ws_tx_config, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x04usize) as _) }
        }
        #[doc = "rgb(w) input."]
        #[inline(always)]
        pub const fn ws_data_0(self) -> crate::common::Reg<regs::ws_data_0, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x10usize) as _) }
        }
        #[doc = "rgb(w) input."]
        #[inline(always)]
        pub const fn ws_data_1(self) -> crate::common::Reg<regs::ws_data_1, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x14usize) as _) }
        }
        #[doc = "rgb(w) input."]
        #[inline(always)]
        pub const fn ws_data_2(self) -> crate::common::Reg<regs::ws_data_2, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x18usize) as _) }
        }
        #[doc = "rgb(w) input."]
        #[inline(always)]
        pub const fn ws_data_3(self) -> crate::common::Reg<regs::ws_data_3, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x1cusize) as _) }
        }
        #[doc = "rgb(w) input."]
        #[inline(always)]
        pub const fn ws_data_4(self) -> crate::common::Reg<regs::ws_data_4, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x20usize) as _) }
        }
        #[doc = "rgb(w) input."]
        #[inline(always)]
        pub const fn ws_data_5(self) -> crate::common::Reg<regs::ws_data_5, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x24usize) as _) }
        }
        #[doc = "rgb(w) input."]
        #[inline(always)]
        pub const fn ws_data_6(self) -> crate::common::Reg<regs::ws_data_6, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x28usize) as _) }
        }
        #[doc = "rgb(w) input."]
        #[inline(always)]
        pub const fn ws_data_7(self) -> crate::common::Reg<regs::ws_data_7, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x2cusize) as _) }
        }
    }
    pub mod regs {
        #[doc = "led control register."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct ws_ctrl(pub u32);
        impl ws_ctrl {
            #[doc = "enable led output (1 = on)."]
            #[must_use]
            #[inline(always)]
            pub const fn enabled(&self) -> bool {
                let val = (self.0 >> 0usize) & 0x01;
                val != 0
            }
            #[doc = "enable led output (1 = on)."]
            #[inline(always)]
            pub const fn set_enabled(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 0usize)) | (((val as u32) & 0x01) << 0usize);
            }
            #[doc = "ws2812 color mode."]
            #[must_use]
            #[inline(always)]
            pub const fn mode(&self) -> super::vals::mode {
                let val = (self.0 >> 1usize) & 0x03;
                super::vals::mode::from_bits(val as u8)
            }
            #[doc = "ws2812 color mode."]
            #[inline(always)]
            pub const fn set_mode(&mut self, val: super::vals::mode) {
                self.0 = (self.0 & !(0x03 << 1usize)) | (((val.to_bits() as u32) & 0x03) << 1usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved(&self) -> u32 {
                let val = (self.0 >> 3usize) & 0x1fff_ffff;
                val as u32
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0x1fff_ffff << 3usize)) | (((val as u32) & 0x1fff_ffff) << 3usize);
            }
        }
        impl Default for ws_ctrl {
            #[inline(always)]
            fn default() -> ws_ctrl {
                ws_ctrl(0)
            }
        }
        impl core::fmt::Debug for ws_ctrl {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("ws_ctrl")
                    .field("enabled", &self.enabled())
                    .field("mode", &self.mode())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for ws_ctrl {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "ws_ctrl {{ enabled: {=bool:?}, mode: {:?}, reserved: {=u32:?} }}",
                    self.enabled(),
                    self.mode(),
                    self.reserved()
                )
            }
        }
        #[doc = "rgb(w) input."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct ws_data_0(pub u32);
        impl ws_data_0 {
            #[must_use]
            #[inline(always)]
            pub const fn b(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_b(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 0usize)) | (((val as u32) & 0xff) << 0usize);
            }
            #[must_use]
            #[inline(always)]
            pub const fn g(&self) -> u8 {
                let val = (self.0 >> 8usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_g(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 8usize)) | (((val as u32) & 0xff) << 8usize);
            }
            #[must_use]
            #[inline(always)]
            pub const fn r(&self) -> u8 {
                let val = (self.0 >> 16usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_r(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 16usize)) | (((val as u32) & 0xff) << 16usize);
            }
            #[must_use]
            #[inline(always)]
            pub const fn w(&self) -> u8 {
                let val = (self.0 >> 24usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_w(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 24usize)) | (((val as u32) & 0xff) << 24usize);
            }
        }
        impl Default for ws_data_0 {
            #[inline(always)]
            fn default() -> ws_data_0 {
                ws_data_0(0)
            }
        }
        impl core::fmt::Debug for ws_data_0 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("ws_data_0")
                    .field("b", &self.b())
                    .field("g", &self.g())
                    .field("r", &self.r())
                    .field("w", &self.w())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for ws_data_0 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "ws_data_0 {{ b: {=u8:?}, g: {=u8:?}, r: {=u8:?}, w: {=u8:?} }}",
                    self.b(),
                    self.g(),
                    self.r(),
                    self.w()
                )
            }
        }
        #[doc = "rgb(w) input."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct ws_data_1(pub u32);
        impl ws_data_1 {
            #[must_use]
            #[inline(always)]
            pub const fn b(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_b(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 0usize)) | (((val as u32) & 0xff) << 0usize);
            }
            #[must_use]
            #[inline(always)]
            pub const fn g(&self) -> u8 {
                let val = (self.0 >> 8usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_g(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 8usize)) | (((val as u32) & 0xff) << 8usize);
            }
            #[must_use]
            #[inline(always)]
            pub const fn r(&self) -> u8 {
                let val = (self.0 >> 16usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_r(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 16usize)) | (((val as u32) & 0xff) << 16usize);
            }
            #[must_use]
            #[inline(always)]
            pub const fn w(&self) -> u8 {
                let val = (self.0 >> 24usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_w(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 24usize)) | (((val as u32) & 0xff) << 24usize);
            }
        }
        impl Default for ws_data_1 {
            #[inline(always)]
            fn default() -> ws_data_1 {
                ws_data_1(0)
            }
        }
        impl core::fmt::Debug for ws_data_1 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("ws_data_1")
                    .field("b", &self.b())
                    .field("g", &self.g())
                    .field("r", &self.r())
                    .field("w", &self.w())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for ws_data_1 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "ws_data_1 {{ b: {=u8:?}, g: {=u8:?}, r: {=u8:?}, w: {=u8:?} }}",
                    self.b(),
                    self.g(),
                    self.r(),
                    self.w()
                )
            }
        }
        #[doc = "rgb(w) input."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct ws_data_2(pub u32);
        impl ws_data_2 {
            #[must_use]
            #[inline(always)]
            pub const fn b(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_b(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 0usize)) | (((val as u32) & 0xff) << 0usize);
            }
            #[must_use]
            #[inline(always)]
            pub const fn g(&self) -> u8 {
                let val = (self.0 >> 8usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_g(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 8usize)) | (((val as u32) & 0xff) << 8usize);
            }
            #[must_use]
            #[inline(always)]
            pub const fn r(&self) -> u8 {
                let val = (self.0 >> 16usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_r(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 16usize)) | (((val as u32) & 0xff) << 16usize);
            }
            #[must_use]
            #[inline(always)]
            pub const fn w(&self) -> u8 {
                let val = (self.0 >> 24usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_w(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 24usize)) | (((val as u32) & 0xff) << 24usize);
            }
        }
        impl Default for ws_data_2 {
            #[inline(always)]
            fn default() -> ws_data_2 {
                ws_data_2(0)
            }
        }
        impl core::fmt::Debug for ws_data_2 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("ws_data_2")
                    .field("b", &self.b())
                    .field("g", &self.g())
                    .field("r", &self.r())
                    .field("w", &self.w())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for ws_data_2 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "ws_data_2 {{ b: {=u8:?}, g: {=u8:?}, r: {=u8:?}, w: {=u8:?} }}",
                    self.b(),
                    self.g(),
                    self.r(),
                    self.w()
                )
            }
        }
        #[doc = "rgb(w) input."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct ws_data_3(pub u32);
        impl ws_data_3 {
            #[must_use]
            #[inline(always)]
            pub const fn b(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_b(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 0usize)) | (((val as u32) & 0xff) << 0usize);
            }
            #[must_use]
            #[inline(always)]
            pub const fn g(&self) -> u8 {
                let val = (self.0 >> 8usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_g(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 8usize)) | (((val as u32) & 0xff) << 8usize);
            }
            #[must_use]
            #[inline(always)]
            pub const fn r(&self) -> u8 {
                let val = (self.0 >> 16usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_r(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 16usize)) | (((val as u32) & 0xff) << 16usize);
            }
            #[must_use]
            #[inline(always)]
            pub const fn w(&self) -> u8 {
                let val = (self.0 >> 24usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_w(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 24usize)) | (((val as u32) & 0xff) << 24usize);
            }
        }
        impl Default for ws_data_3 {
            #[inline(always)]
            fn default() -> ws_data_3 {
                ws_data_3(0)
            }
        }
        impl core::fmt::Debug for ws_data_3 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("ws_data_3")
                    .field("b", &self.b())
                    .field("g", &self.g())
                    .field("r", &self.r())
                    .field("w", &self.w())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for ws_data_3 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "ws_data_3 {{ b: {=u8:?}, g: {=u8:?}, r: {=u8:?}, w: {=u8:?} }}",
                    self.b(),
                    self.g(),
                    self.r(),
                    self.w()
                )
            }
        }
        #[doc = "rgb(w) input."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct ws_data_4(pub u32);
        impl ws_data_4 {
            #[must_use]
            #[inline(always)]
            pub const fn b(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_b(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 0usize)) | (((val as u32) & 0xff) << 0usize);
            }
            #[must_use]
            #[inline(always)]
            pub const fn g(&self) -> u8 {
                let val = (self.0 >> 8usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_g(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 8usize)) | (((val as u32) & 0xff) << 8usize);
            }
            #[must_use]
            #[inline(always)]
            pub const fn r(&self) -> u8 {
                let val = (self.0 >> 16usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_r(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 16usize)) | (((val as u32) & 0xff) << 16usize);
            }
            #[must_use]
            #[inline(always)]
            pub const fn w(&self) -> u8 {
                let val = (self.0 >> 24usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_w(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 24usize)) | (((val as u32) & 0xff) << 24usize);
            }
        }
        impl Default for ws_data_4 {
            #[inline(always)]
            fn default() -> ws_data_4 {
                ws_data_4(0)
            }
        }
        impl core::fmt::Debug for ws_data_4 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("ws_data_4")
                    .field("b", &self.b())
                    .field("g", &self.g())
                    .field("r", &self.r())
                    .field("w", &self.w())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for ws_data_4 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "ws_data_4 {{ b: {=u8:?}, g: {=u8:?}, r: {=u8:?}, w: {=u8:?} }}",
                    self.b(),
                    self.g(),
                    self.r(),
                    self.w()
                )
            }
        }
        #[doc = "rgb(w) input."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct ws_data_5(pub u32);
        impl ws_data_5 {
            #[must_use]
            #[inline(always)]
            pub const fn b(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_b(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 0usize)) | (((val as u32) & 0xff) << 0usize);
            }
            #[must_use]
            #[inline(always)]
            pub const fn g(&self) -> u8 {
                let val = (self.0 >> 8usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_g(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 8usize)) | (((val as u32) & 0xff) << 8usize);
            }
            #[must_use]
            #[inline(always)]
            pub const fn r(&self) -> u8 {
                let val = (self.0 >> 16usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_r(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 16usize)) | (((val as u32) & 0xff) << 16usize);
            }
            #[must_use]
            #[inline(always)]
            pub const fn w(&self) -> u8 {
                let val = (self.0 >> 24usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_w(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 24usize)) | (((val as u32) & 0xff) << 24usize);
            }
        }
        impl Default for ws_data_5 {
            #[inline(always)]
            fn default() -> ws_data_5 {
                ws_data_5(0)
            }
        }
        impl core::fmt::Debug for ws_data_5 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("ws_data_5")
                    .field("b", &self.b())
                    .field("g", &self.g())
                    .field("r", &self.r())
                    .field("w", &self.w())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for ws_data_5 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "ws_data_5 {{ b: {=u8:?}, g: {=u8:?}, r: {=u8:?}, w: {=u8:?} }}",
                    self.b(),
                    self.g(),
                    self.r(),
                    self.w()
                )
            }
        }
        #[doc = "rgb(w) input."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct ws_data_6(pub u32);
        impl ws_data_6 {
            #[must_use]
            #[inline(always)]
            pub const fn b(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_b(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 0usize)) | (((val as u32) & 0xff) << 0usize);
            }
            #[must_use]
            #[inline(always)]
            pub const fn g(&self) -> u8 {
                let val = (self.0 >> 8usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_g(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 8usize)) | (((val as u32) & 0xff) << 8usize);
            }
            #[must_use]
            #[inline(always)]
            pub const fn r(&self) -> u8 {
                let val = (self.0 >> 16usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_r(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 16usize)) | (((val as u32) & 0xff) << 16usize);
            }
            #[must_use]
            #[inline(always)]
            pub const fn w(&self) -> u8 {
                let val = (self.0 >> 24usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_w(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 24usize)) | (((val as u32) & 0xff) << 24usize);
            }
        }
        impl Default for ws_data_6 {
            #[inline(always)]
            fn default() -> ws_data_6 {
                ws_data_6(0)
            }
        }
        impl core::fmt::Debug for ws_data_6 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("ws_data_6")
                    .field("b", &self.b())
                    .field("g", &self.g())
                    .field("r", &self.r())
                    .field("w", &self.w())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for ws_data_6 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "ws_data_6 {{ b: {=u8:?}, g: {=u8:?}, r: {=u8:?}, w: {=u8:?} }}",
                    self.b(),
                    self.g(),
                    self.r(),
                    self.w()
                )
            }
        }
        #[doc = "rgb(w) input."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct ws_data_7(pub u32);
        impl ws_data_7 {
            #[must_use]
            #[inline(always)]
            pub const fn b(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_b(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 0usize)) | (((val as u32) & 0xff) << 0usize);
            }
            #[must_use]
            #[inline(always)]
            pub const fn g(&self) -> u8 {
                let val = (self.0 >> 8usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_g(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 8usize)) | (((val as u32) & 0xff) << 8usize);
            }
            #[must_use]
            #[inline(always)]
            pub const fn r(&self) -> u8 {
                let val = (self.0 >> 16usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_r(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 16usize)) | (((val as u32) & 0xff) << 16usize);
            }
            #[must_use]
            #[inline(always)]
            pub const fn w(&self) -> u8 {
                let val = (self.0 >> 24usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_w(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 24usize)) | (((val as u32) & 0xff) << 24usize);
            }
        }
        impl Default for ws_data_7 {
            #[inline(always)]
            fn default() -> ws_data_7 {
                ws_data_7(0)
            }
        }
        impl core::fmt::Debug for ws_data_7 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("ws_data_7")
                    .field("b", &self.b())
                    .field("g", &self.g())
                    .field("r", &self.r())
                    .field("w", &self.w())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for ws_data_7 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "ws_data_7 {{ b: {=u8:?}, g: {=u8:?}, r: {=u8:?}, w: {=u8:?} }}",
                    self.b(),
                    self.g(),
                    self.r(),
                    self.w()
                )
            }
        }
        #[doc = "ws2812 transmit configuration."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct ws_tx_config(pub u32);
        impl ws_tx_config {
            #[must_use]
            #[inline(always)]
            pub const fn leds_count(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0xff;
                val as u8
            }
            #[inline(always)]
            pub const fn set_leds_count(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 0usize)) | (((val as u32) & 0xff) << 0usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved(&self) -> u32 {
                let val = (self.0 >> 8usize) & 0x00ff_ffff;
                val as u32
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0x00ff_ffff << 8usize)) | (((val as u32) & 0x00ff_ffff) << 8usize);
            }
        }
        impl Default for ws_tx_config {
            #[inline(always)]
            fn default() -> ws_tx_config {
                ws_tx_config(0)
            }
        }
        impl core::fmt::Debug for ws_tx_config {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("ws_tx_config")
                    .field("leds_count", &self.leds_count())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for ws_tx_config {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "ws_tx_config {{ leds_count: {=u8:?}, reserved: {=u32:?} }}",
                    self.leds_count(),
                    self.reserved()
                )
            }
        }
    }
    pub mod vals {
        #[repr(u8)]
        #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum mode {
            #[doc = "rgb ordering."]
            RGB = 0x0,
            #[doc = "rgbw ordering."]
            RGBW = 0x01,
            #[doc = "grb ordering."]
            GRB = 0x02,
            #[doc = "grbw ordering."]
            GRBW = 0x03,
        }
        impl mode {
            #[inline(always)]
            pub const fn from_bits(val: u8) -> mode {
                unsafe { core::mem::transmute(val & 0x03) }
            }
            #[inline(always)]
            pub const fn to_bits(self) -> u8 {
                unsafe { core::mem::transmute(self) }
            }
        }
        impl From<u8> for mode {
            #[inline(always)]
            fn from(val: u8) -> mode {
                mode::from_bits(val)
            }
        }
        impl From<mode> for u8 {
            #[inline(always)]
            fn from(val: mode) -> u8 {
                mode::to_bits(val)
            }
        }
    }
}
