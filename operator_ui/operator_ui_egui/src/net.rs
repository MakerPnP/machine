use std::convert::TryInto;
use std::{pin::pin, time::Duration};
use eframe::epaint::ColorImage;
use egui_mobius::Value;
use ergot::{
    interface_manager::profiles::direct_edge::tokio_udp::InterfaceKind,
    toolkits::tokio_udp::{EdgeStack, new_std_queue, new_target_stack, register_edge_interface},
    topic,
    well_known::DeviceInfo,
};
use tracing::{debug, info};
use tokio::runtime::Handle;
use tokio::{net::UdpSocket, select, time, time::sleep};
use tokio::sync::watch::Sender;
use crate::app::AppState;
use crate::{LOCAL_ADDR, REMOTE_ADDR};
use crate::net::camera::camera_frame_listener;
use crate::net::commands::command_sender;
use crate::net::services::basic_services;

pub mod camera;
pub mod services;
pub mod commands;

pub async fn ergot_task(_spawner: Handle, state: Value<AppState>, tx_out: Sender<ColorImage>) {
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

    tokio::task::spawn(basic_services(stack.clone(), port));
    tokio::task::spawn(command_sender(stack.clone()));
    tokio::task::spawn(yeet_listener(stack.clone()));

    {
        let context = state.lock().unwrap().context.clone();
        tokio::task::spawn(camera_frame_listener(stack.clone(), 0, tx_out, context));
    }


    register_edge_interface(&stack, udp_socket, &queue, InterfaceKind::Target)
        .await
        .unwrap();

    loop {
        println!("Waiting for messages...");
        sleep(Duration::from_secs(1)).await;
    }
}

topic!(YeetTopic, u64, "topic/yeet");

async fn yeet_listener(stack: EdgeStack) {
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
        }
    }
}
