#![no_std]
extern crate alloc;

use alloc::boxed::Box;
use core::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use core::pin::pin;
use ioboard_trace::tracepin;
use log::{info, error};

use embassy_executor::Spawner;
use embedded_io_async::Write;
use embedded_nal_async::TcpConnect;
use embassy_net::driver::Driver;
use embassy_net::{IpEndpoint, Ipv4Address, Runner, StackResources};
use embassy_net::tcp::client::{TcpClient, TcpClientState};
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_time::{Duration, Ticker, Timer, WithTimeout};

use ergot::exports::bbq2::traits::coordination::cas::AtomicCoord;
use ergot::interface_manager::profiles::direct_edge::embassy_net_udp_0_7::RxTxWorker;
use ergot::logging::log_v0_4::LogSink;
use ergot::toolkits::embassy_net_v0_7 as kit;
use ergot::well_known::ErgotPingEndpoint;
use ergot::{Address, topic};
use mutex::raw_impls::cs::CriticalSectionRawMutex;

use static_cell::{ConstStaticCell, StaticCell};

use ioboard_shared::yeet::Yeet;

//
// Ergot configuration
//

const OUT_QUEUE_SIZE: usize = 4096;

// FIXME this depends on the interface being used, maybe need a feature or something
const MAX_PACKET_SIZE: usize = 1514;

/// Statically store receive buffers
static RECV_BUF: ConstStaticCell<[u8; MAX_PACKET_SIZE]> = ConstStaticCell::new([0u8; MAX_PACKET_SIZE]);
static SCRATCH_BUF: ConstStaticCell<[u8; 64]> = ConstStaticCell::new([0u8; 64]);

type Stack = kit::EdgeStack<&'static Queue, CriticalSectionRawMutex>;
type Queue = kit::Queue<OUT_QUEUE_SIZE, AtomicCoord>;

/// Statically store our netstack
static STACK: Stack = kit::new_target_stack(OUTQ.stream_producer(), MAX_PACKET_SIZE as u16);
/// Statically store our outgoing packet buffer
static OUTQ: Queue = kit::Queue::new();
static LOGSINK: LogSink<&'static Stack> = LogSink::new(&STACK);

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

pub fn init<'d, D: Driver>(
    driver: D,
    random_seed: u64,
    spawner: Spawner,
) -> Runner<'d, D> {
    let config = embassy_net::Config::dhcpv4(Default::default());
    //let config = embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
    //    address: Ipv4Cidr::new(Ipv4Address::new(10, 42, 0, 61), 24),
    //    dns_servers: Vec::new(),
    //    gateway: Some(Ipv4Address::new(10, 42, 0, 1)),
    //});

    // Init network stack
    static RESOURCES: StaticCell<StackResources<5>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(driver, config, RESOURCES.init(StackResources::new()), random_seed);

    defmt::info!("Hardware address: {}", stack.hardware_address());

    spawner.spawn(networking_task(stack, spawner.clone(), RECV_BUF.take(), SCRATCH_BUF.take())).unwrap();

    runner
}

#[embassy_executor::task]
async fn networking_task(
    stack: embassy_net::Stack<'static>,
    spawner: Spawner,
    recv_buf: &'static mut [u8],
    scratch_buf: &'static mut [u8],
) -> ! {
    defmt::info!("Network task initialized");

    // Ensure DHCP configuration is up before trying connect
    let mut attempts: u32 = 0;
    let config = loop {
        if let Some(config) = stack.config_v4() {
            break config;
        }

        if attempts % 10 == 0 {
            defmt::info!("Waiting for DHCP address allocation");
        }

        attempts = attempts.wrapping_add(1);
        Timer::after(Duration::from_millis(100)).await;
    };

    defmt::info!(
        "IP address: {}, gateway: {}, dns: {}",
        config.address, config.dns_servers, config.gateway
    );

    let state: TcpClientState<1, 1024, 1024> = TcpClientState::new();
    let tcp_client = TcpClient::new(stack, &state);

    let rx_meta = [PacketMetadata::EMPTY; 1];
    let rx_buffer = [0; 4096];
    let tx_meta = [PacketMetadata::EMPTY; 1];
    let tx_buffer = [0; 4096];

    // move the buffers into the heap, so they don't get dropped
    let rx_meta = Box::new(rx_meta);
    let rx_meta = Box::leak(rx_meta);
    let tx_meta = Box::new(tx_meta);
    let tx_meta = Box::leak(tx_meta);
    let rx_buffer = Box::new(rx_buffer);
    let rx_buffer = Box::leak(rx_buffer);
    let tx_buffer = Box::new(tx_buffer);
    let tx_buffer = Box::leak(tx_buffer);
    // You need to start a server on the host machine, for example: `nc -lu 8000`

    let mut udp_socket = UdpSocket::new(stack, rx_meta, rx_buffer, tx_meta, tx_buffer);

    let port = 8000_u16;
    let remote_endpoint = IpEndpoint::new(Ipv4Address::new(192, 168, 18, 41).into(), port);
    let local_endpoint = IpEndpoint::new(config.address.address().into(), port);
    udp_socket
        .bind(local_endpoint)
        .expect("bound");

    defmt::info!(
        "capacity, receive: {}, send: {}",
        udp_socket.packet_recv_capacity(),
        udp_socket.packet_send_capacity()
    );

    // Spawn I/O worker tasks
    spawner.must_spawn(run_socket(udp_socket, recv_buf, scratch_buf, remote_endpoint));

    // Spawn socket using tasks
    spawner.must_spawn(pingserver());
    spawner.must_spawn(pinger());

    spawner.must_spawn(yeeter());
    spawner.must_spawn(yeet_listener(0));

    LOGSINK.register_static(log::LevelFilter::Info);

    if false {
        spawner.must_spawn(udp_spam_task(stack));

        crate::IoConnection::new(tcp_client)
            .run()
            .await
    }

    let mut tckr = Ticker::every(Duration::from_secs(2));
    let mut ct = 0;
    loop {
        tckr.next().await;
        log::info!("log to log sink: # {ct}");
        ct += 1;
    }
}


