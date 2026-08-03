pub mod ws2812 {
    pub struct Ws2812LedController {
        instance: fpga_pac::ws2812_0::ws2812_0,
    }

    impl Ws2812LedController {
        pub fn update_leds(&mut self, wrgb: &[u32]) {
            for wrgb in wrgb.iter() {
                self.instance.ws_data_0().write(|w| {
                    w.0 = *wrgb;
                });
            }
        }
    }

    impl Ws2812LedController {

        /// use the builder to create a configured instance
        fn new(instance: fpga_pac::ws2812_0::ws2812_0) -> Self {
            Self {
                instance
            }
        }
    }

    pub struct Ws2812LedControllerBuilder {
        instance: usize,
        led_count: u8,
        color_ordering: ColorOrdering,
    }

    impl Ws2812LedControllerBuilder {
        pub fn new(instance: usize) -> Self {
            Self {
                instance,
                led_count: 0,
                color_ordering: ColorOrdering::RGB,
            }
        }

        pub fn with_led_count(mut self, led_count: u8) -> Self {
            self.led_count = led_count;
            self
        }

        pub fn with_mode(mut self, color_ordering: ColorOrdering) -> Self {
            self.color_ordering = color_ordering;
            self
        }

        pub fn enable(self) -> Ws2812LedController {
            let instance = match self.instance {
                0 => fpga_pac::WS2812_0,
                1 => fpga_pac::WS2812_1,
                _ => panic!("Invalid instance"),
            };

            instance.ws_ctrl().modify(|w| {
                w.set_enabled(true);
                w.set_mode(self.color_ordering.into());
            });
            instance.ws_tx_config().write(|w| {
                w.set_leds_count(self.led_count);
            });

            Ws2812LedController::new(instance)
        }
    }

    #[repr(u8)]
    pub enum ColorOrdering {
        RGB,
        RGBW,
        GRB,
        GRBW,
    }

    impl Into<fpga_pac::ws2812_0::vals::mode> for ColorOrdering {
        fn into(self) -> fpga_pac::ws2812_0::vals::mode {
            match self {
                ColorOrdering::RGB => fpga_pac::ws2812_0::vals::mode::RGB,
                ColorOrdering::RGBW => fpga_pac::ws2812_0::vals::mode::RGBW,
                ColorOrdering::GRB => fpga_pac::ws2812_0::vals::mode::GRB,
                ColorOrdering::GRBW => fpga_pac::ws2812_0::vals::mode::GRBW,
            }
        }
    }
}