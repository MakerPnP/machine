pub mod steppers {
    use fpga_pac::steppers::vals::{dir, ramp};
    use ioboard_main::stepper::StepperDirection;
    use crate::stepper::fpgastepper::{RampMode, Segment};

    pub struct FpgaStepperBank {
        pub index: u8,
        instance: fpga_pac::steppers::steppers,
    }

    impl FpgaStepperBank {
        pub fn new(index: u8) -> Self {
            Self {
                index,
                instance: fpga_pac::STEPPERS,
            }
        }

        pub fn enable(&mut self) {
            // currently a No-OP
        }

        pub fn set_pulse_width(&mut self, motor: u8, pulse_width: u16) {

            // TODO math for calculating preset and prescaler from pulse width
            // hard-coded to 1us for now (FPGA sys clock = 50Mhz, stepper clock = 100kHz).
            let preset = 9;
            let prescaler = 4;

            // TODO select the right motor, hardcoded to motor 0 for now
            self.instance.step_pls_config().modify(|w| {
                w.set_preset0(preset);
                defmt::trace!("PLS_CONFIG: {:08x}", w.0);
            });

            // TODO fix silently overriding the prescaler for other motors on the same bank
            // TODO select the right bank, hardcoded to bank 0 for now
            self.instance.step_pls_prescaler().modify(|w| {
                w.set_prescaler0(prescaler);
                defmt::trace!("PLS_PRESCALER: {:08x}", w.0);
            });
        }

        pub fn send_sequence(&self, motor: u8, segments: &[Segment; 1]) {

            // TODO fail gracefully if the sequence is too long or empty
            assert!(segments.len() <= 127, "Current FPGA implementation only supports up to 127 segments per motor");
            assert!(segments.len() > 0, "Current FPGA implementation requires at least one segment");

            self.instance.step_tx_config().write(|w| {
                w.set_motor_instance(motor);
                w.set_num_points(segments.len() as u8);
                defmt::trace!("TX_CONFIG: {:08x}", w.0);
            });

            for segment in segments {

                // TODO extract these match blocks into 'into' impls.

                let ramp = match segment.ramp_mode {
                    RampMode::Up => ramp::UP,
                    RampMode::Down => ramp::DOWN,
                };

                let direction = match segment.direction {
                    StepperDirection::Normal => dir::NORMAL,
                    StepperDirection::Reversed => dir::REVERSE,
                };

                self.instance.step_seg_ctst().write(|w| {
                    w.set_cmd(segment.command.into());
                    w.set_n_steps(segment.steps);
                    w.set_dir(direction);
                    w.set_ramp(ramp);
                    defmt::trace!("CTST: {:08x}", w.0);
                });
                self.instance.step_seg_spdm().write(|w|{
                    w.set_start_period(segment.start_period);
                    w.set_delta_magnitude(segment.delta_magnitude);
                    defmt::trace!("SPDM: {:08x}", w.0);
                });
            }
        }

        pub fn start_motor(&self, motor: u8) {
            if self.index == 0 {
                self.instance.step_ctrl().write(|w| {
                    w.set_start_bank0(1 << motor);
                    defmt::trace!("CTRL: {:08x}", w.0);
                })
            } else {
                self.instance.step_ctrl().write(|w| {
                    w.set_start_bank1(1 << motor);
                    defmt::trace!("CTRL: {:08x}", w.0);
                })
            }
        }
    }
}