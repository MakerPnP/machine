use std::{pin::pin, time::Duration};

use egui_mobius::Value;
use ergot::traits::Endpoint;
use ergot::well_known::{NameRequirement, SocketQuery};
use ergot::{
    FrameKind,
    interface_manager::profiles::direct_edge::tokio_udp::InterfaceKind,
    toolkits::tokio_udp::{EdgeStack, new_std_queue, new_target_stack, register_edge_interface},
    topic,
};
use operator_shared::camera::CameraIdentifier;
use tokio::sync::broadcast;
use tokio::{net::UdpSocket, select, time};
use tracing::{debug, error, info, warn};

use crate::app::{AppState, PaneKind};
use crate::events::AppEvent;
use crate::net::commands::{OperatorCommandEndpoint, heartbeat_sender};
use crate::net::services::basic_services;
use crate::net::shutdown::app_shutdown_handler;
use crate::workspace::{ToggleDefinition, WorkspaceError, Workspaces};
use crate::{LOCAL_ADDR, REMOTE_ADDR};

pub mod camera;
pub mod commands;
pub mod services;
pub mod shutdown;

pub async fn ergot_task(
    state: Value<AppState>,
    workspaces: Value<Workspaces>,
    app_event_tx: broadcast::Sender<AppEvent>,
) -> anyhow::Result<()> {
    info!("Starting networking on: {}", LOCAL_ADDR);

    let queue = new_std_queue(4096);
    let stack: EdgeStack = new_target_stack(&queue, 1024);
    let udp_socket = UdpSocket::bind(LOCAL_ADDR)
        .await
        .unwrap();

    // FIXME show a message in the UI if this fails instead of panicking when the port is already in use
    udp_socket
        .connect(REMOTE_ADDR)
        .await
        .unwrap();

    let port = udp_socket.local_addr().unwrap().port();

    register_edge_interface(&stack, udp_socket, &queue, InterfaceKind::Target)
        .await
        .unwrap();

    let basic_services_handle = tokio::task::Builder::new()
        .name("ergot/basic-services")
        .spawn(basic_services(stack.clone(), port, app_event_tx.subscribe()))?;

    let yeet_listener_handle = tokio::task::Builder::new()
        .name("ergot/yeet-listener")
        .spawn(yeet_listener(stack.clone(), app_event_tx.subscribe()))?;

    let discovery_results = loop {
        let query = SocketQuery {
            key: OperatorCommandEndpoint::REQ_KEY.to_bytes(),
            nash_req: NameRequirement::Any,
            frame_kind: FrameKind::ENDPOINT_REQ,
            broadcast: false,
        };

        let res = stack
            .discovery()
            .discover_sockets(4, Duration::from_secs(1), &query)
            .await;
        if res.is_empty() {
            warn!("No discovery results");
        } else {
            break res;
        }

        time::sleep(Duration::from_millis(250)).await;
    };
    info!("Found {} command endpoints", discovery_results.len());

    // TODO just using the first one for now
    let command_endpoint_remote_address = discovery_results[0].address;

    let heartbeat_sender = tokio::task::spawn(heartbeat_sender(
        stack.clone(),
        command_endpoint_remote_address,
        app_event_tx.subscribe(),
    ));

    // TODO enumerate the available cameras from the server
    let camera_identifiers = [
        CameraIdentifier::new(0),
        CameraIdentifier::new(1),
        // CameraIdentifier::new(2)
    ];

    info!("Starting cameras. ids: {:?}", camera_identifiers);
    for camera_identifier in camera_identifiers.iter() {
        {
            let app_state = state.lock().unwrap();
            app_state.add_camera(*camera_identifier, stack.clone(), command_endpoint_remote_address);
        }

        {
            let mut workspaces = workspaces.lock().unwrap();

            match workspaces.add_toggle(ToggleDefinition {
                key: "camera",
                kind: PaneKind::Camera {
                    id: camera_identifier.clone(),
                },
            }) {
                Err(WorkspaceError::DuplicateToggleKey) => {
                    // ignore, we already have a toggle with this key - from a previous session
                }
                Err(e) => {
                    error!("Failed to add toggle: {:?}", e);
                }
                Ok(()) => {}
            }
        }
    }

    let mut app_event_rx = app_event_tx.subscribe();

    loop {
        if let Ok(event) = app_event_rx.recv().await {
            match event {
                AppEvent::Shutdown => {
                    let state = state.lock().unwrap();
                    state.context.request_repaint();
                    break;
                }
            }
        }
    }
    info!("Network shut down requested");

    let camera_uis = {
        let app_state = state.lock().unwrap();
        app_state.prepare_stop_all_cameras()
    };
    AppState::stop_all_cameras(camera_uis).await;

    info!("Waiting for heartbeat sender to finish");
    let _ = heartbeat_sender.await;

    info!("Waiting for basic services to finish");
    let _ = basic_services_handle.await;
    info!("Waiting for yeet listener to finish");
    let _ = yeet_listener_handle.await;

    info!("Network task shutdown");
    Ok(())
}

topic!(YeetTopic, u64, "topic/yeet");

async fn yeet_listener(stack: EdgeStack, app_event_rx: broadcast::Receiver<AppEvent>) {
    let mut app_shutdown_handler = Box::pin(app_shutdown_handler(app_event_rx));

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
