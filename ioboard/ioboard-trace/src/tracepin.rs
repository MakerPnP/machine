use crate::tracepin::storage::TRACE_PINS;

/// Safety: for speed, any pins used are assumed to be initialized to the correct state.
pub trait TracePins {
    fn set_pin_on(&mut self, pin: u8);
    fn set_pin_off(&mut self, pin: u8);

    fn all_off(&mut self);
    fn all_on(&mut self);
}

//
// API to avoid having to pass around a mutable reference to the trace pins
//

#[inline(always)]
pub fn on(pin: u8) {
    #[cfg(feature = "enable")]
    unsafe {
        use crate::tracepin::storage::TRACE_PINS;

        (*TRACE_PINS.assume_init()).set_pin_on(pin);
    }
}

#[inline(always)]
pub fn off(pin: u8) {
    #[cfg(feature = "enable")]
    unsafe {
        use crate::tracepin::storage::TRACE_PINS;

        (*TRACE_PINS.assume_init()).set_pin_off(pin);
    }
}

#[cfg(feature = "enable")]
mod storage {
    use core::mem::MaybeUninit;

    use crate::tracepin::TracePins;

    pub(crate) static mut TRACE_PINS: MaybeUninit<*mut dyn TracePins> = MaybeUninit::uninit();
}

pub fn init<TRACEPINS: TracePins>(trace_pins: TRACEPINS) {
    #[cfg(feature = "enable")]
    unsafe {
        // FUTURE find a no-alloc way to do this in a safe way, avoiding the need for suppressing errors or warnings

        // Leak the trace_pins to give it a true 'static lifetime
        // This is safe because we never try to drop it or access it from multiple places
        let trace_pins_box = alloc::boxed::Box::new(trace_pins);
        let trace_pins_leaked = alloc::boxed::Box::leak(trace_pins_box);
        let trace_pins_ptr: *mut dyn TracePins = trace_pins_leaked as *mut dyn TracePins as _;

        #[allow(static_mut_refs)]
        TRACE_PINS.write(trace_pins_ptr);
    }
}
