use tokio::sync::broadcast::Receiver;

use crate::events::AppEvent;

pub async fn app_shutdown_handler(mut receiver: Receiver<AppEvent>) {
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
