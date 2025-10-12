#![no_std]

use embassy_net::driver::Driver;
use embassy_net::{Runner, Stack, StackResources};
use static_cell::StaticCell;

pub fn init<'d, D: Driver>(driver: D, random_seed: u64) -> (Stack<'d>, Runner<'d, D>) {
    let config = embassy_net::Config::dhcpv4(Default::default());
    //let config = embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
    //    address: Ipv4Cidr::new(Ipv4Address::new(10, 42, 0, 61), 24),
    //    dns_servers: Vec::new(),
    //    gateway: Some(Ipv4Address::new(10, 42, 0, 1)),
    //});

    // Init network stack
    static RESOURCES: StaticCell<StackResources<5>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(driver, config, RESOURCES.init(StackResources::new()), random_seed);

    (stack, runner)
}
