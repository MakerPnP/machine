use std::collections::HashMap;
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
    time::{self, Duration},
    net::UdpSocket, select
};
use log::{debug, error, info, trace, warn};

use ergot::{endpoint, toolkits::tokio_udp::{EdgeStack, new_std_queue, new_controller_stack, register_edge_interface}, topic, well_known::DeviceInfo, Address, NetStackSendError};

use std::convert::TryInto;
use std::pin::pin;
use ergot::interface_manager::InterfaceSendError;
use ergot::interface_manager::profiles::direct_edge::EDGE_NODE_ID;
use ergot::interface_manager::profiles::direct_edge::tokio_udp::InterfaceKind;
use tokio::sync::broadcast::Receiver;
use tokio::sync::Mutex;
use camerastreamer_ergot_shared::{CameraFrameChunk, CameraFrameChunkKind, CameraFrameImageChunk, CameraFrameMeta, CameraStreamerCommand, CameraStreamerCommandError, CameraStreamerCommandRequest, CameraStreamerCommandResponse, CameraStreamerCommandResult};

topic!(CameraFrameChunkTopic, CameraFrameChunk, "topic/camera_stream");

endpoint!(CameraStreamerCommandEndpoint, CameraStreamerCommandRequest, CameraStreamerCommandResponse, "topic/camera");

// TODO have some system whereby the server broadcasts it's availability via UDP (udis, swarm-discovery, and the camera client finds it) instead of hardcoding the IP address.
const REMOTE_ADDR: &str = "127.0.0.1:5001";
const LOCAL_ADDR: &str = "0.0.0.0:5000";
const WIDTH: u32 = 1920;
// const WIDTH: i32 = 1280;
const HEIGHT: u32 = 1080;
// const HEIGHT: i32 = 720;
const BPP: u32 = 24;
const FPS: u32 = 25;

// must be less than the MTU of the network interface + udp overhead + ergot overhead + chunking overhead
const CHUNK_SIZE: usize = 1024;
const BROADCAST_CAP: usize = (FPS * 2) as usize;

pub struct CameraFrame {
    pub frame_number: u64,
    pub jpeg_bytes: Vec<u8>,
    pub frame_timestamp: chrono::DateTime<chrono::Utc>,
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    // Create broadcast channel for frames (Arc<Bytes> so we cheaply clone for each client)
    let (tx, rx) = broadcast::channel::<Arc<CameraFrame>>(BROADCAST_CAP);

    // Spawn capture loop
    let tx_clone = tx.clone();
    tokio::task::spawn(async move {
        if let Err(e) = capture_loop(tx_clone).await {
            eprintln!("capture loop error: {e:?}");
        }
    });

    const JPEG_COMPRESSION_RATIO_GUESS: u32 = 5; // 5:1
    const BITS_PER_BYTE: u32 = 8;
    let queue_size = (WIDTH * HEIGHT * BPP) / BITS_PER_BYTE / JPEG_COMPRESSION_RATIO_GUESS;
    println!("queue size: {}", queue_size);

    let queue = new_std_queue(queue_size as usize);
    let stack: EdgeStack = new_controller_stack(&queue, 1400);
    let udp_socket = UdpSocket::bind(LOCAL_ADDR).await?;

    udp_socket.connect(REMOTE_ADDR).await?;

    tokio::task::spawn(basic_services(stack.clone(), 0));

    let clients: Arc<Mutex<HashMap<u8, CameraClient>>> = Arc::new(Mutex::new(HashMap::new()));

    tokio::task::spawn(camera_command_handler(stack.clone(), clients.clone(), rx));

    register_edge_interface(&stack, udp_socket, &queue, InterfaceKind::Controller)
        .await
        .unwrap();


    let period = Duration::from_secs(1);
    let mut interval = time::interval(period);

    loop {
        interval.tick().await;
    }
}

struct CameraClient {
    pub address: Address,
    pub handle: tokio::task::JoinHandle<Result<Receiver<Arc<CameraFrame>>,Receiver<Arc<CameraFrame>>>>,
    pub shutdown_flag: Arc<Mutex<bool>>,
}

