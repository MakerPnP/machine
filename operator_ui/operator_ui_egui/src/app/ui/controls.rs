use egui::{Ui, Vec2};
use egui_i18n::tr;

#[derive(Default)]
pub(crate) struct ControlsUi {
    /// Range: 0.0 to 1.0
    speed_scale: f32,

    // XXX
    layout_fail: LayoutFail,
}

// XXX
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum LayoutFail {
    Grid,
    HorizontalTopWithGroups,
    HorizontalWithGroups,
    HorizontalCenteredWithGroups,
    Horizontal,
    HorizontalCentered,
}

impl Default for LayoutFail {
    fn default() -> Self {
        Self::HorizontalTopWithGroups
    }
}

impl ControlsUi {
    pub fn ui(&mut self, ui: &mut Ui) {
        egui::ScrollArea::both()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.label("Controls content");

                // XXX
                if false {
                    egui::ComboBox::from_id_salt("layout_fail")
                        .selected_text(format!("{:?}", self.layout_fail))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.layout_fail,
                                LayoutFail::Grid,
                                format!("{:?}", LayoutFail::Grid),
                            );
                            ui.selectable_value(
                                &mut self.layout_fail,
                                LayoutFail::HorizontalTopWithGroups,
                                format!("{:?}", LayoutFail::HorizontalTopWithGroups),
                            );
                            ui.selectable_value(
                                &mut self.layout_fail,
                                LayoutFail::HorizontalWithGroups,
                                format!("{:?}", LayoutFail::HorizontalWithGroups),
                            );
                            ui.selectable_value(
                                &mut self.layout_fail,
                                LayoutFail::HorizontalCenteredWithGroups,
                                format!("{:?}", LayoutFail::HorizontalCenteredWithGroups),
                            );
                            ui.selectable_value(
                                &mut self.layout_fail,
                                LayoutFail::Horizontal,
                                format!("{:?}", LayoutFail::Horizontal),
                            );
                            ui.selectable_value(
                                &mut self.layout_fail,
                                LayoutFail::HorizontalCentered,
                                format!("{:?}", LayoutFail::HorizontalCentered),
                            );
                        });
                }
                match self.layout_fail {
                    LayoutFail::Grid => {
                        // FIXME using a grid with a single row in combination with ui.group() causes the second group to be vertically misalligned.
                        egui::Grid::new("controls_grid")
                            .num_columns(2)
                            .show(ui, |ui| {
                                ui.group(|ui| {
                                    Self::draw_jogxy_grid(ui);
                                });
                                ui.group(|ui| {
                                    Self::draw_jogz_grid(ui, 0);
                                });
                                ui.end_row();
                            });
                    }
                    LayoutFail::HorizontalTopWithGroups => {
                        // FIXME using ui.horizontal() in combination with ui.group() causes the second group to be vertically misalligned.
                        ui.horizontal_top(|ui| {
                            ui.group(|ui| {
                                Self::draw_jogxy_grid(ui);
                            });
                            ui.group(|ui| {
                                Self::draw_jogz_grid(ui, 0);
                            });
                        });
                    }
                    LayoutFail::HorizontalWithGroups => {
                        // FIXME using ui.horizontal() in combination with ui.group() causes the second group to be vertically misalligned.
                        ui.horizontal(|ui| {
                            ui.group(|ui| {
                                Self::draw_jogxy_grid(ui);
                            });
                            ui.group(|ui| {
                                Self::draw_jogz_grid(ui, 0);
                            });
                        });
                    }
                    LayoutFail::HorizontalCenteredWithGroups => {
                        // FIXME using horizontal_centered() causes the entire window content to be aligned to the bottom.
                        ui.horizontal_centered(|ui| {
                            ui.group(|ui| {
                                Self::draw_jogxy_grid(ui);
                            });
                            ui.group(|ui| {
                                Self::draw_jogz_grid(ui, 0);
                            });
                        });
                    }
                    LayoutFail::Horizontal => {
                        // FIXME we want groups!
                        ui.horizontal(|ui| {
                            Self::draw_jogxy_grid(ui);
                            Self::draw_jogz_grid(ui, 0);
                        });
                    }
                    LayoutFail::HorizontalCentered => {
                        // FIXME we want groups!
                        ui.horizontal_centered(|ui| {
                            Self::draw_jogxy_grid(ui);
                            Self::draw_jogz_grid(ui, 0);
                        });
                    }
                }

                ui.horizontal(|ui| {
                    ui.label("Speed %");
                    ui.add(
                        egui::Slider::new(&mut self.speed_scale, 0.0..=1.0)
                            .custom_formatter(|it, _range| format!("{:3.0}", it * 100.0)),
                    );
                });
            });
    }

    fn draw_jogxy_grid(ui: &mut Ui) {
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
        let mut max_size = Self::calculate_label_max_size(ui, &labels);

        let button_padding = ui.spacing().button_padding;
        max_size += button_padding * 2.0;

        egui::Grid::new("xy_jog_grid")
            .num_columns(3)
            .spacing(egui::vec2(4.0, 4.0))
            .show(ui, |ui| {
                // --- Top row ---
                Self::empty_cell(max_size, ui);
                if ui
                    .add_sized(max_size, egui::Button::new(&labels[JogDirection::YMinus as usize]))
                    .clicked()
                {}
                Self::empty_cell(max_size, ui);
                ui.end_row();

                // --- Middle row ---
                if ui
                    .add_sized(max_size, egui::Button::new(&labels[JogDirection::XMinus as usize]))
                    .clicked()
                {}
                Self::empty_cell(max_size, ui);
                if ui
                    .add_sized(max_size, egui::Button::new(&labels[JogDirection::XPlus as usize]))
                    .clicked()
                {}
                ui.end_row();

                // --- Bottom row ---
                Self::empty_cell(max_size, ui);
                if ui
                    .add_sized(max_size, egui::Button::new(&labels[JogDirection::YPlus as usize]))
                    .clicked()
                {}
                Self::empty_cell(max_size, ui);
                ui.end_row();
            });
    }

    fn draw_jogz_grid(ui: &mut Ui, index: usize) {
        #[repr(usize)]
        enum JogDirection {
            ZMinus = 0,
            ZPlus = 1,
            ZPark = 2,
        }

        let labels = [
            tr!("jog-z-minus", { index: index}),
            tr!("jog-z-plus", { index: index}),
            tr!("jog-z-park", { index: index}),
        ];
        let mut max_size = Self::calculate_label_max_size(ui, &labels);

        let button_padding = ui.spacing().button_padding;
        max_size += button_padding * 2.0;

        egui::Grid::new(format!("z{}_jog_grid", index))
            .num_columns(1)
            .spacing(egui::vec2(4.0, 4.0))
            .show(ui, |ui| {
                // --- Top row ---
                if ui
                    .add_sized(max_size, egui::Button::new(&labels[JogDirection::ZMinus as usize]))
                    .clicked()
                {}
                ui.end_row();

                // --- Middle row ---
                if ui
                    .add_sized(max_size, egui::Button::new(&labels[JogDirection::ZPlus as usize]))
                    .clicked()
                {}
                ui.end_row();

                // --- Bottom row ---
                if ui
                    .add_sized(max_size, egui::Button::new(&labels[JogDirection::ZPark as usize]))
                    .clicked()
                {}
                ui.end_row();
            });
    }

    fn calculate_label_max_size(ui: &mut Ui, labels: &[String]) -> Vec2 {
        let mut max_size = egui::Vec2::ZERO;

        for label in labels {
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

        max_size
    }

    fn empty_cell(max_size: Vec2, ui: &mut Ui) {
        ui.allocate_ui_with_layout(
            max_size,
            egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
            |_ui| {},
        );
    }
}
