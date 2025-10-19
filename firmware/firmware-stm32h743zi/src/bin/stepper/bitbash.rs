use embassy_time::{Duration, Instant, Timer};
use embedded_hal::digital::OutputPin;
use ioboard_main::stepper::{Stepper, StepperDirection, StepperError};

#[allow(dead_code)]
pub enum StepperEnableMode {
    /// needs to high voltage to enable
    ActiveHigh,
    /// needs a low voltage to enable
    ActiveLow,
}
pub struct GpioBitbashStepper<PIN1, PIN2, PIN3> {
    enable_pin: PIN1,
    step_pin: PIN2,
    direction_pin: PIN3,
    enable_mode: StepperEnableMode,
    /// pulse width (us)
    pulse_width: u32,
    /// minimum delay between pulses (us)
    pulse_delay: u32,
}

impl<PIN1, PIN2, PIN3> GpioBitbashStepper<PIN1, PIN2, PIN3> {
    pub fn new(
        enable_pin: PIN1,
        step_pin: PIN2,
        direction_pin: PIN3,
        enable_mode: StepperEnableMode,
        pulse_width: u32,
        pulse_delay: u32,
    ) -> Self {
        let instance = Self {
            enable_pin,
            step_pin,
            direction_pin,
            enable_mode,
            pulse_width,
            pulse_delay,
        };

        instance
    }
}

impl<PIN1, PIN2, PIN3> GpioBitbashStepper<PIN1, PIN2, PIN3>
where
    PIN1: OutputPin,
    PIN2: OutputPin,
    PIN3: OutputPin,
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

impl<PIN1, PIN2, PIN3> Stepper for GpioBitbashStepper<PIN1, PIN2, PIN3>
where
    PIN1: OutputPin,
    PIN2: OutputPin,
    PIN3: OutputPin,
{
    fn set_pulse_width_us(&mut self, pulse_width: u32) {
        self.pulse_width = pulse_width;
    }

    fn set_pulse_delay_us(&mut self, pulse_delay: u32) {
        self.pulse_delay = pulse_delay;
    }

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
    async fn step_and_wait(&mut self) -> Result<(), StepperError> {
        let now = Instant::now();
        self.step_pin
            .set_high()
            .map_err(|_e| StepperError::IoError)?;
        let mut deadline = now + Duration::from_micros(self.pulse_width as u64);
        Timer::at(deadline).await;
        self.step_pin
            .set_low()
            .map_err(|_e| StepperError::IoError)?;
        deadline += Duration::from_micros(self.pulse_width as u64);
        Timer::at(deadline).await;
        Ok(())
    }

    #[inline(always)]
    async fn step(&mut self) -> Result<u32, StepperError> {
        let now = Instant::now();
        self.step_pin
            .set_high()
            .map_err(|_e| StepperError::IoError)?;
        let deadline = now + Duration::from_micros(self.pulse_width as u64);
        Timer::at(deadline).await;
        self.step_pin
            .set_low()
            .map_err(|_e| StepperError::IoError)?;
        Ok(self.pulse_delay)
    }
}
