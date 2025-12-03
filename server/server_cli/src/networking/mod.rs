use std::collections::HashSet;
use std::pin::pin;
use std::time::Duration;

use ergot::toolkits::tokio_udp::RouterStack;
use ergot::topic;
use ergot::well_known::DeviceInfo;
use ergot::wire_frames::MAX_HDR_ENCODED_SIZE;
use ioboard_shared::yeet::Yeet;
use log::{debug, info, warn};
use tokio::sync::broadcast::Receiver;
use tokio::time::interval;
use tokio::{select, time};

use crate::AppEvent;

pub const UDP_OVER_ETH_MTU: usize = 1500;
pub const IP_OVERHEAD_SIZE: usize = 20;
pub const UDP_OVERHEAD_SIZE: usize = 8;
pub const UDP_OVER_ETH_ERGOT_FRAME_SIZE_MAX: usize = UDP_OVER_ETH_MTU - IP_OVERHEAD_SIZE - UDP_OVERHEAD_SIZE;
pub const UDP_OVER_ETH_ERGOT_PAYLOAD_SIZE_MAX: usize = UDP_OVER_ETH_ERGOT_FRAME_SIZE_MAX - MAX_HDR_ENCODED_SIZE;

topic!(YeetTopic, Yeet, "topic/yeet");

pub async fn basic_services(stack: RouterStack, port: u16, app_event_rx: Receiver<AppEvent>) {
    let info = DeviceInfo {
        name: Some("Ergot router".try_into().unwrap()),
        description: Some("A central router".try_into().unwrap()),
        unique_id: port.into(),
    };
    // allow for discovery
    let device_discovery_responder = stack
        .services()
        .device_info_handler::<4>(&info);
    // handle incoming ping requests
    let ping_responder = stack.services().ping_handler::<4>();
    // custom service for doing device discovery regularly
    let device_discovery = tokio::task::Builder::new()
        .name("ergot/device-discovery")
        .spawn(do_device_discovery(stack.clone()))
        .unwrap();
    // forward log messages to the log crate output
    let log_handler = stack.services().log_handler(16);
    // handle socket discovery requests
    let socket_discovery_responder = stack
        .services()
        .socket_query_handler::<4>();

    let app_shutdown_handler = crate::app_shutdown_handler(app_event_rx);

    // These all run together, we run them in a single task
    select! {
        _ = ping_responder => {},
        _ = log_handler => {},
        _ = device_discovery_responder => {},
        _ = socket_discovery_responder => {},
        _ = device_discovery => {},
        _ = app_shutdown_handler => {
            info!("basic services shutdown requested, stopping");
        },
    }
}

async fn do_device_discovery(stack: RouterStack) {
    let mut max = 16;
    let mut seen = HashSet::new();
    let mut ticker = interval(Duration::from_secs(10));
    loop {
        let new_seen = stack
            .discovery()
            .discover(max, Duration::from_millis(250))
            .await;
        max = max.max(seen.len() * 2);
        let new_seen = HashSet::from_iter(new_seen);
        let added = new_seen.difference(&seen);
        for add in added {
            warn!("Added:   {add:?}");
        }
        let removed = seen.difference(&new_seen);
        for rem in removed {
            warn!("Removed: {rem:?}");
        }
        seen = new_seen;

        info!("Discovery list:");
        for (index, item) in seen.iter().enumerate() {
            info!("{}: {:?}", index, item);
        }

        ticker.tick().await;
    }
}

pub async fn yeet_listener(stack: RouterStack, app_event_rx: Receiver<AppEvent>) {
    let mut app_shutdown_handler = Box::pin(crate::app_shutdown_handler(app_event_rx));

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
