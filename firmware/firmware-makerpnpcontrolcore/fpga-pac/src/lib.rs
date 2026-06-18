#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![doc = "Peripheral access API (generated using chiptool v0.1.0 (bcf538a 2026-05-18))"]
#![no_std]
#[doc = "system block 0"]
pub const SYSTEM0: system0::system0 = unsafe { system0::system0::from_ptr(0x9000_0000usize as _) };
#[doc = "led control block"]
pub const LED: led::led = unsafe { led::led::from_ptr(0x9000_0200usize as _) };
#[doc = "buzzer control block"]
pub const BUZZER: buzzer::buzzer = unsafe { buzzer::buzzer::from_ptr(0x9000_0300usize as _) };
#[doc = "io control block"]
pub const IO: io::io = unsafe { io::io::from_ptr(0x9000_0400usize as _) };
#[doc = "ws2812 RGB LED block"]
pub const WS2812_0: ws2812_0::ws2812_0 =
    unsafe { ws2812_0::ws2812_0::from_ptr(0x9000_0800usize as _) };
#[doc = "ws2812 RGB LED block (second instance)"]
pub const WS2812_1: ws2812_0::ws2812_0 =
    unsafe { ws2812_0::ws2812_0::from_ptr(0x9000_0900usize as _) };
#[doc = "hx717 dual channel 24-bit load cell adc. all three registers share one 16-byte OctoSPI prefetch line, so reading any one of them causes reads of all of them; no register here has a read side effect"]
pub const LOADCELL0: loadcell0::loadcell0 =
    unsafe { loadcell0::loadcell0::from_ptr(0x9000_0a00usize as _) };
#[doc = "encoders control block"]
pub const ENCODERS: encoders::encoders =
    unsafe { encoders::encoders::from_ptr(0x9000_0c00usize as _) };
#[doc = "8-channel stepper motor step/dir pulse generator (2 banks of 4)"]
pub const STEPPERS: steppers::steppers =
    unsafe { steppers::steppers::from_ptr(0x9000_0d00usize as _) };
#[doc = "4 independent 8-bit timers (source clock, 8-bit prescaler, 8-bit auto-reload) feeding 12 flexibly-mapped PWM output channels (PM1-4, OT1-8)"]
pub const TIMER_PWM: timer_pwm::timer_pwm =
    unsafe { timer_pwm::timer_pwm::from_ptr(0x9000_0e00usize as _) };
