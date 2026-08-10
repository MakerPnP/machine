use eframe::emath::Vec2;
use egui::Ui;

#[derive(Default)]
pub(crate) struct StatusUi {}

impl StatusUi {
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.label("Status content");
    }

    pub fn tools_ui(&mut self, ui: &mut Ui, button_size: Vec2) {
        // no-op
    }
}
