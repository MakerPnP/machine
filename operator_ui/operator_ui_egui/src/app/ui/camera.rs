use eframe::epaint::ColorImage;
use eframe::epaint::textures::TextureOptions;
use egui::{Image, Ui, Widget};
use tokio::sync::watch::Receiver;
use tracing::trace;

use crate::fps_stats::egui::show_frame_durations;
use crate::fps_stats::{FpsSnapshot, FpsStats};

pub(crate) struct CameraUi {
    rx: Receiver<ColorImage>,
    texture: Option<egui::TextureHandle>,

    camera_frame_number: u64,
    camera_fps_stats: FpsStats,
    camera_fps_snapshot: Option<FpsSnapshot>,
}

impl CameraUi {
    pub fn new(rx: Receiver<ColorImage>) -> Self {
        Self {
            rx,
            texture: None,

            camera_fps_stats: FpsStats::new(300),
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
            self.camera_fps_snapshot = self.camera_fps_stats.update(now);
            trace!(
                "received frame, now: {:?}, frame_number: {}, snapshot: {:?}",
                now, self.camera_frame_number, self.camera_fps_snapshot
            );

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
                ui.label("Waiting for first frame...");
            }
        });

        // TODO use a tool window here, constrained to the camera tile/window.

        egui::Area::new(ui.id().with("camera_stats"))
            .interactable(true)
            .movable(true)
            .show(ui.ctx(), |ui| {
                egui::Frame::window(ui.style()).show(ui, |ui| {
                    egui::Resize::default().show(ui, |ui| {
                        ui.add(egui::Label::new("Stats").selectable(false));
                        ui.separator();
                        egui::ScrollArea::both().show(ui, |ui| {
                            ui.label(format!("Frame: {}", self.camera_frame_number));
                            if let Some(snapshot) = &self.camera_fps_snapshot {
                                ui.label(format!(
                                    "FPS: {:.1} (min {:.1}, max {:.1}, avg {:.1})",
                                    snapshot.latest, snapshot.min, snapshot.max, snapshot.avg
                                ));

                                show_frame_durations(ui, &self.camera_fps_stats);
                            }
                        })
                    });
                });
            });
    }
}
