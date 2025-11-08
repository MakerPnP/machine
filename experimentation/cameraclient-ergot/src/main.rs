use std::collections::HashMap;
// client.rs
use eframe::{egui, App, CreationContext, Frame};
use image::ImageFormat;
use std::thread;
use std::pin::pin;
use std::sync::Arc;
use std::time::Duration;
use eframe::epaint::textures::TextureOptions;
use egui::{ColorImage, Context, ViewportCommand};
use ergot::{endpoint, toolkits::tokio_udp::{EdgeStack, new_std_queue, new_target_stack, register_edge_interface}, topic, well_known::DeviceInfo, FrameKind};
use ergot::interface_manager::profiles::direct_edge::tokio_udp::InterfaceKind;
use ergot::well_known::{NameRequirement, SocketQuery};
use ergot::traits::Endpoint;
use tokio::runtime::Runtime;
use log::{debug, error, info, trace, warn};
use tokio::{net::UdpSocket, select};
use tokio::sync::{broadcast, watch};
use tokio::sync::watch::{Receiver, Sender};
use tokio::time::Instant;
use camerastreamer_ergot_shared::{CameraFrameChunk, CameraFrameChunkKind, CameraStreamerCommand, CameraStreamerCommandRequest, CameraStreamerCommandResponse};
use crate::fps_stats::egui::show_frame_durations;
use crate::fps_stats::FpsStats;

mod fps_stats;
const REMOTE_ADDR: &str = "127.0.0.1:5000";
const LOCAL_ADDR: &str = "0.0.0.0:5001";

topic!(CameraFrameChunkTopic, CameraFrameChunk, "topic/camera_stream");
endpoint!(CameraStreamerCommandEndpoint, CameraStreamerCommandRequest, CameraStreamerCommandResponse, "topic/camera");

fn start_network_thread(tx_out: Sender<ColorImage>, context: Context, app_event_tx: Arc<broadcast::Sender<AppEvent>>) {
    // run a tokio runtime in background thread
    thread::spawn(move || {
        let rt = Runtime::new().expect("create tokio runtime");
        rt.block_on(async move {
            if let Err(e) = network_task(REMOTE_ADDR, tx_out, context, app_event_tx).await {
                error!("network task error: {:?}", e);
            }
        });
    });
}

async fn network_task(addr: &str, tx_out: Sender<ColorImage>, context: Context, app_event_tx: Arc<broadcast::Sender<AppEvent>>) -> anyhow::Result<()> {

    let queue = new_std_queue(1024 * 1024);
    let stack: EdgeStack = new_target_stack(&queue, 1400);
    let udp_socket = UdpSocket::bind(LOCAL_ADDR).await?;

    udp_socket.connect(REMOTE_ADDR).await?;

    tokio::task::spawn(basic_services(stack.clone(), 0));
    let camera_frame_listener_handle = tokio::task::spawn(camera_frame_listener(stack.clone(), 0, tx_out, context.clone(), app_event_tx.subscribe()));

    let mut app_event_rx = app_event_tx.subscribe();

    register_edge_interface(&stack, udp_socket, &queue, InterfaceKind::Target)
        .await
        .unwrap();

    loop {
        if let Ok(event) = app_event_rx.recv().await {
            match event {
                AppEvent::Shutdown => {
                    context.request_repaint();
                    break;
                }
            }
        }
    }

    info!("Waiting for camera frame listener to finish");
    let _ = camera_frame_listener_handle.await;

    info!("Network task shutdown");
    Ok(())
}

