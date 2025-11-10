use eframe::epaint::ColorImage;
use eframe::epaint::textures::TextureOptions;
use egui::{Frame, Ui, Widget};
use egui_i18n::tr;
use egui_mobius::Value;
use egui_tool_windows::ToolWindows;
use tokio::sync::watch::Receiver;
use tracing::trace;

use crate::fps_stats::egui::show_frame_durations;
use crate::fps_stats::{FpsSnapshot, FpsStats};

pub(crate) struct CameraUi {
    rx: Receiver<ColorImage>,
    texture: Option<egui::TextureHandle>,

    camera_frame_number: u64,
    camera_fps_stats: Value<FpsStats>,
    camera_fps_snapshot: Option<FpsSnapshot>,
}

impl CameraUi {
    pub fn new(rx: Receiver<ColorImage>) -> Self {
        Self {
            rx,
            texture: None,

            camera_fps_stats: Value::new(FpsStats::new(300)),
            camera_fps_snapshot: None,
            camera_frame_number: 0,
        }
    }
}

impl CameraUi {
    pub fn ui(&mut self, ui: &mut Ui) {
        let now = std::time::Instant::now();

        if let Ok(true) = self.rx.has_changed() {
            let color_image = self.rx.borrow_and_update().clone();
            self.camera_frame_number += 1;
            if let Ok(snapshot) = self.camera_fps_stats.lock().map(|mut fps_stats| {
                fps_stats.update(now)
            }) {
                self.camera_fps_snapshot = snapshot;
                trace!(
                    "received frame, now: {:?}, frame_number: {}, snapshot: {:?}",
                    now, self.camera_frame_number, self.camera_fps_snapshot
                );
            }

            if let Some(tex) = &mut self.texture {
                tex.set(color_image, TextureOptions::default());
            } else {
                // create texture first time
                self.texture = Some(
                    ui.ctx()
                        .load_texture("camera", color_image, Default::default()),
                );
            }
        }

        egui::ScrollArea::both().show(ui, |ui| {
            if let Some(tex) = &self.texture {
                egui::Image::new(tex)
                    .max_size(ui.available_size())
                    .maintain_aspect_ratio(true)
                    .ui(ui);
            } else {
                ui.label(tr!("camera-message-waiting"));
            }
        });

        let fps_stats_id = ui.make_persistent_id(ui.id().with("camera-toolwindow-fps-stats"));
        ToolWindows::new()
            .windows(ui, |builder|{
                builder
                    .add_window(fps_stats_id)
                    .default_pos([10.0, 10.0])
                    .default_size([400.0, 150.0])
                    .show(tr!("camera-toolwindow-fps-stats-title"), {
                        let camera_fps_stats = self.camera_fps_stats.clone();
                        let camera_fps_snapshot = self.camera_fps_snapshot.clone();
                        let camera_frame_number = self.camera_frame_number;

                        move |ui| {
                            egui::ScrollArea::both().show(ui, |ui| {
                                Frame::group(ui.style()).show(ui, |ui|{
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
