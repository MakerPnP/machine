#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![doc = "Peripheral access API (generated using chiptool v0.1.0 (bcf538a 2026-05-18))"]
#![no_std]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Interrupt {}
unsafe impl cortex_m::interrupt::InterruptNumber for Interrupt {
    #[inline(always)]
    fn number(self) -> u16 {
        self as u16
    }
}
#[cfg(feature = "rt")]
mod _vectors {
    unsafe extern "C" {}
    pub union Vector {
        _handler: unsafe extern "C" fn(),
        _reserved: u32,
    }
    #[unsafe(link_section = ".vector_table.interrupts")]
    #[unsafe(no_mangle)]
    pub static __INTERRUPTS: [Vector; 0] = [];
}
#[doc = "system block 1"]
pub const SYSTEM1: system1::system1 = unsafe { system1::system1::from_ptr(0x9000_0000usize as _) };
#[doc = "led control block"]
pub const LED: led::led = unsafe { led::led::from_ptr(0x9000_0040usize as _) };
#[doc = "io control block"]
pub const IO: io::io = unsafe { io::io::from_ptr(0x9000_0080usize as _) };
#[doc = "buzzer control block"]
pub const BUZZER: buzzer::buzzer = unsafe { buzzer::buzzer::from_ptr(0x9000_00c0usize as _) };
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
    use core::marker::PhantomData;
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
    impl<T: Copy, A: Read> Reg<T, A> {
        #[inline(always)]
        pub fn read(&self) -> T {
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
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for io_in_1 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "io_in_1 {{ user0: {=bool:?}, user1: {=bool:?}, reserved: {=u32:?} }}",
                    self.user0(),
                    self.user1(),
                    self.reserved()
                )
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
