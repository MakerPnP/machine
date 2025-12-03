use std::collections::HashMap;
use std::pin::pin;
use std::sync::Arc;
use std::time::Duration;

use ergot::toolkits::tokio_udp::RouterStack;
use ergot::{Address, endpoint};
use log::{debug, info, warn};
use operator_shared::camera::{
    CameraCommand, CameraCommandError, CameraCommandErrorCode, CameraIdentifier, CameraStreamerCommandResult,
};
use operator_shared::commands::{OperatorCommandRequest, OperatorCommandResponse};
use tokio::select;
use tokio::sync::Mutex;
use tokio_util::sync::CancellationToken;

use crate::AppState;
use crate::camera::{CameraHandle, camera_definition_for_identifier, camera_manager};

// TODO configure these more appropriately.
//      for the operator TX we need to send camera streams and the broadcast packets from the IO boards,
//      so the buffer needs to be fairly large to prevent `InterfaceFull` errors.
pub const OPERATOR_TX_BUFFER_SIZE: usize = 1024 * 1024;

endpoint!(
    OperatorCommandEndpoint,
    OperatorCommandRequest,
    OperatorCommandResponse,
    "topic/operator/command"
);

pub async fn operator_listener(stack: RouterStack, app_state: Arc<Mutex<AppState>>) {
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

    let mut camera_managers = HashMap::new();

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
                        info!("heartbeat received from: {:?}, value: {}", msg.hdr.src, value);
                        OperatorCommandResponse::Acknowledged
                    }
                    OperatorCommandRequest::CameraCommand(identifier, camera_command) => {
                        match camera_command {
                            CameraCommand::StartStreaming { port_id, fps } => {
                                let camera_definition = {
                                    let app_state = app_state.lock().await;
                                    let Some(camera_definition) = camera_definition_for_identifier(&app_state.camera_definitions, identifier) else {
                                        return OperatorCommandResponse::CameraCommandResult(
                                            Err(CameraCommandError::new(CameraCommandErrorCode::InvalidIdentifier))
                                        )
                                    };

                                    let clients = clients.lock().await;
                                    if clients.contains_key(&identifier) {
                                        return OperatorCommandResponse::CameraCommandResult(
                                            Err(CameraCommandError::new(CameraCommandErrorCode::Busy))
                                        )
                                    }
                                    camera_definition.clone()
                                };

                                let address = Address {
                                    network_id: source.network_id,
                                    node_id: source.node_id,
                                    port_id: *port_id
                                };

                                let camera_shutdown_flag = CancellationToken::new();
                                let camera_manager = tokio::spawn(camera_manager(*identifier, camera_definition, address, app_state.clone(), *fps, camera_shutdown_flag.clone(), stack.clone()));
                                camera_managers.insert(*identifier, (camera_manager, camera_shutdown_flag));

                                OperatorCommandResponse::CameraCommandResult(
                                    Ok(CameraStreamerCommandResult::Acknowledged)
                                )
                            }
                            CameraCommand::StopStreaming { port_id } => {
                                if let Some((handle, shutdown_flag)) = camera_managers.remove(&identifier) {
                                    // spawn a task to shutdown the camera manager, then respond immediately.
                                    tokio::spawn({
                                        let port_id = *port_id;
                                        let identifier = *identifier;
                                        async move {
                                            info!("Stopping camera. identifier: {}. port_id: {}", identifier, port_id);
                                            shutdown_flag.cancel();
                                            let _ = handle.await;
                                            info!("Camera stopped. identifier: {}. port_id: {}", identifier, port_id);
                                        }
                                    });

                                    OperatorCommandResponse::CameraCommandResult(
                                        Ok(CameraStreamerCommandResult::Acknowledged)
                                    )
                                } else {
                                    OperatorCommandResponse::CameraCommandResult(
                                        Err(CameraCommandError::new(CameraCommandErrorCode::NotStreaming))
                                    )
                                }
                            },
                        }
                    }
                }
            }) => {}
        }
    }

    info!("Shutting down all cameras");
    for (_identifier, (handle, shutdown_flag)) in camera_managers.into_iter() {
        shutdown_flag.cancel();
        let _ = handle.await;
    }

    info!("Camera command handler stopped");
}