async fn camera_frame_listener(stack: EdgeStack, id: u8, tx_out: Sender<ColorImage>, context: Context, mut app_event_rx: broadcast::Receiver<AppEvent>) -> Result<(), anyhow::Error> {
    let subber = stack.topics().bounded_receiver::<CameraFrameChunkTopic, 320>(None);
    let subber = pin!(subber);
    let mut hdl = subber.subscribe_unicast();
    let port_id = hdl.port();

    info!("camera frame listener started, port: {}", port_id);

    let query = SocketQuery {
        key: CameraStreamerCommandEndpoint::REQ_KEY.to_bytes(),
        nash_req: NameRequirement::Any,
        frame_kind: FrameKind::ENDPOINT_REQ,
        broadcast: false,
    };

    let res = stack
        .discovery()
        .discover_sockets(4, Duration::from_secs(1), &query)
        .await;
    if res.is_empty() {
        return Err(anyhow::anyhow!("No discovery results"));
    }

    let response = stack.endpoints().request::<CameraStreamerCommandEndpoint>(res[0].address, &CameraStreamerCommandRequest { command: CameraStreamerCommand::StartStreaming { port_id } }, None).await;
    if let Err(e) = response {
        return Err(anyhow::anyhow!("Error sending start request: {:?}", e));
    }


    struct InProgressFrame {
        total_chunks: u32,
        chunks: Vec<Option<Vec<u8>>>,
        received_count: u32,
        start_time: Instant,
    }

    let mut in_progress: HashMap<u64, InProgressFrame> = HashMap::new();

    loop {

        select! {
            app_event = app_event_rx.recv() => {
                match app_event {
                    Ok(event) => match event {
                        AppEvent::Shutdown => {
                            break
                        }
                    }
                    Err(_) => {
                        break;
                    }
                }
            }
            msg = hdl.recv() => {
                let chunk = &msg.t;

                let entry_and_image_chunk = match &chunk.kind {
                    CameraFrameChunkKind::Meta(frame_meta) => {
                        in_progress.insert(chunk.frame_number, InProgressFrame {
                            total_chunks: frame_meta.total_chunks,
                            chunks: vec![None; frame_meta.total_chunks as usize],
                            received_count: 0,
                            start_time: Instant::now(),
                        });
                        continue;
                    }
                    CameraFrameChunkKind::ImageChunk(image_chunk) => {
                        in_progress.get_mut(&chunk.frame_number).map(|entry|(entry, image_chunk))
                    }
                };

                let Some((entry, image_chunk)) = entry_and_image_chunk else {
                    continue;
                };

                trace!(
                    "received frame chunk: frame={} chunk={}/{} size={}",
                    chunk.frame_number,
                    image_chunk.chunk_index + 1,
                    entry.total_chunks,
                    image_chunk.bytes.len()
                );

                // Insert chunk if not already present
                let idx = image_chunk.chunk_index as usize;
                if idx >= entry.chunks.len() {
                    trace!("invalid chunk index {} for frame {}", idx, chunk.frame_number);
                    continue;
                }
                if entry.chunks[idx].is_none() {
                    entry.chunks[idx] = Some(image_chunk.bytes.clone());
                    entry.received_count += 1;
                }

                // Check if frame is complete
                if entry.received_count == entry.total_chunks {
                    // Reassemble JPEG data in order
                    let mut jpeg_data = Vec::new();
                    for c in entry.chunks.iter() {
                        if let Some(bytes) = c {
                            jpeg_data.extend_from_slice(bytes);
                        } else {
                            // Missing chunk — shouldn’t happen
                            trace!("missing chunk during reassembly for frame {}", chunk.frame_number);
                            continue;
                        }
                    }

                    let before = std::time::Instant::now();
                    debug!("received camera frame from server, frame_number: {}, chunks: {}, timestamp: {:?}", chunk.frame_number, entry.total_chunks, before);

                    // Decode JPEG
                    let before = std::time::Instant::now();
                    match image::load_from_memory_with_format(&jpeg_data, ImageFormat::Jpeg) {
                        Ok(img) => {
                            let point1 = std::time::Instant::now();
                            let rgba = img.to_rgba8();
                            let (w, h) = (rgba.width() as usize, rgba.height() as usize);
                            let color_image = ColorImage::from_rgba_unmultiplied([w, h], &rgba.into_raw());

                            let _ = tx_out.send(color_image);
                            context.request_repaint();

                            let after = std::time::Instant::now();
                            trace!("sent frame to egui, frame_number: {}, size: {} bytes, timestamp: {:?}, decoding: {}us, imagegen+send: {}us, total-elapsed: {}us",
                                chunk.frame_number,
                                jpeg_data.len(),
                                after,
                                (point1 - before).as_micros(),
                                (after - point1).as_micros(),
                                (after - before).as_micros(),
                            );
                        }
                        Err(e) => {
                            error!("decode error frame {}: {:?}", chunk.frame_number, e);
                        }
                    }


                    // Remove the completed frame from tracking
                    in_progress.remove(&chunk.frame_number);
                }
                // drop old frames (stuck/incomplete)
                let now = Instant::now();
                in_progress.retain(|frame_num, f| {
                    if now.duration_since(f.start_time) > Duration::from_secs(1) {
                        warn!(
                                "discarding incomplete frame {} (got {}/{})",
                                frame_num,
                                f.received_count,
                                f.total_chunks
                            );
                        false
                    } else {
                        true
                    }
                });

            }
        }
    }


    let response = stack.endpoints().request::<CameraStreamerCommandEndpoint>(res[0].address, &CameraStreamerCommandRequest { command: CameraStreamerCommand::StopStreaming { port_id } }, None).await;
    if let Err(e) = response {
        return Err(anyhow::anyhow!("Error sending stop request: {:?}", e));
    }
    info!("camera frame listener stopped, port: {}", port_id);
    Ok(())
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

struct CameraApp {
    rx: Receiver<ColorImage>,
    texture: Option<egui::TextureHandle>,

    gui_fps_stats: FpsStats,
    gui_fps_snapshot: Option<fps_stats::FpsSnapshot>,

    camera_fps_stats: FpsStats,
    camera_fps_snapshot: Option<fps_stats::FpsSnapshot>,

    gui_frame_number: u64,
    camera_frame_number: u64,

    app_event_tx: Arc<broadcast::Sender<AppEvent>>,
    app_event_rx: broadcast::Receiver<AppEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum AppEvent {
    Shutdown,
}

impl CameraApp {
    fn new(cc: &CreationContext) -> Self {

        // Create shutdown channel
        let (app_event_tx, app_event_rx) = broadcast::channel::<AppEvent>(16);

        ctrlc::set_handler({
            let app_event_tx = app_event_tx.clone();
            move || {
                warn!("Ctrl+C received, shutting down.");
                let _ = app_event_tx.send(AppEvent::Shutdown);
            }
        }).expect("Error setting Ctrl+C handler");

        let (camera_image_tx, camera_image_rx) = watch::channel::<ColorImage>(ColorImage::default());

        let app_event_tx = Arc::new(app_event_tx);
        start_network_thread(camera_image_tx, cc.egui_ctx.clone(), app_event_tx.clone());

        Self {
            rx: camera_image_rx,
            texture: None,
            gui_fps_stats: FpsStats::new(300),
            gui_fps_snapshot: None,
            camera_fps_stats: FpsStats::new(300),
            camera_fps_snapshot: None,

            gui_frame_number: 0,
            camera_frame_number: 0,

            app_event_tx,
            app_event_rx,
        }
    }
}

impl App for CameraApp {

    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {

        let now = std::time::Instant::now();

        if ctx.cumulative_frame_nr() != self.gui_frame_number {
            self.gui_fps_snapshot = self.gui_fps_stats.update(now);
            self.gui_frame_number = ctx.cumulative_frame_nr();
        }

        if let Ok(true) = self.rx.has_changed() {
            let color_image = self.rx.borrow_and_update().clone();
            self.camera_frame_number += 1;
            self.camera_fps_snapshot = self.camera_fps_stats.update(now);
            trace!("received frame, now: {:?}, frame_number: {}, snapshot: {:?}", now, self.camera_frame_number, self.camera_fps_snapshot);

            if let Some(tex) = &mut self.texture {
                tex.set(color_image, TextureOptions::default());
            } else {
                // create texture first time
                self.texture = Some(ctx.load_texture("camera", color_image, Default::default()));
            }
        }

        egui::Window::new("Camera").show(ctx, |ui| {
            if let Some(tex) = &self.texture {
                ui.image(tex);
            } else {
                ui.label("Waiting for first frame...");
            }
        });

        egui::Window::new("Stats")
            .scroll(true)
            .show(ctx, |ui| {
                ui.push_id("gui", |ui| {
                    ui.group(|ui| {
                        ui.label("GUI");
                        ui.label(format!("Frame: {}", self.gui_frame_number));
                        if let Some(snapshot) = &self.gui_fps_snapshot {
                            ui.label(format!(
                                "FPS: {:.1} (min {:.1}, max {:.1}, avg {:.1})",
                                snapshot.latest,
                                snapshot.min,
                                snapshot.max,
                                snapshot.avg
                            ));

                            show_frame_durations(ui, &self.gui_fps_stats);
                        }
                    });
                });

                ui.separator();

                ui.push_id("camera", |ui| {
                    ui.group(|ui| {
                        ui.label("Camera");
                        ui.label(format!("Frame: {}", self.camera_frame_number));
                        if let Some(snapshot) = &self.camera_fps_snapshot {
                            ui.label(format!(
                                "FPS: {:.1} (min {:.1}, max {:.1}, avg {:.1})",
                                snapshot.latest,
                                snapshot.min,
                                snapshot.max,
                                snapshot.avg
                            ));

                            show_frame_durations(ui, &self.camera_fps_stats);
                        }
                    });
                });
            });

        if let Ok(event) = self.app_event_rx.try_recv() {
            match event {
                AppEvent::Shutdown => {
                    info!("GUI received shutdown event, shutting down");
                    ctx.send_viewport_cmd(ViewportCommand::Close)
                }
            }
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        info!("GUI shutting down");
        self.app_event_tx.send(AppEvent::Shutdown).unwrap();
    }
}

fn main() -> eframe::Result {
    env_logger::init();

    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Camera Client",
        options,
        Box::new(|cc| Ok(Box::new(CameraApp::new(cc)))),
    )
}

