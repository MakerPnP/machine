/// for windows/msys2/ucrt64 build (requires gnu toolchain):
/// `cargo build --target x86_64-pc-windows-gnu`
/// or:
/// `rustup run stable-x86_64-pc-windows-gnu cargo build --target x86_64-pc-windows-gnu`
///
/// run resulting binary only from msys2 environment, requires msys2 .dlls in the path.
///
/// for windows/vcpkg build using (requires msvc toolchain):
/// `cargo build --target x86_64-pc-windows-msvc`
/// or
/// `rustup run stable-x86_64-pc-windows-msvc cargo build --target x86_64-pc-windows-msvc --release`
///
/// Note: build script copies required dlls from vcpkg into the build directory next to the .exe
///
/// if you need a debug build use:
/// ```
/// set OPENCV_DISABLE_PROBES=vcpkg_cmake
/// rustup run stable-x86_64-pc-windows-msvc cargo build --target x86_64-pc-windows-msvc --release
/// ```
/// Reference: https://github.com/twistedfall/opencv-rust/issues/307
///
/// no other combinations tested.


// server.rs
use anyhow::Result;
use opencv::{imgcodecs, prelude::*, videoio};
use std::sync::Arc;
use tokio::{
    sync::broadcast::{self, Sender},
    time::{self, Duration, sleep},
    net::UdpSocket, select
};
use log::{debug, error, trace};

use ergot::{
    toolkits::tokio_udp::{EdgeStack, new_std_queue, new_controller_stack, register_edge_interface},
    topic,
    well_known::DeviceInfo,
};

use std::convert::TryInto;
use ergot::interface_manager::profiles::direct_edge::tokio_udp::InterfaceKind;
use camerastreamer_ergot_shared::CameraFrame;

topic!(CameraFrameTopic, CameraFrame, "topic/camera_stream");


const REMOTE_ADDR: &str = "127.0.0.1:5001";
const LOCAL_ADDR: &str = "0.0.0.0:5000";
const WIDTH: i32 = 1280;
const HEIGHT: i32 = 720;
const FPS: u32 = 25;
const BROADCAST_CAP: usize = (FPS * 2) as usize;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    // Create broadcast channel for frames (Arc<Bytes> so we cheaply clone for each client)
    let (tx, rx) = broadcast::channel::<Arc<CameraFrame>>(BROADCAST_CAP);

    // Spawn capture loop
    let tx_clone = tx.clone();
    tokio::spawn(async move {
        if let Err(e) = capture_loop(tx_clone).await {
            eprintln!("capture loop error: {e:?}");
        }
    });

    let queue = new_std_queue(1024 * 1024);
    let stack: EdgeStack = new_controller_stack(&queue, 65535);
    let udp_socket = UdpSocket::bind(LOCAL_ADDR).await.unwrap();

    udp_socket.connect(REMOTE_ADDR).await?;

    tokio::task::spawn(basic_services(stack.clone(), 0));
    tokio::task::spawn(camera_streamer(stack.clone(), rx));

    register_edge_interface(&stack, udp_socket, &queue, InterfaceKind::Controller)
        .await
        .unwrap();

    loop {
        println!("Waiting for messages...");
        sleep(Duration::from_secs(1)).await;
    }
}

async fn basic_services(stack: EdgeStack, port: u16) {
    let info = DeviceInfo {
        name: Some("Ergot client".try_into().unwrap()),
        description: Some("An Ergot Client Device".try_into().unwrap()),
        unique_id: port.into(),
    };
    let do_pings = stack.services().ping_handler::<4>();
    let do_info = stack.services().device_info_handler::<4>(&info);

    select! {
        _ = do_pings => {}
        _ = do_info => {}
    }
}

async fn capture_loop(tx: Sender<Arc<CameraFrame>>) -> Result<()> {
    // Open default camera (index 0)
    let mut cam = videoio::VideoCapture::new(0, videoio::CAP_ANY)?; // 0 = default device
    if !videoio::VideoCapture::is_opened(&cam)? {
        anyhow::bail!("Unable to open default camera");
    }
    cam.set(videoio::CAP_PROP_FRAME_WIDTH, f64::from(WIDTH))?;
    cam.set(videoio::CAP_PROP_FRAME_HEIGHT, f64::from(HEIGHT))?;

    let period = Duration::from_millis((1000u32 / FPS) as u64);
    let mut interval = time::interval(period);
    let mut frame_number = 0_usize;
    loop {
        interval.tick().await;
        let mut frame = Mat::default();
        cam.read(&mut frame)?;
        if frame.empty() {
            // skip or try again
            continue;
        }

        // Encode to JPEG (quality default). You can set params to reduce quality/size.
        let encode_start = time::Instant::now();
        let mut buf = opencv::core::Vector::new();
        let params = opencv::core::Vector::new(); // default
        imgcodecs::imencode(".jpg", &frame, &mut buf, &params)?;

        let encode_end = time::Instant::now();
        let encode_duration = (encode_end - encode_start).as_micros() as u32;

        let send_start = time::Instant::now();

        // Wrap bytes into Arc so broadcast clones cheap
        let mut camera_frame = CameraFrame {
            jpeg_bytes: buf.to_vec()
        };
        camera_frame.jpeg_bytes.truncate(1024 * 2);
        let camera_frame_arc = Arc::new(camera_frame);
        // Ignore send error (no subscribers)
        let _ = tx.send(camera_frame_arc);

        let send_end = time::Instant::now();
        let send_duration = (send_end - send_start).as_micros() as u32;

        println!("now: {:?}, frame_number: {}, encode_duration: {}us, send_duration: {}us", time::Instant::now(), frame_number, encode_duration, send_duration);
        frame_number += 1;
    }
}

async fn camera_streamer(stack: EdgeStack, mut rx: tokio::sync::broadcast::Receiver<Arc<CameraFrame>>) -> Result<()> {
    loop {
        // Receive latest frame (await)
        let camera_frame = match rx.recv().await {
            Ok(b) => b,
            Err(tokio::sync::broadcast::error::RecvError::Lagged(skipped_frames)) => {
                // If lagged, try to get the next available
                debug!("lagged, trying to get next frame.  skipped: {}", skipped_frames);
                continue;
            }
            Err(e) => return Err(anyhow::anyhow!(e)),
        };


        let _ = stack.topics().broadcast::<CameraFrameTopic>(&camera_frame, None).inspect_err(|e| {
            error!("Error sending frame: {:?}", e);
        });
    }
}
