use async_std::prelude::StreamExt;
use egui::{Button, Frame, NumExt, Sense, ThemePreference, Vec2};
use egui_i18n::tr;
use egui_mobius::{Slot, Value};
use egui_mobius::types::{Enqueue, ValueGuard};
use tracing::trace;
use crate::config::Config;
use crate::net::ergot_task;
use crate::task;
use crate::runtime::tokio_runtime::TokioRuntime;
use crate::ui_commands::{handle_command, UiCommand};

const MIN_TOUCH_SIZE: Vec2 = Vec2::splat(24.0);

static TOGGLE_DEFINITIONS: [ToggleDefinition; 3] = [
    ToggleDefinition { name: "status" },
    ToggleDefinition { name: "plot" },
    ToggleDefinition { name: "settings" },
];

pub struct AppState {
    pub(crate) command_sender: Enqueue<UiCommand>,
    pub(crate) left_toggles: Vec<&'static ToggleDefinition>,
    pub(crate) toggle_states: Vec<ToggleState>,
}

impl AppState {
    pub fn init(sender: Enqueue<UiCommand>) -> Self {

        let left_toggles = vec![
            &TOGGLE_DEFINITIONS[0],
            &TOGGLE_DEFINITIONS[1],
            &TOGGLE_DEFINITIONS[2],
        ];

        let mut toggle_states = vec![
            ToggleState { name: "status", mode: ViewMode::Window, },
            ToggleState { name: "plot", mode: ViewMode::Disabled, },
            ToggleState { name: "settings", mode: ViewMode::Disabled, },
        ];

        Self {
            command_sender: sender,
            left_toggles,
            toggle_states,
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct OperatorUiApp {
    config: Value<Config>,

    #[serde(skip)]
    state: Option<Value<AppState>>,

    // The command slot for handling UI commands
    #[serde(skip)]
    slot: Slot<UiCommand>,
}

impl Default for OperatorUiApp {
    fn default() -> Self {
        let (_signal, slot) = egui_mobius::factory::create_signal_slot::<UiCommand>();
        Self {
            config: Default::default(),
            state: None,
            slot,
        }
    }
}


impl OperatorUiApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut instance: OperatorUiApp = if let Some(storage) = cc.storage {
            eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default()
        } else {
            Default::default()
        };

        {
            let config = instance.config.lock().unwrap();
            egui_i18n::set_language(&config.language_identifier);

            // Safety: now safe to use i18n translation system (e.g. [`egui_i18n::tr!`])
        }

        let (app_signal, mut app_slot) = egui_mobius::factory::create_signal_slot::<UiCommand>();

        let app_message_sender = app_signal.sender.clone();

        let state = Value::new(AppState::init(app_message_sender.clone()));

        instance.state = Some(state.clone());
        // Safety: `Self::state()` is now safe to call.

        let runtime = TokioRuntime::new();
        let spawner = runtime.runtime().handle().clone();

        // Define a handler function for the slot
        let handler = {
            let config = instance.config.clone();
            let context = cc.egui_ctx.clone();
            let app_message_sender = app_message_sender.clone();

            move |command: UiCommand| {
                let task = handle_command(
                    command,
                    state.clone(),
                    config.clone(),
                    context.clone(),
                );

                if let Some(mut stream) = task::into_stream(task) {
                    runtime.runtime().spawn({
                        let app_message_sender = app_message_sender.clone();
                        async move {
                            trace!("running stream future");
                            while let Some(command) = stream.next().await {
                                trace!("command returned from future: {:?}", command);
                                app_message_sender
                                    .send(command)
                                    .expect("sent");
                            }
                        }
                    });
                }
            }
        };

        spawner.spawn(ergot_task(spawner.clone(), instance.state.clone()));

        // Start the slot with the handler
        app_slot.start(handler);


        instance
    }

    /// provide mutable access to the state.
    fn app_state(&mut self) -> ValueGuard<'_, AppState> {
        // Safety: it's always safe to unwrap here, because `new` sets the value
        self.state
            .as_mut()
            .unwrap()
            .lock()
            .unwrap()
    }
}

