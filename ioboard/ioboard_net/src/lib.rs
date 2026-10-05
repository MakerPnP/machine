#![no_std]
extern crate alloc;

use alloc::boxed::Box;
use core::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use core::pin::pin;

use embassy_executor::Spawner;
use embassy_net::driver::Driver;
use embassy_net::tcp::client::{TcpClient, TcpClientState};
use embassy_net::udp::{PacketMetadata, RecvError, SendError, UdpSocket};
use embassy_net::{IpEndpoint, Ipv4Address, Runner, StackResources};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::channel::{Channel, Receiver, Sender};
use embassy_sync::signal::Signal;
use embassy_futures::select::{select, Either};
use embassy_time::{Duration, Ticker, Timer, WithTimeout};
use embedded_io_async::Write;
use embedded_nal_async::TcpConnect;
use ergot::exports::bbqueue::traits::coordination::cas::AtomicCoord;
use ergot::interface_manager::transports::embassy_net_udp::{
    UDP_OVER_ETH_ERGOT_FRAME_SIZE_MAX, UDP_OVER_ETH_ERGOT_PAYLOAD_SIZE_MAX,
};
use ergot::logging::log_v0_4::LogSink;
use ergot::toolkits::embassy_net_v0_7 as kit;
use ergot::well_known::{DeviceInfo, ErgotPingEndpoint};
use ergot::{Address, topic};
use ergot::interface_manager::InterfaceState;
use ergot::interface_manager::transports::packet::{PacketReceiver, PacketRxTxWorker, PacketSender};
use ergot::prelude::{EdgeFrameProcessor, CENTRAL_NODE_ID};
use ioboard_shared::commands::IoBoardCommand;
use ioboard_shared::discovery::{
    self, ADVERTISEMENT_INTERVAL_MS, ClaimResult, ClaimState, DISCOVERY_FRAME_SIZE_MAX, DISCOVERY_PORT, DecodeError,
    Endpoint, IoBoardAdvertisement, IoBoardAdvertisementTopic, IoBoardClaim, IoBoardClaimTopic, IoBoardRelease,
    IoBoardReleaseTopic, MISSED_RENEWALS_MAX, ReleaseResult,
};
pub use ioboard_shared::discovery::SerialNumber;
use ioboard_shared::yeet::Yeet;
use ioboard_trace::tracepin;
use log::{error, info};
use mutex::raw_impls::cs::CriticalSectionRawMutex;
use static_cell::{ConstStaticCell, StaticCell};
use defmt::unwrap;

//
// Ergot configuration
//

const OUT_QUEUE_SIZE: usize = 4096;

/// Scratch buffer is used for UDP packet reception
static SCRATCH_BUF: ConstStaticCell<[u8; UDP_OVER_ETH_ERGOT_PAYLOAD_SIZE_MAX]> =
    ConstStaticCell::new([0u8; UDP_OVER_ETH_ERGOT_PAYLOAD_SIZE_MAX]);

type Stack = kit::EdgeStack<&'static Queue, CriticalSectionRawMutex>;
type Queue = kit::Queue<OUT_QUEUE_SIZE, AtomicCoord>;

// FIXME should we *really* be using MAX_PACKET_SIZE here?
/// Statically store our netstack
pub static STACK: Stack = kit::new_target_stack(OUTQ.framed_producer(), UDP_OVER_ETH_ERGOT_FRAME_SIZE_MAX as u16);
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
            let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(192, 168, 18, 60), 8000));

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

/// `serial_number` must be unique per board, it is advertised so that a server can find and identify the board.
pub fn init<'d, D: Driver>(driver: D, random_seed: u64, serial_number: SerialNumber, spawner: Spawner) -> Runner<'d, D> {
    let config = embassy_net::Config::dhcpv4(Default::default());
    //let config = embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
    //    address: Ipv4Cidr::new(Ipv4Address::new(10, 42, 0, 61), 24),
    //    dns_servers: Vec::new(),
    //    gateway: Some(Ipv4Address::new(10, 42, 0, 1)),
    //});

    // Init network stack
    static RESOURCES: StaticCell<StackResources<6>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(driver, config, RESOURCES.init(StackResources::new()), random_seed);

    defmt::info!("Hardware address: {}", stack.hardware_address());

    spawner
        .spawn(unwrap!(networking_task(stack, spawner.clone(), SCRATCH_BUF.take(), serial_number)));

    runner
}

