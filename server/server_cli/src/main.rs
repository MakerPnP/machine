use std::collections::{HashMap, HashSet};
use std::convert::TryInto;
use std::sync::Arc;
use std::{io, pin::pin, time::Duration};

use ergot::wire_frames::MAX_HDR_ENCODED_SIZE;
use ergot::{
    Address, endpoint,
    toolkits::tokio_udp::{RouterStack, register_router_interface},
    topic,
    well_known::DeviceInfo,
};
use ioboard_shared::commands::IoBoardCommand;
use ioboard_shared::yeet::Yeet;
use log::{debug, error, info, warn};
use operator_shared::camera::{
    CameraCommand, CameraCommandError, CameraCommandErrorCode, CameraIdentifier, CameraStreamerCommandResult,
};
use operator_shared::commands::{OperatorCommandRequest, OperatorCommandResponse};
use server_common::camera::{CameraDefinition, CameraSource, CameraStreamConfig, OpenCVCameraConfig};
use server_vision::{CameraFrame, capture_loop};
use tokio::sync::broadcast::Receiver;
use tokio::sync::{Mutex, broadcast};
use tokio::time::interval;
use tokio::{net::UdpSocket, select, signal, time};
use tokio_util::sync::CancellationToken;

use crate::camera::{camera_definition_for_identifier, camera_streamer};

pub mod camera;

pub const UDP_OVER_ETH_MTU: usize = 1500;
pub const IP_OVERHEAD_SIZE: usize = 20;
pub const UDP_OVERHEAD_SIZE: usize = 8;
pub const UDP_OVER_ETH_ERGOT_FRAME_SIZE_MAX: usize = UDP_OVER_ETH_MTU - IP_OVERHEAD_SIZE - UDP_OVERHEAD_SIZE;
pub const UDP_OVER_ETH_ERGOT_PAYLOAD_SIZE_MAX: usize = UDP_OVER_ETH_ERGOT_FRAME_SIZE_MAX - MAX_HDR_ENCODED_SIZE;

// TODO configure these more appropriately.
//      for the operator TX we need to send camera streams and the broadcast packets from the IO boards,
//      so the buffer needs to be fairly large to prevent `InterfaceFull` errors.
const OPERATOR_TX_BUFFER_SIZE: usize = 1024 * 1024;
const IOBOARD_TX_BUFFER_SIZE: usize = 4096;

// must be less than the MTU of the network interface + ip + udp + ergot + chunking overhead
const CAMERA_CHUNK_SIZE: usize = 1024;

topic!(YeetTopic, Yeet, "topic/yeet");
topic!(IoBoardCommandTopic, IoBoardCommand, "topic/ioboard/command");
endpoint!(
    OperatorCommandEndpoint,
    OperatorCommandRequest,
    OperatorCommandResponse,
    "topic/operator/command"
);