impl eframe::App for OperatorUiApp {
    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {

            egui::MenuBar::new().ui(ui, |ui| {
                egui::Sides::new().show(
                    ui,
                    |ui| {
                        // NOTE: no File->Quit on web pages!
                        let is_web = cfg!(target_arch = "wasm32");
                        if !is_web {
                            ui.menu_button(tr!("menu-top-level-file"), |ui| {
                                if ui
                                    .button(tr!("menu-item-quit"))
                                    .clicked()
                                {
                                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                }
                            });
                            ui.add_space(16.0);
                        }
                    },
                    |ui| {
                        let theme_preference = ctx.options(|opt| opt.theme_preference);

                        egui::ComboBox::from_id_salt(ui.id().with("theme"))
                            .selected_text({
                                match theme_preference {
                                    ThemePreference::Dark => tr!("theme-button-dark"),
                                    ThemePreference::Light => tr!("theme-button-light"),
                                    ThemePreference::System => tr!("theme-button-system"),
                                }
                            })
                            .show_ui(ui, |ui| {
                                let sender = self.app_state().command_sender.clone();

                                if ui
                                    .add(egui::Button::selectable(
                                        theme_preference.eq(&ThemePreference::Dark),
                                        tr!("theme-button-dark"),
                                    ))
                                    .clicked()
                                {
                                    sender
                                        .send(UiCommand::ThemeChanged(ThemePreference::Dark))
                                        .expect("sent");
                                }
                                if ui
                                    .add(egui::Button::selectable(
                                        theme_preference.eq(&ThemePreference::Light),
                                        tr!("theme-button-light"),
                                    ))
                                    .clicked()
                                {
                                    sender
                                        .send(UiCommand::ThemeChanged(ThemePreference::Light))
                                        .expect("sent");
                                }
                                if ui
                                    .add(egui::Button::selectable(
                                        theme_preference.eq(&ThemePreference::System),
                                        tr!("theme-button-system"),
                                    ))
                                    .clicked()
                                {
                                    sender
                                        .send(UiCommand::ThemeChanged(ThemePreference::System))
                                        .expect("sent");
                                }
                            });

                        let language = egui_i18n::get_language();
                        fn format_language_key(language_identifier: &String) -> String {
                            format!("language-{}", &language_identifier).to_string()
                        }

                        egui::ComboBox::from_id_salt(ui.id().with("language"))
                            .selected_text(tr!(&format_language_key(&language)))
                            .show_ui(ui, |ui| {
                                for other_language in egui_i18n::languages() {
                                    let sender = self.app_state().command_sender.clone();
                                    if ui
                                        .add(egui::Button::selectable(
                                            other_language.eq(&language),
                                            tr!(&format_language_key(&other_language)),
                                        ))
                                        .clicked()
                                    {
                                        sender
                                            .send(UiCommand::LangageChanged(other_language.clone()))
                                            .expect("sent");
                                    }
                                }
                            });
                    },
                );
            });
        });

        let mut state = self.app_state();

        egui::SidePanel::left("left_panel")
            .min_width(MIN_TOUCH_SIZE.x)
            .max_width(200.0)
            .resizable(true)
            .frame(Frame::NONE)
            .show(ctx, |ui| {
                let left_panel_width = ui.available_size_before_wrap().x;
                egui::ScrollArea::both()
                    .auto_shrink([false, false])
                    .min_scrolled_width(MIN_TOUCH_SIZE.x)
                    .show(ui, |ui| {
                    for toggle in state.left_toggles.iter() {
                        let toggle_state = state.toggle_states.iter().find(|candidate| candidate.name == toggle.name).unwrap();

                        let enabled = toggle_state.is_enabled();

                        let response = ui.horizontal(|ui| {
                            ui.set_width(left_panel_width);
                            ui.set_height(MIN_TOUCH_SIZE.y);

                            let button_width = left_panel_width
                                .at_least(MIN_TOUCH_SIZE.x)
                                .at_most(MIN_TOUCH_SIZE.x * 2.0);
                            ui.add_sized(Vec2::new(button_width, MIN_TOUCH_SIZE.y), egui::Label::new(tr!(&format!("panel-{}-icon", toggle.name)))
                                .selectable(false));

                            if left_panel_width > MIN_TOUCH_SIZE.x * 2.0 {
                                ui.add(egui::Label::new(tr!(&format!("panel-{}-name", toggle.name))).selectable(false));
                            }

                        }).response;
                        if response.interact(Sense::click()).clicked() {
                            let mode = if enabled { ViewMode::Disabled } else { ViewMode::Window };
                            state.command_sender.send(UiCommand::SetPanelMode(toggle.name.to_string(), mode)).expect("sent");
                        }
                    }
                });
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            let home_panel_enabled = state.toggle_states.iter().any(|candidate|candidate.name == "status" && candidate.is_windowed());
            if home_panel_enabled {
                egui::Window::new(tr!("panel-status-window-title"))
                    .resizable(true)
                    .show(ctx, |ui|{
                    ui.label("Status content");
                    });
            }

            let plot_panel_enabled = state.toggle_states.iter().any(|candidate|candidate.name == "plot" && candidate.is_windowed());
            if plot_panel_enabled {
                egui::Window::new(tr!("panel-plot-window-title"))
                    .resizable(true)
                    .show(ctx, |ui|{
                        ui.label("Plot content");
                    });
            }

            let settings_panel_enabled = state.toggle_states.iter().any(|candidate|candidate.name == "settings" && candidate.is_windowed());
            if settings_panel_enabled {
                egui::Window::new(tr!("panel-settings-window-title"))
                    .resizable(true)
                    .show(ctx, |ui|{
                        ui.label("Settings content");
                    });
            }
        });
    }
}

pub struct ToggleDefinition {
    name: &'static str,
}

pub struct ToggleState {
    pub(crate) name: &'static str,
    pub(crate) mode: ViewMode,
}

impl ToggleState {
    pub fn is_windowed(&self) -> bool {
        self.mode == ViewMode::Window
    }

    pub fn is_enabled(&self) -> bool {
        self.mode != ViewMode::Disabled
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewMode {
    Disabled,
    //Tile,
    Window,
    //Fullscreen,
    //ViewPort,
}
