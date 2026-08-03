pub mod adc {
    pub struct FpgaAdcMux {}

    impl FpgaAdcMux {
        pub fn new() -> Self {
            Self {}
        }

        pub fn select_port(&mut self, port: usize) {
            assert!(port < 4);

            fpga_pac::IO.io_out_1().modify(|w| {
                w.set_adc_mux_sel(port as u8);
            })
        }
    }
}