#[tokio::main]
async fn main() -> io::Result<()> {
    env_logger::init();
    console_subscriber::init();

    let camera_definitions = vec![CameraDefinition {
        name: "Default camera".to_string(),
        source: CameraSource::OpenCV(OpenCVCameraConfig {
            identifier: "TODO".to_string(),
        }),
        stream_config: CameraStreamConfig {
            jpeg_quality: 95,
        },
        // width: 1920,
        // height: 1280,
        width: 640,
        height: 480,
        fps: 30.0,
        four_cc: Some(['Y', 'U', 'Y', '2']),
    }];

    // Create event channel
    let (app_event_tx, app_event_rx) = broadcast::channel::<AppEvent>(16);
    drop(app_event_rx);

    let stack: RouterStack = RouterStack::new();

    let io_board_udp_socket = UdpSocket::bind("192.168.18.41:8000").await?;
    let io_board_remote_addr = "192.168.18.64:8000";
    io_board_udp_socket
        .connect(io_board_remote_addr)
        .await?;
    register_router_interface(
        &stack,
        io_board_udp_socket,
        UDP_OVER_ETH_ERGOT_PAYLOAD_SIZE_MAX as _,
        IOBOARD_TX_BUFFER_SIZE,
    )
    .await
    .unwrap();

    let operator_udp_socket = UdpSocket::bind("192.168.18.41:8001").await?;
    let operator_remote_addr = "192.168.18.41:8002";
    operator_udp_socket
        .connect(operator_remote_addr)
        .await?;

    register_router_interface(
        &stack,
        operator_udp_socket,
        UDP_OVER_ETH_ERGOT_PAYLOAD_SIZE_MAX as _,
        OPERATOR_TX_BUFFER_SIZE,
    )
    .await
    .unwrap();

    let basic_services_handle = tokio::task::Builder::new()
        .name("ergot/basic-services")
        .spawn(basic_services(stack.clone(), 0_u16, app_event_tx.subscribe()))?;
    let yeet_listener_handle = tokio::task::Builder::new()
        .name("ergot/yeet-listener")
        .spawn(yeet_listener(stack.clone(), app_event_tx.subscribe()))?;

    let app_state = Arc::new(Mutex::new(AppState {
        camera_definitions,
        event_tx: app_event_tx.clone(),
        camera_clients: Arc::new(Mutex::new(HashMap::new())),
    }));

    // TODO give the app_state to these tasks
    let ioboard_command_sender_handle = tokio::task::Builder::new()
        .name("io-board/command-sender")
        .spawn(io_board_command_sender(stack.clone(), app_event_tx.subscribe()))?;

    let operator_listener_handle = tokio::task::Builder::new()
        .name("operator/command-listener")
        .spawn(operator_listener(stack.clone(), app_state))?;

    // Wait for Ctrl+C
    let _ = signal::ctrl_c().await;

    app_event_tx
        .send(AppEvent::Shutdown)
        .unwrap();

    info!("Shut down requested, exiting");

    let _ = ioboard_command_sender_handle.await;
    let _ = operator_listener_handle.await;
    let _ = basic_services_handle.await;
    let _ = yeet_listener_handle.await;

    info!("Shutdown complete");
    Ok(())
}

struct CameraHandle {
    capture_handle: tokio::task::JoinHandle<()>,
    streamer_handle: tokio::task::JoinHandle<()>,
    address: Address,
    shutdown_flag: CancellationToken,
}

struct AppState {
    camera_definitions: Vec<CameraDefinition>,
    event_tx: broadcast::Sender<AppEvent>,
    camera_clients: Arc<Mutex<HashMap<CameraIdentifier, CameraHandle>>>,
}

async fn basic_services(stack: RouterStack, port: u16, app_event_rx: Receiver<AppEvent>) {
    let info = DeviceInfo {
        name: Some("Ergot router".try_into().unwrap()),
        description: Some("A central router".try_into().unwrap()),
        unique_id: port.into(),
    };
    // allow for discovery
    let device_discovery_responder = stack
        .services()
        .device_info_handler::<4>(&info);
    // handle incoming ping requests
    let ping_responder = stack.services().ping_handler::<4>();
    // custom service for doing device discovery regularly
    let device_discovery = tokio::task::Builder::new()
        .name("ergot/device-discovery")
        .spawn(do_device_discovery(stack.clone()))
        .unwrap();
    // forward log messages to the log crate output
    let log_handler = stack.services().log_handler(16);
    // handle socket discovery requests
    let socket_discovery_responder = stack
        .services()
        .socket_query_handler::<4>();

    let app_shutdown_handler = app_shutdown_handler(app_event_rx);

    // These all run together, we run them in a single task
    select! {
        _ = ping_responder => {},
        _ = log_handler => {},
        _ = device_discovery_responder => {},
        _ = socket_discovery_responder => {},
        _ = device_discovery => {},
        _ = app_shutdown_handler => {
            info!("basic services shutdown requested, stopping");
        },
    }
}

async fn app_shutdown_handler(mut receiver: Receiver<AppEvent>) {
    loop {
        let app_event = receiver.recv().await;
        match app_event {
            Ok(event) => match event {
                AppEvent::Shutdown => break,
            },
            Err(_) => break,
        }
    }
}

async fn do_device_discovery(stack: RouterStack) {
    let mut max = 16;
    let mut seen = HashSet::new();
    let mut ticker = interval(Duration::from_secs(10));
    loop {
        let new_seen = stack
            .discovery()
            .discover(max, Duration::from_millis(250))
            .await;
        max = max.max(seen.len() * 2);
        let new_seen = HashSet::from_iter(new_seen);
        let added = new_seen.difference(&seen);
        for add in added {
            warn!("Added:   {add:?}");
        }
        let removed = seen.difference(&new_seen);
        for rem in removed {
            warn!("Removed: {rem:?}");
        }
        seen = new_seen;

        info!("Discovery list:");
        for (index, item) in seen.iter().enumerate() {
            info!("{}: {:?}", index, item);
        }

        ticker.tick().await;
    }
}

