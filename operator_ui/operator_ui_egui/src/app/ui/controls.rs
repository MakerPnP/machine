use egui::{Ui, Vec2};
use egui_i18n::tr;

#[derive(Default)]
pub(crate) struct ControlsUi {

}

impl ControlsUi {
    pub fn ui(&mut self, ui: &mut Ui) {

        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui|{
                ui.vertical(|ui| {
                    ui.label("Controls content");
                    ui.group(|ui| {
                        Self::draw_jog_grid(ui);
                    });
                });
            });
    }

    fn draw_jog_grid(ui: &mut Ui) {

        #[repr(usize)]
        enum JogDirection {
            YMinus = 0,
            XMinus = 1,
            XPlus = 2,
            YPlus = 3,
        }

        let labels = [
            tr!("jog-y-minus"),
            tr!("jog-x-minus"),
            tr!("jog-x-plus"),
            tr!("jog-y-plus"),
        ];
        let mut max_size = egui::Vec2::ZERO;

        for label in &labels {
            let desired = ui
                .fonts_mut(|f| {
                    f.layout_no_wrap(
                        label.to_string(),
                        egui::TextStyle::Button.resolve(ui.style()),
                        egui::Color32::WHITE,
                    )
                })
                .size();
            max_size.x = max_size.x.max(desired.x);
            max_size.y = max_size.y.max(desired.y);
        }

        let button_padding = ui.spacing().button_padding;
        max_size += button_padding * 2.0;

        egui::Grid::new("control_grid")
            //.num_columns(3)
            .spacing(egui::vec2(4.0, 4.0))
            .show(ui, |ui| {
                // --- Top row ---
                Self::empty_cell(max_size, ui);
                if ui.add_sized(max_size, egui::Button::new(&labels[JogDirection::YMinus as usize])).clicked() {
                }
                Self::empty_cell(max_size, ui);
                ui.end_row();

                // --- Middle row ---
                if ui.add_sized(max_size, egui::Button::new(&labels[JogDirection::XMinus as usize])).clicked() {
                }
                Self::empty_cell(max_size, ui);
                if ui.add_sized(max_size, egui::Button::new(&labels[JogDirection::XPlus as usize])).clicked() {
                }
                ui.end_row();

                // --- Bottom row ---
                Self::empty_cell(max_size, ui);
                if ui.add_sized(max_size, egui::Button::new(&labels[JogDirection::YPlus as usize])).clicked() {

                }
                Self::empty_cell(max_size, ui);
                ui.end_row();
            });
    }

    fn empty_cell(max_size: Vec2, ui: &mut Ui) {
        ui.allocate_ui_with_layout(max_size, egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |_ui| {});
    }
}
