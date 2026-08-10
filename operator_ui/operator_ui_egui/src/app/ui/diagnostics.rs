use eframe::emath::Vec2;
use egui::Ui;

#[derive(Default)]
pub(crate) struct DiagnosticsUi {}

impl DiagnosticsUi {
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.label("Diagnostics content");
    }

    pub fn tools_ui(&mut self, ui: &mut Ui, button_size: Vec2) {
        // no-op
    }
}