async fn basic_services(stack: EdgeStack, port: u16) {
    let info = DeviceInfo {
        name: Some("Ergot client".try_into().unwrap()),
        description: Some("An Ergot Client Device".try_into().unwrap()),
        unique_id: port.into(),
    };
    let do_pings = stack.services().ping_handler::<4>();
    let do_info = stack.services().device_info_handler::<4>(&info);
    let do_socket_disco = stack.services().socket_query_handler::<4>();

    select! {
        _ = do_pings => {}
        _ = do_info => {}
        _ = do_socket_disco => {},
    }
}

async fn camera_command_handler(stack: EdgeStack, clients: Arc<Mutex<HashMap<u8, CameraClient>>>, rx: Receiver<Arc<CameraFrame>>) {

    let server_socket = stack.endpoints().single_server::<CameraStreamerCommandEndpoint>(None);
    let server_socket = pin!(server_socket);
    let mut hdl = server_socket.attach();
    let port = hdl.port();

    info!("camera command server, port: {}", port);

    let mut rx = Some(rx);

    loop {
        let _ = hdl.serve(async |request: &CameraStreamerCommandRequest| {
            match request.command {
                CameraStreamerCommand::StartStreaming { port_id } => {

                    let mut clients = clients.lock().await;
                    if rx.is_none() {
                        return CameraStreamerCommandResponse { result: CameraStreamerCommandResult::Error { code: CameraStreamerCommandError::Busy, args: vec![] } }
                    }
                    let rx = rx.take().unwrap();
                    // TODO how do we know the network and node id is correct here?
                    let address = Address {
                        network_id: 0,
                        node_id: EDGE_NODE_ID,
                        port_id,
                    };
                    let shutdown_flag = Arc::new(Mutex::new(false));
                    let handle = tokio::task::spawn(camera_streamer(stack.clone(), rx, address, shutdown_flag.clone()));
                    let camera_client = CameraClient {
                        address,
                        handle,
                        shutdown_flag
                    };
                    clients.insert(port_id, camera_client);

                    CameraStreamerCommandResponse { result: CameraStreamerCommandResult::Ok }
                }
                CameraStreamerCommand::StopStreaming { port_id } => {
                    let mut clients = clients.lock().await;
                    if let Some(client) = clients.remove(&port_id) {
                        *client.shutdown_flag.lock().await = true;
                        match client.handle.await {
                            Ok(join_result) => {
                                let returned_rx = join_result.unwrap_or_else(|returned_rx| returned_rx);
                                rx.replace(returned_rx);
                            },
                            Err(join_error) => {
                                error!("unable to stop streaming, join error: {}", join_error);
                                // rx is lost, cannot recover
                            },
                        };
                    }
                    CameraStreamerCommandResponse { result: CameraStreamerCommandResult::Ok }
                },

            }
        }).await;
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
    let mut frame_number = 0_u64;
    loop {
        interval.tick().await;
        let mut frame = Mat::default();
        cam.read(&mut frame)?;
        if frame.empty() {
            // skip or try again
            continue;
        }
        let frame_timestamp = chrono::Utc::now();

        // Encode to JPEG (quality default). You can set params to reduce quality/size.
        let encode_start = time::Instant::now();
        let mut buf = opencv::core::Vector::new();
        let params = opencv::core::Vector::new(); // default
        imgcodecs::imencode(".jpg", &frame, &mut buf, &params)?;

        let encode_end = time::Instant::now();
        let encode_duration = (encode_end - encode_start).as_micros() as u32;

        let send_start = time::Instant::now();

        // Wrap bytes into Arc so broadcast clones cheap
        let camera_frame = CameraFrame {
            frame_number,
            jpeg_bytes: buf.to_vec(),
            frame_timestamp,
        };

        let camera_frame_arc = Arc::new(camera_frame);
        // Ignore send error (no subscribers)
        let _ = tx.send(camera_frame_arc);

        let send_end = time::Instant::now();
        let send_duration = (send_end - send_start).as_micros() as u32;

        trace!("now: {:?}, frame_number: {}, encode_duration: {}us, send_duration: {}us", time::Instant::now(), frame_number, encode_duration, send_duration);
        frame_number += 1;
    }
}

async fn camera_streamer(stack: EdgeStack, mut rx: Receiver<Arc<CameraFrame>>, destination: Address, shutdown_flag: Arc<Mutex<bool>>) -> Result<Receiver<Arc<CameraFrame>>, Receiver<Arc<CameraFrame>>> {
    loop {
        let do_shutdown_flag = shutdown_flag.lock();
        let do_rx = rx.recv();
        select! {
            flag = do_shutdown_flag => {
                if *flag == true {
                    break
                }
            }
            r = do_rx => {
                // Receive latest frame (await)
                let camera_frame = match r {
                    Ok(b) => b,
                    Err(broadcast::error::RecvError::Lagged(skipped_frames)) => {
                        // If lagged, try to get the next available
                        debug!("lagged, trying to get next frame.  skipped: {}", skipped_frames);
                        continue;
                    }
                    Err(_e) => return Err(rx),
                };

                let CameraFrame { frame_number, jpeg_bytes, frame_timestamp } = &*camera_frame;

                let total_bytes = jpeg_bytes.len() as u32;
                let total_chunks = (total_bytes + (CHUNK_SIZE as u32) - 1) / CHUNK_SIZE as u32;

                let now = time::Instant::now();

                trace!("Sending frame, now: {:?}, frame_number: {}, total_chunks: {}, len: {}", now, camera_frame.frame_number, total_chunks, total_bytes);

                let frame_chunk = CameraFrameChunk {
                    frame_number: *frame_number,
                    kind: CameraFrameChunkKind::Meta(CameraFrameMeta {
                        total_chunks,
                        total_bytes,
                        frame_timestamp: (*frame_timestamp).into(),
                    })
                };
                if stack.topics().unicast_borrowed::<CameraFrameChunkTopic>(destination, &frame_chunk).is_err() {
                    trace!("Unable to send first frame chunk. frame_number: {}", frame_number);
                    // no point even trying to send the chunks if the first chunk failed, drop the frame
                    continue
                }

                let mut ok = true;
                let mut total_retries = 0;
                for (chunk_index, chunk) in jpeg_bytes.chunks(CHUNK_SIZE).enumerate() {
                    let frame_chunk = CameraFrameChunk {
                        frame_number: *frame_number,
                        kind: CameraFrameChunkKind::ImageChunk(CameraFrameImageChunk {
                            chunk_index: chunk_index as u32,
                            bytes: chunk.to_vec(),
                        })
                    };

                    let chunk_start_at = time::Instant::now();

                    // IMPORTANT: back-off delay needs to be as short as possible
                    //            60fps =  16ms total frame time.
                    //            30fps =  33ms total frame time.
                    //            25fps =  40ms total frame time.
                    //            15fps =  66ms total frame time.
                    //            10fps = 100ms total frame time.
                    const INITIAL_BACKOFF: Duration = Duration::from_micros(100);
                    let mut chunk_retries = 0;

                    let result = loop {
                        match stack.topics().unicast_borrowed::<CameraFrameChunkTopic>(destination, &frame_chunk) {
                            r @ Ok(()) => {
                                // reset
                                break r
                            }
                            e1 @ Err(NetStackSendError::InterfaceSend(InterfaceSendError::InterfaceFull)) => {
                                if chunk_start_at.elapsed() > Duration::from_millis(100) {
                                    break e1
                                } else {
                                    let backoff = INITIAL_BACKOFF * (1 << chunk_retries.min(4));
                                    time::sleep_until(chunk_start_at + backoff).await;
                                }
                            }
                            e2@ Err(_) => {
                                error!("error: {:?}", e2);
                                break e2
                            }
                        }

                        chunk_retries += 1;
                        total_retries += 1;
                    };

                    match result {
                        Ok(_) => tokio::task::yield_now().await,
                        Err(e) => {
                            error!("Aborting frame, error sending chunk. frame_number: {}, chunk: {}/{}, retries: {}, error: {:?}", frame_number, chunk_index + 1, total_chunks, chunk_retries, e);
                            ok = false;
                            break
                        }
                    }
                }

                if ok {
                    if total_retries > 0 {
                        warn!("Frame sent with retries. frame_number: {}, total_retries: {}", frame_number, total_retries);
                    }
                } else {
                    error!("Frame failed to send. frame_number: {}, total_retries: {}", frame_number, total_retries);
                }
            }
        }

    }

    Ok(rx)
}
