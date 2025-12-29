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
use anyhow::Result;
use log::{debug, error, info, trace, warn};
use opencv::{imgcodecs, prelude::*, videoio};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::{
    net::UdpSocket,
    select, signal,
    sync::broadcast::{self, Sender},
    time::{self, Duration},
};

use ergot::{
    Address, NetStackSendError, endpoint,
    toolkits::tokio_udp::{
        EdgeStack, new_controller_stack, new_std_queue, register_edge_interface,
    },
    topic,
    well_known::DeviceInfo,
};

use camerastreamer_ergot_shared::{
    CameraFrameChunk, CameraFrameChunkKind, CameraFrameImageChunk, CameraFrameMeta,
    CameraStreamerCommand, CameraStreamerCommandRequest, CameraStreamerCommandResponse,
    CameraStreamerCommandResult,
};
use ergot::interface_manager::InterfaceSendError;
use ergot::interface_manager::profiles::direct_edge::tokio_udp::InterfaceKind;
use std::convert::TryInto;
use std::ffi::CStr;
use std::pin::pin;
use tokio::sync::Mutex;
use tokio::sync::broadcast::Receiver;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

#[cfg(target_os = "macos")]
use objc2::{class, msg_send, runtime::Bool};
#[cfg(target_os = "macos")]
use objc2_foundation::NSString;

topic!(
    CameraFrameChunkTopic,
    CameraFrameChunk,
    "topic/camera_stream"
);

endpoint!(
    CameraStreamerCommandEndpoint,
    CameraStreamerCommandRequest,
    CameraStreamerCommandResponse,
    "topic/camera"
);

