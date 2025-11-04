// client.rs
use eframe::{egui, App, CreationContext, Frame};
use image::ImageFormat;
use std::{net::SocketAddr, thread};
use eframe::epaint::textures::TextureOptions;
use egui::{ColorImage, Context};
use tokio::net::TcpStream;
use tokio::runtime::Runtime;
use log::{error, info, trace};
use tokio::sync::watch;
use tokio::sync::watch::{Receiver, Sender};
use crate::fps_stats::egui::show_frame_durations;
use crate::fps_stats::FpsStats;

mod fps_stats;
const SERVER_ADDR: &str = "127.0.0.1:5000";

fn start_network_thread(tx_out: Sender<ColorImage>, context: Context) {
    // run a tokio runtime in background thread
    thread::spawn(move || {
        let rt = Runtime::new().expect("create tokio runtime");
        rt.block_on(async move {
            if let Err(e) = network_task(SERVER_ADDR, tx_out, context).await {
                error!("network task error: {:?}", e);
            }
        });
    });
}

async fn network_task(addr: &str, tx_out: Sender<ColorImage>, context: Context) -> anyhow::Result<()> {
    use tokio::io::AsyncReadExt;
    let socket_addr: SocketAddr = addr.parse()?;

    loop {
        let mut stream = match tokio::net::TcpStream::connect(socket_addr).await {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to connect to {}: {:?}", addr, e);
                continue;
            }
        };

        info!("Connected to {}", addr);

        stream.set_nodelay(true)?;

        async fn run_loop(stream: &mut TcpStream, tx_out: &Sender<ColorImage>, context: &Context) -> Result<(), anyhow::Error> {
            loop {
                // read 4-byte length
                let mut len_buf = [0u8; 4];
                stream.read_exact(&mut len_buf).await?;
                let len = u32::from_be_bytes(len_buf) as usize;
                let mut buf = vec![0u8; len];
                stream.read_exact(&mut buf).await?;

                // decode JPEG OFF the GUI thread
                let color_image = tokio::task::spawn_blocking(move || -> anyhow::Result<ColorImage> {
                    let img = image::load_from_memory_with_format(&buf, ImageFormat::Jpeg)?;
                    let rgba = img.to_rgba8();
                    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
                    Ok(ColorImage::from_rgba_unmultiplied([w, h], &rgba.into_raw()))
                }).await??;

                // If the receiver is full, drop the frame (non-blocking)
                tx_out.send(color_image)?;
                context.request_repaint();
            }
        }

        match run_loop(&mut stream, &tx_out, &context).await {
            Ok(()) => {
                info!("Disconnected from {}", addr);
            }
            Err(e) => {
                info!("Disconnected from {}, error: {}", addr, e);
            }
        }
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
}

impl CameraApp {
    fn new(cc: &CreationContext) -> Self {


        let (tx, rx) = watch::channel::<ColorImage>(ColorImage::default());

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

