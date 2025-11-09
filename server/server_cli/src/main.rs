use std::collections::HashSet;
use std::convert::TryInto;
use std::sync::Arc;
use std::{io, pin::pin, time::Duration};

use ergot::wire_frames::MAX_HDR_ENCODED_SIZE;
use ergot::{
    toolkits::tokio_udp::{RouterStack, register_router_interface},
    topic,
    well_known::DeviceInfo,
};
use ioboard_shared::commands::IoBoardCommand;
use ioboard_shared::yeet::Yeet;
use log::{debug, error, info, warn};
use operator_shared::commands::OperatorCommand;
use server_common::camera::{CameraDefinition, CameraSource, CameraStreamConfig, OpenCVCameraConfig};
use server_vision::{CameraFrame, capture_loop};
use tokio::sync::{Mutex, broadcast};
use tokio::time::interval;
use tokio::{net::UdpSocket, select, time, time::sleep};

use crate::camera::camera_streamer;

pub mod camera;

pub const UDP_OVER_ETH_MTU: usize = 1500;
pub const IP_OVERHEAD_SIZE: usize = 20;
pub const UDP_OVERHEAD_SIZE: usize = 8;
pub const UDP_OVER_ETH_ERGOT_FRAME_SIZE_MAX: usize = UDP_OVER_ETH_MTU - IP_OVERHEAD_SIZE - UDP_OVERHEAD_SIZE;
pub const UDP_OVER_ETH_ERGOT_PAYLOAD_SIZE_MAX: usize = UDP_OVER_ETH_ERGOT_FRAME_SIZE_MAX - MAX_HDR_ENCODED_SIZE;

// TODO configure these more appropriately
const OPERATOR_TX_BUFFER_SIZE: usize = 1024 * 10;
const IOBOARD_TX_BUFFER_SIZE: usize = 4096;

// must be less than the MTU of the network interface + ip + udp + ergot + chunking overhead
const CAMERA_CHUNK_SIZE: usize = 1024;

topic!(YeetTopic, Yeet, "topic/yeet");
topic!(IoBoardCommandTopic, IoBoardCommand, "topic/ioboard/command");
topic!(OperatorCommandTopic, OperatorCommand, "topic/operator/command");

#[tokio::main]
async fn main() -> io::Result<()> {
    env_logger::init();

    let camera_definitions = vec![CameraDefinition {
        name: "Default camera".to_string(),
        source: CameraSource::OpenCV(OpenCVCameraConfig {
            identifier: "TODO".to_string(),
        }),
        stream_config: CameraStreamConfig {
            jpeg_quality: 95,
        },
        width: 1280,
        height: 768,
        fps: 25,
    }];

    let stack: RouterStack = RouterStack::new();

    let io_board_udp_socket = UdpSocket::bind("192.168.18.41:8000")
        .await
        .unwrap();
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

    let operator_udp_socket = UdpSocket::bind("192.168.18.41:8001")
        .await
        .unwrap();
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

    tokio::task::spawn(basic_services(stack.clone(), 0_u16));
    tokio::task::spawn(yeet_listener(stack.clone()));

    let mut cameras = Vec::new();

    // TODO move this into a command handler so that cameras can be added/removed dynamically at runtime
    for (camera_index, camera_definition) in camera_definitions.iter().enumerate() {
        // TODO document the '* 2' magic number, try reducing it too.
        let broadcast_cap = (camera_definition.fps * 2) as usize;

        // Create broadcast channel for frames (Arc<Bytes> so we cheaply clone for each client)
        let (tx, rx) = broadcast::channel::<Arc<CameraFrame>>(broadcast_cap);

        // Spawn tasks

        let capture_handle = tokio::task::spawn({
            let camera_definition = camera_definition.clone();
            async move {
                if let Err(e) = capture_loop(tx, camera_definition).await {
                    error!("capture loop error: {e:?}");
                }
            }
        });
        let streamer_handle = tokio::task::spawn({
            let camera_definition = camera_definition.clone();
            let stack = stack.clone();
            async move {
                if let Err(e) = camera_streamer(stack, rx, camera_definition, CAMERA_CHUNK_SIZE).await {
                    error!("capture loop error: {e:?}");
                }
            }
        });

        cameras.push(CameraHandle {
            capture_handle,
            streamer_handle,
            camera_index,
        });
    }

    let app_state = Arc::new(Mutex::new(AppState {
        cameras,
    }));

    // TODO give the app_state to these tasks
    tokio::task::spawn(io_board_command_sender(stack.clone()));
    tokio::task::spawn(operator_listener(stack.clone()));

    loop {
        println!("Waiting for messages...");
        sleep(Duration::from_secs(1)).await;
    }
}

struct CameraHandle {
    capture_handle: tokio::task::JoinHandle<()>,
    streamer_handle: tokio::task::JoinHandle<()>,
    camera_index: usize,
}

struct AppState {
    cameras: Vec<CameraHandle>,
}

async fn basic_services(stack: RouterStack, port: u16) {
    let info = DeviceInfo {
        name: Some("Ergot router".try_into().unwrap()),
        description: Some("A central router".try_into().unwrap()),
        unique_id: port.into(),
    };
    // allow for discovery
    let disco_answer = stack
        .services()
        .device_info_handler::<4>(&info);
    // handle incoming ping requests
    let ping_answer = stack.services().ping_handler::<4>();
    // custom service for doing discovery regularly
    let disco_req = tokio::spawn(do_discovery(stack.clone()));
    // forward log messages to the log crate output
    let log_handler = stack.services().log_handler(16);

    // These all run together, we run them in a single task
    select! {
        _ = disco_answer => {},
        _ = ping_answer => {},
        _ = disco_req => {},
        _ = log_handler => {},
    }
}

async fn do_discovery(stack: RouterStack) {
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

async fn io_board_command_sender(stack: RouterStack) {
    let mut ctr = 0;
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let command = IoBoardCommand::Test(ctr);
        stack
            .topics()
            .broadcast::<IoBoardCommandTopic>(&command, None)
            .unwrap();
        ctr += 1;

        tokio::time::sleep(Duration::from_secs(5)).await;
        stack
            .topics()
            .broadcast::<IoBoardCommandTopic>(&IoBoardCommand::BeginYeetTest, None)
            .unwrap();

        tokio::time::sleep(Duration::from_secs(5)).await;
        stack
            .topics()
            .broadcast::<IoBoardCommandTopic>(&IoBoardCommand::EndYeetTest, None)
            .unwrap();
    }
}

async fn yeet_listener(stack: RouterStack) {
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
        }
    }
}

async fn operator_listener(stack: RouterStack) {
    let subber = stack
        .topics()
        .heap_bounded_receiver::<OperatorCommandTopic>(64, None);
    let subber = pin!(subber);
    let mut hdl = subber.subscribe();

    let timeout_duration = Duration::from_secs(10);
    loop {
        let timeout = tokio::time::sleep(timeout_duration);
        select! {
            _ = timeout => {
                warn!("operator timeout (no command received). duration: {}", timeout_duration.as_secs());
            }
            msg = hdl.recv() => {
                debug!("{}: got {:?}", msg.hdr, msg.t);
                match msg.t {
                    OperatorCommand::Heartbeat(value) => {
                        info!("OperatorCommand::Heartbeat.  value: {}", value);
                    }
                }
            }
        }
    }
}
