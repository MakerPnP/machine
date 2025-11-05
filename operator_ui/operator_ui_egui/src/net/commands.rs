use std::time::Duration;
use ergot::toolkits::tokio_udp::EdgeStack;
use ergot::topic;
use tracing::debug;
use operator_shared::commands::OperatorCommand;
use tokio::time;

topic!(OperatorCommandTopic, OperatorCommand, "topic/operator/command");

pub async fn command_sender(stack: EdgeStack) {
    let mut index = 0;
    tokio::time::sleep(Duration::from_secs(1)).await;
    let heartbeat_timeout_duration = Duration::from_secs(10);
    let heartbeat_send_interval = heartbeat_timeout_duration / 2;
    let mut ticker = time::interval(heartbeat_send_interval);
    loop {
        if stack
            .topics()
            .broadcast::<OperatorCommandTopic>(&OperatorCommand::Heartbeat(index), None)
            .inspect_err(|e|{
                debug!("error sending heartbeat. index: {}, error: {:?}", index, e);
            })
            .is_ok()
        {
            index = index.wrapping_add(1);
        };

        ticker.tick().await;
    }
}
