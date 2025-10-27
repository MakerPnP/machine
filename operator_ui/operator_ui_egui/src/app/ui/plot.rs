use egui::Ui;

#[derive(Default)]
pub(crate) struct PlotUi {

}

impl PlotUi {
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.label("Plot content");
    }
}
