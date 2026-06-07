use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;
use embedded_hal::spi::SpiBus;
use ioboard_main::stepper::{Stepper, StepperDirection, StepperError};

use modular_bitfield_to_value::ToValue;

use defmt::*;
use embassy_time::{Duration, Instant, Timer};
use tmc5160::registers::*;
use tmc5160::registers::Registers::{GLOBALSCALER, OTP_READ};

pub struct Tmc5160Stepper<SPI, CS, EN, DELAY, PIN2, PIN3> {
    /// pulse width (us)
    pulse_width: u32,
    /// minimum delay between pulses (us)
    pulse_delay: u32,

    driver: tmc5160::Tmc5160<SPI, CS, EN, DELAY>,

    step_pin: PIN2,
    direction_pin: PIN3,
}

impl<SPI, CS, EN, DELAY, E, PIN2, PIN3> Tmc5160Stepper<SPI, CS, EN, DELAY, PIN2, PIN3>
where
    SPI: SpiBus<Error = E>,
    CS: OutputPin,
    EN: OutputPin,
    DELAY: DelayNs,
    PIN2: OutputPin,
    PIN3: OutputPin,
    E: defmt::Format,
{
    pub fn new(
        spi: SPI,
        cs: CS,
        en: EN,
        delay: DELAY,
        step_pin: PIN2,
        direction_pin: PIN3,
        pulse_width: u32,
        pulse_delay: u32,
    ) -> Self
    where
    {
        let mut driver = tmc5160::Tmc5160::new(spi, cs, delay);
        driver = driver.attach_en(en);

        Self {
            driver,
            step_pin,
            direction_pin,
            pulse_width,
            pulse_delay,
        }
    }

    pub fn initialize_io(&mut self) -> Result<(), StepperError> {
        self.driver.disable()
            .map_err(|_e| StepperError::DriverError)?;
        self.step_pin
            .set_low()
            .map_err(|_e| StepperError::IoError)?;
        self.direction_pin
            .set_low()
            .map_err(|_e| StepperError::IoError)?;

        self.reconfigure()
            .map_err(|_e| StepperError::IoError)
    }

    /// Safety: only call after enabling the driver
    pub fn reconfigure(&mut self) -> Result<(), StepperError> {
        let io_in = self.dump_io_in("initial")
            .map_err(|_|StepperError::DriverError)?;
        defmt::debug_assert!(!io_in.drv_enn(), "Configure attempt without enabling");

        let _packet = self
            .driver
            .clear_g_stat()
            .map_err(|_|StepperError::DriverError)?;

        let otp_read = self.driver
            .read_register(OTP_READ)
            .map(|packet|OtpRead::from_bytes(packet.data.to_le_bytes()))
            .map_err(|_|StepperError::DriverError)?;
        debug!("initial otp_read: {}", otp_read);

        let initial_offset = self.driver.read_offset()
            .map_err(|_|StepperError::DriverError)?;
        debug!("initial offset: {}", initial_offset);


        let config = &mut self.driver.g_conf;
        config.set_recalibrate(true);
        config.set_faststandstill(true);
        config.set_en_pwm_mode(true);

        // TODO set corresponding MCU/FPGA pins as inputs first
        if false {
            config
                .set_diag1_index(true);
            config
                .set_diag0_error(true);
        }

        self.driver.update_g_conf()
            .map_err(|_|StepperError::DriverError)?;

        let drv_status = self.driver.read_drv_status()
            .map_err(|_error|StepperError::DriverError)?;

        debug!("drv_status: {:?}", drv_status);

        // Standard Mode (SpreadCycle)
        self.driver.chop_conf.set_chm(false);
        // 8.1 SpreadCycle Chopper - "With 12 MHz clock this gives a setting of TOFF=3.0, i.e. 3."
        self.driver.chop_conf.set_toff(3);
        // Blanking Time - '2'/`10b` is recommended value.
        self.driver.chop_conf.set_tbl(2);
        // Resolution: 8 microsteps.
        // 0 = 256, 1 = 128, 2 = 64, 3 = 32, 4 = 16, 5 = 8...
        self.driver.chop_conf.set_mres(5);
        // Use interpolation.
        self.driver.chop_conf.set_intpol(true);
        // Edge mode: Single edge
        self.driver.chop_conf.set_dedge(false);

        // using example values for hstrt and hend
        self.driver.chop_conf.set_hstr(4);
        self.driver.chop_conf.set_hend(1);

        info!("Configuring chop conf: {:08x}(LE)", self.driver.chop_conf.to_u32_le());
        info!("{:?}", self.driver.chop_conf);

        self.driver.update_chop_conf()
            .map_err(|_|StepperError::DriverError)?;

        let chop_conf = self.driver.read_chop_conf()
            .map_err(|_|StepperError::DriverError)?;
        info!("Read chop conf: {:08x}(LE)", chop_conf.to_u32_le());

        if self.driver.chop_conf.to_u32_le() != chop_conf.to_u32_le() {
            error!("Failed to configure chopconf. expected: {}, actual: {}",
                self.driver.chop_conf.to_u32_le(),
                chop_conf.to_u32_le()
            );
            return Err(StepperError::DriverError)
        }

        self.driver.ihold_irun.set_i_hold(0x8); // ~25%
        self.driver.ihold_irun.set_i_run(0x8); // ~25%
        self.driver.ihold_irun.set_i_hold_delay(0x8); // ~50%
        self.driver.update_ihold_irun()
            .map_err(|_|StepperError::DriverError)?;

        info!("Configuring ihold_irun: {:08x}(LE)", self.driver.ihold_irun.to_u32_le());
        info!("{:?}", self.driver.ihold_irun);

        // NOTE: IHOLD_IRUN is WRITE-ONLY so we cannot verify it.

        // 25% SCALE global scaler.
        self.driver.set_global_scaler(32)
            .map_err(|_|StepperError::DriverError)?;

        // NOTE: GLOBAL_SCALER is WRITE-ONLY so we cannot verify it.

        let drv_status = self.driver.read_drv_status()
            .map_err(|_error|StepperError::DriverError)?;

        debug!("drv_status: {:?}", drv_status);

        let gstat = self.driver.read_gstat()
            .map_err(|_error|StepperError::DriverError)?;
        info!("gstat: {:?}", gstat);

        Ok(())
    }

    fn dump_io_in(&mut self, message: &str) -> Result<IoIn, tmc5160::Error<E>> {
        self.driver.read_ioin()
            .inspect(|io_in|{
                debug!("io_in: {}, message: {}", io_in, message);
            })
            .inspect_err(|_e|{
                // FUTURE need defmt support on the error
                error!("error reading io_in");
            })
    }
}

impl<SPI, CS, EN, DELAY, PIN2, PIN3, E> Stepper for Tmc5160Stepper<SPI, CS, EN, DELAY, PIN2, PIN3>
where
    SPI: SpiBus<Error = E>,
    CS: OutputPin,
    EN: OutputPin,
    DELAY: DelayNs,
    PIN2: OutputPin,
    PIN3: OutputPin,
    E: defmt::Format,
{
    fn set_pulse_width_us(&mut self, pulse_width: u32) {
        self.pulse_width = pulse_width;
    }

    fn set_pulse_delay_us(&mut self, pulse_delay: u32) {
        self.pulse_delay = pulse_delay;
    }

    fn enable(&mut self) -> Result<(), StepperError> {
        self.driver.enable()
            .map_err(|_error|StepperError::DriverError)?;

        self.dump_io_in("enabled")
            .map_err(|_error|StepperError::DriverError)?;

        Ok(())
    }

    fn disable(&mut self) -> Result<(), StepperError> {
        self.driver.disable()
            .map_err(|_error|StepperError::DriverError)?;

        self.dump_io_in("disabled")
            .map_err(|_error|StepperError::DriverError)?;

        Ok(())
    }

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