#[embassy_executor::task]
async fn run_socket(
    socket: UdpSocket<'static>,
    recv_buf: &'static mut [u8],
    scratch_buf: &'static mut [u8],
    endpoint: IpEndpoint,
) {
    let consumer = OUTQ.stream_consumer();
    let mut rxtx = RxTxWorker::new_target(&STACK, socket, (), consumer, endpoint);

    loop {
        _ = rxtx.run(recv_buf, scratch_buf).await;
    }
}


#[embassy_executor::task]
async fn pinger() {
    let mut ticker = Ticker::every(Duration::from_secs(1));
    let mut ctr = 0u32;
    let client = STACK
        .endpoints()
        .client::<ErgotPingEndpoint>(
            Address {
                network_id: 1,
                node_id: 1,
                port_id: 0,
            },
            None,
        );
    loop {
        ticker.next().await;
        tracepin::on(2);
        let res = client
            .request(&ctr)
            .with_timeout(Duration::from_millis(100))
            .await;
        tracepin::off(2);
        match res {
            Ok(Ok(n)) => {
                defmt::info!("Got ping {=u32} -> {=u32}", ctr, n);
                ctr = ctr.wrapping_add(1);
            }
            Ok(Err(_e)) => {
                defmt::warn!("Net stack ping error");
            }
            Err(_) => {
                defmt::warn!("Ping timeout");
            }
        }
    }
}

/// Respond to any incoming pings
#[embassy_executor::task]
async fn pingserver() {
    STACK
        .services()
        .ping_handler::<4>()
        .await;
}

// TODO replace with the the load-cell data type and topic
topic!(YeetTopic, Yeet, "topic/yeet");

#[embassy_executor::task]
async fn yeeter() {
    let mut counter = 0;
    let mut error_counter = 0;

    defmt::info!("Yeeter started");

    // FIXME remove this arbitrary startup delay
    Timer::after(Duration::from_secs(8)).await;

    // Using a target frequency of 320Hz, the same as the HX717 load-cell ADC sensor
    const TARGET_HZ: u16 = 320;
    let mut cycle_ticker = Ticker::every(Duration::from_micros(1_000_000_u64 / TARGET_HZ as u64));
    loop {
        info!("Sending broadcast message. ctr: {}, errors: {}", counter, error_counter);

        enum Action {
            Retry,
            Wait,
        }

        tracepin::on(1);
        let action = match STACK
            .topics()
            .broadcast::<YeetTopic>(&counter, None)
        {
            Ok(_) => {
                counter += 1;
                Action::Wait
            }
            Err(_e) => {
                error_counter += 1;
                // TODO look at the error and act appropriately instead of just retrying
                Action::Retry
            }
        };
        tracepin::off(1);

        if matches!(action, Action::Retry) {
            Timer::after(Duration::from_millis(100)).await;
            cycle_ticker.reset();
            continue;
        }

        cycle_ticker.next().await;
    }
}

#[embassy_executor::task]
async fn yeet_listener(id: u8) {
    let subber = STACK
        .topics()
        .bounded_receiver::<YeetTopic, 64>(None);
    let subber = pin!(subber);
    let mut hdl = subber.subscribe();

    defmt::info!("Yeet listener started");
    loop {
        tracepin::on(3);
        let msg = hdl.recv().await;
        tracepin::off(3);
        defmt::info!("{:?}: Listener id:{} got {}", msg.hdr, id, msg.t);
    }
}

#[embassy_executor::task]
async fn udp_spam_task(stack: embassy_net::Stack<'static>) -> ! {
    defmt::info!("UDP spam task initialized");

    while stack.config_v4().is_none() {
        Timer::after(Duration::from_millis(100)).await;
    }

    defmt::info!("UDP spamming!");
    let mut rx_meta = [PacketMetadata::EMPTY; 1];
    let mut rx_buffer = [0; 4096];
    let mut tx_meta = [PacketMetadata::EMPTY; 1];
    let mut tx_buffer = [0; 4096];

    // You need to start a server on the host machine, for example: `nc -lu 8000`

    let mut socket = UdpSocket::new(stack, &mut rx_meta, &mut rx_buffer, &mut tx_meta, &mut tx_buffer);

    let remote_endpoint = (Ipv4Address::new(192, 168, 18, 41), 8000);
    socket
        .bind(remote_endpoint)
        .expect("bound");

    let cycle_period_us = 1_000_000 / 200;
    let mut ticker = Ticker::every(Duration::from_micros(cycle_period_us));
    loop {
        tracepin::on(1);
        socket
            .send_to(
                b"\
                0123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789\
                0123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789\
                0123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789\
                0123456789012345678901234567890123456789012345678901234567890123456789012345678901234567890123456789\
                \n",
                remote_endpoint,
            )
            .await
            .expect("sent");
        tracepin::off(1);
        ticker.next().await;
    }
}
