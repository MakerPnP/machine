use std::time::Duration;

use ergot::toolkits::tokio_udp::EdgeStack;
use ergot::{Address, endpoint};
use operator_shared::commands::{OperatorCommandRequest, OperatorCommandResponse};
use tokio::sync::broadcast::Receiver;
use tokio::{select, time};
use tracing::error;

use crate::events::AppEvent;
use crate::net::shutdown::app_shutdown_handler;

endpoint!(
    OperatorCommandEndpoint,
    OperatorCommandRequest,
    OperatorCommandResponse,
    "topic/operator/command"
);

pub async fn heartbeat_sender(stack: EdgeStack, address: Address, app_event_rx: Receiver<AppEvent>) {
    let mut app_shutdown_handler = Box::pin(app_shutdown_handler(app_event_rx));

    let command_client = stack
        .endpoints()
        .client::<OperatorCommandEndpoint>(address, None);
    let command_client = ergot_util::ClientWrapper::new(Duration::from_secs(1), command_client);

    let mut index = 0;
    tokio::time::sleep(Duration::from_secs(1)).await;
    let heartbeat_timeout_duration = Duration::from_secs(10);
    let heartbeat_send_interval = heartbeat_timeout_duration / 2;
    let mut ticker = time::interval(heartbeat_send_interval);

    loop {
        // At either stage (waiting response or waiting for tick) we could receive a shutdown event

        let request = OperatorCommandRequest::Heartbeat(index);
        select! {
            response = command_client.request(&request) => {
                match response {
                    Ok(response) => {
                        match response {
                            OperatorCommandResponse::Acknowledged => {
                                index = index.wrapping_add(1);
                            }
                            _ => {}
                        }
                    }
                    Err(e) => {
                        error!("error sending heartbeat. index: {}, error: {:?}", index, e);
                    }
                }
            }
            _ = &mut app_shutdown_handler => {
                break
            }
        }

        select! {
            _ = ticker.tick() => {
                index = index.wrapping_add(1);
            }
            _ = &mut app_shutdown_handler => {
                break
            }
        }
    }
}
