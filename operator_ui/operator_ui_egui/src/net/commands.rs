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

    select! {
        _ = &mut app_shutdown_handler => {
            // Shutdown received
        }
        _ = heartbeat_loop(stack, address) => {
            // Heartbeat loop completed (shouldn't happen unless there's an error)
        }
    }
}

async fn heartbeat_loop(stack: EdgeStack, address: Address) {
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
        // Wait for tick
        ticker.tick().await;

        // Send heartbeat
        let request = OperatorCommandRequest::Heartbeat(index);
        match command_client.request(&request).await {
            Ok(response) => {
                match response {
                    OperatorCommandResponse::Acknowledged => {
                        // Success - proceed to next iteration
                    }
                    _ => {
                        error!("Unexpected response for heartbeat. index: {}", index);
                    }
                }
            }
            Err(e) => {
                error!("Error sending heartbeat. index: {}, error: {:?}", index, e);
            }
        }

        index = index.wrapping_add(1);
    }
}
