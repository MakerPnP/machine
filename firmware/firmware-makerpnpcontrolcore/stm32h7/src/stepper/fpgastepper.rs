use embassy_time::{Duration, Instant, Timer};
use ioboard_main::stepper::{Stepper, StepperDirection, StepperError};
use crate::fpga::steppers::steppers::FpgaStepperBank;

/// The FPGA stepper driver hardware implementation is not designed for single stepping, but can be made to...
pub struct FpgaStepper {
    stepper_bank: FpgaStepperBank,
    motor_index: u8,
    pulse_width: u32,
    pulse_delay: u32,
    next_direction: StepperDirection,
}

impl FpgaStepper {
    pub fn new(stepper_bank: FpgaStepperBank, motor_index: u8, pulse_width: u32, pulse_delay: u32,) -> Self {
        Self {
            stepper_bank,
            motor_index,
            pulse_width,
            pulse_delay,
            next_direction: StepperDirection::Normal,
        }
    }
}

impl Stepper for FpgaStepper {
    fn set_pulse_width_us(&mut self, pulse_width: u32) {
        self.pulse_width = pulse_width;
        self.stepper_bank.set_pulse_width(self.motor_index, self.pulse_width as u16);
    }

    fn set_pulse_delay_us(&mut self, pulse_delay: u32) {
        self.pulse_delay = pulse_delay;
    }

    fn enable(&mut self) -> Result<(), StepperError> {
        self.stepper_bank.enable();
        Ok(())
    }

    fn disable(&mut self) -> Result<(), StepperError> {
        // NoOp
        Ok(())
    }

    fn direction(&mut self, direction: StepperDirection) -> Result<(), StepperError> {
        self.next_direction = direction;
        Ok(())
    }

    async fn step_and_wait(&mut self) -> Result<(), StepperError> {

        let delay = self.step().await?;
        let now = Instant::now();
        let deadline = now + Duration::from_micros(delay as u64);
        Timer::at(deadline).await;

        // TODO use the API to wait for the motor to stop

        Ok(())
    }

    async fn step(&mut self) -> Result<u32, StepperError> {
        let sequence = [
            Segment::new(Command::MoveHalt, 1, 1, 1, self.next_direction, RampMode::Up),
        ];
        self.stepper_bank.send_sequence(self.motor_index, &sequence);
        defmt::debug!("stepper step. bank: {}, motor: {}, sequence: {:?}", self.stepper_bank.index, self.motor_index, sequence);
        self.stepper_bank.start_motor(self.motor_index);

        Ok(self.pulse_width + self.pulse_delay)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(defmt::Format)]
pub enum Command {
    Move,
    MoveHalt,
    MoveHaltPause,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(defmt::Format)]
pub enum RampMode {
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(defmt::Format)]
pub struct Segment {
    pub command: Command,
    // 24 bit, unsigned
    pub steps: u32,
    pub start_period: u16,
    pub delta_magnitude: u16,
    pub direction: StepperDirection,
    // aka 'increasing'
    pub ramp_mode: RampMode,
}

impl Segment {
    fn new(command: Command, steps: u32, start_period: u16, delta_magnitude: u16, direction: StepperDirection, ramp_mode: RampMode) -> Self {
        Self {
            command,
            steps,
            start_period,
            delta_magnitude,
            direction,
            ramp_mode,
        }
    }
}