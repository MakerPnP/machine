use std::collections::HashMap;
// client.rs
use eframe::{egui, App, CreationContext, Frame, NativeOptions};
use image::ImageFormat;
use std::thread;
use std::pin::pin;
use std::sync::Arc;
use std::time::Duration;
use eframe::epaint::textures::TextureOptions;
use egui::{Color32, ColorImage, Context, Rect, RichText, UiBuilder, Vec2, ViewportBuilder, ViewportCommand};
use ergot::{endpoint, toolkits::tokio_udp::{EdgeStack, new_std_queue, new_target_stack, register_edge_interface}, topic, well_known::DeviceInfo, Address, FrameKind};
use ergot::interface_manager::profiles::direct_edge::EDGE_NODE_ID;
use ergot::interface_manager::profiles::direct_edge::tokio_udp::InterfaceKind;
use ergot::well_known::{NameRequirement, SocketQuery};
use ergot::traits::Endpoint;
use tokio::runtime::Runtime;
use log::{debug, error, info, trace, warn};
use tokio::{net::UdpSocket, select};
use tokio::sync::{broadcast, watch};
use tokio::sync::watch::{Receiver, Sender};
use tokio::time::Instant;
use camerastreamer_ergot_shared::{CameraFrameChunk, CameraFrameChunkKind, CameraStreamerCommand, CameraStreamerCommandRequest, CameraStreamerCommandResponse, TimeStampUTC};
use crate::fps_stats::egui::show_frame_durations;
use crate::fps_stats::FpsStats;

mod fps_stats;
const REMOTE_ADDR: &str = "127.0.0.1:5000";
const LOCAL_ADDR: &str = "0.0.0.0:5001";
const TARGET_FPS: u8 = 30;
const SCHEDULED_FPS_MIN: u8 = 5;
const SCHEDULED_FPS_MAX: u8 = 60;

topic!(CameraFrameChunkTopic, CameraFrameChunk, "topic/camera_stream");
endpoint!(CameraStreamerCommandEndpoint, CameraStreamerCommandRequest, CameraStreamerCommandResponse, "topic/camera");

