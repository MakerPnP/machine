use std::collections::HashMap;
use std::io;
use std::sync::Arc;

use camera::CameraHandle;
use ergot::toolkits::tokio_udp::{RouterStack, register_router_interface};
use ioboard::IOBOARD_TX_BUFFER_SIZE;
use log::info;
use networking::UDP_OVER_ETH_ERGOT_PAYLOAD_SIZE_MAX;
use operator::OPERATOR_TX_BUFFER_SIZE;
use operator_shared::camera::CameraIdentifier;
use server_common::camera::CameraDefinition;
use tokio::sync::broadcast::Receiver;
use tokio::sync::{Mutex, broadcast};
use tokio::{net::UdpSocket, signal};

pub mod camera;
pub mod ioboard;
pub mod networking;
pub mod operator;

pub mod config;

#[tokio::main]
async fn main() -> io::Result<()> {
    env_logger::init();
    console_subscriber::init();

    let _ = server_vision::dump_cameras()
        .inspect_err(|e| info!("Error dumping cameras: {:?}", e));
    
    let camera_definitions = config::camera_definitions();

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
        .spawn(networking::basic_services(
            stack.clone(),
            0_u16,
            app_event_tx.subscribe(),
        ))?;
    let yeet_listener_handle = tokio::task::Builder::new()
        .name("ergot/yeet-listener")
        .spawn(networking::yeet_listener(stack.clone(), app_event_tx.subscribe()))?;

    let app_state = Arc::new(Mutex::new(AppState {
        camera_definitions,
        event_tx: app_event_tx.clone(),
        camera_clients: Arc::new(Mutex::new(HashMap::new())),
    }));

    // TODO give the app_state to these tasks
    let ioboard_command_sender_handle = tokio::task::Builder::new()
        .name("io-board/command-sender")
        .spawn(ioboard::io_board_command_sender(
            stack.clone(),
            app_event_tx.subscribe(),
        ))?;

    let operator_listener_handle = tokio::task::Builder::new()
        .name("operator/command-listener")
        .spawn(operator::operator_listener(stack.clone(), app_state))?;

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

pub struct AppState {
    camera_definitions: Vec<CameraDefinition>,
    event_tx: broadcast::Sender<AppEvent>,
    camera_clients: Arc<Mutex<HashMap<CameraIdentifier, CameraHandle>>>,
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AppEvent {
    Shutdown,
}
