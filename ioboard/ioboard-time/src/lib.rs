#![no_std]

#[allow(async_fn_in_trait)]
pub trait TimeService {
    fn now_micros(&self) -> u64;
    async fn delay_until_us(&self, deadline: u64);
}