fn start_network_thread(tx_out: Sender<CameraFrame>, context: Context, app_event_tx: Arc<broadcast::Sender<AppEvent>>) {
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

async fn network_task(remote_addr: &str, tx_out: Sender<CameraFrame>, context: Context, app_event_tx: Arc<broadcast::Sender<AppEvent>>) -> anyhow::Result<()> {

    let queue = new_std_queue(1024 * 1024);
    let stack: EdgeStack = new_target_stack(&queue, 1400);
    let udp_socket = UdpSocket::bind(LOCAL_ADDR).await?;

    udp_socket.connect(remote_addr).await?;

    tokio::task::spawn(basic_services(stack.clone(), 0));
    let camera_frame_listener_handle = tokio::task::spawn(camera_frame_listener(stack.clone(), tx_out, context.clone(), app_event_tx.subscribe()));

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

async fn camera_frame_listener(stack: EdgeStack, tx_out: Sender<CameraFrame>, context: Context, mut app_event_rx: broadcast::Receiver<AppEvent>) -> Result<(), anyhow::Error> {
    let subber = stack.topics().bounded_receiver::<CameraFrameChunkTopic, 320>(None);
    let subber = pin!(subber);
    let mut hdl = subber.subscribe_unicast();

    let address = Address {
        network_id: 0,
        node_id: EDGE_NODE_ID,
        port_id: hdl.port(),
    };

    info!("camera frame listener started, port: {}", address.port_id);

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

    let response = stack.endpoints().request::<CameraStreamerCommandEndpoint>(res[0].address, &CameraStreamerCommandRequest { command: CameraStreamerCommand::StartStreaming { address } }, None).await;
    if let Err(e) = response {
        return Err(anyhow::anyhow!("Error sending start request: {:?}", e));
    }

    struct InProgressFrame {
        total_chunks: u32,
        chunks: Vec<Option<Vec<u8>>>,
        received_count: u32,
        start_time: Instant,
        frame_timestamp: TimeStampUTC,
        frame_number: u64,
        frame_interval: Duration,
    }

    let mut in_progress: HashMap<u64, InProgressFrame> = HashMap::new();

    let mut target_fps = TARGET_FPS as f32;
    let mut frame_timestamps = std::collections::VecDeque::with_capacity(60);

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

                        // Update timestamps for FPS estimation
                        frame_timestamps.push_back(frame_meta.frame_timestamp);
                        if frame_timestamps.len() > (TARGET_FPS * 2) as usize {
                            frame_timestamps.pop_front();
                        }

                        // Recompute effective FPS
                        if frame_timestamps.len() >= 2 {
                            let mut iter = frame_timestamps.iter();
                            let newest = iter.next_back().unwrap().0;
                            let previous: chrono::DateTime<chrono::Utc> = iter.next_back().unwrap().0;
                            let oldest = frame_timestamps.front().unwrap().0;

                            let newest_previous_span = newest - previous;
                            let total_span = newest - oldest;

                            let frame_count = frame_timestamps.len() - 1;
                            let measured_fps = frame_count as f64 / total_span.as_seconds_f64();


                            // Smooth update using exponential moving average
                            let alpha = 0.1;
                            target_fps = (1.0 - alpha) * target_fps + alpha * (measured_fps as f32);

                            debug!("estimated FPS: {:.1}, target FPS: {:.1}, total_span: {}ms, span: {}ms",
                                measured_fps,
                                target_fps,
                                total_span.num_milliseconds(),
                                newest_previous_span.num_milliseconds(),
                            );
                        }

                        // schedule next render
                        let clamped_fps = target_fps.clamp(SCHEDULED_FPS_MIN.into(), SCHEDULED_FPS_MAX.into());

                        let frame_interval = Duration::from_secs_f64(1.0 / clamped_fps as f64);

                        in_progress.insert(chunk.frame_number, InProgressFrame {
                            total_chunks: frame_meta.total_chunks,
                            chunks: vec![None; frame_meta.total_chunks as usize],
                            received_count: 0,
                            start_time: Instant::now(),
                            frame_timestamp: frame_meta.frame_timestamp.clone(),
                            frame_number: chunk.frame_number,
                            frame_interval,
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
                let frame_complete = entry.received_count == entry.total_chunks;

                if frame_complete {
                    // Remove the completed frame from tracking, also avoids borrowing the entry
                    let entry = in_progress.remove(&chunk.frame_number).unwrap();

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

                    debug!("received camera frame from server, frame_number: {}, chunks: {}, frame_timestamp: {:?}, frame_interval: {}ms", chunk.frame_number, entry.total_chunks, entry.frame_timestamp, entry.frame_interval.as_millis());

                    // Decode JPEG
                    let decode_before = Instant::now();
                    match image::load_from_memory_with_format(&jpeg_data, ImageFormat::Jpeg) {
                        Ok(img) => {
                            let point1 = Instant::now();
                            let rgba = img.to_rgba8();
                            let (w, h) = (rgba.width() as usize, rgba.height() as usize);
                            let color_image = ColorImage::from_rgba_unmultiplied([w, h], &rgba.into_raw());

                            let camera_frame = CameraFrame {
                                image: color_image,
                                timestamp: entry.frame_timestamp,
                                frame_number: entry.frame_number,
                                frame_interval: entry.frame_interval,
                            };

                            context.request_repaint();
                            let _ = tx_out.send(camera_frame);

                            let after = Instant::now();
                            trace!("sent frame to egui, frame_number: {}, size: {} bytes, timestamp: {:?}, decoding: {}us, imagegen+send: {}us, total-elapsed: {}us",
                                chunk.frame_number,
                                jpeg_data.len(),
                                entry.frame_timestamp,
                                (point1 - decode_before).as_micros(),
                                (after - point1).as_micros(),
                                (after - decode_before).as_micros(),
                            );
                        }
                        Err(e) => {
                            error!("decode error frame {}: {:?}", chunk.frame_number, e);
                        }
                    }
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


    let response = stack.endpoints().request::<CameraStreamerCommandEndpoint>(res[0].address, &CameraStreamerCommandRequest { command: CameraStreamerCommand::StopStreaming { address } }, None).await;
    if let Err(e) = response {
        return Err(anyhow::anyhow!("Error sending stop request: {:?}", e));
    }
    info!("camera frame listener stopped, address: {}", address);
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
    rx: Receiver<CameraFrame>,
    texture: Option<egui::TextureHandle>,
    render_after: std::time::Instant,
    timestamp: chrono::DateTime<chrono::Utc>,

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

        // Create event channel
        let (app_event_tx, app_event_rx) = broadcast::channel::<AppEvent>(16);

        ctrlc::set_handler({
            let app_event_tx = app_event_tx.clone();
            move || {
                warn!("Ctrl+C received, shutting down.");
                let _ = app_event_tx.send(AppEvent::Shutdown);
            }
        }).expect("Error setting Ctrl+C handler");

        let (camera_image_tx, camera_image_rx) = watch::channel::<CameraFrame>(CameraFrame::default());

        let app_event_tx = Arc::new(app_event_tx);
        start_network_thread(camera_image_tx, cc.egui_ctx.clone(), app_event_tx.clone());

        Self {
            rx: camera_image_rx,
            texture: None,

            render_after: std::time::Instant::now(),
            timestamp: chrono::Utc::now(),

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

            if now > self.render_after {
                let camera_frame = self.rx.borrow_and_update().clone();

                self.render_after += camera_frame.frame_interval;
                if now > self.render_after {
                    // catch up if we fall behind
                    self.render_after = now + camera_frame.frame_interval;
                    error!("lagged");
                }

                self.camera_frame_number += 1;
                self.camera_fps_snapshot = self.camera_fps_stats.update(now);

                debug!("received frame, frame_number: {}, snapshot: {:?}",
                    self.camera_frame_number,
                    self.camera_fps_snapshot
                );

                self.timestamp = (*camera_frame.timestamp).into();

                if let Some(tex) = &mut self.texture {
                    tex.set(camera_frame.image, TextureOptions::default());
                } else {
                    // create texture first time
                    self.texture = Some(ctx.load_texture("camera", camera_frame.image, Default::default()));
                }
            } else {
                trace!("waiting for next frame to be ready");
            }
        }

        // Schedule next repaint at render_after or sooner
        let repaint_delay = self.render_after.saturating_duration_since(now.into());
        ctx.request_repaint_after(repaint_delay);

        egui::Window::new("Camera")
            .default_pos([10.0, 10.0])
            .default_size([1280.0, 720.0])
            .scroll(true)
            .resizable(true)
            .constrain(false)
            .show(ctx, |ui| {
                if let Some(tex) = &self.texture {
                    ui.add(egui::Image::new(tex).maintain_aspect_ratio(true).max_size(ui.max_rect().size()));
                    let overlay_clip_rect = ui.clip_rect();

                    let mut overlay_ui = ui.new_child(UiBuilder::new()
                        .max_rect(ui.clip_rect())
                    );
                    overlay_ui.set_clip_rect(overlay_clip_rect);
                    overlay_ui.add(egui::Label::new(RichText::new(format!("{}", self.timestamp)).color(Color32::GREEN)).selectable(false));
                } else {
                    ui.label("Waiting for first frame...");
                }
        });

        egui::Window::new("Stats")
            .default_pos([1320.0, 10.0])
            .constrain(false)
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

    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([1700.0, 830.0]),
        ..NativeOptions::default()
    };
    eframe::run_native(
        "Camera Client",
        options,
        Box::new(|cc| Ok(Box::new(CameraApp::new(cc)))),
    )
}

impl Default for CameraFrame {
    fn default() -> Self {
        Self {
            image: Default::default(),
            timestamp: chrono::Utc::now().into(),
            frame_number: 0,
            frame_interval: Duration::from_secs(0),
        }
    }
}
#[derive(Clone, Debug)]
struct CameraFrame {
    image: ColorImage,
    timestamp: TimeStampUTC,
    frame_number: u64,
    frame_interval: Duration,
}
