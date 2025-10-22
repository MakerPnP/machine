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
pub fn on(_pin: u8) {
    #[cfg(feature = "enable")]
    {
        use storage::TRACE_PINS;
        let (restore_state, instance) = TRACE_PINS.acquire();

        unsafe {
            (*instance).set_pin_on(_pin);
        }
        TRACE_PINS.release(restore_state);
    }
}

#[inline(always)]
pub fn off(_pin: u8) {
    #[cfg(feature = "enable")]
    {
        use storage::TRACE_PINS;
        let (restore_state, instance) = TRACE_PINS.acquire();

        unsafe {
            (*instance).set_pin_off(_pin);
        }
        TRACE_PINS.release(restore_state);
    }
}

#[cfg(feature = "enable")]
mod storage {
    use alloc::boxed::Box;
    use core::cell::UnsafeCell;
    use core::mem::MaybeUninit;
    use core::sync::atomic::{AtomicBool, Ordering};

    use critical_section::RestoreState;

    use crate::tracepin::TracePins;

    pub(crate) static TRACE_PINS: TracePin = TracePin::new();

    pub struct TracePin {
        taken: AtomicBool,
        instance: UnsafeCell<MaybeUninit<*mut dyn TracePins>>,
    }
    impl TracePin {
        const fn new() -> Self {
            Self {
                taken: AtomicBool::new(false),
                instance: UnsafeCell::new(MaybeUninit::uninit()),
            }
        }

        /// Acquire the tracepins.
        pub fn acquire(&self) -> (RestoreState, *mut dyn TracePins) {
            let restore_state = unsafe { critical_section::acquire() };

            #[allow(static_mut_refs)]
            unsafe {
                if self.taken.load(Ordering::Relaxed) {
                    panic!("tracepin taken reentrantly")
                }
                // no need for CAS because we are in a critical section
                self.taken
                    .store(true, Ordering::Relaxed);

                let meh = self.instance.get();
                let foo = (*meh).assume_init();
                (restore_state, foo)
            }
        }

        /// Release the tracepins.
        ///
        /// # Safety
        ///
        /// Do not call unless you have called `acquire`. This will release
        /// your lock - do not call `flush` and `write` until you have done another
        /// `acquire`.
        pub fn release(&self, restore_state: RestoreState) {
            #[allow(static_mut_refs)]
            unsafe {
                if !self.taken.load(Ordering::Relaxed) {
                    panic!("tracepin out of context")
                }
                self.taken
                    .store(false, Ordering::Relaxed);
                // paired with exactly one acquire call
                critical_section::release(restore_state);
            }
        }

        pub(crate) fn init<TRACEPINS: TracePins>(&self, trace_pins: TRACEPINS) {
            // FUTURE find a no-alloc way to do this in a safe way, avoiding the need for suppressing errors or warnings

            // Leak the trace_pins to give it a true 'static lifetime
            // This is safe because we never try to drop it or access it from multiple places
            let trace_pins_box = Box::new(trace_pins);
            let trace_pins_leaked = Box::leak(trace_pins_box);
            let trace_pins_ptr: *mut dyn TracePins = trace_pins_leaked as *mut dyn TracePins as _;

            let (restore_state, _instance) = self.acquire();
            unsafe {
                self.instance
                    .get()
                    .write(MaybeUninit::new(trace_pins_ptr));
            }
            self.release(restore_state);
        }
    }

    unsafe impl Sync for TracePin {}
}

pub fn init<TRACEPINS: TracePins>(trace_pins: TRACEPINS) {
    #[cfg(feature = "enable")]
    storage::TRACE_PINS.init(trace_pins);
}
