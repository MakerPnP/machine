use std::collections::BTreeMap;
use std::time::Duration;

use async_std::prelude::StreamExt;
use egui::{Context, Ui, Vec2, ViewportBuilder, ViewportClass, ViewportCommand, ViewportId};
use egui_extras::install_image_loaders;
use egui_i18n::tr;
use egui_mobius::types::{Enqueue, ValueGuard};
use egui_mobius::{Slot, Value};
use ergot::Address;
use ergot::toolkits::tokio_udp::EdgeStack;
use operator_shared::camera::CameraIdentifier;
use tokio::runtime::Handle;
use tokio::sync::{broadcast, watch};
use tracing::{info, trace, warn};
use ui::camera::CameraUi;
use ui::controls::ControlsUi;
use ui::diagnostics::DiagnosticsUi;
use ui::plot::PlotUi;
use ui::settings::SettingsUi;
use ui::status::StatusUi;

use crate::config::Config;
use crate::events::AppEvent;
use crate::net::camera::{CameraFrame, camera_frame_listener};
use crate::net::ergot_task;
use crate::runtime::tokio_runtime::TokioRuntime;
use crate::ui_commands::{UiCommand, handle_command};
use crate::workspace::{ViewportState, Workspaces};
use crate::{TARGET_FPS, task};

mod ui;

pub const MIN_TOUCH_SIZE: Vec2 = Vec2::splat(24.0);

pub struct AppState {
    pub(crate) command_sender: Enqueue<UiCommand>,
    pub(crate) context: egui::Context,
    ui_state: Value<UiState>,
}

pub struct UiState {
    pub(crate) camera_uis: BTreeMap<CameraIdentifier, CameraUi>,

    pub(crate) controls_ui: ControlsUi,
    pub(crate) diagnostics_ui: DiagnosticsUi,
    pub(crate) plot_ui: PlotUi,
    pub(crate) settings_ui: SettingsUi,
    pub(crate) status_ui: StatusUi,
}

impl AppState {
    pub fn init(sender: Enqueue<UiCommand>, context: Context) -> Self {
        let ui_state = UiState {
            camera_uis: BTreeMap::new(),
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
            context,
        }
    }

