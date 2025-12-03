use std::time::Instant;

use eframe::epaint::Color32;
use eframe::epaint::textures::TextureOptions;
use egui::{Frame, RichText, Ui, UiBuilder, Widget};
use egui_i18n::tr;
use egui_mobius::Value;
use egui_tool_windows::ToolWindows;
use tokio::sync::watch::Receiver;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{error, trace};

use crate::fps_stats::egui::show_frame_durations;
use crate::fps_stats::{FpsSnapshot, FpsStats};
use crate::net::camera::CameraFrame;

pub(crate) struct CameraUi {
    rx: Receiver<CameraFrame>,
    texture: Option<egui::TextureHandle>,
    next_frame_at: Instant,
    timestamp: chrono::DateTime<chrono::Utc>,

    camera_frame_listener_handle: JoinHandle<anyhow::Result<()>>,
    shutdown_token: CancellationToken,

    camera_frame_number: u64,
    camera_fps_stats: Value<FpsStats>,
    camera_fps_snapshot: Option<FpsSnapshot>,

    lag_counter: u64,
}

impl CameraUi {
    pub fn new(
        rx: Receiver<CameraFrame>,
        camera_frame_listener_handle: JoinHandle<anyhow::Result<()>>,
        shutdown_token: CancellationToken,
    ) -> Self {
        Self {
            rx,
            texture: None,
            next_frame_at: Instant::now(),
            timestamp: Default::default(),

            camera_frame_listener_handle,
            shutdown_token,

            camera_fps_stats: Value::new(FpsStats::new(300)),
            camera_fps_snapshot: None,
            camera_frame_number: 0,

            lag_counter: 0,
        }
    }

    pub async fn shutdown(self) {
        self.shutdown_token.cancel();
        let _ = self
            .camera_frame_listener_handle
            .await
            .inspect_err(|e| error!("Error shutting down camera frame listener: {:?}", e))
            .map(|r| r.inspect_err(|e| error!("Camera frame listener error: {:?}", e)))
            .ok();
    }
}

impl CameraUi {
    pub fn ui(&mut self, ui: &mut Ui) {
        let now = std::time::Instant::now();

        if let Ok(true) = self.rx.has_changed() {
            if now > self.next_frame_at {
                let camera_frame = self.rx.borrow_and_update().clone();
                self.next_frame_at += camera_frame.frame_interval;
                if now > self.next_frame_at {
                    // catch up if we fall behind
                    self.next_frame_at = now + camera_frame.frame_interval;
                    self.lag_counter = self.lag_counter.wrapping_add(1);
                }

                self.camera_frame_number += 1;
                if let Ok(snapshot) = self
                    .camera_fps_stats
                    .lock()
                    .map(|mut fps_stats| fps_stats.update(now))
                {
                    self.camera_fps_snapshot = snapshot;
                    trace!(
                        "received frame, now: {:?}, frame_number: {}, snapshot: {:?}",
                        now, self.camera_frame_number, self.camera_fps_snapshot
                    );
                }

                self.timestamp = (*camera_frame.timestamp).into();

                if let Some(tex) = &mut self.texture {
                    tex.set(camera_frame.image, TextureOptions::default());
                } else {
                    // create texture first time
                    self.texture = Some(
                        ui.ctx()
                            .load_texture("camera", camera_frame.image, Default::default()),
                    );
                }
            }
        }

        // Schedule next repaint at render_after or sooner
        let repaint_delay = self
            .next_frame_at
            .saturating_duration_since(now.into());
        ui.ctx()
            .request_repaint_after(repaint_delay);

        egui::ScrollArea::both()
            //.id_salt(ui.id().with("content-scroll"))
            .show(ui, |ui| {
                if let Some(tex) = &self.texture {
                    egui::Image::new(tex)
                        .max_size(ui.available_size())
                        .maintain_aspect_ratio(true)
                        .ui(ui);

                    let mut overlay_ui = ui.new_child(
                        UiBuilder::new()
                            //.id_salt(ui.id().with("overlay"))
                            .max_rect(ui.clip_rect()),
                    );
                    overlay_ui.add(
                        egui::Label::new(RichText::new(format!("{}", self.timestamp)).color(Color32::GREEN))
                            .selectable(false),
                    );
                } else {
                    ui.label(tr!("camera-message-waiting"));
                }
            });

        let fps_stats_id = ui.make_persistent_id(
            ui.id()
                .with("camera-toolwindow-fps-stats"),
        );
        ToolWindows::new().windows(ui, |builder| {
            builder
                .add_window(fps_stats_id)
                .default_pos([10.0, 10.0])
                .default_size([400.0, 150.0])
                .show(tr!("camera-toolwindow-fps-stats-title"), {
                    let camera_fps_stats = self.camera_fps_stats.clone();
                    let camera_fps_snapshot = self.camera_fps_snapshot.clone();
                    let camera_frame_number = self.camera_frame_number;

                    move |ui| {
                        egui::ScrollArea::both()
                            .id_salt(ui.id().with("tool-window-scroll"))
                            .show(ui, |ui| {
                                Frame::group(ui.style()).show(ui, |ui| {
                                    ui.label(format!("Frame: {}", camera_frame_number));
                                    if let Some(snapshot) = &camera_fps_snapshot {
                                        ui.label(format!(
                                            "FPS: {:.1} (min {:.1}, max {:.1}, avg {:.1})",
                                            snapshot.latest, snapshot.min, snapshot.max, snapshot.avg
                                        ));

                                        let camera_fps_stats = camera_fps_stats.lock().unwrap();
                                        show_frame_durations(ui, &camera_fps_stats);
                                    }
                                });
                            });
                    }
                });
        });
    }
}