async fn io_board_command_sender(stack: RouterStack, app_event_rx: Receiver<AppEvent>) {
    let mut app_shutdown_handler = Box::pin(crate::app_shutdown_handler(app_event_rx));

    enum Phase {
        One,
        Two,
        Three,
    }
    let mut ctr = 0;
    let mut phase = Phase::One;
    loop {
        match phase {
            Phase::One => {
                select! {
                    _ = &mut app_shutdown_handler => {
                        break
                    }
                    _ = tokio::time::sleep(Duration::from_secs(1)) => {},
                }
                let command = IoBoardCommand::Test(ctr);
                stack
                    .topics()
                    .broadcast::<IoBoardCommandTopic>(&command, None)
                    .unwrap();
                ctr += 1;
                phase = Phase::Two
            }
            Phase::Two => {
                select! {
                    _ = &mut app_shutdown_handler => {
                        break
                    }
                    _ = tokio::time::sleep(Duration::from_secs(5)) => {},
                }
                stack
                    .topics()
                    .broadcast::<IoBoardCommandTopic>(&IoBoardCommand::BeginYeetTest, None)
                    .unwrap();
                phase = Phase::Three
            }
            Phase::Three => {
                select! {
                    _ = &mut app_shutdown_handler => {
                        break
                    }
                    _ = tokio::time::sleep(Duration::from_secs(5)) => {},
                }
                stack
                    .topics()
                    .broadcast::<IoBoardCommandTopic>(&IoBoardCommand::EndYeetTest, None)
                    .unwrap();

                phase = Phase::One
            }
        }
    }
    info!("io board command sender shutdown");
}

async fn yeet_listener(stack: RouterStack, app_event_rx: Receiver<AppEvent>) {
    let mut app_shutdown_handler = Box::pin(crate::app_shutdown_handler(app_event_rx));

    let subber = stack
        .topics()
        .heap_bounded_receiver::<YeetTopic>(64, None);
    let subber = pin!(subber);
    let mut hdl = subber.subscribe();

    let mut packets_this_interval = 0;
    let interval = Duration::from_secs(1);
    let mut ticker = time::interval(interval);
    loop {
        select! {
            _ = ticker.tick() => {
                info!("packet rate: {}/{:?}", packets_this_interval, interval);
                packets_this_interval = 0;
            }
            msg = hdl.recv() => {
                packets_this_interval += 1;
                debug!("{}: got {}", msg.hdr, msg.t);
            }
            _ = &mut app_shutdown_handler => {
                info!("yeet listener shutdown requested, stopping");
                break
            }
        }
    }
}