    /// provide mutable access to the ui state.
    fn ui_state(&mut self) -> ValueGuard<'_, UiState> {
        self.ui_state.lock().unwrap()
    }

    pub fn add_camera(
        &self,
        camera_identifier: CameraIdentifier,
        stack: EdgeStack,
        command_endpoint_remote_address: Address,
        target_fps: f32,
    ) {
        let shutdown_token = tokio_util::sync::CancellationToken::new();
        let (camera_tx, camera_rx) = watch::channel::<CameraFrame>(CameraFrame::default());

        let camera_frame_listener_handle = {
            let context = self.context.clone();
            tokio::task::spawn(camera_frame_listener(
                stack,
                camera_tx,
                context,
                command_endpoint_remote_address,
                shutdown_token.clone(),
                camera_identifier.clone(),
                target_fps,
            ))
        };

        info!("Started camera frame listener.  id: {}", camera_identifier);

        let camera_ui = CameraUi::new(camera_rx, camera_frame_listener_handle, shutdown_token);

        let mut ui_state = self.ui_state.lock().unwrap();
        let result = ui_state
            .camera_uis
            .insert(camera_identifier, camera_ui);
        assert!(result.is_none(), "Camera id already exists");
    }

    pub(crate) fn prepare_stop_all_cameras(&self) -> BTreeMap<CameraIdentifier, CameraUi> {
        let mut ui_state = self.ui_state.lock().unwrap();
        let camera_uis = std::mem::take(&mut ui_state.camera_uis);
        camera_uis
    }

    pub(crate) async fn stop_all_cameras(camera_uis: BTreeMap<CameraIdentifier, CameraUi>) {
        for (camera_identifier, camera_ui) in camera_uis.into_iter() {
            info!("Stopping camera UI.  id: {}", camera_identifier);
            camera_ui.shutdown().await;
        }
        info!("All camera frame listeners finished");
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct OperatorUiApp {
    #[serde(skip)]
    shutdown_state: ShutdownState,

    config: Value<Config>,

    #[serde(skip)]
    state: Option<Value<AppState>>,

    workspaces: Value<Workspaces>,

    #[serde(skip)]
    viewports: Value<Vec<Value<ViewportState>>>,

    // The command slot for handling UI commands
    #[serde(skip)]
    slot: Slot<UiCommand>,

    #[serde(skip)]
    app_event_broadcast: Option<(broadcast::Sender<AppEvent>, broadcast::Receiver<AppEvent>)>,
    #[serde(skip)]
    networking_handle: Option<tokio::task::JoinHandle<()>>,

    #[serde(skip)]
    spawner: Option<Handle>,
    #[serde(skip)]
    runtime: Option<TokioRuntime>,
}

#[derive(Clone, Copy, Debug)]
enum ShutdownState {
    NotStarted,
    ShutdownRequested,
    ShutdownComplete,
}

impl Default for OperatorUiApp {
    fn default() -> Self {
        let (_signal, slot) = egui_mobius::factory::create_signal_slot::<UiCommand>();

        Self {
            shutdown_state: ShutdownState::NotStarted,
            config: Default::default(),
            state: None,
            workspaces: Value::new(Workspaces::default()),
            viewports: Default::default(),
            slot,
            app_event_broadcast: None,
            networking_handle: None,
            spawner: None,
            runtime: None,
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

        // Create event channel
        let (app_event_tx, app_event_rx) = broadcast::channel::<AppEvent>(16);

        ctrlc::set_handler({
            let app_event_tx = app_event_tx.clone();
            move || {
                warn!("Ctrl+C received, shutting down.");
                let _ = app_event_tx.send(AppEvent::Shutdown);
            }
        })
        .expect("Error setting Ctrl+C handler");

        instance.app_event_broadcast = Some((app_event_tx, app_event_rx));

        let (app_signal, mut app_slot) = egui_mobius::factory::create_signal_slot::<UiCommand>();

        let app_message_sender = app_signal.sender.clone();

        let app_state = AppState::init(app_message_sender.clone(), cc.egui_ctx.clone());

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
        let tokio_runtime = runtime.runtime();

        let spawner = tokio_runtime.handle().clone();
        instance.spawner = Some(spawner.clone());

        // Define a handler function for the slot
        let handler = {
            let config = instance.config.clone();
            let context = cc.egui_ctx.clone();
            let app_message_sender = app_message_sender.clone();
            let workspaces = instance.workspaces.clone();
            let viewports = instance.viewports.clone();
            let spawner = spawner.clone();

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
                    spawner.spawn({
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

        // Start the slot with the handler
        app_slot.start(handler);

        // Start networking
        let networking_handle = spawner.spawn({
            let state = instance.state.as_mut().unwrap().clone();
            let workspaces = instance.workspaces.clone();
            let app_event_tx = instance
                .app_event_broadcast
                .as_ref()
                .unwrap()
                .0
                .clone();

            async move {
                let _ = ergot_task(state, workspaces, app_event_tx).await;
                info!("Network task finished");
            }
        });

        instance.networking_handle = Some(networking_handle);
        instance.runtime = Some(runtime);

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

    // FIXME this only appears to show a model on one or the other viewport.
    fn show_shutdown_modal(shutdown_state: ShutdownState, ctx: &Context, viewport_id: ViewportId) {
        if !matches!(shutdown_state, ShutdownState::NotStarted) {
            egui::modal::Modal::new(egui::Id::new("shutdown-modal").with(viewport_id)).show(ctx, |ui| {
                // TODO translate
                ui.heading("Shutting down...");
            });
        }
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
            let (viewport_id, viewport_position, viewport_inner_size) = {
                let viewport = viewport.lock().unwrap();

                let mut workspaces = viewport.workspaces.lock().unwrap();
                let workspace = workspaces.active();
                let viewport_config = &workspace.viewport_configs[&viewport.id];
                (viewport.id, viewport_config.position, viewport_config.inner_size)
            };

            if viewport_id == ViewportId::ROOT {
                let mut viewport = viewport.lock().unwrap();
                viewport.ui(ctx);
                //Self::show_shutdown_modal(self.shutdown_state, ctx, viewport_id);
            } else {
                let unformatted_viewport_id = format!("{:?}", viewport_id);
                let formatted_viewport_id = unformatted_viewport_id.trim_matches('\"');
                let mut viewport_builder =
                    ViewportBuilder::default().with_title(tr!("viewport-title", {id: formatted_viewport_id}));
                if let Some(position) = viewport_position {
                    viewport_builder = viewport_builder.with_position(position)
                }
                if let Some(inner_size) = viewport_inner_size {
                    viewport_builder = viewport_builder.with_inner_size(inner_size);
                }

                ctx.show_viewport_deferred(viewport_id, viewport_builder, {
                    let viewport = viewport.clone();
                    //let viewport_id = viewport_id;
                    //let shutdown_state = self.shutdown_state.clone();

                    move |ctx, viewport_class| {
                        if !matches!(viewport_class, ViewportClass::Deferred) {
                            // TODO support for other viewports when deferred are not available?
                            return;
                        }

                        let mut viewport = viewport.lock().unwrap();

                        viewport.ui(ctx);

                        //Self::show_shutdown_modal(shutdown_state, ctx, viewport_id);

                        let mut workspaces = viewport.workspaces.lock().unwrap();
                        let mut workspace = workspaces.active();
                        workspace
                            .viewport_configs
                            .get_mut(&viewport_id)
                            .unwrap()
                            .update_size_and_position(&ctx);
                    }
                });
            }
        }

        Self::show_shutdown_modal(self.shutdown_state, ctx, ViewportId::ROOT);

        if let Some((_, app_event_rx)) = self.app_event_broadcast.as_mut() {
            if let Ok(event) = app_event_rx.try_recv() {
                match event {
                    AppEvent::Shutdown => {
                        if matches!(self.shutdown_state, ShutdownState::NotStarted) {
                            info!("GUI received shutdown event, starting shutdown");
                            self.shutdown_state = ShutdownState::ShutdownRequested;
                        }
                    }
                }
            }
        }

        if matches!(self.shutdown_state, ShutdownState::ShutdownRequested) {
            let is_done = || {
                // we need to keep broadcasting shutdown events until the networking task has finished.
                // the networking task may not have completed it's own startup
                let _ = self
                    .app_event_broadcast
                    .as_ref()
                    .unwrap()
                    .0
                    .send(AppEvent::Shutdown)
                    .unwrap();

                let Some(networking_handle) = &self.networking_handle else {
                    return true;
                };

                networking_handle.is_finished()
            };

            if is_done() {
                info!("All tasks finished, shutting down");
                ctx.send_viewport_cmd(ViewportCommand::Close);
                self.shutdown_state = ShutdownState::ShutdownComplete;
            } else {
                // force a re-check of the shutdown state
                ctx.request_repaint_after(Duration::from_millis(100));
            }
        }

        if ctx.input(|i| i.viewport().close_requested()) {
            info!("User requested shut down");
            match self.shutdown_state {
                ShutdownState::NotStarted => {
                    self.shutdown_state = ShutdownState::ShutdownRequested;
                    ctx.send_viewport_cmd(ViewportCommand::CancelClose);
                }
                ShutdownState::ShutdownRequested => {
                    // not finished yet
                    ctx.send_viewport_cmd(ViewportCommand::CancelClose);
                }
                ShutdownState::ShutdownComplete => {
                    // allow the close
                }
            }
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if let Some(networking_handle) = &self.networking_handle {
            assert!(networking_handle.is_finished(), "Network task not finished");
        }
        info!("GUI shutdown complete");
    }
}

#[derive(serde::Deserialize, serde::Serialize, PartialEq, Eq, Debug, Clone, Copy, Hash)]
pub enum PaneKind {
    Camera { id: CameraIdentifier },
    Controls,
    Diagnostics,
    Plot,
    Settings,
    Status,
}

pub(crate) fn show_panel_content(kind: &PaneKind, ui: &mut Ui, ui_state: &mut UiState) {
    match kind {
        PaneKind::Camera {
            id,
        } => {
            if let Some(camera_ui) = ui_state.camera_uis.get_mut(id) {
                camera_ui.ui(ui);
            } else {
                ui.spinner();
            }
        }
        PaneKind::Controls => ui_state.controls_ui.ui(ui),
        PaneKind::Diagnostics => ui_state.diagnostics_ui.ui(ui),
        PaneKind::Plot => ui_state.plot_ui.ui(ui),
        PaneKind::Settings => ui_state.settings_ui.ui(ui),
        PaneKind::Status => ui_state.status_ui.ui(ui),
    }
}
