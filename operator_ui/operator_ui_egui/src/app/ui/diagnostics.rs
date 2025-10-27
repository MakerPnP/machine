use egui::Ui;

#[derive(Default)]
pub(crate) struct DiagnosticsUi {

}

impl DiagnosticsUi {
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.label("Diagnostics content");
    }
}
