#![no_std]

use core::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use embedded_io_async::Write;
use embedded_nal_async::TcpConnect;
use ioboard_time::TimeService;
use ioboard_trace::tracepin;
use log::{info, error};

pub struct IoConnection<TIME: TimeService, CLIENT: TcpConnect> {
    time: TIME,
    client: CLIENT,
}

impl<TIME: TimeService, CLIENT: TcpConnect> IoConnection<TIME, CLIENT> {
    pub fn new(time: TIME, client: CLIENT) -> IoConnection<TIME, CLIENT> {
        Self {
            time,
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
                self.time
                    .delay_until_us(self.time.now_micros() + 1_000_000)
                    .await;
                continue;
            }
            tracepin::on(3);
            let mut connection = r.unwrap();
            info!("connected!");

            let cycle_period_us = 1_000_000 / 10;
            let mut deadline = self.time.now_micros();
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
                deadline += cycle_period_us;
                self.time.delay_until_us(deadline).await;
            }
            tracepin::off(3);
        }
    }
}