#[embassy_executor::task]
async fn networking_task(
    stack: embassy_net::Stack<'static>,
    spawner: Spawner,
    scratch_buf: &'static mut [u8],
    serial_number: SerialNumber,
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
        config.address,
        config.dns_servers,
        config.gateway
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

    let local_endpoint = IpEndpoint::new(config.address.address().into(), ERGOT_PORT);
    udp_socket
        .bind(local_endpoint)
        .expect("bound");

    defmt::info!(
        "capacity, receive: {}, send: {}",
        udp_socket.packet_recv_capacity(),
        udp_socket.packet_send_capacity()
    );

    let mut discovery_socket = UdpSocket::new(
        stack,
        Box::leak(Box::new([PacketMetadata::EMPTY; 4])),
        Box::leak(Box::new([0; 4 * DISCOVERY_FRAME_SIZE_MAX])),
        Box::leak(Box::new([PacketMetadata::EMPTY; 2])),
        Box::leak(Box::new([0; 2 * DISCOVERY_FRAME_SIZE_MAX])),
    );
    discovery_socket
        .bind(DISCOVERY_PORT)
        .expect("bound");

    // Spawn I/O worker tasks
    spawner.spawn(unwrap!(discovery_task(discovery_socket, serial_number)));
    spawner.spawn(unwrap!(run_socket(udp_socket, scratch_buf)));

    // Spawn socket using tasks
    spawner.spawn(unwrap!(pingserver()));
    spawner.spawn(unwrap!(pinger()));
    spawner.spawn(unwrap!(discovery_responder()));

    let yeet_command_sender = YEET_COMMAND_CHANNEL.sender();
    let yeet_command_receiver = YEET_COMMAND_CHANNEL.receiver();

    spawner.spawn(unwrap!(yeeter(yeet_command_receiver)));
    spawner.spawn(unwrap!(command_listener(yeet_command_sender)));

    LOGSINK.register_static(log::LevelFilter::Info);

    if false {
        spawner.spawn(unwrap!(udp_spam_task(stack)));

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

/// The UDP port of the board's ergot interface.
const ERGOT_PORT: u16 = 8000;

/// The server ergot endpoint, signalled with `Some` when the board is claimed and `None` when it's released.
static SERVER_ENDPOINT: Signal<ThreadModeRawMutex, Option<Endpoint>> = Signal::new();

fn ip_endpoint(endpoint: Endpoint) -> IpEndpoint {
    IpEndpoint::new(Ipv4Address::from(endpoint.ip).into(), endpoint.port)
}

enum DiscoveryMessage {
    Claim(IoBoardClaim),
    Release(IoBoardRelease),
}

fn decode_discovery_message(data: &[u8]) -> Result<DiscoveryMessage, DecodeError> {
    match discovery::decode::<IoBoardClaimTopic>(data) {
        Ok(claim) => Ok(DiscoveryMessage::Claim(claim)),
        Err(DecodeError::UnknownKey) => discovery::decode::<IoBoardReleaseTopic>(data).map(DiscoveryMessage::Release),
        Err(e) => Err(e),
    }
}

/// Broadcasts advertisements and handles claims and releases from servers.
///
/// A claim is only accepted when the board is unclaimed, the board stays claimed by that server endpoint until the
/// server releases it, the claim expires, or the board is reset.  See [`ClaimState`].
#[embassy_executor::task]
async fn discovery_task(socket: UdpSocket<'static>, serial_number: SerialNumber) -> ! {
    defmt::info!("Discovery started, serial number: {}", serial_number);

    let broadcast_endpoint = IpEndpoint::new(Ipv4Address::BROADCAST.into(), DISCOVERY_PORT);
    let mut ticker = Ticker::every(Duration::from_millis(ADVERTISEMENT_INTERVAL_MS));
    let mut rx_buf = [0u8; DISCOVERY_FRAME_SIZE_MAX];
    let mut claim_state = ClaimState::new();

    loop {
        let reply_to = match select(ticker.next(), socket.recv_from(&mut rx_buf)).await {
            Either::First(_) => {
                if let Some(claimed_by) = claim_state.advertising() {
                    defmt::warn!(
                        "Claim expired, no renewals from server. claimed_by: {}, missed renewals: {}",
                        claimed_by,
                        MISSED_RENEWALS_MAX
                    );
                    SERVER_ENDPOINT.signal(None);
                }
                broadcast_endpoint
            }
            Either::Second(Ok((len, metadata))) => {
                match decode_discovery_message(&rx_buf[..len]) {
                    Ok(DiscoveryMessage::Claim(claim)) if claim.serial_number == serial_number => {
                        match claim_state.claim(claim.server) {
                            ClaimResult::Claimed => {
                                defmt::info!("Claimed by server, endpoint: {}", claim.server);
                                SERVER_ENDPOINT.signal(Some(claim.server));
                            }
                            // don't reply to renewals, they are sent in reply to our advertisements.
                            ClaimResult::Renewed => continue,
                            ClaimResult::ClaimedByOther(claimed_by) => {
                                defmt::warn!(
                                    "Ignoring claim, already claimed. claimed_by: {}, claim: {}",
                                    claimed_by,
                                    claim.server
                                );
                            }
                        }
                        // reply immediately, so the server knows who the board is claimed by
                        metadata.endpoint
                    }
                    Ok(DiscoveryMessage::Release(release)) if release.serial_number == serial_number => {
                        match claim_state.release(release.server) {
                            ReleaseResult::Released => {
                                defmt::info!("Released by server, endpoint: {}", release.server);
                                SERVER_ENDPOINT.signal(None);
                            }
                            ReleaseResult::NotClaimed => {}
                            ReleaseResult::ClaimedByOther(claimed_by) => {
                                defmt::warn!(
                                    "Ignoring release, claimed by another server. claimed_by: {}, release: {}",
                                    claimed_by,
                                    release.server
                                );
                            }
                        }
                        // reply immediately, so the server knows the board was released
                        metadata.endpoint
                    }
                    Ok(_) => {
                        defmt::warn!("Ignoring discovery message for another board");
                        continue;
                    }
                    // advertisements from other boards, ignore
                    Err(DecodeError::UnknownKey) => continue,
                    Err(e) => {
                        defmt::warn!("Invalid discovery packet, error: {}", e);
                        continue;
                    }
                }
            }
            Either::Second(Err(_)) => {
                defmt::warn!("Discovery packet too large, ignoring");
                continue;
            }
        };

        let advertisement = IoBoardAdvertisement {
            serial_number,
            ergot_port: ERGOT_PORT,
            claimed_by: claim_state.claimed_by(),
        };
        let mut tx_buf = [0u8; DISCOVERY_FRAME_SIZE_MAX];
        let Ok(frame) = discovery::encode::<IoBoardAdvertisementTopic>(&mut tx_buf, &advertisement) else {
            defmt::error!("Unable to encode advertisement");
            continue;
        };
        if let Err(e) = socket.send_to(frame, reply_to).await {
            defmt::warn!("Unable to send advertisement, error: {}", e);
        }
    }
}

/// Receives ergot frames from the server the board is claimed by, frames from any other source are dropped.
struct ServerReceiver<'a> {
    socket: &'a UdpSocket<'static>,
    server: IpEndpoint,
}

impl PacketReceiver for ServerReceiver<'_> {
    type Error = RecvError;

    // cancel-safe, each `recv_from` either consumes a whole datagram or nothing.
    async fn recv(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error> {
        loop {
            match self.socket.recv_from(buf).await {
                Ok((len, metadata)) if metadata.endpoint == self.server => return Ok(len),
                Ok(_) => {}
                // dropping one oversized datagram must not take the interface down
                Err(RecvError::Truncated) => defmt::warn!("Dropping oversized ergot datagram"),
            }
        }
    }
}

/// Sends ergot frames to the server the board is claimed by.
struct ServerSender<'a> {
    socket: &'a UdpSocket<'static>,
    server: IpEndpoint,
}

