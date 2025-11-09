use std::{pin::pin, time::Duration};

use eframe::epaint::ColorImage;
use egui_mobius::Value;
use ergot::traits::Endpoint;
use ergot::well_known::{NameRequirement, SocketQuery};
use ergot::{
    FrameKind,
    interface_manager::profiles::direct_edge::tokio_udp::InterfaceKind,
    toolkits::tokio_udp::{EdgeStack, new_std_queue, new_target_stack, register_edge_interface},
    topic,
};
use tokio::runtime::Handle;
use tokio::sync::broadcast;
use tokio::sync::watch::Sender;
use tokio::{net::UdpSocket, select, time};
use tracing::{debug, info, warn};

use crate::app::AppState;
use crate::events::AppEvent;
use crate::net::camera::camera_frame_listener;
use crate::net::commands::{OperatorCommandEndpoint, heartbeat_sender};
use crate::net::services::basic_services;
use crate::{LOCAL_ADDR, REMOTE_ADDR};

pub mod camera;
pub mod commands;
pub mod services;

pub async fn ergot_task(
    _spawner: Handle,
    state: Value<AppState>,
    tx_out: Sender<ColorImage>,
    app_event_tx: broadcast::Sender<AppEvent>,
) -> anyhow::Result<()> {
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

    tokio::task::spawn(basic_services(stack.clone(), port));
    tokio::task::spawn(yeet_listener(stack.clone(), app_event_tx.clone()));

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

        time::sleep(Duration::from_secs(1)).await;
    };

    // TODO just using the first one for now
    let command_endpoint_remote_address = discovery_results[0].address;

    let heartbeat_sender = tokio::task::spawn(heartbeat_sender(
        stack.clone(),
        command_endpoint_remote_address,
        app_event_tx.subscribe(),
    ));

    let camera_frame_listener_handle = {
        let context = state.lock().unwrap().context.clone();
        tokio::task::spawn(camera_frame_listener(
            stack.clone(),
            tx_out,
            context,
            command_endpoint_remote_address,
            app_event_tx.subscribe(),
        ))
    };

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

    info!("Waiting for camera frame listener to finish");
    let _ = camera_frame_listener_handle.await;
    info!("Waiting for heartbeat sender to finish");
    let _ = heartbeat_sender.await;

    info!("Network task shutdown");
    Ok(())
}

topic!(YeetTopic, u64, "topic/yeet");

async fn yeet_listener(stack: EdgeStack, app_event_tx: broadcast::Sender<AppEvent>) {
    let mut app_event_rx = app_event_tx.subscribe();
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
            app_event = app_event_rx.recv() => {
                match app_event {
                    Ok(event) => match event {
                        AppEvent::Shutdown => {
                            break
                        }
                    }
                    Err(_) => {
                        break
                    }
                }
            }
        }
    }
}
