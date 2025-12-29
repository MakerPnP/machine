use std::collections::HashMap;
use std::fs;
use std::sync::Arc;

use anyhow::bail;
use camera::CameraHandle;
use clap::Parser;
use config::{IO_BOARD_LOCAL_ADDR, IO_BOARD_REMOTE_ADDR, OPERATOR_LOCAL_ADDR, OPERATOR_REMOTE_ADDR};
use ergot::toolkits::tokio_udp::{RouterStack, register_router_interface};
use ioboard::IOBOARD_TX_BUFFER_SIZE;
use log::info;
use networking::UDP_OVER_ETH_ERGOT_PAYLOAD_SIZE_MAX;
use operator::OPERATOR_TX_BUFFER_SIZE;
use operator_shared::camera::CameraIdentifier;
use tokio::sync::broadcast::Receiver;
use tokio::sync::{Mutex, broadcast};
use tokio::{net::UdpSocket, signal};

use crate::config::Config;

pub mod camera;
pub mod ioboard;
pub mod networking;
pub mod operator;

pub mod cli;
pub mod config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = cli::Args::parse();

    init_logging(args.verbosity_level);

    console_subscriber::init();

    let _ = server_vision::dump_cameras().inspect_err(|e| info!("Error dumping cameras: {:?}", e));

    let confile_filename = args.config;
    let Ok(config_content) = fs::read_to_string(&confile_filename) else {
        bail!(
            "Unable to read config file, make sure it exists and is readable. filename: {:?}",
            confile_filename
        )
    };
    let Ok(config) =
        ron::from_str::<Config>(&config_content).inspect_err(|e| info!("Error parsing config file: {:?}", e))
    else {
        bail!("Unable to load config. filename: {:?}", confile_filename)
    };

    // Create event channel
    let (app_event_tx, app_event_rx) = broadcast::channel::<AppEvent>(16);
    drop(app_event_rx);

    let stack: RouterStack = RouterStack::new();

    let io_board_udp_socket = UdpSocket::bind(IO_BOARD_LOCAL_ADDR)
        .await
        .map_err(|e| {
            anyhow::format_err!(
                "Unable to create local UDP socket for io boards. address: {}, error: {}",
                IO_BOARD_LOCAL_ADDR,
                e
            )
        })?;
    io_board_udp_socket
        .connect(IO_BOARD_REMOTE_ADDR)
        .await
        .map_err(|e| {
            anyhow::format_err!(
                "Unable to create remote UDP socket for io boards. address: {}, error: {}",
                IO_BOARD_REMOTE_ADDR,
                e
            )
        })?;

    register_router_interface(
        &stack,
        io_board_udp_socket,
        UDP_OVER_ETH_ERGOT_PAYLOAD_SIZE_MAX as _,
        IOBOARD_TX_BUFFER_SIZE,
    )
    .await
    .unwrap();

    let operator_udp_socket = UdpSocket::bind(OPERATOR_LOCAL_ADDR)
        .await
        .map_err(|e| {
            anyhow::format_err!(
                "Unable to create local UDP socket for operator UI. address: {}, error: {}",
                IO_BOARD_LOCAL_ADDR,
                e
            )
        })?;
    operator_udp_socket
        .connect(OPERATOR_REMOTE_ADDR)
        .await
        .map_err(|e| {
            anyhow::format_err!(
                "Unable to create UDP socket for operator UI. address: {}, error: {}",
                OPERATOR_REMOTE_ADDR,
                e
            )
        })?;

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
        config,
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
    config: Config,
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

fn init_logging(verbosity_level: u8) {
    let mut builder = env_logger::Builder::from_default_env();

    // Only override the default filter if RUST_LOG is NOT set
    if std::env::var_os("RUST_LOG").is_none() {
        let level = match verbosity_level {
            0 => log::LevelFilter::Warn,
            1 => log::LevelFilter::Info,
            2 => log::LevelFilter::Debug,
            _ => log::LevelFilter::Trace,
        };

        builder.filter_level(level);
    }

    builder.init();
}
