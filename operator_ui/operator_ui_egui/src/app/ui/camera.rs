use egui::Ui;

#[derive(Default)]
pub(crate) struct CameraUi {}

impl CameraUi {
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.label("Camera content");
    }
}