impl PacketSender for ServerSender<'_> {
    type Error = SendError;

    async fn send(&mut self, data: &[u8]) -> Result<(), Self::Error> {
        self.socket
            .send_to(data, self.server)
            .await
    }
}

/// Runs the ergot interface while the board is claimed.
///
/// A new worker, and frame processor, is created for each claim, so the net_id assigned by a previous server's router
/// is not re-used.  While unclaimed, the interface is down.
#[embassy_executor::task]
async fn run_socket(socket: UdpSocket<'static>, scratch_buf: &'static mut [u8]) {
    let mut server_endpoint = None;

    loop {
        let endpoint = match server_endpoint {
            Some(endpoint) => endpoint,
            None => {
                defmt::info!("Waiting for a server to claim this board");
                server_endpoint = SERVER_ENDPOINT.wait().await;
                continue;
            }
        };

        // discard frames queued for a previous server, they are addressed using the previous server's net_id.
        let consumer = OUTQ.framed_consumer();
        while let Ok(grant) = consumer.read() {
            grant.release();
        }

        let server = ip_endpoint(endpoint);
        let mut worker = PacketRxTxWorker::new(
            &STACK,
            ServerReceiver {
                socket: &socket,
                server,
            },
            ServerSender {
                socket: &socket,
                server,
            },
            EdgeFrameProcessor::new(),
            (),
            consumer,
        );

        // The net_id is assigned by the server's router, it's learnt from the first frame addressed to us.
        let result = select(
            worker.run(InterfaceState::edge_link_local(), scratch_buf),
            SERVER_ENDPOINT.wait(),
        )
        .await;
        // the interface goes down when the worker is dropped
        drop(worker);

        match result {
            Either::First(Ok(())) => {}
            Either::First(Err(_e)) => {
                defmt::warn!("ergot socket error, restarting");
                Timer::after(Duration::from_millis(100)).await;
            }
            Either::Second(endpoint) => server_endpoint = endpoint,
        }
    }
}

