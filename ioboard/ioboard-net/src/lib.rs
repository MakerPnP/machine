#![no_std]
#![no_std]

use core::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use embedded_io_async::Write;
use embedded_nal_async::TcpConnect;
use ioboard_trace::tracepin;
use log::{info, error};

use embassy_net::driver::Driver;
use embassy_net::{Runner, Stack, StackResources};
use static_cell::StaticCell;
use embassy_time::{Duration, Instant, Ticker, Timer};

pub struct IoConnection<CLIENT: TcpConnect> {
    client: CLIENT,
}

impl<CLIENT: TcpConnect> IoConnection<CLIENT> {
    pub fn new(client: CLIENT) -> IoConnection<CLIENT> {
        Self {
            client,
        }
    }

    pub async fn run(&mut self) -> ! {
        loop {
            // You need to start a server on the host machine, for example: `nc -l 8000`
            let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(192, 168, 18, 41), 8000));

            info!("Connecting...");
            let r = self.client.connect(addr).await;
            if let Err(e) = r {
                error!("Connect error: {:?}", e);
                Timer::after(Duration::from_secs(1)).await;
                continue;
            }
            tracepin::on(3);
            let mut connection = r.unwrap();
            info!("connected!");

            let cycle_period_us = 1_000_000 / 10;
            let mut cycle_ticker = Ticker::every(Duration::from_micros(cycle_period_us));
            loop {
                tracepin::on(2);
                let r = connection.write_all(
                    b"\
                    0123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789\
                    0123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789\
                    0123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789\
                    0123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789\
                    \n"
                ).await;
                tracepin::off(2);
                if let Err(e) = r {
                    error!("write error: {:?}", e);
                    break;
                }
                cycle_ticker.next().await;
            }
            tracepin::off(3);
        }
    }
}

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
