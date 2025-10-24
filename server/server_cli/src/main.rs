use ergot::{
    toolkits::tokio_udp::{EdgeStack, new_std_queue, new_controller_stack, register_edge_interface},
    topic,
    well_known::DeviceInfo,
};
use log::{info, warn, debug};
use tokio::{net::UdpSocket, select, time, time::sleep};

use std::{io, pin::pin, time::Duration};
use std::collections::HashSet;
use std::convert::TryInto;
use ergot::interface_manager::profiles::direct_edge::tokio_udp::InterfaceKind;
use ergot::toolkits::tokio_udp::{register_router_interface, RouterStack};
use tokio::time::interval;
use ioboard_shared::yeet::Yeet;
use ioboard_shared::commands::Command;

// TODO configure these appropriately
const MAX_ERGOT_PACKET_SIZE: u16 = 1024;
const TX_BUFFER_SIZE: usize = 4096;

topic!(YeetTopic, Yeet, "topic/yeet");
topic!(CommandTopic, Command, "topic/command");

#[tokio::main]
async fn main() -> io::Result<()> {
    env_logger::init();

    let stack: RouterStack = RouterStack::new();

    let io_board_udp_socket = UdpSocket::bind("192.168.18.41:8000").await.unwrap();
    let io_board_remote_addr = "192.168.18.64:8000";
    io_board_udp_socket.connect(io_board_remote_addr).await?;
    register_router_interface(&stack, io_board_udp_socket, MAX_ERGOT_PACKET_SIZE, TX_BUFFER_SIZE)
        .await
        .unwrap();

    let operator_udp_socket = UdpSocket::bind("192.168.18.41:8001").await.unwrap();
    let operator_remote_addr = "192.168.18.41:8002";
    operator_udp_socket.connect(operator_remote_addr).await?;
    register_router_interface(&stack, operator_udp_socket, MAX_ERGOT_PACKET_SIZE, TX_BUFFER_SIZE)
        .await
        .unwrap();

    tokio::task::spawn(basic_services(stack.clone(), 0_u16));
    tokio::task::spawn(command_sender(stack.clone()));
    tokio::task::spawn(yeet_listener(stack.clone()));

    loop {
        println!("Waiting for messages...");
        sleep(Duration::from_secs(1)).await;
    }
}


async fn basic_services(stack: RouterStack, port: u16) {
    let info = DeviceInfo {
        name: Some("Ergot router".try_into().unwrap()),
        description: Some("A central router".try_into().unwrap()),
        unique_id: port.into(),
    };
    // allow for discovery
    let disco_answer = stack.services().device_info_handler::<4>(&info);
    // handle incoming ping requests
    let ping_answer = stack.services().ping_handler::<4>();
    // custom service for doing discovery regularly
    let disco_req = tokio::spawn(do_discovery(stack.clone()));
    // forward log messages to the log crate output
    let log_handler = stack.services().log_handler(16);

    // These all run together, we run them in a single task
    select! {
        _ = disco_answer => {},
        _ = ping_answer => {},
        _ = disco_req => {},
        _ = log_handler => {},
    }
}

async fn do_discovery(stack: RouterStack) {
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

async fn command_sender(stack: RouterStack) {
    let mut ctr = 0;
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await;
        let command = Command::Test(ctr);
        stack.topics().broadcast::<CommandTopic>(&command, None).unwrap();
        ctr += 1;

        tokio::time::sleep(Duration::from_secs(5)).await;
        stack.topics().broadcast::<CommandTopic>(&Command::BeginYeetTest, None).unwrap();

        tokio::time::sleep(Duration::from_secs(5)).await;
        stack.topics().broadcast::<CommandTopic>(&Command::EndYeetTest, None).unwrap();
    }
}

async fn yeet_listener(stack: RouterStack) {
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
