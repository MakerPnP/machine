#[cfg(feature = "machine-vision")]
use std::collections::HashMap;
use std::fs;
use std::io::IsTerminal;
use std::sync::Arc;

use anyhow::bail;
#[cfg(feature = "machine-vision")]
use camera::CameraHandle;
use clap::Parser;
use config::OPERATOR_LOCAL_ADDR;
use ergot::toolkits::tokio_udp::{RouterStack, register_router_interface};
use log::info;
use networking::UDP_OVER_ETH_ERGOT_PAYLOAD_SIZE_MAX;
use operator::OPERATOR_TX_BUFFER_SIZE;
#[cfg(feature = "machine-vision")]
use operator_shared::camera::CameraIdentifier;
#[cfg(feature = "machine-vision")]
use server_common::camera::DetectedCamera;
use tokio::sync::broadcast::Receiver;
use tokio::select;
use tokio::sync::{Mutex, broadcast, watch};
use tokio::{net::UdpSocket, signal};

use crate::config::Config;
use crate::logging::store::LogStore;
use crate::tui::Tui;
use crate::tui::app::App;

#[cfg(feature = "machine-vision")]
pub mod camera;
pub mod ioboard;
pub mod logging;
pub mod networking;
pub mod operator;
pub mod tui;

pub mod cli;
pub mod config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = cli::Args::parse();

    let use_tui = !args.no_tui && std::io::stdin().is_terminal() && std::io::stdout().is_terminal();
    let log_store = if use_tui {
        Some(logging::init_collector(args.verbosity_level, args.log_capacity))
    } else {
        logging::init_plain(args.verbosity_level);
        None
    };

    let result = run(args, log_store.clone()).await;

    // the log was only visible in the tui, which is gone
    if let Some((store, _)) = log_store {
        let _ = store.write_tail(tui::EXIT_LOG_ENTRIES, &mut std::io::stderr());
    }

    result
}

/// Runs the server until shutdown, with the tui if `log_store` is given.
async fn run(args: cli::Args, log_store: Option<(Arc<LogStore>, Option<String>)>) -> anyhow::Result<()> {
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
    config
        .validate()
        .map_err(|e| anyhow::format_err!("Invalid config. filename: {:?}, error: {}", confile_filename, e))?;

    // Create event channel
    let (app_event_tx, app_shutdown_rx) = broadcast::channel::<AppEvent>(16);

    // first, so the server fails to start if another server is already running on this machine.
    let io_board_discovery_socket = ioboard::discovery::bind_discovery_socket().await?;

    // after the discovery socket is bound, so cameras that another server is using are not opened.
    #[cfg(feature = "machine-vision")]
    let detected_cameras = {
        let definitions = config.cameras.clone();
        // opening OpenCV cameras blocks
        tokio::task::spawn_blocking(move || server_vision::detect_cameras(&definitions)).await?
    };

    let stack: RouterStack = RouterStack::new();

    // Not connected, the operator UI's address is learnt from the first packet it sends.
    let operator_udp_socket = UdpSocket::bind(OPERATOR_LOCAL_ADDR)
        .await
        .map_err(|e| {
            anyhow::format_err!(
                "Unable to create local UDP socket for operator UI. address: {}, error: {}",
                OPERATOR_LOCAL_ADDR,
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
    #[cfg(feature = "machine-vision")]
    let (camera_status_tx, camera_status_rx) = watch::channel(camera::status::camera_statuses(
        &config.cameras,
        &detected_cameras,
        &std::collections::HashSet::new(),
        camera::status::CaptureApis::ENABLED,
    ));

    let (io_board_status_tx, io_board_status_rx) = watch::channel(Vec::new());
    let io_board_discovery_handle = tokio::task::Builder::new()
        .name("io-board/discovery")
        .spawn(ioboard::discovery::io_board_discovery(
            stack.clone(),
            config.io_boards.clone(),
            io_board_discovery_socket,
            app_event_tx.subscribe(),
            io_board_status_tx,
        ))?;

    let app_state = Arc::new(Mutex::new(AppState {
        config,
        event_tx: app_event_tx.clone(),
        #[cfg(feature = "machine-vision")]
        camera_clients: Arc::new(Mutex::new(HashMap::new())),
        #[cfg(feature = "machine-vision")]
        detected_cameras,
        #[cfg(feature = "machine-vision")]
        camera_status_tx,
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

    // dropped after shutdown completes, so the shutdown can be seen in the tui
    let _tui = match log_store {
        Some((store, filter)) => Some(Tui::spawn(App::new(
            store,
            io_board_status_rx,
            #[cfg(feature = "machine-vision")]
            camera_status_rx,
            app_event_tx.clone(),
            filter,
        ))?),
        None => None,
    };

    // Wait for Ctrl+C, or for the tui to request shutdown, the tui receives Ctrl+C as a key press.
    select! {
        _ = signal::ctrl_c() => {
            let _ = app_event_tx.send(AppEvent::Shutdown);
        },
        _ = app_shutdown_handler(app_shutdown_rx) => {},
    }

    info!("Shut down requested, exiting");

    let _ = ioboard_command_sender_handle.await;
    let _ = operator_listener_handle.await;
    let _ = basic_services_handle.await;
    let _ = yeet_listener_handle.await;
    let _ = io_board_discovery_handle.await;

    info!("Shutdown complete");
    Ok(())
}

pub struct AppState {
    config: Config,
    event_tx: broadcast::Sender<AppEvent>,
    #[cfg(feature = "machine-vision")]
    camera_clients: Arc<Mutex<HashMap<CameraIdentifier, CameraHandle>>>,
    /// Detected at startup.
    #[cfg(feature = "machine-vision")]
    detected_cameras: Vec<DetectedCamera>,
    #[cfg(feature = "machine-vision")]
    camera_status_tx: watch::Sender<Vec<camera::status::CameraStatus>>,
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
