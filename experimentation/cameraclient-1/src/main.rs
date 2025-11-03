// client.rs
use crossbeam_channel::{Receiver, Sender};
use eframe::{egui, App, CreationContext, Frame};
use image::ImageFormat;
use std::{net::SocketAddr, thread};
use eframe::epaint::textures::TextureOptions;
use egui::Context;
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use log::{error, info, trace, warn};
use crate::fps_stats::egui::show_frame_durations;
use crate::fps_stats::FpsStats;

mod fps_stats;
const SERVER_ADDR: &str = "127.0.0.1:5000";

fn start_network_thread(tx_out: crossbeam_channel::Sender<Vec<u8>>, context: Context) {
    // run a tokio runtime in background thread
    thread::spawn(move || {
        let rt = Runtime::new().expect("create tokio runtime");
        rt.block_on(async move {
            if let Err(e) = network_task(SERVER_ADDR, tx_out, context).await {
                eprintln!("network task error: {:?}", e);
            }
        });
    });
}

async fn network_task(addr: &str, tx_out: crossbeam_channel::Sender<Vec<u8>>, context: Context) -> anyhow::Result<()> {
    use tokio::io::AsyncReadExt;
    let socket_addr: SocketAddr = addr.parse()?;

    loop {
        let mut stream = match tokio::net::TcpStream::connect(socket_addr).await {
            Ok(s) => s,
            Err(e) => {
                println!("Failed to connect to {}: {:?}", addr, e);
                continue;
            }
        };

        println!("Connected to {}", addr);

        stream.set_nodelay(true)?;

        async fn run_loop(stream: &mut TcpStream, tx_out: &Sender<Vec<u8>>, context: &Context) -> Result<(), anyhow::Error> {
            loop {
                // read 4-byte length
                let mut len_buf = [0u8; 4];
                stream.read_exact(&mut len_buf).await?;
                let len = u32::from_be_bytes(len_buf) as usize;
                let mut buf = vec![0u8; len];
                stream.read_exact(&mut buf).await?;
                // send to UI
                // If the receiver is full, drop the frame (non-blocking)
                let _ = tx_out.try_send(buf);
                context.request_repaint();
            }
            Ok(())
        }

        match run_loop(&mut stream, &tx_out, &context).await {
            Ok(()) => {
                println!("Disconnected from {}", addr);
            }
            Err(e) => {
                println!("Disconnected from {}, error: {}", addr, e);
            }
        }
    }
}

struct CameraApp {
    rx: Receiver<Vec<u8>>,
    texture: Option<egui::TextureHandle>,

    gui_fps_stats: FpsStats,
    gui_fps_snapshot: Option<fps_stats::FpsSnapshot>,

    camera_fps_stats: FpsStats,
    camera_fps_snapshot: Option<fps_stats::FpsSnapshot>,

    gui_frame_number: u64,
    camera_frame_number: u64,
}

impl CameraApp {
    fn new(cc: &CreationContext) -> Self {

        // bounded channel to avoid OOM; keep one frame only
        let (tx, rx) = crossbeam_channel::bounded::<Vec<u8>>(4);

        start_network_thread(tx, cc.egui_ctx.clone());

        Self {

            rx,
            texture: None,
            gui_fps_stats: FpsStats::new(300),
            gui_fps_snapshot: None,
            camera_fps_stats: FpsStats::new(300),
            camera_fps_snapshot: None,

            gui_frame_number: 0,
            camera_frame_number: 0,
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

        // Drain latest frame (if multiple arrived, take the most recent)
        let mut latest: Option<Vec<u8>> = None;

        if let Ok(mut frame) = self.rx.try_recv() {
            self.camera_frame_number += 1;
            // Drop old ones, keep only the most recent
            while let Ok(next) = self.rx.try_recv() {
                warn!("dropping old frame");
                frame = next;
                self.camera_frame_number += 1;
            }

            self.camera_fps_snapshot = self.camera_fps_stats.update(now);
            latest = Some(frame);
            trace!("received frame, now: {:?}, frame_number: {}", now, self.camera_frame_number);
        }

        if let Some(jpeg_bytes) = latest {
            if let Ok(img) = image::load_from_memory_with_format(&jpeg_bytes, ImageFormat::Jpeg) {
                let rgba = img.to_rgba8();
                let (w, h) = (rgba.width() as usize, rgba.height() as usize);
                let pixels = rgba.into_raw(); // Vec<u8> RGBA
                let color_image = egui::ColorImage::from_rgba_unmultiplied([w, h], &pixels);

                if let Some(tex) = &mut self.texture {
                    tex.set(color_image, TextureOptions::default());
                } else {
                    // create texture first time
                    self.texture = Some(ctx.load_texture("camera", color_image, Default::default()));
                }
            } else {
                error!("Failed to decode JPEG frame");
            }
        }

        egui::Window::new("Camera").show(ctx, |ui| {
            if let Some(tex) = &self.texture {
                //let size = tex.size_vec2();
                //ui.image(tex, size);
                ui.image(tex);
            } else {
                ui.label("Waiting for first frame...");
            }
        });

        egui::Window::new("Stats")
            .scroll(true)
            .show(ctx, |ui| {
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
            ui.separator();
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

        // Request repaint for smooth video
        ctx.request_repaint();
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Camera Client",
        options,
        Box::new(|cc| Ok(Box::new(CameraApp::new(cc)))),
    )
}

