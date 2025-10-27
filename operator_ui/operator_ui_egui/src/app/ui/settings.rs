use egui::Ui;

#[derive(Default)]
pub(crate) struct SettingsUi {

}

impl SettingsUi {
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.label("Settings content");
    }
}