// TODO have some system whereby the server broadcasts it's availability via UDP (udis, swarm-discovery, etc) and the camera client finds it, instead of hardcoding the IP address.
const REMOTE_ADDR: &str = "127.0.0.1:5001";
const LOCAL_ADDR: &str = "0.0.0.0:5000";
//const WIDTH: u32 = 1920;
const WIDTH: u32 = 1280;
//const HEIGHT: u32 = 1080;
const HEIGHT: u32 = 720;
const BPP: u32 = 24;
const FPS: u32 = 30;

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
    if !check_camera_permission() {
        request_camera_permission();
    }

    let tracker = TaskTracker::new();
    let shutdown_flag = CancellationToken::new();

    // Create broadcast channel for frames (Arc<Bytes> so we cheaply clone for each client)
    let (tx, rx) = broadcast::channel::<Arc<CameraFrame>>(BROADCAST_CAP);
    // wait till some clients subscribe to receive frames
    drop(rx);

    // Spawn capture loop
    let camera_capture_handle = tracker.spawn({
        let tx = tx.clone();
        let shutdown_flag = shutdown_flag.clone();
        async move {
            if let Err(e) = capture_loop(tx, shutdown_flag).await {
                error!("capture loop error: {e:?}");
            }
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

    let basic_services_handle =
        tracker.spawn(basic_services(stack.clone(), 0, shutdown_flag.clone()));

    let clients: Arc<Mutex<HashMap<Address, CameraClient>>> = Arc::new(Mutex::new(HashMap::new()));

    let camera_command_handler_handle = tracker.spawn(camera_command_handler(
        stack.clone(),
        clients.clone(),
        tx.clone(),
        shutdown_flag.clone(),
    ));

    register_edge_interface(&stack, udp_socket, &queue, InterfaceKind::Controller)
        .await
        .unwrap();

    tracker.close();

    // Wait for Ctrl+C
    let _ = signal::ctrl_c().await;
    info!("Shut down requested");

    shutdown_flag.cancel();

    tracker.wait().await;

    // TODO gracefully tell clients server is shutting down
    let _ = camera_capture_handle.await;
    debug!("Capture complete.");
    let _ = camera_command_handler_handle.await;
    debug!("Camera command handler complete.");
    let _ = basic_services_handle.await;
    debug!("Basic services complete.");

    info!("Shutdown complete.");

    Ok(())
}

#[cfg(target_os = "macos")]
fn request_camera_permission() {
    info!("Requesting camera permission");
    unsafe {
        let av_media_type = NSString::from_str("vide");
        type CompletionBlock = Option<extern "C" fn(Bool)>;
        let completion_block: CompletionBlock = None;
        let _: () = msg_send![
            class!(AVCaptureDevice),
            requestAccessForMediaType: &*av_media_type,
            completionHandler: completion_block
        ];
    }
}

#[cfg(not(target_os = "macos"))]
fn request_camera_permission() {}

#[cfg(target_os = "macos")]
fn check_camera_permission() -> bool {
    unsafe {
        let av_media_type = NSString::from_str("vide");
        let status: i32 = msg_send![
            class!(AVCaptureDevice),
            authorizationStatusForMediaType: &*av_media_type
        ];

        status == 3
    }
}
#[cfg(not(target_os = "macos"))]
fn check_camera_permission() {
    return true;
}

struct CameraClient {
    pub handle: tokio::task::JoinHandle<()>,
    pub shutdown_flag: CancellationToken,
}

async fn basic_services(stack: EdgeStack, port: u16, shutdown_flag: CancellationToken) {
    let info = DeviceInfo {
        name: Some("Ergot client".try_into().unwrap()),
        description: Some("An Ergot Client Device".try_into().unwrap()),
        unique_id: port.into(),
    };
    let do_pings = stack.services().ping_handler::<4>();
    let do_info = stack.services().device_info_handler::<4>(&info);
    let do_socket_disco = stack.services().socket_query_handler::<4>();

    select! {
        _ = shutdown_flag.cancelled() => {},
        _ = do_pings => {}
        _ = do_info => {}
        _ = do_socket_disco => {},
    }

    info!("Basic services stopped");
}

async fn camera_command_handler(
    stack: EdgeStack,
    clients: Arc<Mutex<HashMap<Address, CameraClient>>>,
    tx: Sender<Arc<CameraFrame>>,
    global_shutdown_flag: CancellationToken,
) {
    let server_socket = stack
        .endpoints()
        .single_server::<CameraStreamerCommandEndpoint>(None);
    let server_socket = pin!(server_socket);
    let mut hdl = server_socket.attach();
    let port = hdl.port();

    info!("Camera command server, port: {}", port);

    loop {
        let do_serve = {
            async |request: &CameraStreamerCommandRequest| {
                match request.command {
                    CameraStreamerCommand::StartStreaming { address } => {
                        let mut clients = clients.lock().await;
                        let rx = tx.subscribe();

                        // TODO how do we know the network and node id is correct here?

                        let client_shutdown_flag = CancellationToken::new();
                        let handle = tokio::task::spawn(camera_streamer(
                            stack.clone(),
                            rx,
                            address,
                            client_shutdown_flag.clone(),
                        ));
                        let camera_client = CameraClient {
                            handle,
                            shutdown_flag: client_shutdown_flag,
                        };
                        clients.insert(address, camera_client);

                        info!("Streaming started. address: {}", address);

                        CameraStreamerCommandResponse {
                            result: CameraStreamerCommandResult::Ok,
                        }
                    }
                    CameraStreamerCommand::StopStreaming { address } => {
                        let mut clients = clients.lock().await;
                        if let Some(client) = clients.remove(&address) {
                            client.shutdown_flag.cancel();
                            let _ = client.handle.await;
                            info!("Streaming stopped. address: {}", address);
                        } else {
                            warn!(
                                "Request to stop streaming received with no active client. address: {}",
                                address
                            );
                        }

                        CameraStreamerCommandResponse {
                            result: CameraStreamerCommandResult::Ok,
                        }
                    }
                }
            }
        };

        select! {
            _ = hdl.serve(do_serve) => {},
            _ = global_shutdown_flag.cancelled() => {
                info!("Shutting down camera command handler");
                break
            }
        }
    }

    let mut clients = clients.lock().await;
    let clients_to_cancel = clients.drain().collect::<Vec<_>>();

    for (index, (address, client)) in clients_to_cancel.into_iter().enumerate() {
        info!("Stopping streaming client {}. address: {}", index, address);

        // TODO notify client that streaming is stopped

        client.shutdown_flag.cancel();
        let _ = client.handle.await;
    }
    info!("Camera command handler stopped");
}

async fn capture_loop(
    tx: Sender<Arc<CameraFrame>>,
    shutdown_flag: CancellationToken,
) -> Result<()> {
    // Open default camera (index 0)
    let mut cam = videoio::VideoCapture::new(0, videoio::CAP_ANY)?; // 0 = default device
    if !videoio::VideoCapture::is_opened(&cam)? {
        anyhow::bail!("Unable to open default camera");
    }
    info!(
        "Backend: {}",
        cam.get_backend_name().unwrap_or("Unknown".to_string())
    );
    info!("GUID: {}", cam.get(videoio::CAP_PROP_GUID)?);
    info!("HW_DEVICE: {}", cam.get(videoio::CAP_PROP_HW_DEVICE)?);
    cam.set(videoio::CAP_PROP_FRAME_WIDTH, f64::from(WIDTH))?;
    cam.set(videoio::CAP_PROP_FRAME_HEIGHT, f64::from(HEIGHT))?;
    cam.set(videoio::CAP_PROP_FPS, f64::from(FPS)).unwrap();
    cam.set(videoio::CAP_PROP_BUFFERSIZE, f64::from(1)).unwrap();

    let period = Duration::from_secs_f64(1.0 / FPS as f64);
    let mut interval = time::interval(period);
    interval.set_missed_tick_behavior(time::MissedTickBehavior::Skip);
    let mut frame_number = 0_u64;
    let mut previous_frame_at = time::Instant::now();
    let mut frame = Mat::default();
    loop {
        interval.tick().await;

        if tx.receiver_count() > 0 {
            let frame_timestamp = chrono::Utc::now();
            let frame_instant = time::Instant::now();

            cam.read(&mut frame)?;

            if frame.empty() {
                // skip or try again
                error!("Empty frame");
                continue;
            }
            let frame_duration = (frame_instant - previous_frame_at).as_millis();
            previous_frame_at = frame_instant;

            // Encode to JPEG (quality default). You can set params to reduce quality/size.
            let encode_start = time::Instant::now();
            let mut buf = opencv::core::Vector::new();
            let params = opencv::core::Vector::new(); // default
            imgcodecs::imencode(".jpg", &frame, &mut buf, &params)?;

            let encode_end = time::Instant::now();
            let encode_duration = (encode_end - encode_start).as_micros();

            let send_start = time::Instant::now();

            // Wrap bytes into Arc so broadcast clones cheap
            let camera_frame = CameraFrame {
                frame_number,
                jpeg_bytes: buf.to_vec(),
                frame_timestamp,
            };

            let camera_frame_arc = Arc::new(camera_frame);

            // safe to ignore the error, no subscribers yet, however we're only sending a frame if we
            // have subscribers, so this should never fail anyway.
            let _ = tx.send(camera_frame_arc);

            let send_end = time::Instant::now();
            let send_duration = (send_end - send_start).as_micros();

            info!(
                "frame_timestamp: {:?}, frame_number: {}, encode_duration: {}us, send_duration: {}us, frame_duration: {}ms",
                frame_timestamp, frame_number, encode_duration, send_duration, frame_duration
            );
            frame_number += 1;
        }

        if shutdown_flag.is_cancelled() {
            info!("Shutting down camera capture");
            break;
        }
    }

    Ok(())
}

async fn camera_streamer(
    stack: EdgeStack,
    mut rx: Receiver<Arc<CameraFrame>>,
    destination: Address,
    shutdown_flag: CancellationToken,
) {
    info!("camera streamer started. destination: {}", destination);
    let mut interval = time::interval(Duration::from_secs(1));
    loop {
        select! {
            r = rx.recv() => {
                // Receive latest frame (await)
                let camera_frame = match r {
                    Ok(b) => b,
                    Err(broadcast::error::RecvError::Lagged(skipped_frames)) => {
                        // If lagged, try to get the next available
                        debug!("lagged, trying to get next frame.  skipped: {}", skipped_frames);
                        continue;
                    }
                    Err(_e) => {
                        error!("error receiving frame. shutting down camera streamer. error: {:?}", _e);
                        return
                    },
                };

                let CameraFrame { frame_number, jpeg_bytes, frame_timestamp } = &*camera_frame;

                let total_bytes = jpeg_bytes.len() as u32;
                let total_chunks = (total_bytes + (CHUNK_SIZE as u32) - 1) / CHUNK_SIZE as u32;

                trace!("Sending frame, frame_number: {}, total_chunks: {}, len: {}", camera_frame.frame_number, total_chunks, total_bytes);

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
            _ = interval.tick() => {
                if shutdown_flag.is_cancelled() {
                    info!("Shutting down camera streamer");
                    break
                }
            }
        }
    }
}