#[doc = "system block 1"]
pub const SYSTEM1: system1::system1 = unsafe { system1::system1::from_ptr(0x9000_ff00usize as _) };
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
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved1(&self) -> u8 {
                let val = (self.0 >> 4usize) & 0x0f;
                val as u8
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved1(&mut self, val: u8) {
                self.0 = (self.0 & !(0x0f << 4usize)) | (((val as u32) & 0x0f) << 4usize);
            }
            #[doc = "base present detect."]
            #[must_use]
            #[inline(always)]
            pub const fn base_present(&self) -> bool {
                let val = (self.0 >> 8usize) & 0x01;
                val != 0
            }
            #[doc = "base present detect."]
            #[inline(always)]
            pub const fn set_base_present(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 8usize)) | (((val as u32) & 0x01) << 8usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved2(&self) -> u8 {
                let val = (self.0 >> 9usize) & 0x07;
                val as u8
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved2(&mut self, val: u8) {
                self.0 = (self.0 & !(0x07 << 9usize)) | (((val as u32) & 0x07) << 9usize);
            }
            #[doc = "port present detect."]
            #[must_use]
            #[inline(always)]
            pub const fn port_present(&self) -> u8 {
                let val = (self.0 >> 12usize) & 0x0f;
                val as u8
            }
            #[doc = "port present detect."]
            #[inline(always)]
            pub const fn set_port_present(&mut self, val: u8) {
                self.0 = (self.0 & !(0x0f << 12usize)) | (((val as u32) & 0x0f) << 12usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved3(&self) -> u16 {
                let val = (self.0 >> 16usize) & 0xffff;
                val as u16
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved3(&mut self, val: u16) {
                self.0 = (self.0 & !(0xffff << 16usize)) | (((val as u32) & 0xffff) << 16usize);
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
                    .field("iak2", &self.iak2())
                    .field("reserved1", &self.reserved1())
                    .field("base_present", &self.base_present())
                    .field("reserved2", &self.reserved2())
                    .field("port_present", &self.port_present())
                    .field("reserved3", &self.reserved3())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for io_in_1 {
            fn format(&self, f: defmt::Formatter) {
                defmt :: write ! (f , "io_in_1 {{ user0: {=bool:?}, user1: {=bool:?}, iak1: {=bool:?}, iak2: {=bool:?}, reserved1: {=u8:?}, base_present: {=bool:?}, reserved2: {=u8:?}, port_present: {=u8:?}, reserved3: {=u16:?} }}" , self . user0 () , self . user1 () , self . iak1 () , self . iak2 () , self . reserved1 () , self . base_present () , self . reserved2 () , self . port_present () , self . reserved3 ())
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
pub mod loadcell0 {
    #[doc = "hx717 dual channel 24-bit load cell adc. all three registers share one 16-byte OctoSPI prefetch line, so reading any one of them causes reads of all of them; no register here has a read side effect."]
    #[derive(Copy, Clone, Eq, PartialEq)]
    pub struct loadcell0 {
        ptr: *mut u8,
    }
    unsafe impl Send for loadcell0 {}
    unsafe impl Sync for loadcell0 {}
    impl loadcell0 {
        #[inline(always)]
        pub const unsafe fn from_ptr(ptr: *mut ()) -> Self {
            Self { ptr: ptr as _ }
        }
        #[inline(always)]
        pub const fn as_ptr(&self) -> *mut () {
            self.ptr as _
        }
        #[doc = "load cell control register."]
        #[inline(always)]
        pub const fn lc_ctrl(self) -> crate::common::Reg<regs::lc_ctrl, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x0usize) as _) }
        }
        #[doc = "load cell status register."]
        #[inline(always)]
        pub const fn lc_status(self) -> crate::common::Reg<regs::lc_status, crate::common::R> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x04usize) as _) }
        }
        #[doc = "most recent conversion result plus its sequence number. reading has no side effects; a single 32-bit read gets value and sequence together so they cannot skew."]
        #[inline(always)]
        pub const fn lc_value(self) -> crate::common::Reg<regs::lc_value, crate::common::R> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x08usize) as _) }
        }
    }
    pub mod regs {
        #[doc = "load cell control register."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct lc_ctrl(pub u32);
        impl lc_ctrl {
            #[doc = "run conversions back to back (1 = on)."]
            #[must_use]
            #[inline(always)]
            pub const fn enable(&self) -> bool {
                let val = (self.0 >> 0usize) & 0x01;
                val != 0
            }
            #[doc = "run conversions back to back (1 = on)."]
            #[inline(always)]
            pub const fn set_enable(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 0usize)) | (((val as u32) & 0x01) << 0usize);
            }
            #[doc = "request power-down. honoured only once the conversion in flight completes, so the channel and gain setup is saved. clear to wake; the device resumes with the setup it had before sleeping."]
            #[must_use]
            #[inline(always)]
            pub const fn pd(&self) -> bool {
                let val = (self.0 >> 1usize) & 0x01;
                val != 0
            }
            #[doc = "request power-down. honoured only once the conversion in flight completes, so the channel and gain setup is saved. clear to wake; the device resumes with the setup it had before sleeping."]
            #[inline(always)]
            pub const fn set_pd(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 1usize)) | (((val as u32) & 0x01) << 1usize);
            }
            #[doc = "output data rate, driven onto the S1 and S0 pins. changing this while enabled takes effect on whichever conversion the device happens to be in."]
            #[must_use]
            #[inline(always)]
            pub const fn rate(&self) -> super::vals::rate {
                let val = (self.0 >> 2usize) & 0x03;
                super::vals::rate::from_bits(val as u8)
            }
            #[doc = "output data rate, driven onto the S1 and S0 pins. changing this while enabled takes effect on whichever conversion the device happens to be in."]
            #[inline(always)]
            pub const fn set_rate(&mut self, val: super::vals::rate) {
                self.0 = (self.0 & !(0x03 << 2usize)) | (((val.to_bits() as u32) & 0x03) << 2usize);
            }
            #[doc = "input channel and pga gain for the NEXT conversion. selected by the number of PD_SCK pulses the peripheral issues, so it applies one conversion after it is written. allow 4 conversions of settling after any change before trusting a sample."]
            #[must_use]
            #[inline(always)]
            pub const fn mode(&self) -> super::vals::mode {
                let val = (self.0 >> 4usize) & 0x03;
                super::vals::mode::from_bits(val as u8)
            }
            #[doc = "input channel and pga gain for the NEXT conversion. selected by the number of PD_SCK pulses the peripheral issues, so it applies one conversion after it is written. allow 4 conversions of settling after any change before trusting a sample."]
            #[inline(always)]
            pub const fn set_mode(&mut self, val: super::vals::mode) {
                self.0 = (self.0 & !(0x03 << 4usize)) | (((val.to_bits() as u32) & 0x03) << 4usize);
            }
            #[doc = "power-down depth, applied when the pd bit is honoured."]
            #[must_use]
            #[inline(always)]
            pub const fn pdmode(&self) -> super::vals::pdmode {
                let val = (self.0 >> 6usize) & 0x03;
                super::vals::pdmode::from_bits(val as u8)
            }
            #[doc = "power-down depth, applied when the pd bit is honoured."]
            #[inline(always)]
            pub const fn set_pdmode(&mut self, val: super::vals::pdmode) {
                self.0 = (self.0 & !(0x03 << 6usize)) | (((val.to_bits() as u32) & 0x03) << 6usize);
            }
            #[doc = "write 1 to take exactly one sample without setting enable. self-clearing, always reads back 0."]
            #[must_use]
            #[inline(always)]
            pub const fn single(&self) -> bool {
                let val = (self.0 >> 8usize) & 0x01;
                val != 0
            }
            #[doc = "write 1 to take exactly one sample without setting enable. self-clearing, always reads back 0."]
            #[inline(always)]
            pub const fn set_single(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 8usize)) | (((val as u32) & 0x01) << 8usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved(&self) -> u32 {
                let val = (self.0 >> 9usize) & 0x007f_ffff;
                val as u32
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0x007f_ffff << 9usize)) | (((val as u32) & 0x007f_ffff) << 9usize);
            }
        }
        impl Default for lc_ctrl {
            #[inline(always)]
            fn default() -> lc_ctrl {
                lc_ctrl(0)
            }
        }
        impl core::fmt::Debug for lc_ctrl {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("lc_ctrl")
                    .field("enable", &self.enable())
                    .field("pd", &self.pd())
                    .field("rate", &self.rate())
                    .field("mode", &self.mode())
                    .field("pdmode", &self.pdmode())
                    .field("single", &self.single())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for lc_ctrl {
            fn format(&self, f: defmt::Formatter) {
                defmt :: write ! (f , "lc_ctrl {{ enable: {=bool:?}, pd: {=bool:?}, rate: {:?}, mode: {:?}, pdmode: {:?}, single: {=bool:?}, reserved: {=u32:?} }}" , self . enable () , self . pd () , self . rate () , self . mode () , self . pdmode () , self . single () , self . reserved ())
            }
        }
        #[doc = "load cell status register."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct lc_status(pub u32);
        impl lc_status {
            #[doc = "mirrors lc_ctrl.enable."]
            #[must_use]
            #[inline(always)]
            pub const fn enabled(&self) -> bool {
                let val = (self.0 >> 0usize) & 0x01;
                val != 0
            }
            #[doc = "mirrors lc_ctrl.enable."]
            #[inline(always)]
            pub const fn set_enabled(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 0usize)) | (((val as u32) & 0x01) << 0usize);
            }
            #[doc = "a conversion has completed since the last write to lc_ctrl, so lc_value is valid for the current configuration. cleared by writing lc_ctrl, never by a read. use the lc_value sequence number, not this bit, to detect each new sample."]
            #[must_use]
            #[inline(always)]
            pub const fn ready(&self) -> bool {
                let val = (self.0 >> 1usize) & 0x01;
                val != 0
            }
            #[doc = "a conversion has completed since the last write to lc_ctrl, so lc_value is valid for the current configuration. cleared by writing lc_ctrl, never by a read. use the lc_value sequence number, not this bit, to detect each new sample."]
            #[inline(always)]
            pub const fn set_ready(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 1usize)) | (((val as u32) & 0x01) << 1usize);
            }
            #[doc = "a conversion cycle is in progress. covers the wait for data as well as the pulse train, so in continuous mode this is high almost all the time and is not a useful poll target."]
            #[must_use]
            #[inline(always)]
            pub const fn busy(&self) -> bool {
                let val = (self.0 >> 2usize) & 0x01;
                val != 0
            }
            #[doc = "a conversion cycle is in progress. covers the wait for data as well as the pulse train, so in continuous mode this is high almost all the time and is not a useful poll target."]
            #[inline(always)]
            pub const fn set_busy(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 2usize)) | (((val as u32) & 0x01) << 2usize);
            }
            #[doc = "the device is being held in power-down."]
            #[must_use]
            #[inline(always)]
            pub const fn pd(&self) -> bool {
                let val = (self.0 >> 3usize) & 0x01;
                val != 0
            }
            #[doc = "the device is being held in power-down."]
            #[inline(always)]
            pub const fn set_pd(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 3usize)) | (((val as u32) & 0x01) << 3usize);
            }
            #[doc = "DOUT did not go low within the watchdog interval, so no device is responding. sticky; cleared by any write to lc_ctrl."]
            #[must_use]
            #[inline(always)]
            pub const fn timeout(&self) -> bool {
                let val = (self.0 >> 4usize) & 0x01;
                val != 0
            }
            #[doc = "DOUT did not go low within the watchdog interval, so no device is responding. sticky; cleared by any write to lc_ctrl."]
            #[inline(always)]
            pub const fn set_timeout(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 4usize)) | (((val as u32) & 0x01) << 4usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved(&self) -> u32 {
                let val = (self.0 >> 5usize) & 0x07ff_ffff;
                val as u32
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0x07ff_ffff << 5usize)) | (((val as u32) & 0x07ff_ffff) << 5usize);
            }
        }
        impl Default for lc_status {
            #[inline(always)]
            fn default() -> lc_status {
                lc_status(0)
            }
        }
        impl core::fmt::Debug for lc_status {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("lc_status")
                    .field("enabled", &self.enabled())
                    .field("ready", &self.ready())
                    .field("busy", &self.busy())
                    .field("pd", &self.pd())
                    .field("timeout", &self.timeout())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for lc_status {
            fn format(&self, f: defmt::Formatter) {
                defmt :: write ! (f , "lc_status {{ enabled: {=bool:?}, ready: {=bool:?}, busy: {=bool:?}, pd: {=bool:?}, timeout: {=bool:?}, reserved: {=u32:?} }}" , self . enabled () , self . ready () , self . busy () , self . pd () , self . timeout () , self . reserved ())
            }
        }
        #[doc = "most recent conversion result plus its sequence number. reading has no side effects; a single 32-bit read gets value and sequence together so they cannot skew."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct lc_value(pub u32);
        impl lc_value {
            #[doc = "raw 24-bit result in two's complement, exactly as shifted off the wire. 0x00800000 is negative full scale and 0x007FFFFF positive full scale; both are also the saturation codes when the input is out of range. the generated accessor masks to 24 bits, so sign extend from bit 23 rather than from bit 31: let sample = ((r.value() as i32) << 8) >> 8; taking the raw register word instead needs ((r.0 << 8) as i32) >> 8, which drops the sequence byte in the same step. omitting the shift pair yields a value that is never negative."]
            #[must_use]
            #[inline(always)]
            pub const fn value(&self) -> u32 {
                let val = (self.0 >> 0usize) & 0x00ff_ffff;
                val as u32
            }
            #[doc = "raw 24-bit result in two's complement, exactly as shifted off the wire. 0x00800000 is negative full scale and 0x007FFFFF positive full scale; both are also the saturation codes when the input is out of range. the generated accessor masks to 24 bits, so sign extend from bit 23 rather than from bit 31: let sample = ((r.value() as i32) << 8) >> 8; taking the raw register word instead needs ((r.0 << 8) as i32) >> 8, which drops the sequence byte in the same step. omitting the shift pair yields a value that is never negative."]
            #[inline(always)]
            pub const fn set_value(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0x00ff_ffff << 0usize)) | (((val as u32) & 0x00ff_ffff) << 0usize);
            }
            #[doc = "sample sequence number, incremented once per completed conversion. compare r.seq() against the last value seen using wrapping_sub, since it wraps every 256 conversions: 0.8s at 320sps, 3.2s at 80sps, 12.8s at 20sps, 25.6s at 10sps. a difference of 0 means no new conversion has completed and the value is a repeat of the one already seen; 1 is the expected case; more than 1 means that many samples were produced and never read. polling slower than the conversion period therefore reports gaps, and polling faster reports repeats - both are visible here and in neither case is lc_status.ready able to tell them apart."]
            #[must_use]
            #[inline(always)]
            pub const fn seq(&self) -> u8 {
                let val = (self.0 >> 24usize) & 0xff;
                val as u8
            }
            #[doc = "sample sequence number, incremented once per completed conversion. compare r.seq() against the last value seen using wrapping_sub, since it wraps every 256 conversions: 0.8s at 320sps, 3.2s at 80sps, 12.8s at 20sps, 25.6s at 10sps. a difference of 0 means no new conversion has completed and the value is a repeat of the one already seen; 1 is the expected case; more than 1 means that many samples were produced and never read. polling slower than the conversion period therefore reports gaps, and polling faster reports repeats - both are visible here and in neither case is lc_status.ready able to tell them apart."]
            #[inline(always)]
            pub const fn set_seq(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 24usize)) | (((val as u32) & 0xff) << 24usize);
            }
        }
        impl Default for lc_value {
            #[inline(always)]
            fn default() -> lc_value {
                lc_value(0)
            }
        }
        impl core::fmt::Debug for lc_value {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("lc_value")
                    .field("value", &self.value())
                    .field("seq", &self.seq())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for lc_value {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "lc_value {{ value: {=u32:?}, seq: {=u8:?} }}",
                    self.value(),
                    self.seq()
                )
            }
        }
    }
    pub mod vals {
        #[repr(u8)]
        #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum mode {
            #[doc = "channel A, gain 128, full scale +/-20mV at VREF 5V (25 pulses)."]
            A128 = 0x0,
            #[doc = "channel B, gain 64, full scale +/-40mV at VREF 5V (26 pulses)."]
            B64 = 0x01,
            #[doc = "channel A, gain 64, full scale +/-40mV at VREF 5V (27 pulses)."]
            A64 = 0x02,
            #[doc = "channel B, gain 8, full scale +/-320mV at VREF 5V (28 pulses)."]
            B8 = 0x03,
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
        #[repr(u8)]
        #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum pdmode {
            #[doc = "adc only, under 360uA. keeps the pulse count at the mode selection so channel and gain survive the sleep."]
            ADC = 0x0,
            #[doc = "adc and analog regulator, under 280uA. clocks 29 pulses, past the selection window, so the mode field is not applied by this conversion."]
            ADC_REG = 0x01,
            #[doc = "everything, under 1uA. clocks 30 pulses, past the selection window, so the mode field is not applied by this conversion."]
            ALL = 0x02,
            #[doc = "unused encoding, behaves as ADC."]
            RESERVED = 0x03,
        }
        impl pdmode {
            #[inline(always)]
            pub const fn from_bits(val: u8) -> pdmode {
                unsafe { core::mem::transmute(val & 0x03) }
            }
            #[inline(always)]
            pub const fn to_bits(self) -> u8 {
                unsafe { core::mem::transmute(self) }
            }
        }
        impl From<u8> for pdmode {
            #[inline(always)]
            fn from(val: u8) -> pdmode {
                pdmode::from_bits(val)
            }
        }
        impl From<pdmode> for u8 {
            #[inline(always)]
            fn from(val: pdmode) -> u8 {
                pdmode::to_bits(val)
            }
        }
        #[repr(u8)]
        #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum rate {
            #[doc = "10 sps, 18.2 noise-free bits."]
            RATE_10HZ = 0x0,
            #[doc = "20 sps, 17.7 noise-free bits."]
            RATE_20HZ = 0x01,
            #[doc = "80 sps, 16.7 noise-free bits."]
            RATE_80HZ = 0x02,
            #[doc = "320 sps, 15.8 noise-free bits."]
            RATE_320HZ = 0x03,
        }
        impl rate {
            #[inline(always)]
            pub const fn from_bits(val: u8) -> rate {
                unsafe { core::mem::transmute(val & 0x03) }
            }
            #[inline(always)]
            pub const fn to_bits(self) -> u8 {
                unsafe { core::mem::transmute(self) }
            }
        }
        impl From<u8> for rate {
            #[inline(always)]
            fn from(val: u8) -> rate {
                rate::from_bits(val)
            }
        }
        impl From<rate> for u8 {
            #[inline(always)]
            fn from(val: rate) -> u8 {
                rate::to_bits(val)
            }
        }
    }
}
pub mod steppers {
    #[doc = "8-channel stepper motor step/dir pulse generator (2 banks of 4)."]
    #[derive(Copy, Clone, Eq, PartialEq)]
    pub struct steppers {
        ptr: *mut u8,
    }
    unsafe impl Send for steppers {}
    unsafe impl Sync for steppers {}
    impl steppers {
        #[inline(always)]
        pub const unsafe fn from_ptr(ptr: *mut ()) -> Self {
            Self { ptr: ptr as _ }
        }
        #[inline(always)]
        pub const fn as_ptr(&self) -> *mut () {
            self.ptr as _
        }
        #[doc = "start/stop control - WRITE: start/stop strobes per motor (bank 0 = motors 0-3, bank 1 = motors 4-7); READ: per-motor moving status."]
        #[inline(always)]
        pub const fn step_ctrl(self) -> crate::common::Reg<regs::step_ctrl, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x0usize) as _) }
        }
        #[doc = "segment table streaming configuration - selects which motor subsequent step_seg_ctst/step_seg_spdm writes and reads target, and rewinds both the write and read pointers to segment 0."]
        #[inline(always)]
        pub const fn step_tx_config(
            self,
        ) -> crate::common::Reg<regs::step_tx_config, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x04usize) as _) }
        }
        #[doc = "per-motor step pulse width preset (tick count - 1). Combined with that motor's bank prescaler in step_pls_prescaler: pulse width = (prescaler+1) * (preset+1) sys_clk cycles."]
        #[inline(always)]
        pub const fn step_pls_config(
            self,
        ) -> crate::common::Reg<regs::step_pls_config, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x08usize) as _) }
        }
        #[doc = "per-bank step pulse width prescaler (sys_clk cycles per tick - 1). Bank 0 = motors 0-3 (low 16-bit half, bits\\[5:0\\]), bank 1 = motors 4-7 (high 16-bit half, bits\\[21:16\\]) - split this way so either field can widen, or other per-bank config bits can be added, without touching the other bank's half."]
        #[inline(always)]
        pub const fn step_pls_prescaler(
            self,
        ) -> crate::common::Reg<regs::step_pls_prescaler, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x0cusize) as _) }
        }
        #[doc = "segment table CTST word - command/direction/step-count for the segment addressed by step_tx_config's motor_instance and the current streaming pointer (advances on step_seg_spdm access, not this one)."]
        #[inline(always)]
        pub const fn step_seg_ctst(
            self,
        ) -> crate::common::Reg<regs::step_seg_ctst, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x10usize) as _) }
        }
        #[doc = "segment table SPDM word - start period/delta magnitude for the segment addressed by step_tx_config's motor_instance and the current streaming pointer (advances the streaming pointer on access - write or read)."]
        #[inline(always)]
        pub const fn step_seg_spdm(
            self,
        ) -> crate::common::Reg<regs::step_seg_spdm, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x14usize) as _) }
        }
        #[doc = "motor 0 status."]
        #[inline(always)]
        pub const fn step_status_0(
            self,
        ) -> crate::common::Reg<regs::step_status_0, crate::common::R> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x20usize) as _) }
        }
        #[doc = "motor 1 status."]
        #[inline(always)]
        pub const fn step_status_1(
            self,
        ) -> crate::common::Reg<regs::step_status_1, crate::common::R> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x24usize) as _) }
        }
        #[doc = "motor 2 status."]
        #[inline(always)]
        pub const fn step_status_2(
            self,
        ) -> crate::common::Reg<regs::step_status_2, crate::common::R> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x28usize) as _) }
        }
        #[doc = "motor 3 status."]
        #[inline(always)]
        pub const fn step_status_3(
            self,
        ) -> crate::common::Reg<regs::step_status_3, crate::common::R> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x2cusize) as _) }
        }
        #[doc = "motor 4 status."]
        #[inline(always)]
        pub const fn step_status_4(
            self,
        ) -> crate::common::Reg<regs::step_status_4, crate::common::R> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x30usize) as _) }
        }
        #[doc = "motor 5 status."]
        #[inline(always)]
        pub const fn step_status_5(
            self,
        ) -> crate::common::Reg<regs::step_status_5, crate::common::R> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x34usize) as _) }
        }
        #[doc = "motor 6 status."]
        #[inline(always)]
        pub const fn step_status_6(
            self,
        ) -> crate::common::Reg<regs::step_status_6, crate::common::R> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x38usize) as _) }
        }
        #[doc = "motor 7 status."]
        #[inline(always)]
        pub const fn step_status_7(
            self,
        ) -> crate::common::Reg<regs::step_status_7, crate::common::R> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x3cusize) as _) }
        }
        #[doc = "motor 0 current absolute position, in steps (signed, two's complement)."]
        #[inline(always)]
        pub const fn step_pos_0(self) -> crate::common::Reg<regs::step_pos_0, crate::common::R> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x40usize) as _) }
        }
        #[doc = "motor 1 current absolute position, in steps (signed, two's complement)."]
        #[inline(always)]
        pub const fn step_pos_1(self) -> crate::common::Reg<regs::step_pos_1, crate::common::R> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x44usize) as _) }
        }
        #[doc = "motor 2 current absolute position, in steps (signed, two's complement)."]
        #[inline(always)]
        pub const fn step_pos_2(self) -> crate::common::Reg<regs::step_pos_2, crate::common::R> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x48usize) as _) }
        }
        #[doc = "motor 3 current absolute position, in steps (signed, two's complement)."]
        #[inline(always)]
        pub const fn step_pos_3(self) -> crate::common::Reg<regs::step_pos_3, crate::common::R> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x4cusize) as _) }
        }
        #[doc = "motor 4 current absolute position, in steps (signed, two's complement)."]
        #[inline(always)]
        pub const fn step_pos_4(self) -> crate::common::Reg<regs::step_pos_4, crate::common::R> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x50usize) as _) }
        }
        #[doc = "motor 5 current absolute position, in steps (signed, two's complement)."]
        #[inline(always)]
        pub const fn step_pos_5(self) -> crate::common::Reg<regs::step_pos_5, crate::common::R> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x54usize) as _) }
        }
        #[doc = "motor 6 current absolute position, in steps (signed, two's complement)."]
        #[inline(always)]
        pub const fn step_pos_6(self) -> crate::common::Reg<regs::step_pos_6, crate::common::R> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x58usize) as _) }
        }
        #[doc = "motor 7 current absolute position, in steps (signed, two's complement)."]
        #[inline(always)]
        pub const fn step_pos_7(self) -> crate::common::Reg<regs::step_pos_7, crate::common::R> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x5cusize) as _) }
        }
    }
    pub mod regs {
        #[doc = "start/stop control - WRITE: start/stop strobes per motor (bank 0 = motors 0-3, bank 1 = motors 4-7); READ: per-motor moving status."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct step_ctrl(pub u32);
        impl step_ctrl {
            #[doc = "per-motor moving status (bit N = motor N is moving) - read-only, ignored on write."]
            #[must_use]
            #[inline(always)]
            pub const fn moving(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0xff;
                val as u8
            }
            #[doc = "per-motor moving status (bit N = motor N is moving) - read-only, ignored on write."]
            #[inline(always)]
            pub const fn set_moving(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 0usize)) | (((val as u32) & 0xff) << 0usize);
            }
            #[doc = "start strobe, bit N = motor N (bank 0, motors 0-3) - write-only, always reads as 0."]
            #[must_use]
            #[inline(always)]
            pub const fn start_bank0(&self) -> u8 {
                let val = (self.0 >> 8usize) & 0x0f;
                val as u8
            }
            #[doc = "start strobe, bit N = motor N (bank 0, motors 0-3) - write-only, always reads as 0."]
            #[inline(always)]
            pub const fn set_start_bank0(&mut self, val: u8) {
                self.0 = (self.0 & !(0x0f << 8usize)) | (((val as u32) & 0x0f) << 8usize);
            }
            #[doc = "stop strobe, bit N = motor N (bank 0, motors 0-3) - write-only, always reads as 0."]
            #[must_use]
            #[inline(always)]
            pub const fn stop_bank0(&self) -> u8 {
                let val = (self.0 >> 12usize) & 0x0f;
                val as u8
            }
            #[doc = "stop strobe, bit N = motor N (bank 0, motors 0-3) - write-only, always reads as 0."]
            #[inline(always)]
            pub const fn set_stop_bank0(&mut self, val: u8) {
                self.0 = (self.0 & !(0x0f << 12usize)) | (((val as u32) & 0x0f) << 12usize);
            }
            #[doc = "start strobe, bit N = motor (N+4) (bank 1, motors 4-7) - write-only, always reads as 0."]
            #[must_use]
            #[inline(always)]
            pub const fn start_bank1(&self) -> u8 {
                let val = (self.0 >> 16usize) & 0x0f;
                val as u8
            }
            #[doc = "start strobe, bit N = motor (N+4) (bank 1, motors 4-7) - write-only, always reads as 0."]
            #[inline(always)]
            pub const fn set_start_bank1(&mut self, val: u8) {
                self.0 = (self.0 & !(0x0f << 16usize)) | (((val as u32) & 0x0f) << 16usize);
            }
            #[doc = "stop strobe, bit N = motor (N+4) (bank 1, motors 4-7) - write-only, always reads as 0."]
            #[must_use]
            #[inline(always)]
            pub const fn stop_bank1(&self) -> u8 {
                let val = (self.0 >> 20usize) & 0x0f;
                val as u8
            }
            #[doc = "stop strobe, bit N = motor (N+4) (bank 1, motors 4-7) - write-only, always reads as 0."]
            #[inline(always)]
            pub const fn set_stop_bank1(&mut self, val: u8) {
                self.0 = (self.0 & !(0x0f << 20usize)) | (((val as u32) & 0x0f) << 20usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved(&self) -> u8 {
                let val = (self.0 >> 24usize) & 0xff;
                val as u8
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 24usize)) | (((val as u32) & 0xff) << 24usize);
            }
        }
        impl Default for step_ctrl {
            #[inline(always)]
            fn default() -> step_ctrl {
                step_ctrl(0)
            }
        }
        impl core::fmt::Debug for step_ctrl {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("step_ctrl")
                    .field("moving", &self.moving())
                    .field("start_bank0", &self.start_bank0())
                    .field("stop_bank0", &self.stop_bank0())
                    .field("start_bank1", &self.start_bank1())
                    .field("stop_bank1", &self.stop_bank1())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for step_ctrl {
            fn format(&self, f: defmt::Formatter) {
                defmt :: write ! (f , "step_ctrl {{ moving: {=u8:?}, start_bank0: {=u8:?}, stop_bank0: {=u8:?}, start_bank1: {=u8:?}, stop_bank1: {=u8:?}, reserved: {=u8:?} }}" , self . moving () , self . start_bank0 () , self . stop_bank0 () , self . start_bank1 () , self . stop_bank1 () , self . reserved ())
            }
        }
        #[doc = "per-motor step pulse width preset (tick count - 1). Combined with that motor's bank prescaler in step_pls_prescaler: pulse width = (prescaler+1) * (preset+1) sys_clk cycles."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct step_pls_config(pub u32);
        impl step_pls_config {
            #[doc = "motor 0 pulse-width preset (tick count - 1)."]
            #[must_use]
            #[inline(always)]
            pub const fn preset0(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0x0f;
                val as u8
            }
            #[doc = "motor 0 pulse-width preset (tick count - 1)."]
            #[inline(always)]
            pub const fn set_preset0(&mut self, val: u8) {
                self.0 = (self.0 & !(0x0f << 0usize)) | (((val as u32) & 0x0f) << 0usize);
            }
            #[doc = "motor 1 pulse-width preset (tick count - 1)."]
            #[must_use]
            #[inline(always)]
            pub const fn preset1(&self) -> u8 {
                let val = (self.0 >> 4usize) & 0x0f;
                val as u8
            }
            #[doc = "motor 1 pulse-width preset (tick count - 1)."]
            #[inline(always)]
            pub const fn set_preset1(&mut self, val: u8) {
                self.0 = (self.0 & !(0x0f << 4usize)) | (((val as u32) & 0x0f) << 4usize);
            }
            #[doc = "motor 2 pulse-width preset (tick count - 1)."]
            #[must_use]
            #[inline(always)]
            pub const fn preset2(&self) -> u8 {
                let val = (self.0 >> 8usize) & 0x0f;
                val as u8
            }
            #[doc = "motor 2 pulse-width preset (tick count - 1)."]
            #[inline(always)]
            pub const fn set_preset2(&mut self, val: u8) {
                self.0 = (self.0 & !(0x0f << 8usize)) | (((val as u32) & 0x0f) << 8usize);
            }
            #[doc = "motor 3 pulse-width preset (tick count - 1)."]
            #[must_use]
            #[inline(always)]
            pub const fn preset3(&self) -> u8 {
                let val = (self.0 >> 12usize) & 0x0f;
                val as u8
            }
            #[doc = "motor 3 pulse-width preset (tick count - 1)."]
            #[inline(always)]
            pub const fn set_preset3(&mut self, val: u8) {
                self.0 = (self.0 & !(0x0f << 12usize)) | (((val as u32) & 0x0f) << 12usize);
            }
            #[doc = "motor 4 pulse-width preset (tick count - 1)."]
            #[must_use]
            #[inline(always)]
            pub const fn preset4(&self) -> u8 {
                let val = (self.0 >> 16usize) & 0x0f;
                val as u8
            }
            #[doc = "motor 4 pulse-width preset (tick count - 1)."]
            #[inline(always)]
            pub const fn set_preset4(&mut self, val: u8) {
                self.0 = (self.0 & !(0x0f << 16usize)) | (((val as u32) & 0x0f) << 16usize);
            }
            #[doc = "motor 5 pulse-width preset (tick count - 1)."]
            #[must_use]
            #[inline(always)]
            pub const fn preset5(&self) -> u8 {
                let val = (self.0 >> 20usize) & 0x0f;
                val as u8
            }
            #[doc = "motor 5 pulse-width preset (tick count - 1)."]
            #[inline(always)]
            pub const fn set_preset5(&mut self, val: u8) {
                self.0 = (self.0 & !(0x0f << 20usize)) | (((val as u32) & 0x0f) << 20usize);
            }
            #[doc = "motor 6 pulse-width preset (tick count - 1)."]
            #[must_use]
            #[inline(always)]
            pub const fn preset6(&self) -> u8 {
                let val = (self.0 >> 24usize) & 0x0f;
                val as u8
            }
            #[doc = "motor 6 pulse-width preset (tick count - 1)."]
            #[inline(always)]
            pub const fn set_preset6(&mut self, val: u8) {
                self.0 = (self.0 & !(0x0f << 24usize)) | (((val as u32) & 0x0f) << 24usize);
            }
            #[doc = "motor 7 pulse-width preset (tick count - 1)."]
            #[must_use]
            #[inline(always)]
            pub const fn preset7(&self) -> u8 {
                let val = (self.0 >> 28usize) & 0x0f;
                val as u8
            }
            #[doc = "motor 7 pulse-width preset (tick count - 1)."]
            #[inline(always)]
            pub const fn set_preset7(&mut self, val: u8) {
                self.0 = (self.0 & !(0x0f << 28usize)) | (((val as u32) & 0x0f) << 28usize);
            }
        }
        impl Default for step_pls_config {
            #[inline(always)]
            fn default() -> step_pls_config {
                step_pls_config(0)
            }
        }
        impl core::fmt::Debug for step_pls_config {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("step_pls_config")
                    .field("preset0", &self.preset0())
                    .field("preset1", &self.preset1())
                    .field("preset2", &self.preset2())
                    .field("preset3", &self.preset3())
                    .field("preset4", &self.preset4())
                    .field("preset5", &self.preset5())
                    .field("preset6", &self.preset6())
                    .field("preset7", &self.preset7())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for step_pls_config {
            fn format(&self, f: defmt::Formatter) {
                defmt :: write ! (f , "step_pls_config {{ preset0: {=u8:?}, preset1: {=u8:?}, preset2: {=u8:?}, preset3: {=u8:?}, preset4: {=u8:?}, preset5: {=u8:?}, preset6: {=u8:?}, preset7: {=u8:?} }}" , self . preset0 () , self . preset1 () , self . preset2 () , self . preset3 () , self . preset4 () , self . preset5 () , self . preset6 () , self . preset7 ())
            }
        }
        #[doc = "per-bank step pulse width prescaler (sys_clk cycles per tick - 1). Bank 0 = motors 0-3 (low 16-bit half, bits\\[5:0\\]), bank 1 = motors 4-7 (high 16-bit half, bits\\[21:16\\]) - split this way so either field can widen, or other per-bank config bits can be added, without touching the other bank's half."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct step_pls_prescaler(pub u32);
        impl step_pls_prescaler {
            #[doc = "bank 0 (motors 0-3) prescaler (sys_clk cycles per tick - 1), low 16-bit half."]
            #[must_use]
            #[inline(always)]
            pub const fn prescaler0(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0x3f;
                val as u8
            }
            #[doc = "bank 0 (motors 0-3) prescaler (sys_clk cycles per tick - 1), low 16-bit half."]
            #[inline(always)]
            pub const fn set_prescaler0(&mut self, val: u8) {
                self.0 = (self.0 & !(0x3f << 0usize)) | (((val as u32) & 0x3f) << 0usize);
            }
            #[doc = "reserved, keep at reset value (rest of bank 0's 16-bit half)."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved0(&self) -> u16 {
                let val = (self.0 >> 6usize) & 0x03ff;
                val as u16
            }
            #[doc = "reserved, keep at reset value (rest of bank 0's 16-bit half)."]
            #[inline(always)]
            pub const fn set_reserved0(&mut self, val: u16) {
                self.0 = (self.0 & !(0x03ff << 6usize)) | (((val as u32) & 0x03ff) << 6usize);
            }
            #[doc = "bank 1 (motors 4-7) prescaler (sys_clk cycles per tick - 1), high 16-bit half."]
            #[must_use]
            #[inline(always)]
            pub const fn prescaler1(&self) -> u8 {
                let val = (self.0 >> 16usize) & 0x3f;
                val as u8
            }
            #[doc = "bank 1 (motors 4-7) prescaler (sys_clk cycles per tick - 1), high 16-bit half."]
            #[inline(always)]
            pub const fn set_prescaler1(&mut self, val: u8) {
                self.0 = (self.0 & !(0x3f << 16usize)) | (((val as u32) & 0x3f) << 16usize);
            }
            #[doc = "reserved, keep at reset value (rest of bank 1's 16-bit half)."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved1(&self) -> u16 {
                let val = (self.0 >> 22usize) & 0x03ff;
                val as u16
            }
            #[doc = "reserved, keep at reset value (rest of bank 1's 16-bit half)."]
            #[inline(always)]
            pub const fn set_reserved1(&mut self, val: u16) {
                self.0 = (self.0 & !(0x03ff << 22usize)) | (((val as u32) & 0x03ff) << 22usize);
            }
        }
        impl Default for step_pls_prescaler {
            #[inline(always)]
            fn default() -> step_pls_prescaler {
                step_pls_prescaler(0)
            }
        }
        impl core::fmt::Debug for step_pls_prescaler {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("step_pls_prescaler")
                    .field("prescaler0", &self.prescaler0())
                    .field("reserved0", &self.reserved0())
                    .field("prescaler1", &self.prescaler1())
                    .field("reserved1", &self.reserved1())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for step_pls_prescaler {
            fn format(&self, f: defmt::Formatter) {
                defmt :: write ! (f , "step_pls_prescaler {{ prescaler0: {=u8:?}, reserved0: {=u16:?}, prescaler1: {=u8:?}, reserved1: {=u16:?} }}" , self . prescaler0 () , self . reserved0 () , self . prescaler1 () , self . reserved1 ())
            }
        }
        #[doc = "motor 0 current absolute position, in steps (signed, two's complement)."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct step_pos_0(pub u32);
        impl step_pos_0 {
            #[doc = "current absolute position (signed, two's complement)."]
            #[must_use]
            #[inline(always)]
            pub const fn position(&self) -> u32 {
                let val = (self.0 >> 0usize) & 0xffff_ffff;
                val as u32
            }
            #[doc = "current absolute position (signed, two's complement)."]
            #[inline(always)]
            pub const fn set_position(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0xffff_ffff << 0usize)) | (((val as u32) & 0xffff_ffff) << 0usize);
            }
        }
        impl Default for step_pos_0 {
            #[inline(always)]
            fn default() -> step_pos_0 {
                step_pos_0(0)
            }
        }
        impl core::fmt::Debug for step_pos_0 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("step_pos_0")
                    .field("position", &self.position())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for step_pos_0 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(f, "step_pos_0 {{ position: {=u32:?} }}", self.position())
            }
        }
        #[doc = "motor 1 current absolute position, in steps (signed, two's complement)."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct step_pos_1(pub u32);
        impl step_pos_1 {
            #[doc = "current absolute position (signed, two's complement)."]
            #[must_use]
            #[inline(always)]
            pub const fn position(&self) -> u32 {
                let val = (self.0 >> 0usize) & 0xffff_ffff;
                val as u32
            }
            #[doc = "current absolute position (signed, two's complement)."]
            #[inline(always)]
            pub const fn set_position(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0xffff_ffff << 0usize)) | (((val as u32) & 0xffff_ffff) << 0usize);
            }
        }
        impl Default for step_pos_1 {
            #[inline(always)]
            fn default() -> step_pos_1 {
                step_pos_1(0)
            }
        }
        impl core::fmt::Debug for step_pos_1 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("step_pos_1")
                    .field("position", &self.position())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for step_pos_1 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(f, "step_pos_1 {{ position: {=u32:?} }}", self.position())
            }
        }
        #[doc = "motor 2 current absolute position, in steps (signed, two's complement)."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct step_pos_2(pub u32);
        impl step_pos_2 {
            #[doc = "current absolute position (signed, two's complement)."]
            #[must_use]
            #[inline(always)]
            pub const fn position(&self) -> u32 {
                let val = (self.0 >> 0usize) & 0xffff_ffff;
                val as u32
            }
            #[doc = "current absolute position (signed, two's complement)."]
            #[inline(always)]
            pub const fn set_position(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0xffff_ffff << 0usize)) | (((val as u32) & 0xffff_ffff) << 0usize);
            }
        }
        impl Default for step_pos_2 {
            #[inline(always)]
            fn default() -> step_pos_2 {
                step_pos_2(0)
            }
        }
        impl core::fmt::Debug for step_pos_2 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("step_pos_2")
                    .field("position", &self.position())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for step_pos_2 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(f, "step_pos_2 {{ position: {=u32:?} }}", self.position())
            }
        }
        #[doc = "motor 3 current absolute position, in steps (signed, two's complement)."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct step_pos_3(pub u32);
        impl step_pos_3 {
            #[doc = "current absolute position (signed, two's complement)."]
            #[must_use]
            #[inline(always)]
            pub const fn position(&self) -> u32 {
                let val = (self.0 >> 0usize) & 0xffff_ffff;
                val as u32
            }
            #[doc = "current absolute position (signed, two's complement)."]
            #[inline(always)]
            pub const fn set_position(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0xffff_ffff << 0usize)) | (((val as u32) & 0xffff_ffff) << 0usize);
            }
        }
        impl Default for step_pos_3 {
            #[inline(always)]
            fn default() -> step_pos_3 {
                step_pos_3(0)
            }
        }
        impl core::fmt::Debug for step_pos_3 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("step_pos_3")
                    .field("position", &self.position())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for step_pos_3 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(f, "step_pos_3 {{ position: {=u32:?} }}", self.position())
            }
        }
        #[doc = "motor 4 current absolute position, in steps (signed, two's complement)."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct step_pos_4(pub u32);
        impl step_pos_4 {
            #[doc = "current absolute position (signed, two's complement)."]
            #[must_use]
            #[inline(always)]
            pub const fn position(&self) -> u32 {
                let val = (self.0 >> 0usize) & 0xffff_ffff;
                val as u32
            }
            #[doc = "current absolute position (signed, two's complement)."]
            #[inline(always)]
            pub const fn set_position(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0xffff_ffff << 0usize)) | (((val as u32) & 0xffff_ffff) << 0usize);
            }
        }
        impl Default for step_pos_4 {
            #[inline(always)]
            fn default() -> step_pos_4 {
                step_pos_4(0)
            }
        }
        impl core::fmt::Debug for step_pos_4 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("step_pos_4")
                    .field("position", &self.position())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for step_pos_4 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(f, "step_pos_4 {{ position: {=u32:?} }}", self.position())
            }
        }
        #[doc = "motor 5 current absolute position, in steps (signed, two's complement)."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct step_pos_5(pub u32);
        impl step_pos_5 {
            #[doc = "current absolute position (signed, two's complement)."]
            #[must_use]
            #[inline(always)]
            pub const fn position(&self) -> u32 {
                let val = (self.0 >> 0usize) & 0xffff_ffff;
                val as u32
            }
            #[doc = "current absolute position (signed, two's complement)."]
            #[inline(always)]
            pub const fn set_position(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0xffff_ffff << 0usize)) | (((val as u32) & 0xffff_ffff) << 0usize);
            }
        }
        impl Default for step_pos_5 {
            #[inline(always)]
            fn default() -> step_pos_5 {
                step_pos_5(0)
            }
        }
        impl core::fmt::Debug for step_pos_5 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("step_pos_5")
                    .field("position", &self.position())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for step_pos_5 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(f, "step_pos_5 {{ position: {=u32:?} }}", self.position())
            }
        }
        #[doc = "motor 6 current absolute position, in steps (signed, two's complement)."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct step_pos_6(pub u32);
        impl step_pos_6 {
            #[doc = "current absolute position (signed, two's complement)."]
            #[must_use]
            #[inline(always)]
            pub const fn position(&self) -> u32 {
                let val = (self.0 >> 0usize) & 0xffff_ffff;
                val as u32
            }
            #[doc = "current absolute position (signed, two's complement)."]
            #[inline(always)]
            pub const fn set_position(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0xffff_ffff << 0usize)) | (((val as u32) & 0xffff_ffff) << 0usize);
            }
        }
        impl Default for step_pos_6 {
            #[inline(always)]
            fn default() -> step_pos_6 {
                step_pos_6(0)
            }
        }
        impl core::fmt::Debug for step_pos_6 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("step_pos_6")
                    .field("position", &self.position())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for step_pos_6 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(f, "step_pos_6 {{ position: {=u32:?} }}", self.position())
            }
        }
        #[doc = "motor 7 current absolute position, in steps (signed, two's complement)."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct step_pos_7(pub u32);
        impl step_pos_7 {
            #[doc = "current absolute position (signed, two's complement)."]
            #[must_use]
            #[inline(always)]
            pub const fn position(&self) -> u32 {
                let val = (self.0 >> 0usize) & 0xffff_ffff;
                val as u32
            }
            #[doc = "current absolute position (signed, two's complement)."]
            #[inline(always)]
            pub const fn set_position(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0xffff_ffff << 0usize)) | (((val as u32) & 0xffff_ffff) << 0usize);
            }
        }
        impl Default for step_pos_7 {
            #[inline(always)]
            fn default() -> step_pos_7 {
                step_pos_7(0)
            }
        }
        impl core::fmt::Debug for step_pos_7 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("step_pos_7")
                    .field("position", &self.position())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for step_pos_7 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(f, "step_pos_7 {{ position: {=u32:?} }}", self.position())
            }
        }
        #[doc = "segment table CTST word - command/direction/step-count for the segment addressed by step_tx_config's motor_instance and the current streaming pointer (advances on step_seg_spdm access, not this one)."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct step_seg_ctst(pub u32);
        impl step_seg_ctst {
            #[doc = "number of steps in this segment."]
            #[must_use]
            #[inline(always)]
            pub const fn n_steps(&self) -> u32 {
                let val = (self.0 >> 0usize) & 0x00ff_ffff;
                val as u32
            }
            #[doc = "number of steps in this segment."]
            #[inline(always)]
            pub const fn set_n_steps(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0x00ff_ffff << 0usize)) | (((val as u32) & 0x00ff_ffff) << 0usize);
            }
            #[doc = "segment command."]
            #[must_use]
            #[inline(always)]
            pub const fn cmd(&self) -> super::vals::cmd {
                let val = (self.0 >> 24usize) & 0x03;
                super::vals::cmd::from_bits(val as u8)
            }
            #[doc = "segment command."]
            #[inline(always)]
            pub const fn set_cmd(&mut self, val: super::vals::cmd) {
                self.0 =
                    (self.0 & !(0x03 << 24usize)) | (((val.to_bits() as u32) & 0x03) << 24usize);
            }
            #[doc = "step direction for this segment."]
            #[must_use]
            #[inline(always)]
            pub const fn dir(&self) -> super::vals::dir {
                let val = (self.0 >> 26usize) & 0x01;
                super::vals::dir::from_bits(val as u8)
            }
            #[doc = "step direction for this segment."]
            #[inline(always)]
            pub const fn set_dir(&mut self, val: super::vals::dir) {
                self.0 =
                    (self.0 & !(0x01 << 26usize)) | (((val.to_bits() as u32) & 0x01) << 26usize);
            }
            #[doc = "whether this segment's period ramps up or down progresses."]
            #[must_use]
            #[inline(always)]
            pub const fn ramp(&self) -> super::vals::ramp {
                let val = (self.0 >> 27usize) & 0x01;
                super::vals::ramp::from_bits(val as u8)
            }
            #[doc = "whether this segment's period ramps up or down progresses."]
            #[inline(always)]
            pub const fn set_ramp(&mut self, val: super::vals::ramp) {
                self.0 =
                    (self.0 & !(0x01 << 27usize)) | (((val.to_bits() as u32) & 0x01) << 27usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved(&self) -> u8 {
                let val = (self.0 >> 28usize) & 0x0f;
                val as u8
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved(&mut self, val: u8) {
                self.0 = (self.0 & !(0x0f << 28usize)) | (((val as u32) & 0x0f) << 28usize);
            }
        }
        impl Default for step_seg_ctst {
            #[inline(always)]
            fn default() -> step_seg_ctst {
                step_seg_ctst(0)
            }
        }
        impl core::fmt::Debug for step_seg_ctst {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("step_seg_ctst")
                    .field("n_steps", &self.n_steps())
                    .field("cmd", &self.cmd())
                    .field("dir", &self.dir())
                    .field("ramp", &self.ramp())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for step_seg_ctst {
            fn format(&self, f: defmt::Formatter) {
                defmt :: write ! (f , "step_seg_ctst {{ n_steps: {=u32:?}, cmd: {:?}, dir: {:?}, ramp: {:?}, reserved: {=u8:?} }}" , self . n_steps () , self . cmd () , self . dir () , self . ramp () , self . reserved ())
            }
        }
        #[doc = "segment table SPDM word - start period/delta magnitude for the segment addressed by step_tx_config's motor_instance and the current streaming pointer (advances the streaming pointer on access - write or read)."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct step_seg_spdm(pub u32);
        impl step_seg_spdm {
            #[doc = "per-step period change magnitude, in stepper ticks."]
            #[must_use]
            #[inline(always)]
            pub const fn delta_magnitude(&self) -> u16 {
                let val = (self.0 >> 0usize) & 0xffff;
                val as u16
            }
            #[doc = "per-step period change magnitude, in stepper ticks."]
            #[inline(always)]
            pub const fn set_delta_magnitude(&mut self, val: u16) {
                self.0 = (self.0 & !(0xffff << 0usize)) | (((val as u32) & 0xffff) << 0usize);
            }
            #[doc = "starting period for this segment, in stepper ticks."]
            #[must_use]
            #[inline(always)]
            pub const fn start_period(&self) -> u16 {
                let val = (self.0 >> 16usize) & 0xffff;
                val as u16
            }
            #[doc = "starting period for this segment, in stepper ticks."]
            #[inline(always)]
            pub const fn set_start_period(&mut self, val: u16) {
                self.0 = (self.0 & !(0xffff << 16usize)) | (((val as u32) & 0xffff) << 16usize);
            }
        }
        impl Default for step_seg_spdm {
            #[inline(always)]
            fn default() -> step_seg_spdm {
                step_seg_spdm(0)
            }
        }
        impl core::fmt::Debug for step_seg_spdm {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("step_seg_spdm")
                    .field("delta_magnitude", &self.delta_magnitude())
                    .field("start_period", &self.start_period())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for step_seg_spdm {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "step_seg_spdm {{ delta_magnitude: {=u16:?}, start_period: {=u16:?} }}",
                    self.delta_magnitude(),
                    self.start_period()
                )
            }
        }
        #[doc = "motor 0 status."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct step_status_0(pub u32);
        impl step_status_0 {
            #[doc = "1 = motor is currently moving."]
            #[must_use]
            #[inline(always)]
            pub const fn moving(&self) -> bool {
                let val = (self.0 >> 0usize) & 0x01;
                val != 0
            }
            #[doc = "1 = motor is currently moving."]
            #[inline(always)]
            pub const fn set_moving(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 0usize)) | (((val as u32) & 0x01) << 0usize);
            }
            #[doc = "reserved."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved(&self) -> u32 {
                let val = (self.0 >> 1usize) & 0x7fff_ffff;
                val as u32
            }
            #[doc = "reserved."]
            #[inline(always)]
            pub const fn set_reserved(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0x7fff_ffff << 1usize)) | (((val as u32) & 0x7fff_ffff) << 1usize);
            }
        }
        impl Default for step_status_0 {
            #[inline(always)]
            fn default() -> step_status_0 {
                step_status_0(0)
            }
        }
        impl core::fmt::Debug for step_status_0 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("step_status_0")
                    .field("moving", &self.moving())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for step_status_0 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "step_status_0 {{ moving: {=bool:?}, reserved: {=u32:?} }}",
                    self.moving(),
                    self.reserved()
                )
            }
        }
        #[doc = "motor 1 status."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct step_status_1(pub u32);
        impl step_status_1 {
            #[doc = "1 = motor is currently moving."]
            #[must_use]
            #[inline(always)]
            pub const fn moving(&self) -> bool {
                let val = (self.0 >> 0usize) & 0x01;
                val != 0
            }
            #[doc = "1 = motor is currently moving."]
            #[inline(always)]
            pub const fn set_moving(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 0usize)) | (((val as u32) & 0x01) << 0usize);
            }
            #[doc = "reserved."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved(&self) -> u32 {
                let val = (self.0 >> 1usize) & 0x7fff_ffff;
                val as u32
            }
            #[doc = "reserved."]
            #[inline(always)]
            pub const fn set_reserved(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0x7fff_ffff << 1usize)) | (((val as u32) & 0x7fff_ffff) << 1usize);
            }
        }
        impl Default for step_status_1 {
            #[inline(always)]
            fn default() -> step_status_1 {
                step_status_1(0)
            }
        }
        impl core::fmt::Debug for step_status_1 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("step_status_1")
                    .field("moving", &self.moving())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for step_status_1 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "step_status_1 {{ moving: {=bool:?}, reserved: {=u32:?} }}",
                    self.moving(),
                    self.reserved()
                )
            }
        }
        #[doc = "motor 2 status."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct step_status_2(pub u32);
        impl step_status_2 {
            #[doc = "1 = motor is currently moving."]
            #[must_use]
            #[inline(always)]
            pub const fn moving(&self) -> bool {
                let val = (self.0 >> 0usize) & 0x01;
                val != 0
            }
            #[doc = "1 = motor is currently moving."]
            #[inline(always)]
            pub const fn set_moving(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 0usize)) | (((val as u32) & 0x01) << 0usize);
            }
            #[doc = "reserved."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved(&self) -> u32 {
                let val = (self.0 >> 1usize) & 0x7fff_ffff;
                val as u32
            }
            #[doc = "reserved."]
            #[inline(always)]
            pub const fn set_reserved(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0x7fff_ffff << 1usize)) | (((val as u32) & 0x7fff_ffff) << 1usize);
            }
        }
        impl Default for step_status_2 {
            #[inline(always)]
            fn default() -> step_status_2 {
                step_status_2(0)
            }
        }
        impl core::fmt::Debug for step_status_2 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("step_status_2")
                    .field("moving", &self.moving())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for step_status_2 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "step_status_2 {{ moving: {=bool:?}, reserved: {=u32:?} }}",
                    self.moving(),
                    self.reserved()
                )
            }
        }
        #[doc = "motor 3 status."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct step_status_3(pub u32);
        impl step_status_3 {
            #[doc = "1 = motor is currently moving."]
            #[must_use]
            #[inline(always)]
            pub const fn moving(&self) -> bool {
                let val = (self.0 >> 0usize) & 0x01;
                val != 0
            }
            #[doc = "1 = motor is currently moving."]
            #[inline(always)]
            pub const fn set_moving(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 0usize)) | (((val as u32) & 0x01) << 0usize);
            }
            #[doc = "reserved."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved(&self) -> u32 {
                let val = (self.0 >> 1usize) & 0x7fff_ffff;
                val as u32
            }
            #[doc = "reserved."]
            #[inline(always)]
            pub const fn set_reserved(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0x7fff_ffff << 1usize)) | (((val as u32) & 0x7fff_ffff) << 1usize);
            }
        }
        impl Default for step_status_3 {
            #[inline(always)]
            fn default() -> step_status_3 {
                step_status_3(0)
            }
        }
        impl core::fmt::Debug for step_status_3 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("step_status_3")
                    .field("moving", &self.moving())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for step_status_3 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "step_status_3 {{ moving: {=bool:?}, reserved: {=u32:?} }}",
                    self.moving(),
                    self.reserved()
                )
            }
        }
        #[doc = "motor 4 status."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct step_status_4(pub u32);
        impl step_status_4 {
            #[doc = "1 = motor is currently moving."]
            #[must_use]
            #[inline(always)]
            pub const fn moving(&self) -> bool {
                let val = (self.0 >> 0usize) & 0x01;
                val != 0
            }
            #[doc = "1 = motor is currently moving."]
            #[inline(always)]
            pub const fn set_moving(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 0usize)) | (((val as u32) & 0x01) << 0usize);
            }
            #[doc = "reserved."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved(&self) -> u32 {
                let val = (self.0 >> 1usize) & 0x7fff_ffff;
                val as u32
            }
            #[doc = "reserved."]
            #[inline(always)]
            pub const fn set_reserved(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0x7fff_ffff << 1usize)) | (((val as u32) & 0x7fff_ffff) << 1usize);
            }
        }
        impl Default for step_status_4 {
            #[inline(always)]
            fn default() -> step_status_4 {
                step_status_4(0)
            }
        }
        impl core::fmt::Debug for step_status_4 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("step_status_4")
                    .field("moving", &self.moving())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for step_status_4 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "step_status_4 {{ moving: {=bool:?}, reserved: {=u32:?} }}",
                    self.moving(),
                    self.reserved()
                )
            }
        }
        #[doc = "motor 5 status."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct step_status_5(pub u32);
        impl step_status_5 {
            #[doc = "1 = motor is currently moving."]
            #[must_use]
            #[inline(always)]
            pub const fn moving(&self) -> bool {
                let val = (self.0 >> 0usize) & 0x01;
                val != 0
            }
            #[doc = "1 = motor is currently moving."]
            #[inline(always)]
            pub const fn set_moving(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 0usize)) | (((val as u32) & 0x01) << 0usize);
            }
            #[doc = "reserved."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved(&self) -> u32 {
                let val = (self.0 >> 1usize) & 0x7fff_ffff;
                val as u32
            }
            #[doc = "reserved."]
            #[inline(always)]
            pub const fn set_reserved(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0x7fff_ffff << 1usize)) | (((val as u32) & 0x7fff_ffff) << 1usize);
            }
        }
        impl Default for step_status_5 {
            #[inline(always)]
            fn default() -> step_status_5 {
                step_status_5(0)
            }
        }
        impl core::fmt::Debug for step_status_5 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("step_status_5")
                    .field("moving", &self.moving())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for step_status_5 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "step_status_5 {{ moving: {=bool:?}, reserved: {=u32:?} }}",
                    self.moving(),
                    self.reserved()
                )
            }
        }
        #[doc = "motor 6 status."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct step_status_6(pub u32);
        impl step_status_6 {
            #[doc = "1 = motor is currently moving."]
            #[must_use]
            #[inline(always)]
            pub const fn moving(&self) -> bool {
                let val = (self.0 >> 0usize) & 0x01;
                val != 0
            }
            #[doc = "1 = motor is currently moving."]
            #[inline(always)]
            pub const fn set_moving(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 0usize)) | (((val as u32) & 0x01) << 0usize);
            }
            #[doc = "reserved."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved(&self) -> u32 {
                let val = (self.0 >> 1usize) & 0x7fff_ffff;
                val as u32
            }
            #[doc = "reserved."]
            #[inline(always)]
            pub const fn set_reserved(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0x7fff_ffff << 1usize)) | (((val as u32) & 0x7fff_ffff) << 1usize);
            }
        }
        impl Default for step_status_6 {
            #[inline(always)]
            fn default() -> step_status_6 {
                step_status_6(0)
            }
        }
        impl core::fmt::Debug for step_status_6 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("step_status_6")
                    .field("moving", &self.moving())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for step_status_6 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "step_status_6 {{ moving: {=bool:?}, reserved: {=u32:?} }}",
                    self.moving(),
                    self.reserved()
                )
            }
        }
        #[doc = "motor 7 status."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct step_status_7(pub u32);
        impl step_status_7 {
            #[doc = "1 = motor is currently moving."]
            #[must_use]
            #[inline(always)]
            pub const fn moving(&self) -> bool {
                let val = (self.0 >> 0usize) & 0x01;
                val != 0
            }
            #[doc = "1 = motor is currently moving."]
            #[inline(always)]
            pub const fn set_moving(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 0usize)) | (((val as u32) & 0x01) << 0usize);
            }
            #[doc = "reserved."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved(&self) -> u32 {
                let val = (self.0 >> 1usize) & 0x7fff_ffff;
                val as u32
            }
            #[doc = "reserved."]
            #[inline(always)]
            pub const fn set_reserved(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0x7fff_ffff << 1usize)) | (((val as u32) & 0x7fff_ffff) << 1usize);
            }
        }
        impl Default for step_status_7 {
            #[inline(always)]
            fn default() -> step_status_7 {
                step_status_7(0)
            }
        }
        impl core::fmt::Debug for step_status_7 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("step_status_7")
                    .field("moving", &self.moving())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for step_status_7 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "step_status_7 {{ moving: {=bool:?}, reserved: {=u32:?} }}",
                    self.moving(),
                    self.reserved()
                )
            }
        }
        #[doc = "segment table streaming configuration - selects which motor subsequent step_seg_ctst/step_seg_spdm writes and reads target, and rewinds both the write and read pointers to segment 0."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct step_tx_config(pub u32);
        impl step_tx_config {
            #[doc = "number of segments about to be streamed (informational - clamped internally to the segment table's actual capacity)."]
            #[must_use]
            #[inline(always)]
            pub const fn num_points(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0xff;
                val as u8
            }
            #[doc = "number of segments about to be streamed (informational - clamped internally to the segment table's actual capacity)."]
            #[inline(always)]
            pub const fn set_num_points(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 0usize)) | (((val as u32) & 0xff) << 0usize);
            }
            #[doc = "which motor (0-7) subsequent step_seg_ctst/step_seg_spdm accesses target."]
            #[must_use]
            #[inline(always)]
            pub const fn motor_instance(&self) -> u8 {
                let val = (self.0 >> 8usize) & 0x07;
                val as u8
            }
            #[doc = "which motor (0-7) subsequent step_seg_ctst/step_seg_spdm accesses target."]
            #[inline(always)]
            pub const fn set_motor_instance(&mut self, val: u8) {
                self.0 = (self.0 & !(0x07 << 8usize)) | (((val as u32) & 0x07) << 8usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved(&self) -> u32 {
                let val = (self.0 >> 11usize) & 0x001f_ffff;
                val as u32
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved(&mut self, val: u32) {
                self.0 = (self.0 & !(0x001f_ffff << 11usize))
                    | (((val as u32) & 0x001f_ffff) << 11usize);
            }
        }
        impl Default for step_tx_config {
            #[inline(always)]
            fn default() -> step_tx_config {
                step_tx_config(0)
            }
        }
        impl core::fmt::Debug for step_tx_config {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("step_tx_config")
                    .field("num_points", &self.num_points())
                    .field("motor_instance", &self.motor_instance())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for step_tx_config {
            fn format(&self, f: defmt::Formatter) {
                defmt :: write ! (f , "step_tx_config {{ num_points: {=u8:?}, motor_instance: {=u8:?}, reserved: {=u32:?} }}" , self . num_points () , self . motor_instance () , self . reserved ())
            }
        }
    }
    pub mod vals {
        #[repr(u8)]
        #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum cmd {
            #[doc = "reserved, not a valid command."]
            RESERVED = 0x0,
            #[doc = "move this segment then seamlessly continue into the next segment."]
            MOVE = 0x01,
            #[doc = "move this segment then halt, requiring a fresh start strobe."]
            MOVE_HALT = 0x02,
            #[doc = "move this segment then halt, requiring a fresh start strobe (reserved alias of MOVE_HALT)."]
            MOVE_HALT_WAIT = 0x03,
        }
        impl cmd {
            #[inline(always)]
            pub const fn from_bits(val: u8) -> cmd {
                unsafe { core::mem::transmute(val & 0x03) }
            }
            #[inline(always)]
            pub const fn to_bits(self) -> u8 {
                unsafe { core::mem::transmute(self) }
            }
        }
        impl From<u8> for cmd {
            #[inline(always)]
            fn from(val: u8) -> cmd {
                cmd::from_bits(val)
            }
        }
        impl From<cmd> for u8 {
            #[inline(always)]
            fn from(val: cmd) -> u8 {
                cmd::to_bits(val)
            }
        }
        #[repr(u8)]
        #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum dir {
            #[doc = "forward/increasing position."]
            NORMAL = 0x0,
            #[doc = "reverse/decreasing position."]
            REVERSE = 0x01,
        }
        impl dir {
            #[inline(always)]
            pub const fn from_bits(val: u8) -> dir {
                unsafe { core::mem::transmute(val & 0x01) }
            }
            #[inline(always)]
            pub const fn to_bits(self) -> u8 {
                unsafe { core::mem::transmute(self) }
            }
        }
        impl From<u8> for dir {
            #[inline(always)]
            fn from(val: u8) -> dir {
                dir::from_bits(val)
            }
        }
        impl From<dir> for u8 {
            #[inline(always)]
            fn from(val: dir) -> u8 {
                dir::to_bits(val)
            }
        }
        #[repr(u8)]
        #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum ramp {
            #[doc = "period decreases each step (accelerating)."]
            UP = 0x0,
            #[doc = "period increases each step (decelerating)."]
            DOWN = 0x01,
        }
        impl ramp {
            #[inline(always)]
            pub const fn from_bits(val: u8) -> ramp {
                unsafe { core::mem::transmute(val & 0x01) }
            }
            #[inline(always)]
            pub const fn to_bits(self) -> u8 {
                unsafe { core::mem::transmute(self) }
            }
        }
        impl From<u8> for ramp {
            #[inline(always)]
            fn from(val: u8) -> ramp {
                ramp::from_bits(val)
            }
        }
        impl From<ramp> for u8 {
            #[inline(always)]
            fn from(val: ramp) -> u8 {
                ramp::to_bits(val)
            }
        }
    }
}
pub mod system0 {
    #[doc = "system block 0."]
    #[derive(Copy, Clone, Eq, PartialEq)]
    pub struct system0 {
        ptr: *mut u8,
    }
    unsafe impl Send for system0 {}
    unsafe impl Sync for system0 {}
    impl system0 {
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
        #[doc = "fixed marker value."]
        #[inline(always)]
        pub const fn marker(self) -> crate::common::Reg<regs::marker, crate::common::R> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0xfcusize) as _) }
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
pub mod timer_pwm {
    #[doc = "4 independent 8-bit timers (source clock, 8-bit prescaler, 8-bit auto-reload) feeding 12 flexibly-mapped PWM output channels (PM1-4, OT1-8)."]
    #[derive(Copy, Clone, Eq, PartialEq)]
    pub struct timer_pwm {
        ptr: *mut u8,
    }
    unsafe impl Send for timer_pwm {}
    unsafe impl Sync for timer_pwm {}
    impl timer_pwm {
        #[inline(always)]
        pub const unsafe fn from_ptr(ptr: *mut ()) -> Self {
            Self { ptr: ptr as _ }
        }
        #[inline(always)]
        pub const fn as_ptr(&self) -> *mut () {
            self.ptr as _
        }
        #[doc = "master PWM output control - disabling forces every one of the 12 PWM pins LOW regardless of per-channel enable/polarity, and drives ot_en to its inactive (isolated) level."]
        #[inline(always)]
        pub const fn pwm_ctrl(self) -> crate::common::Reg<regs::pwm_ctrl, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x0usize) as _) }
        }
        #[doc = "global timer counting gate - lets every configured timer be started phase-aligned on the same sys_clk edge, plus a shared prescaler ahead of all 4 timers' own prescalers."]
        #[inline(always)]
        pub const fn tim_sync(self) -> crate::common::Reg<regs::tim_sync, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x04usize) as _) }
        }
        #[doc = "timer 1 control."]
        #[inline(always)]
        pub const fn tim1_ctrl(self) -> crate::common::Reg<regs::tim1_ctrl, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x08usize) as _) }
        }
        #[doc = "timer 1 auto-reload value - the counter wraps to 0 the tick after reaching this value."]
        #[inline(always)]
        pub const fn tim1_arr(self) -> crate::common::Reg<regs::tim1_arr, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x0cusize) as _) }
        }
        #[doc = "timer 1 current counter value - also host-writable, to set an initial counter value before starting via tim_sync."]
        #[inline(always)]
        pub const fn tim1_cnt(self) -> crate::common::Reg<regs::tim1_cnt, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x10usize) as _) }
        }
        #[doc = "timer 2 control."]
        #[inline(always)]
        pub const fn tim2_ctrl(self) -> crate::common::Reg<regs::tim2_ctrl, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x14usize) as _) }
        }
        #[doc = "timer 2 auto-reload value - the counter wraps to 0 the tick after reaching this value."]
        #[inline(always)]
        pub const fn tim2_arr(self) -> crate::common::Reg<regs::tim2_arr, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x18usize) as _) }
        }
        #[doc = "timer 2 current counter value - also host-writable, to set an initial counter value before starting via tim_sync."]
        #[inline(always)]
        pub const fn tim2_cnt(self) -> crate::common::Reg<regs::tim2_cnt, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x1cusize) as _) }
        }
        #[doc = "timer 3 control."]
        #[inline(always)]
        pub const fn tim3_ctrl(self) -> crate::common::Reg<regs::tim3_ctrl, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x20usize) as _) }
        }
        #[doc = "timer 3 auto-reload value - the counter wraps to 0 the tick after reaching this value."]
        #[inline(always)]
        pub const fn tim3_arr(self) -> crate::common::Reg<regs::tim3_arr, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x24usize) as _) }
        }
        #[doc = "timer 3 current counter value - also host-writable, to set an initial counter value before starting via tim_sync."]
        #[inline(always)]
        pub const fn tim3_cnt(self) -> crate::common::Reg<regs::tim3_cnt, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x28usize) as _) }
        }
        #[doc = "timer 4 control."]
        #[inline(always)]
        pub const fn tim4_ctrl(self) -> crate::common::Reg<regs::tim4_ctrl, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x2cusize) as _) }
        }
        #[doc = "timer 4 auto-reload value - the counter wraps to 0 the tick after reaching this value."]
        #[inline(always)]
        pub const fn tim4_arr(self) -> crate::common::Reg<regs::tim4_arr, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x30usize) as _) }
        }
        #[doc = "timer 4 current counter value - also host-writable, to set an initial counter value before starting via tim_sync."]
        #[inline(always)]
        pub const fn tim4_cnt(self) -> crate::common::Reg<regs::tim4_cnt, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x34usize) as _) }
        }
        #[doc = "PWM channel 1 (PM1) control."]
        #[inline(always)]
        pub const fn pwm_ctrl1(self) -> crate::common::Reg<regs::pwm_ctrl1, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x38usize) as _) }
        }
        #[doc = "PWM channel 1 (PM1) compare value."]
        #[inline(always)]
        pub const fn pwm_cmp1(self) -> crate::common::Reg<regs::pwm_cmp1, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x3cusize) as _) }
        }
        #[doc = "PWM channel 2 (PM2) control."]
        #[inline(always)]
        pub const fn pwm_ctrl2(self) -> crate::common::Reg<regs::pwm_ctrl2, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x40usize) as _) }
        }
        #[doc = "PWM channel 2 (PM2) compare value."]
        #[inline(always)]
        pub const fn pwm_cmp2(self) -> crate::common::Reg<regs::pwm_cmp2, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x44usize) as _) }
        }
        #[doc = "PWM channel 3 (PM3) control."]
        #[inline(always)]
        pub const fn pwm_ctrl3(self) -> crate::common::Reg<regs::pwm_ctrl3, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x48usize) as _) }
        }
        #[doc = "PWM channel 3 (PM3) compare value."]
        #[inline(always)]
        pub const fn pwm_cmp3(self) -> crate::common::Reg<regs::pwm_cmp3, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x4cusize) as _) }
        }
        #[doc = "PWM channel 4 (PM4) control."]
        #[inline(always)]
        pub const fn pwm_ctrl4(self) -> crate::common::Reg<regs::pwm_ctrl4, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x50usize) as _) }
        }
        #[doc = "PWM channel 4 (PM4) compare value."]
        #[inline(always)]
        pub const fn pwm_cmp4(self) -> crate::common::Reg<regs::pwm_cmp4, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x54usize) as _) }
        }
        #[doc = "PWM channel 5 (OT1) control."]
        #[inline(always)]
        pub const fn pwm_ctrl5(self) -> crate::common::Reg<regs::pwm_ctrl5, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x58usize) as _) }
        }
        #[doc = "PWM channel 5 (OT1) compare value."]
        #[inline(always)]
        pub const fn pwm_cmp5(self) -> crate::common::Reg<regs::pwm_cmp5, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x5cusize) as _) }
        }
        #[doc = "PWM channel 6 (OT2) control."]
        #[inline(always)]
        pub const fn pwm_ctrl6(self) -> crate::common::Reg<regs::pwm_ctrl6, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x60usize) as _) }
        }
        #[doc = "PWM channel 6 (OT2) compare value."]
        #[inline(always)]
        pub const fn pwm_cmp6(self) -> crate::common::Reg<regs::pwm_cmp6, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x64usize) as _) }
        }
        #[doc = "PWM channel 7 (OT3) control."]
        #[inline(always)]
        pub const fn pwm_ctrl7(self) -> crate::common::Reg<regs::pwm_ctrl7, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x68usize) as _) }
        }
        #[doc = "PWM channel 7 (OT3) compare value."]
        #[inline(always)]
        pub const fn pwm_cmp7(self) -> crate::common::Reg<regs::pwm_cmp7, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x6cusize) as _) }
        }
        #[doc = "PWM channel 8 (OT4) control."]
        #[inline(always)]
        pub const fn pwm_ctrl8(self) -> crate::common::Reg<regs::pwm_ctrl8, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x70usize) as _) }
        }
        #[doc = "PWM channel 8 (OT4) compare value."]
        #[inline(always)]
        pub const fn pwm_cmp8(self) -> crate::common::Reg<regs::pwm_cmp8, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x74usize) as _) }
        }
        #[doc = "PWM channel 9 (OT5) control."]
        #[inline(always)]
        pub const fn pwm_ctrl9(self) -> crate::common::Reg<regs::pwm_ctrl9, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x78usize) as _) }
        }
        #[doc = "PWM channel 9 (OT5) compare value."]
        #[inline(always)]
        pub const fn pwm_cmp9(self) -> crate::common::Reg<regs::pwm_cmp9, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x7cusize) as _) }
        }
        #[doc = "PWM channel 10 (OT6) control."]
        #[inline(always)]
        pub const fn pwm_ctrl10(self) -> crate::common::Reg<regs::pwm_ctrl10, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x80usize) as _) }
        }
        #[doc = "PWM channel 10 (OT6) compare value."]
        #[inline(always)]
        pub const fn pwm_cmp10(self) -> crate::common::Reg<regs::pwm_cmp10, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x84usize) as _) }
        }
        #[doc = "PWM channel 11 (OT7) control."]
        #[inline(always)]
        pub const fn pwm_ctrl11(self) -> crate::common::Reg<regs::pwm_ctrl11, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x88usize) as _) }
        }
        #[doc = "PWM channel 11 (OT7) compare value."]
        #[inline(always)]
        pub const fn pwm_cmp11(self) -> crate::common::Reg<regs::pwm_cmp11, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x8cusize) as _) }
        }
        #[doc = "PWM channel 12 (OT8) control."]
        #[inline(always)]
        pub const fn pwm_ctrl12(self) -> crate::common::Reg<regs::pwm_ctrl12, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x90usize) as _) }
        }
        #[doc = "PWM channel 12 (OT8) compare value."]
        #[inline(always)]
        pub const fn pwm_cmp12(self) -> crate::common::Reg<regs::pwm_cmp12, crate::common::RW> {
            unsafe { crate::common::Reg::from_ptr(self.ptr.wrapping_add(0x94usize) as _) }
        }
    }
    pub mod regs {
        #[doc = "PWM channel 1 (PM1) compare value."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct pwm_cmp1(pub u32);
        impl pwm_cmp1 {
            #[doc = "8-bit compare value, compared against the selected timer's counter."]
            #[must_use]
            #[inline(always)]
            pub const fn value(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0xff;
                val as u8
            }
            #[doc = "8-bit compare value, compared against the selected timer's counter."]
            #[inline(always)]
            pub const fn set_value(&mut self, val: u8) {
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
        impl Default for pwm_cmp1 {
            #[inline(always)]
            fn default() -> pwm_cmp1 {
                pwm_cmp1(0)
            }
        }
        impl core::fmt::Debug for pwm_cmp1 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("pwm_cmp1")
                    .field("value", &self.value())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for pwm_cmp1 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "pwm_cmp1 {{ value: {=u8:?}, reserved: {=u32:?} }}",
                    self.value(),
                    self.reserved()
                )
            }
        }
        #[doc = "PWM channel 10 (OT6) compare value."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct pwm_cmp10(pub u32);
        impl pwm_cmp10 {
            #[doc = "8-bit compare value, compared against the selected timer's counter."]
            #[must_use]
            #[inline(always)]
            pub const fn value(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0xff;
                val as u8
            }
            #[doc = "8-bit compare value, compared against the selected timer's counter."]
            #[inline(always)]
            pub const fn set_value(&mut self, val: u8) {
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
        impl Default for pwm_cmp10 {
            #[inline(always)]
            fn default() -> pwm_cmp10 {
                pwm_cmp10(0)
            }
        }
        impl core::fmt::Debug for pwm_cmp10 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("pwm_cmp10")
                    .field("value", &self.value())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for pwm_cmp10 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "pwm_cmp10 {{ value: {=u8:?}, reserved: {=u32:?} }}",
                    self.value(),
                    self.reserved()
                )
            }
        }
        #[doc = "PWM channel 11 (OT7) compare value."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct pwm_cmp11(pub u32);
        impl pwm_cmp11 {
            #[doc = "8-bit compare value, compared against the selected timer's counter."]
            #[must_use]
            #[inline(always)]
            pub const fn value(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0xff;
                val as u8
            }
            #[doc = "8-bit compare value, compared against the selected timer's counter."]
            #[inline(always)]
            pub const fn set_value(&mut self, val: u8) {
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
        impl Default for pwm_cmp11 {
            #[inline(always)]
            fn default() -> pwm_cmp11 {
                pwm_cmp11(0)
            }
        }
        impl core::fmt::Debug for pwm_cmp11 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("pwm_cmp11")
                    .field("value", &self.value())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for pwm_cmp11 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "pwm_cmp11 {{ value: {=u8:?}, reserved: {=u32:?} }}",
                    self.value(),
                    self.reserved()
                )
            }
        }
        #[doc = "PWM channel 12 (OT8) compare value."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct pwm_cmp12(pub u32);
        impl pwm_cmp12 {
            #[doc = "8-bit compare value, compared against the selected timer's counter."]
            #[must_use]
            #[inline(always)]
            pub const fn value(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0xff;
                val as u8
            }
            #[doc = "8-bit compare value, compared against the selected timer's counter."]
            #[inline(always)]
            pub const fn set_value(&mut self, val: u8) {
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
        impl Default for pwm_cmp12 {
            #[inline(always)]
            fn default() -> pwm_cmp12 {
                pwm_cmp12(0)
            }
        }
        impl core::fmt::Debug for pwm_cmp12 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("pwm_cmp12")
                    .field("value", &self.value())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for pwm_cmp12 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "pwm_cmp12 {{ value: {=u8:?}, reserved: {=u32:?} }}",
                    self.value(),
                    self.reserved()
                )
            }
        }
        #[doc = "PWM channel 2 (PM2) compare value."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct pwm_cmp2(pub u32);
        impl pwm_cmp2 {
            #[doc = "8-bit compare value, compared against the selected timer's counter."]
            #[must_use]
            #[inline(always)]
            pub const fn value(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0xff;
                val as u8
            }
            #[doc = "8-bit compare value, compared against the selected timer's counter."]
            #[inline(always)]
            pub const fn set_value(&mut self, val: u8) {
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
        impl Default for pwm_cmp2 {
            #[inline(always)]
            fn default() -> pwm_cmp2 {
                pwm_cmp2(0)
            }
        }
        impl core::fmt::Debug for pwm_cmp2 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("pwm_cmp2")
                    .field("value", &self.value())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for pwm_cmp2 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "pwm_cmp2 {{ value: {=u8:?}, reserved: {=u32:?} }}",
                    self.value(),
                    self.reserved()
                )
            }
        }
        #[doc = "PWM channel 3 (PM3) compare value."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct pwm_cmp3(pub u32);
        impl pwm_cmp3 {
            #[doc = "8-bit compare value, compared against the selected timer's counter."]
            #[must_use]
            #[inline(always)]
            pub const fn value(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0xff;
                val as u8
            }
            #[doc = "8-bit compare value, compared against the selected timer's counter."]
            #[inline(always)]
            pub const fn set_value(&mut self, val: u8) {
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
        impl Default for pwm_cmp3 {
            #[inline(always)]
            fn default() -> pwm_cmp3 {
                pwm_cmp3(0)
            }
        }
        impl core::fmt::Debug for pwm_cmp3 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("pwm_cmp3")
                    .field("value", &self.value())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for pwm_cmp3 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "pwm_cmp3 {{ value: {=u8:?}, reserved: {=u32:?} }}",
                    self.value(),
                    self.reserved()
                )
            }
        }
        #[doc = "PWM channel 4 (PM4) compare value."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct pwm_cmp4(pub u32);
        impl pwm_cmp4 {
            #[doc = "8-bit compare value, compared against the selected timer's counter."]
            #[must_use]
            #[inline(always)]
            pub const fn value(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0xff;
                val as u8
            }
            #[doc = "8-bit compare value, compared against the selected timer's counter."]
            #[inline(always)]
            pub const fn set_value(&mut self, val: u8) {
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
        impl Default for pwm_cmp4 {
            #[inline(always)]
            fn default() -> pwm_cmp4 {
                pwm_cmp4(0)
            }
        }
        impl core::fmt::Debug for pwm_cmp4 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("pwm_cmp4")
                    .field("value", &self.value())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for pwm_cmp4 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "pwm_cmp4 {{ value: {=u8:?}, reserved: {=u32:?} }}",
                    self.value(),
                    self.reserved()
                )
            }
        }
        #[doc = "PWM channel 5 (OT1) compare value."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct pwm_cmp5(pub u32);
        impl pwm_cmp5 {
            #[doc = "8-bit compare value, compared against the selected timer's counter."]
            #[must_use]
            #[inline(always)]
            pub const fn value(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0xff;
                val as u8
            }
            #[doc = "8-bit compare value, compared against the selected timer's counter."]
            #[inline(always)]
            pub const fn set_value(&mut self, val: u8) {
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
        impl Default for pwm_cmp5 {
            #[inline(always)]
            fn default() -> pwm_cmp5 {
                pwm_cmp5(0)
            }
        }
        impl core::fmt::Debug for pwm_cmp5 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("pwm_cmp5")
                    .field("value", &self.value())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for pwm_cmp5 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "pwm_cmp5 {{ value: {=u8:?}, reserved: {=u32:?} }}",
                    self.value(),
                    self.reserved()
                )
            }
        }
        #[doc = "PWM channel 6 (OT2) compare value."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct pwm_cmp6(pub u32);
        impl pwm_cmp6 {
            #[doc = "8-bit compare value, compared against the selected timer's counter."]
            #[must_use]
            #[inline(always)]
            pub const fn value(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0xff;
                val as u8
            }
            #[doc = "8-bit compare value, compared against the selected timer's counter."]
            #[inline(always)]
            pub const fn set_value(&mut self, val: u8) {
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
        impl Default for pwm_cmp6 {
            #[inline(always)]
            fn default() -> pwm_cmp6 {
                pwm_cmp6(0)
            }
        }
        impl core::fmt::Debug for pwm_cmp6 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("pwm_cmp6")
                    .field("value", &self.value())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for pwm_cmp6 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "pwm_cmp6 {{ value: {=u8:?}, reserved: {=u32:?} }}",
                    self.value(),
                    self.reserved()
                )
            }
        }
        #[doc = "PWM channel 7 (OT3) compare value."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct pwm_cmp7(pub u32);
        impl pwm_cmp7 {
            #[doc = "8-bit compare value, compared against the selected timer's counter."]
            #[must_use]
            #[inline(always)]
            pub const fn value(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0xff;
                val as u8
            }
            #[doc = "8-bit compare value, compared against the selected timer's counter."]
            #[inline(always)]
            pub const fn set_value(&mut self, val: u8) {
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
        impl Default for pwm_cmp7 {
            #[inline(always)]
            fn default() -> pwm_cmp7 {
                pwm_cmp7(0)
            }
        }
        impl core::fmt::Debug for pwm_cmp7 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("pwm_cmp7")
                    .field("value", &self.value())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for pwm_cmp7 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "pwm_cmp7 {{ value: {=u8:?}, reserved: {=u32:?} }}",
                    self.value(),
                    self.reserved()
                )
            }
        }
        #[doc = "PWM channel 8 (OT4) compare value."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct pwm_cmp8(pub u32);
        impl pwm_cmp8 {
            #[doc = "8-bit compare value, compared against the selected timer's counter."]
            #[must_use]
            #[inline(always)]
            pub const fn value(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0xff;
                val as u8
            }
            #[doc = "8-bit compare value, compared against the selected timer's counter."]
            #[inline(always)]
            pub const fn set_value(&mut self, val: u8) {
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
        impl Default for pwm_cmp8 {
            #[inline(always)]
            fn default() -> pwm_cmp8 {
                pwm_cmp8(0)
            }
        }
        impl core::fmt::Debug for pwm_cmp8 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("pwm_cmp8")
                    .field("value", &self.value())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for pwm_cmp8 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "pwm_cmp8 {{ value: {=u8:?}, reserved: {=u32:?} }}",
                    self.value(),
                    self.reserved()
                )
            }
        }
        #[doc = "PWM channel 9 (OT5) compare value."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct pwm_cmp9(pub u32);
        impl pwm_cmp9 {
            #[doc = "8-bit compare value, compared against the selected timer's counter."]
            #[must_use]
            #[inline(always)]
            pub const fn value(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0xff;
                val as u8
            }
            #[doc = "8-bit compare value, compared against the selected timer's counter."]
            #[inline(always)]
            pub const fn set_value(&mut self, val: u8) {
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
        impl Default for pwm_cmp9 {
            #[inline(always)]
            fn default() -> pwm_cmp9 {
                pwm_cmp9(0)
            }
        }
        impl core::fmt::Debug for pwm_cmp9 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("pwm_cmp9")
                    .field("value", &self.value())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for pwm_cmp9 {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "pwm_cmp9 {{ value: {=u8:?}, reserved: {=u32:?} }}",
                    self.value(),
                    self.reserved()
                )
            }
        }
        #[doc = "master PWM output control - disabling forces every one of the 12 PWM pins LOW regardless of per-channel enable/polarity, and drives ot_en to its inactive (isolated) level."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct pwm_ctrl(pub u32);
        impl pwm_ctrl {
            #[doc = "1 = PWM outputs active and ot_en drives OT1-8's isolation buffer enabled; 0 (reset default) = every PWM pin forced LOW and OT1-8 electrically isolated."]
            #[must_use]
            #[inline(always)]
            pub const fn output_enable(&self) -> bool {
                let val = (self.0 >> 0usize) & 0x01;
                val != 0
            }
            #[doc = "1 = PWM outputs active and ot_en drives OT1-8's isolation buffer enabled; 0 (reset default) = every PWM pin forced LOW and OT1-8 electrically isolated."]
            #[inline(always)]
            pub const fn set_output_enable(&mut self, val: bool) {
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
        impl Default for pwm_ctrl {
            #[inline(always)]
            fn default() -> pwm_ctrl {
                pwm_ctrl(0)
            }
        }
        impl core::fmt::Debug for pwm_ctrl {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("pwm_ctrl")
                    .field("output_enable", &self.output_enable())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for pwm_ctrl {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "pwm_ctrl {{ output_enable: {=bool:?}, reserved: {=u32:?} }}",
                    self.output_enable(),
                    self.reserved()
                )
            }
        }
        #[doc = "PWM channel 1 (PM1) control."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct pwm_ctrl1(pub u32);
        impl pwm_ctrl1 {
            #[doc = "1 = this channel drives its pin from its selected timer's comparator; 0 (reset default) = pin forced LOW regardless of polarity."]
            #[must_use]
            #[inline(always)]
            pub const fn enable(&self) -> bool {
                let val = (self.0 >> 0usize) & 0x01;
                val != 0
            }
            #[doc = "1 = this channel drives its pin from its selected timer's comparator; 0 (reset default) = pin forced LOW regardless of polarity."]
            #[inline(always)]
            pub const fn set_enable(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 0usize)) | (((val as u32) & 0x01) << 0usize);
            }
            #[doc = "1 = pin HIGH while counter < compare, LOW once counter >= compare; 0 = pin LOW while counter < compare, HIGH once counter >= compare. Either way the pin returns to its pre-compare level the instant the timer's counter resets to 0 (auto-reload wrap or an explicit timN_ctrl.reset)."]
            #[must_use]
            #[inline(always)]
            pub const fn polarity(&self) -> bool {
                let val = (self.0 >> 1usize) & 0x01;
                val != 0
            }
            #[doc = "1 = pin HIGH while counter < compare, LOW once counter >= compare; 0 = pin LOW while counter < compare, HIGH once counter >= compare. Either way the pin returns to its pre-compare level the instant the timer's counter resets to 0 (auto-reload wrap or an explicit timN_ctrl.reset)."]
            #[inline(always)]
            pub const fn set_polarity(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 1usize)) | (((val as u32) & 0x01) << 1usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved0(&self) -> u8 {
                let val = (self.0 >> 2usize) & 0x03;
                val as u8
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved0(&mut self, val: u8) {
                self.0 = (self.0 & !(0x03 << 2usize)) | (((val as u32) & 0x03) << 2usize);
            }
            #[doc = "which of the 4 timers this channel compares against."]
            #[must_use]
            #[inline(always)]
            pub const fn timer_src(&self) -> super::vals::pwm_ctrl1_timer_src {
                let val = (self.0 >> 4usize) & 0x03;
                super::vals::pwm_ctrl1_timer_src::from_bits(val as u8)
            }
            #[doc = "which of the 4 timers this channel compares against."]
            #[inline(always)]
            pub const fn set_timer_src(&mut self, val: super::vals::pwm_ctrl1_timer_src) {
                self.0 = (self.0 & !(0x03 << 4usize)) | (((val.to_bits() as u32) & 0x03) << 4usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved1(&self) -> u32 {
                let val = (self.0 >> 6usize) & 0x03ff_ffff;
                val as u32
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved1(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0x03ff_ffff << 6usize)) | (((val as u32) & 0x03ff_ffff) << 6usize);
            }
        }
        impl Default for pwm_ctrl1 {
            #[inline(always)]
            fn default() -> pwm_ctrl1 {
                pwm_ctrl1(0)
            }
        }
        impl core::fmt::Debug for pwm_ctrl1 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("pwm_ctrl1")
                    .field("enable", &self.enable())
                    .field("polarity", &self.polarity())
                    .field("reserved0", &self.reserved0())
                    .field("timer_src", &self.timer_src())
                    .field("reserved1", &self.reserved1())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for pwm_ctrl1 {
            fn format(&self, f: defmt::Formatter) {
                defmt :: write ! (f , "pwm_ctrl1 {{ enable: {=bool:?}, polarity: {=bool:?}, reserved0: {=u8:?}, timer_src: {:?}, reserved1: {=u32:?} }}" , self . enable () , self . polarity () , self . reserved0 () , self . timer_src () , self . reserved1 ())
            }
        }
        #[doc = "PWM channel 10 (OT6) control."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct pwm_ctrl10(pub u32);
        impl pwm_ctrl10 {
            #[doc = "1 = this channel drives its pin from its selected timer's comparator; 0 (reset default) = pin forced LOW regardless of polarity."]
            #[must_use]
            #[inline(always)]
            pub const fn enable(&self) -> bool {
                let val = (self.0 >> 0usize) & 0x01;
                val != 0
            }
            #[doc = "1 = this channel drives its pin from its selected timer's comparator; 0 (reset default) = pin forced LOW regardless of polarity."]
            #[inline(always)]
            pub const fn set_enable(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 0usize)) | (((val as u32) & 0x01) << 0usize);
            }
            #[doc = "1 = pin HIGH while counter < compare, LOW once counter >= compare; 0 = pin LOW while counter < compare, HIGH once counter >= compare. Either way the pin returns to its pre-compare level the instant the timer's counter resets to 0 (auto-reload wrap or an explicit timN_ctrl.reset)."]
            #[must_use]
            #[inline(always)]
            pub const fn polarity(&self) -> bool {
                let val = (self.0 >> 1usize) & 0x01;
                val != 0
            }
            #[doc = "1 = pin HIGH while counter < compare, LOW once counter >= compare; 0 = pin LOW while counter < compare, HIGH once counter >= compare. Either way the pin returns to its pre-compare level the instant the timer's counter resets to 0 (auto-reload wrap or an explicit timN_ctrl.reset)."]
            #[inline(always)]
            pub const fn set_polarity(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 1usize)) | (((val as u32) & 0x01) << 1usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved0(&self) -> u8 {
                let val = (self.0 >> 2usize) & 0x03;
                val as u8
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved0(&mut self, val: u8) {
                self.0 = (self.0 & !(0x03 << 2usize)) | (((val as u32) & 0x03) << 2usize);
            }
            #[doc = "which of the 4 timers this channel compares against."]
            #[must_use]
            #[inline(always)]
            pub const fn timer_src(&self) -> super::vals::pwm_ctrl10_timer_src {
                let val = (self.0 >> 4usize) & 0x03;
                super::vals::pwm_ctrl10_timer_src::from_bits(val as u8)
            }
            #[doc = "which of the 4 timers this channel compares against."]
            #[inline(always)]
            pub const fn set_timer_src(&mut self, val: super::vals::pwm_ctrl10_timer_src) {
                self.0 = (self.0 & !(0x03 << 4usize)) | (((val.to_bits() as u32) & 0x03) << 4usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved1(&self) -> u32 {
                let val = (self.0 >> 6usize) & 0x03ff_ffff;
                val as u32
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved1(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0x03ff_ffff << 6usize)) | (((val as u32) & 0x03ff_ffff) << 6usize);
            }
        }
        impl Default for pwm_ctrl10 {
            #[inline(always)]
            fn default() -> pwm_ctrl10 {
                pwm_ctrl10(0)
            }
        }
        impl core::fmt::Debug for pwm_ctrl10 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("pwm_ctrl10")
                    .field("enable", &self.enable())
                    .field("polarity", &self.polarity())
                    .field("reserved0", &self.reserved0())
                    .field("timer_src", &self.timer_src())
                    .field("reserved1", &self.reserved1())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for pwm_ctrl10 {
            fn format(&self, f: defmt::Formatter) {
                defmt :: write ! (f , "pwm_ctrl10 {{ enable: {=bool:?}, polarity: {=bool:?}, reserved0: {=u8:?}, timer_src: {:?}, reserved1: {=u32:?} }}" , self . enable () , self . polarity () , self . reserved0 () , self . timer_src () , self . reserved1 ())
            }
        }
        #[doc = "PWM channel 11 (OT7) control."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct pwm_ctrl11(pub u32);
        impl pwm_ctrl11 {
            #[doc = "1 = this channel drives its pin from its selected timer's comparator; 0 (reset default) = pin forced LOW regardless of polarity."]
            #[must_use]
            #[inline(always)]
            pub const fn enable(&self) -> bool {
                let val = (self.0 >> 0usize) & 0x01;
                val != 0
            }
            #[doc = "1 = this channel drives its pin from its selected timer's comparator; 0 (reset default) = pin forced LOW regardless of polarity."]
            #[inline(always)]
            pub const fn set_enable(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 0usize)) | (((val as u32) & 0x01) << 0usize);
            }
            #[doc = "1 = pin HIGH while counter < compare, LOW once counter >= compare; 0 = pin LOW while counter < compare, HIGH once counter >= compare. Either way the pin returns to its pre-compare level the instant the timer's counter resets to 0 (auto-reload wrap or an explicit timN_ctrl.reset)."]
            #[must_use]
            #[inline(always)]
            pub const fn polarity(&self) -> bool {
                let val = (self.0 >> 1usize) & 0x01;
                val != 0
            }
            #[doc = "1 = pin HIGH while counter < compare, LOW once counter >= compare; 0 = pin LOW while counter < compare, HIGH once counter >= compare. Either way the pin returns to its pre-compare level the instant the timer's counter resets to 0 (auto-reload wrap or an explicit timN_ctrl.reset)."]
            #[inline(always)]
            pub const fn set_polarity(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 1usize)) | (((val as u32) & 0x01) << 1usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved0(&self) -> u8 {
                let val = (self.0 >> 2usize) & 0x03;
                val as u8
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved0(&mut self, val: u8) {
                self.0 = (self.0 & !(0x03 << 2usize)) | (((val as u32) & 0x03) << 2usize);
            }
            #[doc = "which of the 4 timers this channel compares against."]
            #[must_use]
            #[inline(always)]
            pub const fn timer_src(&self) -> super::vals::pwm_ctrl11_timer_src {
                let val = (self.0 >> 4usize) & 0x03;
                super::vals::pwm_ctrl11_timer_src::from_bits(val as u8)
            }
            #[doc = "which of the 4 timers this channel compares against."]
            #[inline(always)]
            pub const fn set_timer_src(&mut self, val: super::vals::pwm_ctrl11_timer_src) {
                self.0 = (self.0 & !(0x03 << 4usize)) | (((val.to_bits() as u32) & 0x03) << 4usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved1(&self) -> u32 {
                let val = (self.0 >> 6usize) & 0x03ff_ffff;
                val as u32
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved1(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0x03ff_ffff << 6usize)) | (((val as u32) & 0x03ff_ffff) << 6usize);
            }
        }
        impl Default for pwm_ctrl11 {
            #[inline(always)]
            fn default() -> pwm_ctrl11 {
                pwm_ctrl11(0)
            }
        }
        impl core::fmt::Debug for pwm_ctrl11 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("pwm_ctrl11")
                    .field("enable", &self.enable())
                    .field("polarity", &self.polarity())
                    .field("reserved0", &self.reserved0())
                    .field("timer_src", &self.timer_src())
                    .field("reserved1", &self.reserved1())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for pwm_ctrl11 {
            fn format(&self, f: defmt::Formatter) {
                defmt :: write ! (f , "pwm_ctrl11 {{ enable: {=bool:?}, polarity: {=bool:?}, reserved0: {=u8:?}, timer_src: {:?}, reserved1: {=u32:?} }}" , self . enable () , self . polarity () , self . reserved0 () , self . timer_src () , self . reserved1 ())
            }
        }
        #[doc = "PWM channel 12 (OT8) control."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct pwm_ctrl12(pub u32);
        impl pwm_ctrl12 {
            #[doc = "1 = this channel drives its pin from its selected timer's comparator; 0 (reset default) = pin forced LOW regardless of polarity."]
            #[must_use]
            #[inline(always)]
            pub const fn enable(&self) -> bool {
                let val = (self.0 >> 0usize) & 0x01;
                val != 0
            }
            #[doc = "1 = this channel drives its pin from its selected timer's comparator; 0 (reset default) = pin forced LOW regardless of polarity."]
            #[inline(always)]
            pub const fn set_enable(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 0usize)) | (((val as u32) & 0x01) << 0usize);
            }
            #[doc = "1 = pin HIGH while counter < compare, LOW once counter >= compare; 0 = pin LOW while counter < compare, HIGH once counter >= compare. Either way the pin returns to its pre-compare level the instant the timer's counter resets to 0 (auto-reload wrap or an explicit timN_ctrl.reset)."]
            #[must_use]
            #[inline(always)]
            pub const fn polarity(&self) -> bool {
                let val = (self.0 >> 1usize) & 0x01;
                val != 0
            }
            #[doc = "1 = pin HIGH while counter < compare, LOW once counter >= compare; 0 = pin LOW while counter < compare, HIGH once counter >= compare. Either way the pin returns to its pre-compare level the instant the timer's counter resets to 0 (auto-reload wrap or an explicit timN_ctrl.reset)."]
            #[inline(always)]
            pub const fn set_polarity(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 1usize)) | (((val as u32) & 0x01) << 1usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved0(&self) -> u8 {
                let val = (self.0 >> 2usize) & 0x03;
                val as u8
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved0(&mut self, val: u8) {
                self.0 = (self.0 & !(0x03 << 2usize)) | (((val as u32) & 0x03) << 2usize);
            }
            #[doc = "which of the 4 timers this channel compares against."]
            #[must_use]
            #[inline(always)]
            pub const fn timer_src(&self) -> super::vals::pwm_ctrl12_timer_src {
                let val = (self.0 >> 4usize) & 0x03;
                super::vals::pwm_ctrl12_timer_src::from_bits(val as u8)
            }
            #[doc = "which of the 4 timers this channel compares against."]
            #[inline(always)]
            pub const fn set_timer_src(&mut self, val: super::vals::pwm_ctrl12_timer_src) {
                self.0 = (self.0 & !(0x03 << 4usize)) | (((val.to_bits() as u32) & 0x03) << 4usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved1(&self) -> u32 {
                let val = (self.0 >> 6usize) & 0x03ff_ffff;
                val as u32
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved1(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0x03ff_ffff << 6usize)) | (((val as u32) & 0x03ff_ffff) << 6usize);
            }
        }
        impl Default for pwm_ctrl12 {
            #[inline(always)]
            fn default() -> pwm_ctrl12 {
                pwm_ctrl12(0)
            }
        }
        impl core::fmt::Debug for pwm_ctrl12 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("pwm_ctrl12")
                    .field("enable", &self.enable())
                    .field("polarity", &self.polarity())
                    .field("reserved0", &self.reserved0())
                    .field("timer_src", &self.timer_src())
                    .field("reserved1", &self.reserved1())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for pwm_ctrl12 {
            fn format(&self, f: defmt::Formatter) {
                defmt :: write ! (f , "pwm_ctrl12 {{ enable: {=bool:?}, polarity: {=bool:?}, reserved0: {=u8:?}, timer_src: {:?}, reserved1: {=u32:?} }}" , self . enable () , self . polarity () , self . reserved0 () , self . timer_src () , self . reserved1 ())
            }
        }
        #[doc = "PWM channel 2 (PM2) control."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct pwm_ctrl2(pub u32);
        impl pwm_ctrl2 {
            #[doc = "1 = this channel drives its pin from its selected timer's comparator; 0 (reset default) = pin forced LOW regardless of polarity."]
            #[must_use]
            #[inline(always)]
            pub const fn enable(&self) -> bool {
                let val = (self.0 >> 0usize) & 0x01;
                val != 0
            }
            #[doc = "1 = this channel drives its pin from its selected timer's comparator; 0 (reset default) = pin forced LOW regardless of polarity."]
            #[inline(always)]
            pub const fn set_enable(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 0usize)) | (((val as u32) & 0x01) << 0usize);
            }
            #[doc = "1 = pin HIGH while counter < compare, LOW once counter >= compare; 0 = pin LOW while counter < compare, HIGH once counter >= compare. Either way the pin returns to its pre-compare level the instant the timer's counter resets to 0 (auto-reload wrap or an explicit timN_ctrl.reset)."]
            #[must_use]
            #[inline(always)]
            pub const fn polarity(&self) -> bool {
                let val = (self.0 >> 1usize) & 0x01;
                val != 0
            }
            #[doc = "1 = pin HIGH while counter < compare, LOW once counter >= compare; 0 = pin LOW while counter < compare, HIGH once counter >= compare. Either way the pin returns to its pre-compare level the instant the timer's counter resets to 0 (auto-reload wrap or an explicit timN_ctrl.reset)."]
            #[inline(always)]
            pub const fn set_polarity(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 1usize)) | (((val as u32) & 0x01) << 1usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved0(&self) -> u8 {
                let val = (self.0 >> 2usize) & 0x03;
                val as u8
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved0(&mut self, val: u8) {
                self.0 = (self.0 & !(0x03 << 2usize)) | (((val as u32) & 0x03) << 2usize);
            }
            #[doc = "which of the 4 timers this channel compares against."]
            #[must_use]
            #[inline(always)]
            pub const fn timer_src(&self) -> super::vals::pwm_ctrl2_timer_src {
                let val = (self.0 >> 4usize) & 0x03;
                super::vals::pwm_ctrl2_timer_src::from_bits(val as u8)
            }
            #[doc = "which of the 4 timers this channel compares against."]
            #[inline(always)]
            pub const fn set_timer_src(&mut self, val: super::vals::pwm_ctrl2_timer_src) {
                self.0 = (self.0 & !(0x03 << 4usize)) | (((val.to_bits() as u32) & 0x03) << 4usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved1(&self) -> u32 {
                let val = (self.0 >> 6usize) & 0x03ff_ffff;
                val as u32
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved1(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0x03ff_ffff << 6usize)) | (((val as u32) & 0x03ff_ffff) << 6usize);
            }
        }
        impl Default for pwm_ctrl2 {
            #[inline(always)]
            fn default() -> pwm_ctrl2 {
                pwm_ctrl2(0)
            }
        }
        impl core::fmt::Debug for pwm_ctrl2 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("pwm_ctrl2")
                    .field("enable", &self.enable())
                    .field("polarity", &self.polarity())
                    .field("reserved0", &self.reserved0())
                    .field("timer_src", &self.timer_src())
                    .field("reserved1", &self.reserved1())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for pwm_ctrl2 {
            fn format(&self, f: defmt::Formatter) {
                defmt :: write ! (f , "pwm_ctrl2 {{ enable: {=bool:?}, polarity: {=bool:?}, reserved0: {=u8:?}, timer_src: {:?}, reserved1: {=u32:?} }}" , self . enable () , self . polarity () , self . reserved0 () , self . timer_src () , self . reserved1 ())
            }
        }
        #[doc = "PWM channel 3 (PM3) control."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct pwm_ctrl3(pub u32);
        impl pwm_ctrl3 {
            #[doc = "1 = this channel drives its pin from its selected timer's comparator; 0 (reset default) = pin forced LOW regardless of polarity."]
            #[must_use]
            #[inline(always)]
            pub const fn enable(&self) -> bool {
                let val = (self.0 >> 0usize) & 0x01;
                val != 0
            }
            #[doc = "1 = this channel drives its pin from its selected timer's comparator; 0 (reset default) = pin forced LOW regardless of polarity."]
            #[inline(always)]
            pub const fn set_enable(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 0usize)) | (((val as u32) & 0x01) << 0usize);
            }
            #[doc = "1 = pin HIGH while counter < compare, LOW once counter >= compare; 0 = pin LOW while counter < compare, HIGH once counter >= compare. Either way the pin returns to its pre-compare level the instant the timer's counter resets to 0 (auto-reload wrap or an explicit timN_ctrl.reset)."]
            #[must_use]
            #[inline(always)]
            pub const fn polarity(&self) -> bool {
                let val = (self.0 >> 1usize) & 0x01;
                val != 0
            }
            #[doc = "1 = pin HIGH while counter < compare, LOW once counter >= compare; 0 = pin LOW while counter < compare, HIGH once counter >= compare. Either way the pin returns to its pre-compare level the instant the timer's counter resets to 0 (auto-reload wrap or an explicit timN_ctrl.reset)."]
            #[inline(always)]
            pub const fn set_polarity(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 1usize)) | (((val as u32) & 0x01) << 1usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved0(&self) -> u8 {
                let val = (self.0 >> 2usize) & 0x03;
                val as u8
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved0(&mut self, val: u8) {
                self.0 = (self.0 & !(0x03 << 2usize)) | (((val as u32) & 0x03) << 2usize);
            }
            #[doc = "which of the 4 timers this channel compares against."]
            #[must_use]
            #[inline(always)]
            pub const fn timer_src(&self) -> super::vals::pwm_ctrl3_timer_src {
                let val = (self.0 >> 4usize) & 0x03;
                super::vals::pwm_ctrl3_timer_src::from_bits(val as u8)
            }
            #[doc = "which of the 4 timers this channel compares against."]
            #[inline(always)]
            pub const fn set_timer_src(&mut self, val: super::vals::pwm_ctrl3_timer_src) {
                self.0 = (self.0 & !(0x03 << 4usize)) | (((val.to_bits() as u32) & 0x03) << 4usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved1(&self) -> u32 {
                let val = (self.0 >> 6usize) & 0x03ff_ffff;
                val as u32
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved1(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0x03ff_ffff << 6usize)) | (((val as u32) & 0x03ff_ffff) << 6usize);
            }
        }
        impl Default for pwm_ctrl3 {
            #[inline(always)]
            fn default() -> pwm_ctrl3 {
                pwm_ctrl3(0)
            }
        }
        impl core::fmt::Debug for pwm_ctrl3 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("pwm_ctrl3")
                    .field("enable", &self.enable())
                    .field("polarity", &self.polarity())
                    .field("reserved0", &self.reserved0())
                    .field("timer_src", &self.timer_src())
                    .field("reserved1", &self.reserved1())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for pwm_ctrl3 {
            fn format(&self, f: defmt::Formatter) {
                defmt :: write ! (f , "pwm_ctrl3 {{ enable: {=bool:?}, polarity: {=bool:?}, reserved0: {=u8:?}, timer_src: {:?}, reserved1: {=u32:?} }}" , self . enable () , self . polarity () , self . reserved0 () , self . timer_src () , self . reserved1 ())
            }
        }
        #[doc = "PWM channel 4 (PM4) control."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct pwm_ctrl4(pub u32);
        impl pwm_ctrl4 {
            #[doc = "1 = this channel drives its pin from its selected timer's comparator; 0 (reset default) = pin forced LOW regardless of polarity."]
            #[must_use]
            #[inline(always)]
            pub const fn enable(&self) -> bool {
                let val = (self.0 >> 0usize) & 0x01;
                val != 0
            }
            #[doc = "1 = this channel drives its pin from its selected timer's comparator; 0 (reset default) = pin forced LOW regardless of polarity."]
            #[inline(always)]
            pub const fn set_enable(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 0usize)) | (((val as u32) & 0x01) << 0usize);
            }
            #[doc = "1 = pin HIGH while counter < compare, LOW once counter >= compare; 0 = pin LOW while counter < compare, HIGH once counter >= compare. Either way the pin returns to its pre-compare level the instant the timer's counter resets to 0 (auto-reload wrap or an explicit timN_ctrl.reset)."]
            #[must_use]
            #[inline(always)]
            pub const fn polarity(&self) -> bool {
                let val = (self.0 >> 1usize) & 0x01;
                val != 0
            }
            #[doc = "1 = pin HIGH while counter < compare, LOW once counter >= compare; 0 = pin LOW while counter < compare, HIGH once counter >= compare. Either way the pin returns to its pre-compare level the instant the timer's counter resets to 0 (auto-reload wrap or an explicit timN_ctrl.reset)."]
            #[inline(always)]
            pub const fn set_polarity(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 1usize)) | (((val as u32) & 0x01) << 1usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved0(&self) -> u8 {
                let val = (self.0 >> 2usize) & 0x03;
                val as u8
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved0(&mut self, val: u8) {
                self.0 = (self.0 & !(0x03 << 2usize)) | (((val as u32) & 0x03) << 2usize);
            }
            #[doc = "which of the 4 timers this channel compares against."]
            #[must_use]
            #[inline(always)]
            pub const fn timer_src(&self) -> super::vals::pwm_ctrl4_timer_src {
                let val = (self.0 >> 4usize) & 0x03;
                super::vals::pwm_ctrl4_timer_src::from_bits(val as u8)
            }
            #[doc = "which of the 4 timers this channel compares against."]
            #[inline(always)]
            pub const fn set_timer_src(&mut self, val: super::vals::pwm_ctrl4_timer_src) {
                self.0 = (self.0 & !(0x03 << 4usize)) | (((val.to_bits() as u32) & 0x03) << 4usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved1(&self) -> u32 {
                let val = (self.0 >> 6usize) & 0x03ff_ffff;
                val as u32
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved1(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0x03ff_ffff << 6usize)) | (((val as u32) & 0x03ff_ffff) << 6usize);
            }
        }
        impl Default for pwm_ctrl4 {
            #[inline(always)]
            fn default() -> pwm_ctrl4 {
                pwm_ctrl4(0)
            }
        }
        impl core::fmt::Debug for pwm_ctrl4 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("pwm_ctrl4")
                    .field("enable", &self.enable())
                    .field("polarity", &self.polarity())
                    .field("reserved0", &self.reserved0())
                    .field("timer_src", &self.timer_src())
                    .field("reserved1", &self.reserved1())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for pwm_ctrl4 {
            fn format(&self, f: defmt::Formatter) {
                defmt :: write ! (f , "pwm_ctrl4 {{ enable: {=bool:?}, polarity: {=bool:?}, reserved0: {=u8:?}, timer_src: {:?}, reserved1: {=u32:?} }}" , self . enable () , self . polarity () , self . reserved0 () , self . timer_src () , self . reserved1 ())
            }
        }
        #[doc = "PWM channel 5 (OT1) control."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct pwm_ctrl5(pub u32);
        impl pwm_ctrl5 {
            #[doc = "1 = this channel drives its pin from its selected timer's comparator; 0 (reset default) = pin forced LOW regardless of polarity."]
            #[must_use]
            #[inline(always)]
            pub const fn enable(&self) -> bool {
                let val = (self.0 >> 0usize) & 0x01;
                val != 0
            }
            #[doc = "1 = this channel drives its pin from its selected timer's comparator; 0 (reset default) = pin forced LOW regardless of polarity."]
            #[inline(always)]
            pub const fn set_enable(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 0usize)) | (((val as u32) & 0x01) << 0usize);
            }
            #[doc = "1 = pin HIGH while counter < compare, LOW once counter >= compare; 0 = pin LOW while counter < compare, HIGH once counter >= compare. Either way the pin returns to its pre-compare level the instant the timer's counter resets to 0 (auto-reload wrap or an explicit timN_ctrl.reset)."]
            #[must_use]
            #[inline(always)]
            pub const fn polarity(&self) -> bool {
                let val = (self.0 >> 1usize) & 0x01;
                val != 0
            }
            #[doc = "1 = pin HIGH while counter < compare, LOW once counter >= compare; 0 = pin LOW while counter < compare, HIGH once counter >= compare. Either way the pin returns to its pre-compare level the instant the timer's counter resets to 0 (auto-reload wrap or an explicit timN_ctrl.reset)."]
            #[inline(always)]
            pub const fn set_polarity(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 1usize)) | (((val as u32) & 0x01) << 1usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved0(&self) -> u8 {
                let val = (self.0 >> 2usize) & 0x03;
                val as u8
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved0(&mut self, val: u8) {
                self.0 = (self.0 & !(0x03 << 2usize)) | (((val as u32) & 0x03) << 2usize);
            }
            #[doc = "which of the 4 timers this channel compares against."]
            #[must_use]
            #[inline(always)]
            pub const fn timer_src(&self) -> super::vals::pwm_ctrl5_timer_src {
                let val = (self.0 >> 4usize) & 0x03;
                super::vals::pwm_ctrl5_timer_src::from_bits(val as u8)
            }
            #[doc = "which of the 4 timers this channel compares against."]
            #[inline(always)]
            pub const fn set_timer_src(&mut self, val: super::vals::pwm_ctrl5_timer_src) {
                self.0 = (self.0 & !(0x03 << 4usize)) | (((val.to_bits() as u32) & 0x03) << 4usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved1(&self) -> u32 {
                let val = (self.0 >> 6usize) & 0x03ff_ffff;
                val as u32
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved1(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0x03ff_ffff << 6usize)) | (((val as u32) & 0x03ff_ffff) << 6usize);
            }
        }
        impl Default for pwm_ctrl5 {
            #[inline(always)]
            fn default() -> pwm_ctrl5 {
                pwm_ctrl5(0)
            }
        }
        impl core::fmt::Debug for pwm_ctrl5 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("pwm_ctrl5")
                    .field("enable", &self.enable())
                    .field("polarity", &self.polarity())
                    .field("reserved0", &self.reserved0())
                    .field("timer_src", &self.timer_src())
                    .field("reserved1", &self.reserved1())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for pwm_ctrl5 {
            fn format(&self, f: defmt::Formatter) {
                defmt :: write ! (f , "pwm_ctrl5 {{ enable: {=bool:?}, polarity: {=bool:?}, reserved0: {=u8:?}, timer_src: {:?}, reserved1: {=u32:?} }}" , self . enable () , self . polarity () , self . reserved0 () , self . timer_src () , self . reserved1 ())
            }
        }
        #[doc = "PWM channel 6 (OT2) control."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct pwm_ctrl6(pub u32);
        impl pwm_ctrl6 {
            #[doc = "1 = this channel drives its pin from its selected timer's comparator; 0 (reset default) = pin forced LOW regardless of polarity."]
            #[must_use]
            #[inline(always)]
            pub const fn enable(&self) -> bool {
                let val = (self.0 >> 0usize) & 0x01;
                val != 0
            }
            #[doc = "1 = this channel drives its pin from its selected timer's comparator; 0 (reset default) = pin forced LOW regardless of polarity."]
            #[inline(always)]
            pub const fn set_enable(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 0usize)) | (((val as u32) & 0x01) << 0usize);
            }
            #[doc = "1 = pin HIGH while counter < compare, LOW once counter >= compare; 0 = pin LOW while counter < compare, HIGH once counter >= compare. Either way the pin returns to its pre-compare level the instant the timer's counter resets to 0 (auto-reload wrap or an explicit timN_ctrl.reset)."]
            #[must_use]
            #[inline(always)]
            pub const fn polarity(&self) -> bool {
                let val = (self.0 >> 1usize) & 0x01;
                val != 0
            }
            #[doc = "1 = pin HIGH while counter < compare, LOW once counter >= compare; 0 = pin LOW while counter < compare, HIGH once counter >= compare. Either way the pin returns to its pre-compare level the instant the timer's counter resets to 0 (auto-reload wrap or an explicit timN_ctrl.reset)."]
            #[inline(always)]
            pub const fn set_polarity(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 1usize)) | (((val as u32) & 0x01) << 1usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved0(&self) -> u8 {
                let val = (self.0 >> 2usize) & 0x03;
                val as u8
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved0(&mut self, val: u8) {
                self.0 = (self.0 & !(0x03 << 2usize)) | (((val as u32) & 0x03) << 2usize);
            }
            #[doc = "which of the 4 timers this channel compares against."]
            #[must_use]
            #[inline(always)]
            pub const fn timer_src(&self) -> super::vals::pwm_ctrl6_timer_src {
                let val = (self.0 >> 4usize) & 0x03;
                super::vals::pwm_ctrl6_timer_src::from_bits(val as u8)
            }
            #[doc = "which of the 4 timers this channel compares against."]
            #[inline(always)]
            pub const fn set_timer_src(&mut self, val: super::vals::pwm_ctrl6_timer_src) {
                self.0 = (self.0 & !(0x03 << 4usize)) | (((val.to_bits() as u32) & 0x03) << 4usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved1(&self) -> u32 {
                let val = (self.0 >> 6usize) & 0x03ff_ffff;
                val as u32
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved1(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0x03ff_ffff << 6usize)) | (((val as u32) & 0x03ff_ffff) << 6usize);
            }
        }
        impl Default for pwm_ctrl6 {
            #[inline(always)]
            fn default() -> pwm_ctrl6 {
                pwm_ctrl6(0)
            }
        }
        impl core::fmt::Debug for pwm_ctrl6 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("pwm_ctrl6")
                    .field("enable", &self.enable())
                    .field("polarity", &self.polarity())
                    .field("reserved0", &self.reserved0())
                    .field("timer_src", &self.timer_src())
                    .field("reserved1", &self.reserved1())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for pwm_ctrl6 {
            fn format(&self, f: defmt::Formatter) {
                defmt :: write ! (f , "pwm_ctrl6 {{ enable: {=bool:?}, polarity: {=bool:?}, reserved0: {=u8:?}, timer_src: {:?}, reserved1: {=u32:?} }}" , self . enable () , self . polarity () , self . reserved0 () , self . timer_src () , self . reserved1 ())
            }
        }
        #[doc = "PWM channel 7 (OT3) control."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct pwm_ctrl7(pub u32);
        impl pwm_ctrl7 {
            #[doc = "1 = this channel drives its pin from its selected timer's comparator; 0 (reset default) = pin forced LOW regardless of polarity."]
            #[must_use]
            #[inline(always)]
            pub const fn enable(&self) -> bool {
                let val = (self.0 >> 0usize) & 0x01;
                val != 0
            }
            #[doc = "1 = this channel drives its pin from its selected timer's comparator; 0 (reset default) = pin forced LOW regardless of polarity."]
            #[inline(always)]
            pub const fn set_enable(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 0usize)) | (((val as u32) & 0x01) << 0usize);
            }
            #[doc = "1 = pin HIGH while counter < compare, LOW once counter >= compare; 0 = pin LOW while counter < compare, HIGH once counter >= compare. Either way the pin returns to its pre-compare level the instant the timer's counter resets to 0 (auto-reload wrap or an explicit timN_ctrl.reset)."]
            #[must_use]
            #[inline(always)]
            pub const fn polarity(&self) -> bool {
                let val = (self.0 >> 1usize) & 0x01;
                val != 0
            }
            #[doc = "1 = pin HIGH while counter < compare, LOW once counter >= compare; 0 = pin LOW while counter < compare, HIGH once counter >= compare. Either way the pin returns to its pre-compare level the instant the timer's counter resets to 0 (auto-reload wrap or an explicit timN_ctrl.reset)."]
            #[inline(always)]
            pub const fn set_polarity(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 1usize)) | (((val as u32) & 0x01) << 1usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved0(&self) -> u8 {
                let val = (self.0 >> 2usize) & 0x03;
                val as u8
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved0(&mut self, val: u8) {
                self.0 = (self.0 & !(0x03 << 2usize)) | (((val as u32) & 0x03) << 2usize);
            }
            #[doc = "which of the 4 timers this channel compares against."]
            #[must_use]
            #[inline(always)]
            pub const fn timer_src(&self) -> super::vals::pwm_ctrl7_timer_src {
                let val = (self.0 >> 4usize) & 0x03;
                super::vals::pwm_ctrl7_timer_src::from_bits(val as u8)
            }
            #[doc = "which of the 4 timers this channel compares against."]
            #[inline(always)]
            pub const fn set_timer_src(&mut self, val: super::vals::pwm_ctrl7_timer_src) {
                self.0 = (self.0 & !(0x03 << 4usize)) | (((val.to_bits() as u32) & 0x03) << 4usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved1(&self) -> u32 {
                let val = (self.0 >> 6usize) & 0x03ff_ffff;
                val as u32
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved1(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0x03ff_ffff << 6usize)) | (((val as u32) & 0x03ff_ffff) << 6usize);
            }
        }
        impl Default for pwm_ctrl7 {
            #[inline(always)]
            fn default() -> pwm_ctrl7 {
                pwm_ctrl7(0)
            }
        }
        impl core::fmt::Debug for pwm_ctrl7 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("pwm_ctrl7")
                    .field("enable", &self.enable())
                    .field("polarity", &self.polarity())
                    .field("reserved0", &self.reserved0())
                    .field("timer_src", &self.timer_src())
                    .field("reserved1", &self.reserved1())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for pwm_ctrl7 {
            fn format(&self, f: defmt::Formatter) {
                defmt :: write ! (f , "pwm_ctrl7 {{ enable: {=bool:?}, polarity: {=bool:?}, reserved0: {=u8:?}, timer_src: {:?}, reserved1: {=u32:?} }}" , self . enable () , self . polarity () , self . reserved0 () , self . timer_src () , self . reserved1 ())
            }
        }
        #[doc = "PWM channel 8 (OT4) control."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct pwm_ctrl8(pub u32);
        impl pwm_ctrl8 {
            #[doc = "1 = this channel drives its pin from its selected timer's comparator; 0 (reset default) = pin forced LOW regardless of polarity."]
            #[must_use]
            #[inline(always)]
            pub const fn enable(&self) -> bool {
                let val = (self.0 >> 0usize) & 0x01;
                val != 0
            }
            #[doc = "1 = this channel drives its pin from its selected timer's comparator; 0 (reset default) = pin forced LOW regardless of polarity."]
            #[inline(always)]
            pub const fn set_enable(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 0usize)) | (((val as u32) & 0x01) << 0usize);
            }
            #[doc = "1 = pin HIGH while counter < compare, LOW once counter >= compare; 0 = pin LOW while counter < compare, HIGH once counter >= compare. Either way the pin returns to its pre-compare level the instant the timer's counter resets to 0 (auto-reload wrap or an explicit timN_ctrl.reset)."]
            #[must_use]
            #[inline(always)]
            pub const fn polarity(&self) -> bool {
                let val = (self.0 >> 1usize) & 0x01;
                val != 0
            }
            #[doc = "1 = pin HIGH while counter < compare, LOW once counter >= compare; 0 = pin LOW while counter < compare, HIGH once counter >= compare. Either way the pin returns to its pre-compare level the instant the timer's counter resets to 0 (auto-reload wrap or an explicit timN_ctrl.reset)."]
            #[inline(always)]
            pub const fn set_polarity(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 1usize)) | (((val as u32) & 0x01) << 1usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved0(&self) -> u8 {
                let val = (self.0 >> 2usize) & 0x03;
                val as u8
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved0(&mut self, val: u8) {
                self.0 = (self.0 & !(0x03 << 2usize)) | (((val as u32) & 0x03) << 2usize);
            }
            #[doc = "which of the 4 timers this channel compares against."]
            #[must_use]
            #[inline(always)]
            pub const fn timer_src(&self) -> super::vals::pwm_ctrl8_timer_src {
                let val = (self.0 >> 4usize) & 0x03;
                super::vals::pwm_ctrl8_timer_src::from_bits(val as u8)
            }
            #[doc = "which of the 4 timers this channel compares against."]
            #[inline(always)]
            pub const fn set_timer_src(&mut self, val: super::vals::pwm_ctrl8_timer_src) {
                self.0 = (self.0 & !(0x03 << 4usize)) | (((val.to_bits() as u32) & 0x03) << 4usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved1(&self) -> u32 {
                let val = (self.0 >> 6usize) & 0x03ff_ffff;
                val as u32
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved1(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0x03ff_ffff << 6usize)) | (((val as u32) & 0x03ff_ffff) << 6usize);
            }
        }
        impl Default for pwm_ctrl8 {
            #[inline(always)]
            fn default() -> pwm_ctrl8 {
                pwm_ctrl8(0)
            }
        }
        impl core::fmt::Debug for pwm_ctrl8 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("pwm_ctrl8")
                    .field("enable", &self.enable())
                    .field("polarity", &self.polarity())
                    .field("reserved0", &self.reserved0())
                    .field("timer_src", &self.timer_src())
                    .field("reserved1", &self.reserved1())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for pwm_ctrl8 {
            fn format(&self, f: defmt::Formatter) {
                defmt :: write ! (f , "pwm_ctrl8 {{ enable: {=bool:?}, polarity: {=bool:?}, reserved0: {=u8:?}, timer_src: {:?}, reserved1: {=u32:?} }}" , self . enable () , self . polarity () , self . reserved0 () , self . timer_src () , self . reserved1 ())
            }
        }
        #[doc = "PWM channel 9 (OT5) control."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct pwm_ctrl9(pub u32);
        impl pwm_ctrl9 {
            #[doc = "1 = this channel drives its pin from its selected timer's comparator; 0 (reset default) = pin forced LOW regardless of polarity."]
            #[must_use]
            #[inline(always)]
            pub const fn enable(&self) -> bool {
                let val = (self.0 >> 0usize) & 0x01;
                val != 0
            }
            #[doc = "1 = this channel drives its pin from its selected timer's comparator; 0 (reset default) = pin forced LOW regardless of polarity."]
            #[inline(always)]
            pub const fn set_enable(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 0usize)) | (((val as u32) & 0x01) << 0usize);
            }
            #[doc = "1 = pin HIGH while counter < compare, LOW once counter >= compare; 0 = pin LOW while counter < compare, HIGH once counter >= compare. Either way the pin returns to its pre-compare level the instant the timer's counter resets to 0 (auto-reload wrap or an explicit timN_ctrl.reset)."]
            #[must_use]
            #[inline(always)]
            pub const fn polarity(&self) -> bool {
                let val = (self.0 >> 1usize) & 0x01;
                val != 0
            }
            #[doc = "1 = pin HIGH while counter < compare, LOW once counter >= compare; 0 = pin LOW while counter < compare, HIGH once counter >= compare. Either way the pin returns to its pre-compare level the instant the timer's counter resets to 0 (auto-reload wrap or an explicit timN_ctrl.reset)."]
            #[inline(always)]
            pub const fn set_polarity(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 1usize)) | (((val as u32) & 0x01) << 1usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved0(&self) -> u8 {
                let val = (self.0 >> 2usize) & 0x03;
                val as u8
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved0(&mut self, val: u8) {
                self.0 = (self.0 & !(0x03 << 2usize)) | (((val as u32) & 0x03) << 2usize);
            }
            #[doc = "which of the 4 timers this channel compares against."]
            #[must_use]
            #[inline(always)]
            pub const fn timer_src(&self) -> super::vals::pwm_ctrl9_timer_src {
                let val = (self.0 >> 4usize) & 0x03;
                super::vals::pwm_ctrl9_timer_src::from_bits(val as u8)
            }
            #[doc = "which of the 4 timers this channel compares against."]
            #[inline(always)]
            pub const fn set_timer_src(&mut self, val: super::vals::pwm_ctrl9_timer_src) {
                self.0 = (self.0 & !(0x03 << 4usize)) | (((val.to_bits() as u32) & 0x03) << 4usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved1(&self) -> u32 {
                let val = (self.0 >> 6usize) & 0x03ff_ffff;
                val as u32
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved1(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0x03ff_ffff << 6usize)) | (((val as u32) & 0x03ff_ffff) << 6usize);
            }
        }
        impl Default for pwm_ctrl9 {
            #[inline(always)]
            fn default() -> pwm_ctrl9 {
                pwm_ctrl9(0)
            }
        }
        impl core::fmt::Debug for pwm_ctrl9 {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("pwm_ctrl9")
                    .field("enable", &self.enable())
                    .field("polarity", &self.polarity())
                    .field("reserved0", &self.reserved0())
                    .field("timer_src", &self.timer_src())
                    .field("reserved1", &self.reserved1())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for pwm_ctrl9 {
            fn format(&self, f: defmt::Formatter) {
                defmt :: write ! (f , "pwm_ctrl9 {{ enable: {=bool:?}, polarity: {=bool:?}, reserved0: {=u8:?}, timer_src: {:?}, reserved1: {=u32:?} }}" , self . enable () , self . polarity () , self . reserved0 () , self . timer_src () , self . reserved1 ())
            }
        }
        #[doc = "timer 1 auto-reload value - the counter wraps to 0 the tick after reaching this value."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct tim1_arr(pub u32);
        impl tim1_arr {
            #[doc = "auto-reload value."]
            #[must_use]
            #[inline(always)]
            pub const fn value(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0xff;
                val as u8
            }
            #[doc = "auto-reload value."]
            #[inline(always)]
            pub const fn set_value(&mut self, val: u8) {
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
        impl Default for tim1_arr {
            #[inline(always)]
            fn default() -> tim1_arr {
                tim1_arr(0)
            }
        }
        impl core::fmt::Debug for tim1_arr {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("tim1_arr")
                    .field("value", &self.value())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for tim1_arr {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "tim1_arr {{ value: {=u8:?}, reserved: {=u32:?} }}",
                    self.value(),
                    self.reserved()
                )
            }
        }
        #[doc = "timer 1 current counter value - also host-writable, to set an initial counter value before starting via tim_sync."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct tim1_cnt(pub u32);
        impl tim1_cnt {
            #[doc = "current counter value."]
            #[must_use]
            #[inline(always)]
            pub const fn value(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0xff;
                val as u8
            }
            #[doc = "current counter value."]
            #[inline(always)]
            pub const fn set_value(&mut self, val: u8) {
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
        impl Default for tim1_cnt {
            #[inline(always)]
            fn default() -> tim1_cnt {
                tim1_cnt(0)
            }
        }
        impl core::fmt::Debug for tim1_cnt {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("tim1_cnt")
                    .field("value", &self.value())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for tim1_cnt {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "tim1_cnt {{ value: {=u8:?}, reserved: {=u32:?} }}",
                    self.value(),
                    self.reserved()
                )
            }
        }
        #[doc = "timer 1 control."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct tim1_ctrl(pub u32);
        impl tim1_ctrl {
            #[doc = "1 = this timer counts (subject to tim_sync.enable)."]
            #[must_use]
            #[inline(always)]
            pub const fn enable(&self) -> bool {
                let val = (self.0 >> 0usize) & 0x01;
                val != 0
            }
            #[doc = "1 = this timer counts (subject to tim_sync.enable)."]
            #[inline(always)]
            pub const fn set_enable(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 0usize)) | (((val as u32) & 0x01) << 0usize);
            }
            #[doc = "write 1 to reset this timer's counter to 0. self-clearing, always reads back 0."]
            #[must_use]
            #[inline(always)]
            pub const fn reset(&self) -> bool {
                let val = (self.0 >> 1usize) & 0x01;
                val != 0
            }
            #[doc = "write 1 to reset this timer's counter to 0. self-clearing, always reads back 0."]
            #[inline(always)]
            pub const fn set_reset(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 1usize)) | (((val as u32) & 0x01) << 1usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved0(&self) -> u8 {
                let val = (self.0 >> 2usize) & 0x3f;
                val as u8
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved0(&mut self, val: u8) {
                self.0 = (self.0 & !(0x3f << 2usize)) | (((val as u32) & 0x3f) << 2usize);
            }
            #[doc = "divide-1: the counter advances one tick every (prescaler+1) sys_clk cycles."]
            #[must_use]
            #[inline(always)]
            pub const fn prescaler(&self) -> u8 {
                let val = (self.0 >> 8usize) & 0xff;
                val as u8
            }
            #[doc = "divide-1: the counter advances one tick every (prescaler+1) sys_clk cycles."]
            #[inline(always)]
            pub const fn set_prescaler(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 8usize)) | (((val as u32) & 0xff) << 8usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved1(&self) -> u16 {
                let val = (self.0 >> 16usize) & 0xffff;
                val as u16
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved1(&mut self, val: u16) {
                self.0 = (self.0 & !(0xffff << 16usize)) | (((val as u32) & 0xffff) << 16usize);
            }
        }
        impl Default for tim1_ctrl {
            #[inline(always)]
            fn default() -> tim1_ctrl {
                tim1_ctrl(0)
            }
        }
        impl core::fmt::Debug for tim1_ctrl {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("tim1_ctrl")
                    .field("enable", &self.enable())
                    .field("reset", &self.reset())
                    .field("reserved0", &self.reserved0())
                    .field("prescaler", &self.prescaler())
                    .field("reserved1", &self.reserved1())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for tim1_ctrl {
            fn format(&self, f: defmt::Formatter) {
                defmt :: write ! (f , "tim1_ctrl {{ enable: {=bool:?}, reset: {=bool:?}, reserved0: {=u8:?}, prescaler: {=u8:?}, reserved1: {=u16:?} }}" , self . enable () , self . reset () , self . reserved0 () , self . prescaler () , self . reserved1 ())
            }
        }
        #[doc = "timer 2 auto-reload value - the counter wraps to 0 the tick after reaching this value."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct tim2_arr(pub u32);
        impl tim2_arr {
            #[doc = "auto-reload value."]
            #[must_use]
            #[inline(always)]
            pub const fn value(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0xff;
                val as u8
            }
            #[doc = "auto-reload value."]
            #[inline(always)]
            pub const fn set_value(&mut self, val: u8) {
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
        impl Default for tim2_arr {
            #[inline(always)]
            fn default() -> tim2_arr {
                tim2_arr(0)
            }
        }
        impl core::fmt::Debug for tim2_arr {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("tim2_arr")
                    .field("value", &self.value())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for tim2_arr {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "tim2_arr {{ value: {=u8:?}, reserved: {=u32:?} }}",
                    self.value(),
                    self.reserved()
                )
            }
        }
        #[doc = "timer 2 current counter value - also host-writable, to set an initial counter value before starting via tim_sync."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct tim2_cnt(pub u32);
        impl tim2_cnt {
            #[doc = "current counter value."]
            #[must_use]
            #[inline(always)]
            pub const fn value(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0xff;
                val as u8
            }
            #[doc = "current counter value."]
            #[inline(always)]
            pub const fn set_value(&mut self, val: u8) {
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
        impl Default for tim2_cnt {
            #[inline(always)]
            fn default() -> tim2_cnt {
                tim2_cnt(0)
            }
        }
        impl core::fmt::Debug for tim2_cnt {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("tim2_cnt")
                    .field("value", &self.value())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for tim2_cnt {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "tim2_cnt {{ value: {=u8:?}, reserved: {=u32:?} }}",
                    self.value(),
                    self.reserved()
                )
            }
        }
        #[doc = "timer 2 control."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct tim2_ctrl(pub u32);
        impl tim2_ctrl {
            #[doc = "1 = this timer counts (subject to tim_sync.enable)."]
            #[must_use]
            #[inline(always)]
            pub const fn enable(&self) -> bool {
                let val = (self.0 >> 0usize) & 0x01;
                val != 0
            }
            #[doc = "1 = this timer counts (subject to tim_sync.enable)."]
            #[inline(always)]
            pub const fn set_enable(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 0usize)) | (((val as u32) & 0x01) << 0usize);
            }
            #[doc = "write 1 to reset this timer's counter to 0. self-clearing, always reads back 0."]
            #[must_use]
            #[inline(always)]
            pub const fn reset(&self) -> bool {
                let val = (self.0 >> 1usize) & 0x01;
                val != 0
            }
            #[doc = "write 1 to reset this timer's counter to 0. self-clearing, always reads back 0."]
            #[inline(always)]
            pub const fn set_reset(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 1usize)) | (((val as u32) & 0x01) << 1usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved0(&self) -> u8 {
                let val = (self.0 >> 2usize) & 0x3f;
                val as u8
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved0(&mut self, val: u8) {
                self.0 = (self.0 & !(0x3f << 2usize)) | (((val as u32) & 0x3f) << 2usize);
            }
            #[doc = "divide-1: the counter advances one tick every (prescaler+1) sys_clk cycles."]
            #[must_use]
            #[inline(always)]
            pub const fn prescaler(&self) -> u8 {
                let val = (self.0 >> 8usize) & 0xff;
                val as u8
            }
            #[doc = "divide-1: the counter advances one tick every (prescaler+1) sys_clk cycles."]
            #[inline(always)]
            pub const fn set_prescaler(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 8usize)) | (((val as u32) & 0xff) << 8usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved1(&self) -> u16 {
                let val = (self.0 >> 16usize) & 0xffff;
                val as u16
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved1(&mut self, val: u16) {
                self.0 = (self.0 & !(0xffff << 16usize)) | (((val as u32) & 0xffff) << 16usize);
            }
        }
        impl Default for tim2_ctrl {
            #[inline(always)]
            fn default() -> tim2_ctrl {
                tim2_ctrl(0)
            }
        }
        impl core::fmt::Debug for tim2_ctrl {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("tim2_ctrl")
                    .field("enable", &self.enable())
                    .field("reset", &self.reset())
                    .field("reserved0", &self.reserved0())
                    .field("prescaler", &self.prescaler())
                    .field("reserved1", &self.reserved1())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for tim2_ctrl {
            fn format(&self, f: defmt::Formatter) {
                defmt :: write ! (f , "tim2_ctrl {{ enable: {=bool:?}, reset: {=bool:?}, reserved0: {=u8:?}, prescaler: {=u8:?}, reserved1: {=u16:?} }}" , self . enable () , self . reset () , self . reserved0 () , self . prescaler () , self . reserved1 ())
            }
        }
        #[doc = "timer 3 auto-reload value - the counter wraps to 0 the tick after reaching this value."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct tim3_arr(pub u32);
        impl tim3_arr {
            #[doc = "auto-reload value."]
            #[must_use]
            #[inline(always)]
            pub const fn value(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0xff;
                val as u8
            }
            #[doc = "auto-reload value."]
            #[inline(always)]
            pub const fn set_value(&mut self, val: u8) {
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
        impl Default for tim3_arr {
            #[inline(always)]
            fn default() -> tim3_arr {
                tim3_arr(0)
            }
        }
        impl core::fmt::Debug for tim3_arr {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("tim3_arr")
                    .field("value", &self.value())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for tim3_arr {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "tim3_arr {{ value: {=u8:?}, reserved: {=u32:?} }}",
                    self.value(),
                    self.reserved()
                )
            }
        }
        #[doc = "timer 3 current counter value - also host-writable, to set an initial counter value before starting via tim_sync."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct tim3_cnt(pub u32);
        impl tim3_cnt {
            #[doc = "current counter value."]
            #[must_use]
            #[inline(always)]
            pub const fn value(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0xff;
                val as u8
            }
            #[doc = "current counter value."]
            #[inline(always)]
            pub const fn set_value(&mut self, val: u8) {
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
        impl Default for tim3_cnt {
            #[inline(always)]
            fn default() -> tim3_cnt {
                tim3_cnt(0)
            }
        }
        impl core::fmt::Debug for tim3_cnt {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("tim3_cnt")
                    .field("value", &self.value())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for tim3_cnt {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "tim3_cnt {{ value: {=u8:?}, reserved: {=u32:?} }}",
                    self.value(),
                    self.reserved()
                )
            }
        }
        #[doc = "timer 3 control."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct tim3_ctrl(pub u32);
        impl tim3_ctrl {
            #[doc = "1 = this timer counts (subject to tim_sync.enable)."]
            #[must_use]
            #[inline(always)]
            pub const fn enable(&self) -> bool {
                let val = (self.0 >> 0usize) & 0x01;
                val != 0
            }
            #[doc = "1 = this timer counts (subject to tim_sync.enable)."]
            #[inline(always)]
            pub const fn set_enable(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 0usize)) | (((val as u32) & 0x01) << 0usize);
            }
            #[doc = "write 1 to reset this timer's counter to 0. self-clearing, always reads back 0."]
            #[must_use]
            #[inline(always)]
            pub const fn reset(&self) -> bool {
                let val = (self.0 >> 1usize) & 0x01;
                val != 0
            }
            #[doc = "write 1 to reset this timer's counter to 0. self-clearing, always reads back 0."]
            #[inline(always)]
            pub const fn set_reset(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 1usize)) | (((val as u32) & 0x01) << 1usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved0(&self) -> u8 {
                let val = (self.0 >> 2usize) & 0x3f;
                val as u8
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved0(&mut self, val: u8) {
                self.0 = (self.0 & !(0x3f << 2usize)) | (((val as u32) & 0x3f) << 2usize);
            }
            #[doc = "divide-1: the counter advances one tick every (prescaler+1) sys_clk cycles."]
            #[must_use]
            #[inline(always)]
            pub const fn prescaler(&self) -> u8 {
                let val = (self.0 >> 8usize) & 0xff;
                val as u8
            }
            #[doc = "divide-1: the counter advances one tick every (prescaler+1) sys_clk cycles."]
            #[inline(always)]
            pub const fn set_prescaler(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 8usize)) | (((val as u32) & 0xff) << 8usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved1(&self) -> u16 {
                let val = (self.0 >> 16usize) & 0xffff;
                val as u16
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved1(&mut self, val: u16) {
                self.0 = (self.0 & !(0xffff << 16usize)) | (((val as u32) & 0xffff) << 16usize);
            }
        }
        impl Default for tim3_ctrl {
            #[inline(always)]
            fn default() -> tim3_ctrl {
                tim3_ctrl(0)
            }
        }
        impl core::fmt::Debug for tim3_ctrl {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("tim3_ctrl")
                    .field("enable", &self.enable())
                    .field("reset", &self.reset())
                    .field("reserved0", &self.reserved0())
                    .field("prescaler", &self.prescaler())
                    .field("reserved1", &self.reserved1())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for tim3_ctrl {
            fn format(&self, f: defmt::Formatter) {
                defmt :: write ! (f , "tim3_ctrl {{ enable: {=bool:?}, reset: {=bool:?}, reserved0: {=u8:?}, prescaler: {=u8:?}, reserved1: {=u16:?} }}" , self . enable () , self . reset () , self . reserved0 () , self . prescaler () , self . reserved1 ())
            }
        }
        #[doc = "timer 4 auto-reload value - the counter wraps to 0 the tick after reaching this value."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct tim4_arr(pub u32);
        impl tim4_arr {
            #[doc = "auto-reload value."]
            #[must_use]
            #[inline(always)]
            pub const fn value(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0xff;
                val as u8
            }
            #[doc = "auto-reload value."]
            #[inline(always)]
            pub const fn set_value(&mut self, val: u8) {
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
        impl Default for tim4_arr {
            #[inline(always)]
            fn default() -> tim4_arr {
                tim4_arr(0)
            }
        }
        impl core::fmt::Debug for tim4_arr {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("tim4_arr")
                    .field("value", &self.value())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for tim4_arr {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "tim4_arr {{ value: {=u8:?}, reserved: {=u32:?} }}",
                    self.value(),
                    self.reserved()
                )
            }
        }
        #[doc = "timer 4 current counter value - also host-writable, to set an initial counter value before starting via tim_sync."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct tim4_cnt(pub u32);
        impl tim4_cnt {
            #[doc = "current counter value."]
            #[must_use]
            #[inline(always)]
            pub const fn value(&self) -> u8 {
                let val = (self.0 >> 0usize) & 0xff;
                val as u8
            }
            #[doc = "current counter value."]
            #[inline(always)]
            pub const fn set_value(&mut self, val: u8) {
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
        impl Default for tim4_cnt {
            #[inline(always)]
            fn default() -> tim4_cnt {
                tim4_cnt(0)
            }
        }
        impl core::fmt::Debug for tim4_cnt {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("tim4_cnt")
                    .field("value", &self.value())
                    .field("reserved", &self.reserved())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for tim4_cnt {
            fn format(&self, f: defmt::Formatter) {
                defmt::write!(
                    f,
                    "tim4_cnt {{ value: {=u8:?}, reserved: {=u32:?} }}",
                    self.value(),
                    self.reserved()
                )
            }
        }
        #[doc = "timer 4 control."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct tim4_ctrl(pub u32);
        impl tim4_ctrl {
            #[doc = "1 = this timer counts (subject to tim_sync.enable)."]
            #[must_use]
            #[inline(always)]
            pub const fn enable(&self) -> bool {
                let val = (self.0 >> 0usize) & 0x01;
                val != 0
            }
            #[doc = "1 = this timer counts (subject to tim_sync.enable)."]
            #[inline(always)]
            pub const fn set_enable(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 0usize)) | (((val as u32) & 0x01) << 0usize);
            }
            #[doc = "write 1 to reset this timer's counter to 0. self-clearing, always reads back 0."]
            #[must_use]
            #[inline(always)]
            pub const fn reset(&self) -> bool {
                let val = (self.0 >> 1usize) & 0x01;
                val != 0
            }
            #[doc = "write 1 to reset this timer's counter to 0. self-clearing, always reads back 0."]
            #[inline(always)]
            pub const fn set_reset(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 1usize)) | (((val as u32) & 0x01) << 1usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved0(&self) -> u8 {
                let val = (self.0 >> 2usize) & 0x3f;
                val as u8
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved0(&mut self, val: u8) {
                self.0 = (self.0 & !(0x3f << 2usize)) | (((val as u32) & 0x3f) << 2usize);
            }
            #[doc = "divide-1: the counter advances one tick every (prescaler+1) sys_clk cycles."]
            #[must_use]
            #[inline(always)]
            pub const fn prescaler(&self) -> u8 {
                let val = (self.0 >> 8usize) & 0xff;
                val as u8
            }
            #[doc = "divide-1: the counter advances one tick every (prescaler+1) sys_clk cycles."]
            #[inline(always)]
            pub const fn set_prescaler(&mut self, val: u8) {
                self.0 = (self.0 & !(0xff << 8usize)) | (((val as u32) & 0xff) << 8usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved1(&self) -> u16 {
                let val = (self.0 >> 16usize) & 0xffff;
                val as u16
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved1(&mut self, val: u16) {
                self.0 = (self.0 & !(0xffff << 16usize)) | (((val as u32) & 0xffff) << 16usize);
            }
        }
        impl Default for tim4_ctrl {
            #[inline(always)]
            fn default() -> tim4_ctrl {
                tim4_ctrl(0)
            }
        }
        impl core::fmt::Debug for tim4_ctrl {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("tim4_ctrl")
                    .field("enable", &self.enable())
                    .field("reset", &self.reset())
                    .field("reserved0", &self.reserved0())
                    .field("prescaler", &self.prescaler())
                    .field("reserved1", &self.reserved1())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for tim4_ctrl {
            fn format(&self, f: defmt::Formatter) {
                defmt :: write ! (f , "tim4_ctrl {{ enable: {=bool:?}, reset: {=bool:?}, reserved0: {=u8:?}, prescaler: {=u8:?}, reserved1: {=u16:?} }}" , self . enable () , self . reset () , self . reserved0 () , self . prescaler () , self . reserved1 ())
            }
        }
        #[doc = "global timer counting gate - lets every configured timer be started phase-aligned on the same sys_clk edge, plus a shared prescaler ahead of all 4 timers' own prescalers."]
        #[repr(transparent)]
        #[derive(Copy, Clone, Eq, PartialEq)]
        pub struct tim_sync(pub u32);
        impl tim_sync {
            #[doc = "1 = every timer with its own timN_ctrl.enable set counts; 0 (reset default) = no timer counts, regardless of its own enable bit."]
            #[must_use]
            #[inline(always)]
            pub const fn enable(&self) -> bool {
                let val = (self.0 >> 0usize) & 0x01;
                val != 0
            }
            #[doc = "1 = every timer with its own timN_ctrl.enable set counts; 0 (reset default) = no timer counts, regardless of its own enable bit."]
            #[inline(always)]
            pub const fn set_enable(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 0usize)) | (((val as u32) & 0x01) << 0usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved0(&self) -> bool {
                let val = (self.0 >> 1usize) & 0x01;
                val != 0
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved0(&mut self, val: bool) {
                self.0 = (self.0 & !(0x01 << 1usize)) | (((val as u32) & 0x01) << 1usize);
            }
            #[doc = "divide-1, shared by all 4 timers ahead of each timer's own timN_ctrl.prescaler: a timer only advances on a sys_clk cycle this shared stage ticks on, so the effective divide to one timer tick is (prescaler+1) * (timN_ctrl.prescaler+1) sys_clk cycles. Parked at this value (not counting) whenever enable=0, so every timer's phase relative to it is deterministic the instant sync starts."]
            #[must_use]
            #[inline(always)]
            pub const fn prescaler(&self) -> u8 {
                let val = (self.0 >> 2usize) & 0x3f;
                val as u8
            }
            #[doc = "divide-1, shared by all 4 timers ahead of each timer's own timN_ctrl.prescaler: a timer only advances on a sys_clk cycle this shared stage ticks on, so the effective divide to one timer tick is (prescaler+1) * (timN_ctrl.prescaler+1) sys_clk cycles. Parked at this value (not counting) whenever enable=0, so every timer's phase relative to it is deterministic the instant sync starts."]
            #[inline(always)]
            pub const fn set_prescaler(&mut self, val: u8) {
                self.0 = (self.0 & !(0x3f << 2usize)) | (((val as u32) & 0x3f) << 2usize);
            }
            #[doc = "reserved, keep at reset value."]
            #[must_use]
            #[inline(always)]
            pub const fn reserved1(&self) -> u32 {
                let val = (self.0 >> 8usize) & 0x00ff_ffff;
                val as u32
            }
            #[doc = "reserved, keep at reset value."]
            #[inline(always)]
            pub const fn set_reserved1(&mut self, val: u32) {
                self.0 =
                    (self.0 & !(0x00ff_ffff << 8usize)) | (((val as u32) & 0x00ff_ffff) << 8usize);
            }
        }
        impl Default for tim_sync {
            #[inline(always)]
            fn default() -> tim_sync {
                tim_sync(0)
            }
        }
        impl core::fmt::Debug for tim_sync {
            fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
                f.debug_struct("tim_sync")
                    .field("enable", &self.enable())
                    .field("reserved0", &self.reserved0())
                    .field("prescaler", &self.prescaler())
                    .field("reserved1", &self.reserved1())
                    .finish()
            }
        }
        #[cfg(feature = "defmt")]
        impl defmt::Format for tim_sync {
            fn format(&self, f: defmt::Formatter) {
                defmt :: write ! (f , "tim_sync {{ enable: {=bool:?}, reserved0: {=bool:?}, prescaler: {=u8:?}, reserved1: {=u32:?} }}" , self . enable () , self . reserved0 () , self . prescaler () , self . reserved1 ())
            }
        }
    }
    pub mod vals {
        #[repr(u8)]
        #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum pwm_ctrl10_timer_src {
            #[doc = "compare against timer 1's counter."]
            TIM1 = 0x0,
            #[doc = "compare against timer 2's counter."]
            TIM2 = 0x01,
            #[doc = "compare against timer 3's counter."]
            TIM3 = 0x02,
            #[doc = "compare against timer 4's counter."]
            TIM4 = 0x03,
        }
        impl pwm_ctrl10_timer_src {
            #[inline(always)]
            pub const fn from_bits(val: u8) -> pwm_ctrl10_timer_src {
                unsafe { core::mem::transmute(val & 0x03) }
            }
            #[inline(always)]
            pub const fn to_bits(self) -> u8 {
                unsafe { core::mem::transmute(self) }
            }
        }
        impl From<u8> for pwm_ctrl10_timer_src {
            #[inline(always)]
            fn from(val: u8) -> pwm_ctrl10_timer_src {
                pwm_ctrl10_timer_src::from_bits(val)
            }
        }
        impl From<pwm_ctrl10_timer_src> for u8 {
            #[inline(always)]
            fn from(val: pwm_ctrl10_timer_src) -> u8 {
                pwm_ctrl10_timer_src::to_bits(val)
            }
        }
        #[repr(u8)]
        #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum pwm_ctrl11_timer_src {
            #[doc = "compare against timer 1's counter."]
            TIM1 = 0x0,
            #[doc = "compare against timer 2's counter."]
            TIM2 = 0x01,
            #[doc = "compare against timer 3's counter."]
            TIM3 = 0x02,
            #[doc = "compare against timer 4's counter."]
            TIM4 = 0x03,
        }
        impl pwm_ctrl11_timer_src {
            #[inline(always)]
            pub const fn from_bits(val: u8) -> pwm_ctrl11_timer_src {
                unsafe { core::mem::transmute(val & 0x03) }
            }
            #[inline(always)]
            pub const fn to_bits(self) -> u8 {
                unsafe { core::mem::transmute(self) }
            }
        }
        impl From<u8> for pwm_ctrl11_timer_src {
            #[inline(always)]
            fn from(val: u8) -> pwm_ctrl11_timer_src {
                pwm_ctrl11_timer_src::from_bits(val)
            }
        }
        impl From<pwm_ctrl11_timer_src> for u8 {
            #[inline(always)]
            fn from(val: pwm_ctrl11_timer_src) -> u8 {
                pwm_ctrl11_timer_src::to_bits(val)
            }
        }
        #[repr(u8)]
        #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum pwm_ctrl12_timer_src {
            #[doc = "compare against timer 1's counter."]
            TIM1 = 0x0,
            #[doc = "compare against timer 2's counter."]
            TIM2 = 0x01,
            #[doc = "compare against timer 3's counter."]
            TIM3 = 0x02,
            #[doc = "compare against timer 4's counter."]
            TIM4 = 0x03,
        }
        impl pwm_ctrl12_timer_src {
            #[inline(always)]
            pub const fn from_bits(val: u8) -> pwm_ctrl12_timer_src {
                unsafe { core::mem::transmute(val & 0x03) }
            }
            #[inline(always)]
            pub const fn to_bits(self) -> u8 {
                unsafe { core::mem::transmute(self) }
            }
        }
        impl From<u8> for pwm_ctrl12_timer_src {
            #[inline(always)]
            fn from(val: u8) -> pwm_ctrl12_timer_src {
                pwm_ctrl12_timer_src::from_bits(val)
            }
        }
        impl From<pwm_ctrl12_timer_src> for u8 {
            #[inline(always)]
            fn from(val: pwm_ctrl12_timer_src) -> u8 {
                pwm_ctrl12_timer_src::to_bits(val)
            }
        }
        #[repr(u8)]
        #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum pwm_ctrl1_timer_src {
            #[doc = "compare against timer 1's counter."]
            TIM1 = 0x0,
            #[doc = "compare against timer 2's counter."]
            TIM2 = 0x01,
            #[doc = "compare against timer 3's counter."]
            TIM3 = 0x02,
            #[doc = "compare against timer 4's counter."]
            TIM4 = 0x03,
        }
        impl pwm_ctrl1_timer_src {
            #[inline(always)]
            pub const fn from_bits(val: u8) -> pwm_ctrl1_timer_src {
                unsafe { core::mem::transmute(val & 0x03) }
            }
            #[inline(always)]
            pub const fn to_bits(self) -> u8 {
                unsafe { core::mem::transmute(self) }
            }
        }
        impl From<u8> for pwm_ctrl1_timer_src {
            #[inline(always)]
            fn from(val: u8) -> pwm_ctrl1_timer_src {
                pwm_ctrl1_timer_src::from_bits(val)
            }
        }
        impl From<pwm_ctrl1_timer_src> for u8 {
            #[inline(always)]
            fn from(val: pwm_ctrl1_timer_src) -> u8 {
                pwm_ctrl1_timer_src::to_bits(val)
            }
        }
        #[repr(u8)]
        #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum pwm_ctrl2_timer_src {
            #[doc = "compare against timer 1's counter."]
            TIM1 = 0x0,
            #[doc = "compare against timer 2's counter."]
            TIM2 = 0x01,
            #[doc = "compare against timer 3's counter."]
            TIM3 = 0x02,
            #[doc = "compare against timer 4's counter."]
            TIM4 = 0x03,
        }
        impl pwm_ctrl2_timer_src {
            #[inline(always)]
            pub const fn from_bits(val: u8) -> pwm_ctrl2_timer_src {
                unsafe { core::mem::transmute(val & 0x03) }
            }
            #[inline(always)]
            pub const fn to_bits(self) -> u8 {
                unsafe { core::mem::transmute(self) }
            }
        }
        impl From<u8> for pwm_ctrl2_timer_src {
            #[inline(always)]
            fn from(val: u8) -> pwm_ctrl2_timer_src {
                pwm_ctrl2_timer_src::from_bits(val)
            }
        }
        impl From<pwm_ctrl2_timer_src> for u8 {
            #[inline(always)]
            fn from(val: pwm_ctrl2_timer_src) -> u8 {
                pwm_ctrl2_timer_src::to_bits(val)
            }
        }
        #[repr(u8)]
        #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum pwm_ctrl3_timer_src {
            #[doc = "compare against timer 1's counter."]
            TIM1 = 0x0,
            #[doc = "compare against timer 2's counter."]
            TIM2 = 0x01,
            #[doc = "compare against timer 3's counter."]
            TIM3 = 0x02,
            #[doc = "compare against timer 4's counter."]
            TIM4 = 0x03,
        }
        impl pwm_ctrl3_timer_src {
            #[inline(always)]
            pub const fn from_bits(val: u8) -> pwm_ctrl3_timer_src {
                unsafe { core::mem::transmute(val & 0x03) }
            }
            #[inline(always)]
            pub const fn to_bits(self) -> u8 {
                unsafe { core::mem::transmute(self) }
            }
        }
        impl From<u8> for pwm_ctrl3_timer_src {
            #[inline(always)]
            fn from(val: u8) -> pwm_ctrl3_timer_src {
                pwm_ctrl3_timer_src::from_bits(val)
            }
        }
        impl From<pwm_ctrl3_timer_src> for u8 {
            #[inline(always)]
            fn from(val: pwm_ctrl3_timer_src) -> u8 {
                pwm_ctrl3_timer_src::to_bits(val)
            }
        }
        #[repr(u8)]
        #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum pwm_ctrl4_timer_src {
            #[doc = "compare against timer 1's counter."]
            TIM1 = 0x0,
            #[doc = "compare against timer 2's counter."]
            TIM2 = 0x01,
            #[doc = "compare against timer 3's counter."]
            TIM3 = 0x02,
            #[doc = "compare against timer 4's counter."]
            TIM4 = 0x03,
        }
        impl pwm_ctrl4_timer_src {
            #[inline(always)]
            pub const fn from_bits(val: u8) -> pwm_ctrl4_timer_src {
                unsafe { core::mem::transmute(val & 0x03) }
            }
            #[inline(always)]
            pub const fn to_bits(self) -> u8 {
                unsafe { core::mem::transmute(self) }
            }
        }
        impl From<u8> for pwm_ctrl4_timer_src {
            #[inline(always)]
            fn from(val: u8) -> pwm_ctrl4_timer_src {
                pwm_ctrl4_timer_src::from_bits(val)
            }
        }
        impl From<pwm_ctrl4_timer_src> for u8 {
            #[inline(always)]
            fn from(val: pwm_ctrl4_timer_src) -> u8 {
                pwm_ctrl4_timer_src::to_bits(val)
            }
        }
        #[repr(u8)]
        #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum pwm_ctrl5_timer_src {
            #[doc = "compare against timer 1's counter."]
            TIM1 = 0x0,
            #[doc = "compare against timer 2's counter."]
            TIM2 = 0x01,
            #[doc = "compare against timer 3's counter."]
            TIM3 = 0x02,
            #[doc = "compare against timer 4's counter."]
            TIM4 = 0x03,
        }
        impl pwm_ctrl5_timer_src {
            #[inline(always)]
            pub const fn from_bits(val: u8) -> pwm_ctrl5_timer_src {
                unsafe { core::mem::transmute(val & 0x03) }
            }
            #[inline(always)]
            pub const fn to_bits(self) -> u8 {
                unsafe { core::mem::transmute(self) }
            }
        }
        impl From<u8> for pwm_ctrl5_timer_src {
            #[inline(always)]
            fn from(val: u8) -> pwm_ctrl5_timer_src {
                pwm_ctrl5_timer_src::from_bits(val)
            }
        }
        impl From<pwm_ctrl5_timer_src> for u8 {
            #[inline(always)]
            fn from(val: pwm_ctrl5_timer_src) -> u8 {
                pwm_ctrl5_timer_src::to_bits(val)
            }
        }
        #[repr(u8)]
        #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum pwm_ctrl6_timer_src {
            #[doc = "compare against timer 1's counter."]
            TIM1 = 0x0,
            #[doc = "compare against timer 2's counter."]
            TIM2 = 0x01,
            #[doc = "compare against timer 3's counter."]
            TIM3 = 0x02,
            #[doc = "compare against timer 4's counter."]
            TIM4 = 0x03,
        }
        impl pwm_ctrl6_timer_src {
            #[inline(always)]
            pub const fn from_bits(val: u8) -> pwm_ctrl6_timer_src {
                unsafe { core::mem::transmute(val & 0x03) }
            }
            #[inline(always)]
            pub const fn to_bits(self) -> u8 {
                unsafe { core::mem::transmute(self) }
            }
        }
        impl From<u8> for pwm_ctrl6_timer_src {
            #[inline(always)]
            fn from(val: u8) -> pwm_ctrl6_timer_src {
                pwm_ctrl6_timer_src::from_bits(val)
            }
        }
        impl From<pwm_ctrl6_timer_src> for u8 {
            #[inline(always)]
            fn from(val: pwm_ctrl6_timer_src) -> u8 {
                pwm_ctrl6_timer_src::to_bits(val)
            }
        }
        #[repr(u8)]
        #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum pwm_ctrl7_timer_src {
            #[doc = "compare against timer 1's counter."]
            TIM1 = 0x0,
            #[doc = "compare against timer 2's counter."]
            TIM2 = 0x01,
            #[doc = "compare against timer 3's counter."]
            TIM3 = 0x02,
            #[doc = "compare against timer 4's counter."]
            TIM4 = 0x03,
        }
        impl pwm_ctrl7_timer_src {
            #[inline(always)]
            pub const fn from_bits(val: u8) -> pwm_ctrl7_timer_src {
                unsafe { core::mem::transmute(val & 0x03) }
            }
            #[inline(always)]
            pub const fn to_bits(self) -> u8 {
                unsafe { core::mem::transmute(self) }
            }
        }
        impl From<u8> for pwm_ctrl7_timer_src {
            #[inline(always)]
            fn from(val: u8) -> pwm_ctrl7_timer_src {
                pwm_ctrl7_timer_src::from_bits(val)
            }
        }
        impl From<pwm_ctrl7_timer_src> for u8 {
            #[inline(always)]
            fn from(val: pwm_ctrl7_timer_src) -> u8 {
                pwm_ctrl7_timer_src::to_bits(val)
            }
        }
        #[repr(u8)]
        #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum pwm_ctrl8_timer_src {
            #[doc = "compare against timer 1's counter."]
            TIM1 = 0x0,
            #[doc = "compare against timer 2's counter."]
            TIM2 = 0x01,
            #[doc = "compare against timer 3's counter."]
            TIM3 = 0x02,
            #[doc = "compare against timer 4's counter."]
            TIM4 = 0x03,
        }
        impl pwm_ctrl8_timer_src {
            #[inline(always)]
            pub const fn from_bits(val: u8) -> pwm_ctrl8_timer_src {
                unsafe { core::mem::transmute(val & 0x03) }
            }
            #[inline(always)]
            pub const fn to_bits(self) -> u8 {
                unsafe { core::mem::transmute(self) }
            }
        }
        impl From<u8> for pwm_ctrl8_timer_src {
            #[inline(always)]
            fn from(val: u8) -> pwm_ctrl8_timer_src {
                pwm_ctrl8_timer_src::from_bits(val)
            }
        }
        impl From<pwm_ctrl8_timer_src> for u8 {
            #[inline(always)]
            fn from(val: pwm_ctrl8_timer_src) -> u8 {
                pwm_ctrl8_timer_src::to_bits(val)
            }
        }
        #[repr(u8)]
        #[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
        #[cfg_attr(feature = "defmt", derive(defmt::Format))]
        pub enum pwm_ctrl9_timer_src {
            #[doc = "compare against timer 1's counter."]
            TIM1 = 0x0,
            #[doc = "compare against timer 2's counter."]
            TIM2 = 0x01,
            #[doc = "compare against timer 3's counter."]
            TIM3 = 0x02,
            #[doc = "compare against timer 4's counter."]
            TIM4 = 0x03,
        }
        impl pwm_ctrl9_timer_src {
            #[inline(always)]
            pub const fn from_bits(val: u8) -> pwm_ctrl9_timer_src {
                unsafe { core::mem::transmute(val & 0x03) }
            }
            #[inline(always)]
            pub const fn to_bits(self) -> u8 {
                unsafe { core::mem::transmute(self) }
            }
        }
        impl From<u8> for pwm_ctrl9_timer_src {
            #[inline(always)]
            fn from(val: u8) -> pwm_ctrl9_timer_src {
                pwm_ctrl9_timer_src::from_bits(val)
            }
        }
        impl From<pwm_ctrl9_timer_src> for u8 {
            #[inline(always)]
            fn from(val: pwm_ctrl9_timer_src) -> u8 {
                pwm_ctrl9_timer_src::to_bits(val)
            }
        }
    }
}
pub mod ws2812_0 {
    #[doc = "ws2812 RGB LED block."]
    #[derive(Copy, Clone, Eq, PartialEq)]
    pub struct ws2812_0 {
        ptr: *mut u8,
    }
    unsafe impl Send for ws2812_0 {}
    unsafe impl Sync for ws2812_0 {}
    impl ws2812_0 {
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
