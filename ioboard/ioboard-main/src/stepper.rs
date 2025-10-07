#[derive(Debug, Default, PartialEq, Clone)]
pub enum StepperDirection {
    #[default]
    Normal,
    Reversed,
}

/// A simple synchronous stepper trait.
pub trait Stepper {
    // configuration

    /// pulse width in microseconds
    fn set_pulse_width_us(&mut self, pulse_width: u32);

    /// delay in microseconds
    fn set_pulse_delay_us(&mut self, pulse_delay: u32);

    // operation

    fn enable(&mut self) -> Result<(), StepperError>;
    fn disable(&mut self) -> Result<(), StepperError>;
    fn direction(&mut self, direction: StepperDirection) -> Result<(), StepperError>;

    /// Perform a single step pulse
    /// returns the minimum duration before it can be called again, or an error.
    fn step_and_wait(&mut self) -> Result<(), StepperError>;
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum StepperError {
    IoError,
}
