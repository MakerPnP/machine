use egui::Ui;

#[derive(Default)]
pub(crate) struct StatusUi {}

impl StatusUi {
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.label("Status content");
    }
}
