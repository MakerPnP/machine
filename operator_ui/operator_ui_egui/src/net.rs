use egui_mobius::Value;
use ergot::{
    toolkits::tokio_udp::{EdgeStack, new_std_queue, new_target_stack, register_edge_interface},
    interface_manager::profiles::direct_edge::tokio_udp::InterfaceKind,
    topic,
    well_known::DeviceInfo,
};
use log::{debug, info, warn};
use tokio::{net::UdpSocket, select, time, time::sleep};

use std::{pin::pin, time::Duration};
use std::convert::TryInto;
use operator_shared::commands::OperatorCommand;
use tokio::runtime::Handle;
use crate::app::AppState;

pub async fn ergot_task(spawner: Handle, state: Option<Value<AppState>>) {
    let queue = new_std_queue(4096);
    let stack: EdgeStack = new_target_stack(&queue, 1024);
    let udp_socket = UdpSocket::bind("192.168.18.41:8002").await.unwrap();
    let remote_addr = "192.168.18.41:8001";

    // FIXME show a message in the UI if this fails instead of panicking when the port is already in use
    udp_socket.connect(remote_addr).await.unwrap();

    let port = udp_socket.local_addr().unwrap().port();

    tokio::task::spawn(basic_services(stack.clone(), port));
    tokio::task::spawn(command_sender(stack.clone()));
    tokio::task::spawn(yeet_listener(stack.clone()));

    register_edge_interface(&stack, udp_socket, &queue, InterfaceKind::Target)
        .await
        .unwrap();

    loop {
        println!("Waiting for messages...");
        sleep(Duration::from_secs(1)).await;
    }
}

async fn basic_services(stack: EdgeStack, port: u16) {
    let info = DeviceInfo {
        name: Some("OperatorUI".try_into().unwrap()),
        description: Some("MakerPnP - Operator UI".try_into().unwrap()),
        unique_id: port.into(),
    };
    let do_pings = stack.services().ping_handler::<4>();
    let do_info = stack.services().device_info_handler::<4>(&info);

    select! {
        _ = do_pings => {}
        _ = do_info => {}
    }
}

topic!(OperatorCommandTopic, OperatorCommand, "topic/operator/command");

async fn command_sender(stack: EdgeStack) {
    let mut ctr = 0;
    tokio::time::sleep(Duration::from_secs(1)).await;
    let heartbeat_timeout_duration = Duration::from_secs(10);
    let heartbeat_send_interval = heartbeat_timeout_duration / 2;
    let mut ticker = time::interval(heartbeat_send_interval);
    loop {
        stack.topics().broadcast::<OperatorCommandTopic>(&OperatorCommand::Heartbeat(ctr), None).unwrap();
        ctr += 1;

        ticker.tick().await;
    }
}

topic!(YeetTopic, u64, "topic/yeet");

async fn yeet_listener(stack: EdgeStack) {
    let subber = stack.topics().heap_bounded_receiver::<YeetTopic>(64, None);
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
        }
    }
}
