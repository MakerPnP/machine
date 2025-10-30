use async_std::prelude::StreamExt;
use egui::{Ui, Vec2, ViewportBuilder, ViewportClass, ViewportId};
use egui_extras::install_image_loaders;
use egui_mobius::types::{Enqueue, ValueGuard};
use egui_mobius::{Slot, Value};
use tracing::trace;
use ui::camera::CameraUi;
use ui::controls::ControlsUi;
use ui::diagnostics::DiagnosticsUi;
use ui::plot::PlotUi;
use ui::settings::SettingsUi;
use ui::status::StatusUi;

use crate::config::Config;
use crate::net::ergot_task;
use crate::runtime::tokio_runtime::TokioRuntime;
use crate::task;
use crate::ui_commands::{UiCommand, handle_command};
use crate::workspace::{ToggleDefinition, ViewportState, Workspaces};

mod ui;

pub const MIN_TOUCH_SIZE: Vec2 = Vec2::splat(24.0);

pub static TOGGLE_DEFINITIONS: [ToggleDefinition; 6] = [
    ToggleDefinition {
        key: "camera",
        kind: PaneKind::Camera,
    },
    ToggleDefinition {
        key: "controls",
        kind: PaneKind::Controls,
    },
    ToggleDefinition {
        key: "diagnostics",
        kind: PaneKind::Diagnostics,
    },
    ToggleDefinition {
        key: "plot",
        kind: PaneKind::Plot,
    },
    ToggleDefinition {
        key: "settings",
        kind: PaneKind::Settings,
    },
    ToggleDefinition {
        key: "status",
        kind: PaneKind::Status,
    },
];

pub struct AppState {
    pub(crate) command_sender: Enqueue<UiCommand>,
    ui_state: Value<UiState>,
}

pub struct UiState {
    pub(crate) camera_ui: CameraUi,
    pub(crate) controls_ui: ControlsUi,
    pub(crate) diagnostics_ui: DiagnosticsUi,
    pub(crate) plot_ui: PlotUi,
    pub(crate) settings_ui: SettingsUi,
    pub(crate) status_ui: StatusUi,
}

impl AppState {
    pub fn init(sender: Enqueue<UiCommand>) -> Self {
        let ui_state = UiState {
            camera_ui: CameraUi::default(),
            controls_ui: ControlsUi::default(),
            diagnostics_ui: DiagnosticsUi::default(),
            plot_ui: PlotUi::default(),
            settings_ui: SettingsUi::default(),
            status_ui: StatusUi::default(),
        };

        let ui_state = Value::new(ui_state);

        Self {
            command_sender: sender.clone(),
            ui_state,
        }
    }

    /// provide mutable access to the ui state.
    fn ui_state(&mut self) -> ValueGuard<'_, UiState> {
        self.ui_state.lock().unwrap()
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct OperatorUiApp {
    config: Value<Config>,

    #[serde(skip)]
    state: Option<Value<AppState>>,

    workspaces: Value<Workspaces>,

    #[serde(skip)]
    viewports: Value<Vec<Value<ViewportState>>>,

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
            workspaces: Value::new(Workspaces::default()),
            viewports: Default::default(),
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

        install_image_loaders(&cc.egui_ctx);

        let (app_signal, mut app_slot) = egui_mobius::factory::create_signal_slot::<UiCommand>();

        let app_message_sender = app_signal.sender.clone();

        let app_state = AppState::init(app_message_sender.clone());

        {
            let mut viewports = instance.viewports.lock().unwrap();
            if viewports.is_empty() {
                let id = ViewportId::ROOT;
                let root_viewport = ViewportState::new(
                    id,
                    app_message_sender.clone(),
                    app_state.ui_state.clone(),
                    instance.workspaces.clone(),
                );
                viewports.push(Value::new(root_viewport));
            }

            {
                let id = ViewportId::from_hash_of("__test__");
                let viewport_state = ViewportState::new(
                    id,
                    app_message_sender.clone(),
                    app_state.ui_state.clone(),
                    instance.workspaces.clone(),
                );
                viewports.push(Value::new(viewport_state));
            }
        }

        let state = Value::new(app_state);

        instance.state = Some(state.clone());
        // Safety: `Self::state()` is now safe to call.

        let runtime = TokioRuntime::new();
        let spawner = runtime.runtime().handle().clone();

        // Define a handler function for the slot
        let handler = {
            let config = instance.config.clone();
            let context = cc.egui_ctx.clone();
            let app_message_sender = app_message_sender.clone();
            let workspaces = instance.workspaces.clone();
            let viewports = instance.viewports.clone();

            move |command: UiCommand| {
                let task = handle_command(
                    command,
                    state.clone(),
                    config.clone(),
                    workspaces.clone(),
                    viewports.clone(),
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

        {
            instance
                .viewports
                .lock()
                .unwrap()
                .iter_mut()
                .for_each(|viewport| {
                    viewport.lock().unwrap().init();
                });
        }

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
        let viewports = self.viewports.lock().unwrap();

        for viewport in viewports.iter() {
            let viewport_id = viewport.lock().unwrap().id;

            if viewport_id == ViewportId::ROOT {
                let mut viewport = viewport.lock().unwrap();
                viewport.ui(ctx);
            } else {
                ctx.show_viewport_deferred(viewport_id, ViewportBuilder::default(), {
                    let viewport = viewport.clone();

                    move |ctx, viewport_class| {
                        if !matches!(viewport_class, ViewportClass::Deferred) {
                            // TODO support for other viewports when deferred are not available?
                            return;
                        }

                        let mut viewport = viewport.lock().unwrap();

                        viewport.ui(ctx);
                    }
                });
            }
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Eq, Debug, Clone, Copy)]
pub enum PaneKind {
    Camera,
    Controls,
    Diagnostics,
    Plot,
    Settings,
    Status,
}

pub(crate) fn show_panel_content(kind: &PaneKind, ui: &mut Ui, ui_state: &mut UiState) {
    match kind {
        PaneKind::Camera => ui_state.camera_ui.ui(ui),
        PaneKind::Controls => ui_state.controls_ui.ui(ui),
        PaneKind::Diagnostics => ui_state.diagnostics_ui.ui(ui),
        PaneKind::Plot => ui_state.plot_ui.ui(ui),
        PaneKind::Settings => ui_state.settings_ui.ui(ui),
        PaneKind::Status => ui_state.status_ui.ui(ui),
    }
}
