use eframe::emath::Vec2;
use egui::Ui;

#[derive(Default)]
pub(crate) struct SettingsUi {}

impl SettingsUi {
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.label("Settings content");
    }

    pub fn tools_ui(&mut self, ui: &mut Ui, button_size: Vec2) {
        // no-op
    }
}
