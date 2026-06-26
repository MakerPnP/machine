use embassy_stm32::adc;
use embassy_stm32::adc::{Adc, AdcChannel, BasicAdcRegs, BasicInstance};
use crate::fpga::adc::FpgaAdcMux;

pub struct Mux<'a, ADC, IN1, IN2>
where
    ADC: adc::Instance,
    <ADC as BasicInstance>::Regs: BasicAdcRegs,
    IN1: AdcChannel<ADC>,
    IN2: AdcChannel<ADC>,
{
    fpga_adc_mux: FpgaAdcMux,
    adc: Adc<'a, ADC>,
    in1: IN1,
    in2: IN2,
    sample_time: <<ADC as BasicInstance>::Regs as BasicAdcRegs>::SampleTime,
}

impl<'a, ADC: adc::Instance + adc::BasicInstance, IN1: AdcChannel<ADC>, IN2: AdcChannel<ADC>> Mux<'a, ADC, IN1, IN2> {
    pub fn new(
        fpga_adc_mux: FpgaAdcMux,
        adc: Adc<'a, ADC>,
        in1: IN1,
        in2: IN2,
        sample_time: <<ADC as BasicInstance>::Regs as BasicAdcRegs>::SampleTime,
    ) -> Self {
        Self {
            fpga_adc_mux,
            adc,
            in1,
            in2,
            sample_time,
        }
    }

    pub fn select_port(&mut self, port: usize) {
        self.fpga_adc_mux.select_port(port);
    }

    pub fn read_pair(&mut self) -> (u16, u16) {
        let ch1 = self.adc.blocking_read(&mut self.in1, self.sample_time);
        let ch2 = self.adc.blocking_read(&mut self.in2, self.sample_time);
        let measured = (ch1, ch2);
        measured
    }
}