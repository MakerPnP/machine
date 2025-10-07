use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;
use ioboard_main::stepper::{Stepper, StepperDirection, StepperError};

#[allow(dead_code)]
pub enum StepperEnableMode {
    /// needs to high voltage to enable
    ActiveHigh,
    /// needs a low voltage to enable
    ActiveLow,
}
pub struct GpioBitbashStepper<PIN1, PIN2, PIN3, DELAY> {
    enable_pin: PIN1,
    step_pin: PIN2,
    direction_pin: PIN3,
    enable_mode: StepperEnableMode,
    delay: DELAY,
    /// pulse width (us)
    pulse_width: u32,
    /// minimum delay between pulses (us)
    pulse_delay: u32,
}

impl<PIN1, PIN2, PIN3, DELAY> GpioBitbashStepper<PIN1, PIN2, PIN3, DELAY> {
    pub fn new(
        enable_pin: PIN1,
        step_pin: PIN2,
        direction_pin: PIN3,
        enable_mode: StepperEnableMode,
        delay: DELAY,
        pulse_width: u32,
        pulse_delay: u32,
    ) -> Self {
        let instance = Self {
            enable_pin,
            step_pin,
            direction_pin,
            enable_mode,
            delay,
            pulse_width,
            pulse_delay,
        };

        instance
    }
}

impl<PIN1, PIN2, PIN3, DELAY> GpioBitbashStepper<PIN1, PIN2, PIN3, DELAY>
where
    PIN1: OutputPin,
    PIN2: OutputPin,
    PIN3: OutputPin,
    DELAY: DelayNs,
{
    pub fn initialize_io(&mut self) -> Result<(), StepperError> {
        self.step_pin
            .set_low()
            .map_err(|_e| StepperError::IoError)?;
        self.direction_pin
            .set_low()
            .map_err(|_e| StepperError::IoError)?;
        self.disable()
            .map_err(|_e| StepperError::IoError)
    }
}

impl<PIN1, PIN2, PIN3, DELAY> Stepper for GpioBitbashStepper<PIN1, PIN2, PIN3, DELAY>
where
    PIN1: OutputPin,
    PIN2: OutputPin,
    PIN3: OutputPin,
    DELAY: DelayNs,
{
    #[inline(always)]
    fn enable(&mut self) -> Result<(), StepperError> {
        match &self.enable_mode {
            StepperEnableMode::ActiveHigh => self.enable_pin.set_high(),
            StepperEnableMode::ActiveLow => self.enable_pin.set_low(),
        }
        .map_err(|_e| StepperError::IoError)
    }

    #[inline(always)]
    fn disable(&mut self) -> Result<(), StepperError> {
        match &self.enable_mode {
            StepperEnableMode::ActiveHigh => self.enable_pin.set_low(),
            StepperEnableMode::ActiveLow => self.enable_pin.set_high(),
        }
        .map_err(|_e| StepperError::IoError)
    }

    #[inline(always)]
    fn direction(&mut self, direction: StepperDirection) -> Result<(), StepperError> {
        match direction {
            StepperDirection::Normal => self.direction_pin.set_low(),
            StepperDirection::Reversed => self.direction_pin.set_high(),
        }
        .map_err(|_e| StepperError::IoError)
    }

    #[inline(always)]
    fn step_and_wait(&mut self) -> Result<(), StepperError> {
        self.step_pin
            .set_high()
            .map_err(|_e| StepperError::IoError)?;
        self.delay.delay_us(self.pulse_width);
        self.step_pin
            .set_low()
            .map_err(|_e| StepperError::IoError)?;
        self.delay.delay_us(self.pulse_delay);
        Ok(())
    }
}
