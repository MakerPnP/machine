use eframe::emath::Vec2;
use egui::Ui;

#[derive(Default)]
pub(crate) struct PlotUi {}

impl PlotUi {
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.label("Plot content");
    }

    pub fn tools_ui(&mut self, ui: &mut Ui, button_size: Vec2) {
        // no-op
    }
}