async fn operator_listener(stack: RouterStack, app_state: Arc<Mutex<AppState>>) {
    let (app_event_rx, clients) = {
        let app_state = app_state.lock().await;
        let app_event_rx = app_state.event_tx.subscribe();
        let clients: Arc<Mutex<HashMap<CameraIdentifier, CameraHandle>>> = app_state.camera_clients.clone();
        (app_event_rx, clients)
    };

    let mut app_shutdown_handler = Box::pin(crate::app_shutdown_handler(app_event_rx));

    let server_socket = stack
        .endpoints()
        .single_server::<OperatorCommandEndpoint>(None);
    let server_socket = pin!(server_socket);
    let mut hdl = server_socket.attach();
    let command_server_port_id = hdl.port();

    info!("Camera command server, port_id: {}", command_server_port_id);

    let timeout_duration = Duration::from_secs(10);
    loop {
        let timeout = tokio::time::sleep(timeout_duration);
        select! {
            _ = timeout => {
                warn!("operator timeout (no command received). duration: {}", timeout_duration.as_secs());
            }
            _ = &mut app_shutdown_handler => {
                info!("operator shutdown requested, stopping camera command handler");
                break
            }
            _ = hdl.serve_full(async |msg| {
                let request = &msg.t;
                let source = &msg.hdr.src;
                match request {
                    OperatorCommandRequest::Heartbeat(value) => {
                        // TODO ergot API currently doesn't give us the message header, so we can't track who the message was from.
                        //debug!("heartbeat received from: {:?}, value: {}", hdr.address, value);
                        debug!("heartbeat received. value: {}", value);
                        OperatorCommandResponse::Acknowledged
                    }
                    OperatorCommandRequest::CameraCommand(identifier, camera_command) => {
                        match camera_command {
                            CameraCommand::StartStreaming { port_id } => {
                                let address = Address {
                                    network_id: source.network_id,
                                    node_id: source.node_id,
                                    port_id: *port_id
                                };
                                let app_state = app_state.lock().await;
                                let mut clients = clients.lock().await;
                                let Some(camera_definition) = camera_definition_for_identifier(&app_state.camera_definitions, identifier) else {
                                    return OperatorCommandResponse::CameraCommandResult(
                                        Err(CameraCommandError { code: CameraCommandErrorCode::InvalidIdentifier, args: Vec::new()})
                                    )
                                };

                                if clients.contains_key(&identifier) {
                                    return OperatorCommandResponse::CameraCommandResult(
                                        Err(CameraCommandError { code: CameraCommandErrorCode::Busy, args: Vec::new()})
                                    )
                                }

                                // TODO document the '* 2' magic number, try reducing it too.
                                let broadcast_cap = (camera_definition.fps * 2_f32).round() as usize;

                                // Create broadcast channel for frames (Arc<Bytes> so we cheaply clone for each client)
                                let (tx, rx) = broadcast::channel::<Arc<CameraFrame>>(broadcast_cap);

                                let client_shutdown_flag = CancellationToken::new();

                                // Spawn tasks

                                let capture_handle = tokio::task::Builder::new().name(&format!("camera-{}/capture", identifier)).spawn({
                                    let camera_definition = camera_definition.clone();
                                    let client_shutdown_flag = client_shutdown_flag.clone();
                                    async move {
                                        if let Err(e) = capture_loop(tx, camera_definition, client_shutdown_flag).await {
                                            error!("capture loop error: {e:?}");
                                        }
                                    }
                                }).unwrap();
                                let streamer_handle = tokio::task::Builder::new().name(&format!("camera-{}/streamer", identifier)).spawn({
                                    let camera_definition = camera_definition.clone();
                                    let stack = stack.clone();
                                    let client_shutdown_flag = client_shutdown_flag.clone();
                                    async move {
                                        if let Err(e) = camera_streamer(stack, rx, camera_definition, CAMERA_CHUNK_SIZE, address, client_shutdown_flag).await {
                                            error!("streamer loop error: {e:?}");
                                        }
                                    }
                                }).unwrap();

                                clients.insert(identifier.clone(), CameraHandle {
                                    capture_handle,
                                    streamer_handle,
                                    address,
                                    shutdown_flag: client_shutdown_flag
                                });

                                info!("Streaming started. identifier: {}, port_id: {}", identifier, port_id);

                                OperatorCommandResponse::CameraCommandResult(
                                    Ok(CameraStreamerCommandResult::Acknowledged)
                                )
                            }
                            CameraCommand::StopStreaming { port_id } => {
                                let mut clients = clients.lock().await;
                                if let Some(client) = clients.remove(&identifier) {
                                    client.shutdown_flag.cancel();

                                    // wait for the capture first, then the streamer
                                    let _ = client.capture_handle.await;
                                    let _ = client.streamer_handle.await;
                                }
                                info!("Streaming stopped. port_id: {}", port_id);

                                OperatorCommandResponse::CameraCommandResult(
                                    Ok(CameraStreamerCommandResult::Acknowledged)
                                )
                            },
                        }
                    }
                }
            }) => {}
        }
    }

    let mut clients = clients.lock().await;
    let clients_to_cancel = clients.drain().collect::<Vec<_>>();

    for (index, (identifier, client)) in clients_to_cancel
        .into_iter()
        .enumerate()
    {
        info!(
            "Stopping streaming client {}. identifier: {}, address: {}",
            index, identifier, client.address
        );

        // TODO notify client that streaming is stopped

        client.shutdown_flag.cancel();

        let _ = client.capture_handle.await;
        let _ = client.streamer_handle.await;
    }
    info!("Camera command handler stopped");
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum AppEvent {
    Shutdown,
}
