use ergot::toolkits::tokio_udp::RouterStack;
use ergot::topic;
use ioboard_shared::commands::IoBoardCommand;
use log::info;
use tokio::select;
use tokio::sync::broadcast::Receiver;
use tokio::time::Duration;

use crate::AppEvent;

pub const IOBOARD_TX_BUFFER_SIZE: usize = 4096;

topic!(IoBoardCommandTopic, IoBoardCommand, "topic/ioboard/command");

pub async fn io_board_command_sender(stack: RouterStack, app_event_rx: Receiver<AppEvent>) {
    let mut app_shutdown_handler = Box::pin(crate::app_shutdown_handler(app_event_rx));

    enum Phase {
        One,
        Two,
        Three,
    }
    let mut ctr = 0;
    let mut phase = Phase::One;
    loop {
        match phase {
            Phase::One => {
                select! {
                    _ = &mut app_shutdown_handler => {
                        break
                    }
                    _ = tokio::time::sleep(Duration::from_secs(1)) => {},
                }
                let command = IoBoardCommand::Test(ctr);
                stack
                    .topics()
                    .broadcast::<IoBoardCommandTopic>(&command, None)
                    .unwrap();
                ctr += 1;
                phase = Phase::Two
            }
            Phase::Two => {
                select! {
                    _ = &mut app_shutdown_handler => {
                        break
                    }
                    _ = tokio::time::sleep(Duration::from_secs(5)) => {},
                }
                stack
                    .topics()
                    .broadcast::<IoBoardCommandTopic>(&IoBoardCommand::BeginYeetTest, None)
                    .unwrap();
                phase = Phase::Three
            }
            Phase::Three => {
                select! {
                    _ = &mut app_shutdown_handler => {
                        break
                    }
                    _ = tokio::time::sleep(Duration::from_secs(5)) => {},
                }
                stack
                    .topics()
                    .broadcast::<IoBoardCommandTopic>(&IoBoardCommand::EndYeetTest, None)
                    .unwrap();

                phase = Phase::One
            }
        }
    }
    info!("io board command sender shutdown");
}
