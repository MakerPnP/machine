pub mod loadcell {
    use fpga_pac::loadcell0::vals::{mode, pdmode, rate};

    #[derive(Debug, Copy, Clone)]
    #[derive(defmt::Format)]
    pub enum LoadCellError {
        NotReady,
        Timeout,
        PoweredDown,
    }

    pub struct FpgaLoadcell {
        instance: fpga_pac::loadcell0::loadcell0,
        rate: rate
    }

    impl FpgaLoadcell {
        pub fn new(instance: fpga_pac::loadcell0::loadcell0) -> Self {
            Self {
                instance,
                rate: rate::RATE_10HZ,
            }
        }

        pub fn set_rate(&mut self, rate: rate) {
            self.rate = rate;
        }

        pub fn start_continuous(&self) {
            self.instance.lc_ctrl().write(|w|{
                w.set_rate(self.rate);
                w.set_mode(mode::A128);
                w.set_enable(true);
                w.set_single(false);
            });
        }

        pub fn start_single(&self) {
            self.instance.lc_ctrl().write(|w|{
                w.set_rate(self.rate);
                w.set_mode(mode::A128);
                w.set_enable(false);
                w.set_single(true);
            });
        }

        pub fn stop(&self) {
            self.instance.lc_ctrl().write(|w|{
                w.set_enable(false);
                w.set_pd(true);
                w.set_pdmode(pdmode::ALL);
            });
        }

        pub fn read(&self) -> Result<(i32, u8), LoadCellError> {

            let status = self.instance.lc_status().read();
            if status.pd() == true {
                return Err(LoadCellError::PoweredDown);
            }

            // timeout must be checked before checking the ready bit.
            if status.timeout() == true {
                return Err(LoadCellError::Timeout);
            }

            if status.ready() == false {
                return Err(LoadCellError::NotReady);
            }

            let lc_value = self.instance.lc_value().read();
            // bit shifting to keep the sign bit.
            let value =  ((lc_value.value() as i32) << 8) >> 8;
            let sequence = lc_value.seq();

            Ok((value, sequence))
        }
    }
}