#[embassy_executor::task]
async fn pinger() {
    let mut ticker = Ticker::every(Duration::from_secs(1));
    let mut ctr = 0u32;
    let client = STACK
        .endpoints()
        .client::<ErgotPingEndpoint>(
            // link-local address of the server's router, the router rewrites it to our assigned net_id.
            Address {
                network_id: 0,
                node_id: CENTRAL_NODE_ID,
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

#[embassy_executor::task]
async fn discovery_responder() {
    let info = DeviceInfo {
        name: Some("IOBoard".try_into().unwrap()),
        description: Some("MakerPnP - IOBoard".try_into().unwrap()),
        unique_id: 0,
    };

    STACK
        .services()
        .device_info_handler::<4>(&info)
        .await;
}

// TODO replace with the the load-cell data type and topic
topic!(YeetTopic, Yeet, "topic/yeet");

#[derive(Debug, Clone, Copy)]
enum YeetCommand {
    Begin,
    End,
}

static YEET_COMMAND_CHANNEL: Channel<ThreadModeRawMutex, YeetCommand, 1> = Channel::new();

type YeetCommandSender = Sender<'static, ThreadModeRawMutex, YeetCommand, 1>;
type YeetCommandReceiver = Receiver<'static, ThreadModeRawMutex, YeetCommand, 1>;

#[embassy_executor::task]
async fn yeeter(receiver: YeetCommandReceiver) {
    let mut counter = 0;
    let mut error_counter = 0;

    defmt::info!("Yeeter started");

    // FIXME remove this arbitrary startup delay
    Timer::after(Duration::from_secs(8)).await;

    // Using a target frequency of 320Hz, the same as the HX717 load-cell ADC sensor
    const TARGET_HZ: u16 = 320;
    let mut cycle_ticker: Option<Ticker> = None;
    let mut report_ticker = Ticker::every(Duration::from_secs(10));

    loop {
        let cycle_ticker_fut = async {
            if let Some(ticker) = &mut cycle_ticker {
                ticker.next().await
            } else {
                core::future::pending().await
            }
        };

        match embassy_futures::select::select3(report_ticker.next(), receiver.receive(), cycle_ticker_fut).await {
            embassy_futures::select::Either3::First(_) => {
                info!("Yeet report, counter: {}, errors: {}", counter, error_counter);
            }
            embassy_futures::select::Either3::Second(cmd) => match cmd {
                YeetCommand::Begin => {
                    cycle_ticker = Some(Ticker::every(Duration::from_micros(1_000_000_u64 / TARGET_HZ as u64)));
                }
                YeetCommand::End => {
                    cycle_ticker = None;
                }
            },
            embassy_futures::select::Either3::Third(_) => {
                let Some(ref mut cycle_ticker) = cycle_ticker else {
                    continue;
                };

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
            }
        }
    }
}

topic!(CommandTopic, IoBoardCommand, "topic/ioboard/command");

#[embassy_executor::task]
async fn command_listener(yeet_command_sender: YeetCommandSender) {
    let subber = STACK
        .topics()
        .bounded_receiver::<CommandTopic, 32>(None);
    let subber = pin!(subber);
    let mut hdl = subber.subscribe();

    defmt::info!("Command listener started");
    loop {
        tracepin::on(3);
        let msg = hdl.recv().await;
        tracepin::off(3);
        match msg.t {
            IoBoardCommand::Test(counter) => {
                defmt::info!("Test command received: {}", counter);
            }
            IoBoardCommand::BeginYeetTest => {
                yeet_command_sender
                    .send(YeetCommand::Begin)
                    .await;
            }
            IoBoardCommand::EndYeetTest => {
                yeet_command_sender
                    .send(YeetCommand::End)
                    .await;
            }
        }
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

    let remote_endpoint = (Ipv4Address::new(192, 168, 18, 60), 8000);
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
