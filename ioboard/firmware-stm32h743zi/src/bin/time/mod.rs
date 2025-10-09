use embassy_time::{Instant, Timer};
use ioboard_time::TimeService;

#[derive(Default, Copy, Clone)]
pub struct EmbassyTimeService {}

impl TimeService for EmbassyTimeService {
    #[inline]
    fn now_micros(&self) -> u64 {
        Instant::now().as_micros()
    }

    async fn delay_until_us(&self, deadline: u64) {
        Timer::at(Instant::from_micros(deadline)).await;
    }
}
