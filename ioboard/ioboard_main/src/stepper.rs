#[derive(Debug, Default, PartialEq, Clone)]
pub enum StepperDirection {
    #[default]
    Normal,
    Reversed,
}

/// A simple synchronous stepper trait.
#[allow(async_fn_in_trait)]
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

    /// Perform a single step pulse and waits for the pulse delay to expire
    async fn step_and_wait(&mut self) -> Result<(), StepperError>;

    /// Perform a single step pulse and return the pulse delay so the caller can schedule the next
    /// step without an additional await.
    async fn step(&mut self) -> Result<u32, StepperError>;
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum StepperError {
    IoError,
}
