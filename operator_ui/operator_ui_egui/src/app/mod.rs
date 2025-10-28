use std::sync::mpsc::Sender;
use ui::status::StatusUi;
use ui::plot::PlotUi;
use ui::settings::SettingsUi;
use async_std::prelude::StreamExt;
use egui::{Color32, CornerRadius, Frame, Image, NumExt, Sense, ThemePreference, Ui, Vec2, WidgetText};
use egui_extras::install_image_loaders;
use egui_i18n::tr;
use egui_mobius::{Slot, Value};
use egui_mobius::types::{Enqueue, ValueGuard};
use egui_tiles::{ContainerKind, SimplificationOptions, Tabs, Tile, TileId, Tiles, Tree, UiResponse};
use tracing::{debug, trace};
use ui::camera::CameraUi;
use ui::controls::ControlsUi;
use ui::diagnostics::DiagnosticsUi;
use crate::app::ui::egui_tree::{add_pane_to_root, dump_tiles};
use crate::config::Config;
use crate::net::ergot_task;
use crate::{task, LOGO};
use crate::app::ui::egui::bring_window_to_front;
use crate::runtime::tokio_runtime::TokioRuntime;
use crate::ui_commands::{handle_command, UiCommand};

mod ui;

const MIN_TOUCH_SIZE: Vec2 = Vec2::splat(24.0);

static TOGGLE_DEFINITIONS: [ToggleDefinition; 6] = [
    ToggleDefinition { name: "camera", kind: PaneKind::Camera },
    ToggleDefinition { name: "controls", kind: PaneKind::Controls },
    ToggleDefinition { name: "diagnostics", kind: PaneKind::Diagnostics },
    ToggleDefinition { name: "plot", kind: PaneKind::Plot },
    ToggleDefinition { name: "settings", kind: PaneKind::Settings },
    ToggleDefinition { name: "status", kind: PaneKind::Status },
];

#[derive(serde::Deserialize, serde::Serialize, Clone)]
#[serde(default)]
pub struct Workspace {
    pub(crate) toggle_states: Vec<ToggleState>,
    pub(crate) left_toggles: Vec<PaneKind>,
    pub(crate) tree: Tree<PaneKind>,
}

pub struct AppState {
    pub(crate) command_sender: Enqueue<UiCommand>,
    pub(crate) tree_behavior: TreeBehavior,

    ui_state: Value<UiState>,
}

impl Workspace {
    pub(crate) fn update_tree(&mut self) {
        if self.tree.is_empty() {
            self.tree = Self::create_tree();
        }

        for toggle_state in self.toggle_states.iter() {
            if !matches!(toggle_state.mode, ViewMode::Tile) {
                continue;
            }

            // is there a tile for this one?
            let is_open = self.tree.tiles.iter().any(|(_tile_id, tile_kind)| {
                matches!(tile_kind, Tile::Pane(pane_kind) if *pane_kind == toggle_state.kind)
            });

            if !is_open {
                debug!("tree:");
                let root = self.tree.root();
                dump_tiles(&mut self.tree.tiles, root);

                add_pane_to_root(&mut self.tree, toggle_state.kind, ContainerKind::Tabs);
            }
        }

        // now deal with existing tiles that should be closed
        let tiles_to_close = self.tree.tiles.iter().filter_map(|(tile_id, tile)| {
            let should_close = self.toggle_states.iter().any(|candidate| {
                candidate.mode != ViewMode::Tile && matches!(tile, Tile::Pane(kind) if *kind == candidate.kind)
            });
            if should_close {
                Some(*tile_id)
            } else {
                None
            }
        }).collect::<Vec<_>>();

        for id in tiles_to_close.into_iter() {
            self.tree.remove_recursively(id);
        }
    }

    pub fn create_tree() -> Tree<PaneKind> {
        let mut tiles = egui_tiles::Tiles::default();

        let root_tabs = vec![];
        let root = tiles.insert_grid_tile(root_tabs);

        let tree = egui_tiles::Tree::new("tile_tree", root, tiles);

        tree
    }
}

impl Default for Workspace {
    fn default() -> Self {
        let left_toggles = TOGGLE_DEFINITIONS.iter().map(|candidate| candidate.kind).collect::<Vec<_>>();

        let toggle_states = vec![
            ToggleState { mode: ViewMode::Tile, kind: PaneKind::Camera },
            ToggleState { mode: ViewMode::Tile, kind: PaneKind::Controls },
            ToggleState { mode: ViewMode::Window, kind: PaneKind::Diagnostics },
            ToggleState { mode: ViewMode::Disabled, kind: PaneKind::Plot },
            ToggleState { mode: ViewMode::Window, kind: PaneKind::Settings },
            ToggleState { mode: ViewMode::Tile, kind: PaneKind::Status },
        ];

        let tree = Self::create_tree();

        Self {
            left_toggles,
            toggle_states,
            tree,
        }
    }
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
            tree_behavior: TreeBehavior::new(ui_state.clone(), sender),
            ui_state,
        }
    }

    /// provide mutable access to the ui state.
    fn ui_state(&mut self) -> ValueGuard<'_, UiState> {
        self.ui_state
            .lock()
            .unwrap()
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Workspaces {
    workspaces: Vec<Value<Workspace>>,
    active_workspace: usize,
}

impl Default for Workspaces {
    fn default() -> Self {
        Self {
            workspaces: vec![Value::new(Workspace::default())],
            active_workspace: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceError {
    InvalidWorkspaceIndex,
    AlreadyActive,
    CannotRemoveActiveWorkspace,
}

impl Workspaces {
    pub fn active(&mut self) -> ValueGuard<'_, Workspace> {
        self.workspaces[self.active_workspace].lock().unwrap()
    }

    pub fn active_index(&mut self) -> usize {
        self.active_workspace
    }

    pub fn set_active(&mut self, index: usize) -> Result<(), WorkspaceError> {
        if index >= self.workspaces.len() {
            return Err(WorkspaceError::InvalidWorkspaceIndex)
        }
        if index == self.active_workspace {
            return Err(WorkspaceError::AlreadyActive)
        }
        self.active_workspace = index;

        Ok(())
    }

    pub fn clone_active(&mut self) -> usize {

        let cloned = {
            let active = self.active();
            Value::new((*active).clone())
        };
        self.workspaces.push(cloned);
        self.workspaces.len() - 1
    }

    pub fn remove(&mut self, index: usize) -> Result<(), WorkspaceError>{
        if index >= self.workspaces.len() {
            return Err(WorkspaceError::InvalidWorkspaceIndex)
        }
        if index == self.active_workspace {
            return Err(WorkspaceError::CannotRemoveActiveWorkspace)
        }

        self.workspaces.remove(index);

        Ok(())
    }

    pub fn count(&self) -> usize {
        self.workspaces.len()
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct OperatorUiApp {
    config: Value<Config>,

    #[serde(skip)]
    state: Option<Value<AppState>>,

    workspaces: Value<Workspaces>,

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

            move |command: UiCommand| {
                let task = handle_command(
                    command,
                    state.clone(),
                    config.clone(),
                    workspaces.clone(),
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
    fn app_state<'a>(&'a mut self) -> ValueGuard<'a, AppState> {
        // Safety: it's always safe to unwrap here, because `new` sets the value
        self.state
            .as_mut()
            .unwrap()
            .lock()
            .unwrap()
    }
}
fn kind_key(kind: &PaneKind) -> &str {
    TOGGLE_DEFINITIONS.iter().find_map(|candidate| if candidate.kind == *kind { Some(candidate.name) } else { None }).unwrap()
}

fn title_key(kind_key: &str) -> String {
    format!("panel-{}-window-title", kind_key)
}


impl eframe::App for OperatorUiApp {
    /// Called by the framework to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut request_workspace_toggle = false;
        {
            let mut workspaces = self.workspaces.lock().unwrap();
            let mut workspace = workspaces.active();

            workspace.update_tree();
        }

        let sender = self.app_state().command_sender.clone();

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

        let panel_fill_color = ctx.style().visuals.panel_fill;
        let side_panel_fill_color = panel_fill_color.gamma_multiply(0.9);

        let mut request_make_visible: Option<ToggleState> = None;

        egui::SidePanel::left("left_panel")
            .min_width(MIN_TOUCH_SIZE.x * 2.0)
            .max_width(200.0)
            .resizable(true)
            .frame(Frame::NONE.fill(side_panel_fill_color))
            .show(ctx, |ui| {
                let left_panel_width = ui.available_size_before_wrap().x;
                ui.vertical(|ui| {
                    let mut workspaces = self.workspaces.lock().unwrap();
                    let mut workspace = workspaces.active();

                    egui::ScrollArea::both()
                        // FIXME the 4.0 is a guess at the height of a separator and margins and such
                        .max_height(ui.available_height() - ((MIN_TOUCH_SIZE.y * 2.0) + 2.0))
                        .auto_shrink([false, false])
                        .min_scrolled_width(MIN_TOUCH_SIZE.x)
                        .show(ui, |ui| {

                            for kind in workspace.left_toggles.iter() {
                                let toggle_definition = TOGGLE_DEFINITIONS.iter().find(|candidate| candidate.kind == *kind).unwrap();

                                let toggle_state = workspace.toggle_states.iter().find(|candidate| candidate.kind == *kind).unwrap();

                                let enabled = toggle_state.is_enabled();

                                let response = ui.horizontal(|ui| {
                                    ui.set_width(left_panel_width);
                                    ui.set_height(MIN_TOUCH_SIZE.y * 2.0);

                                    let visuals = ui.style().interact_selectable(&ui.response(), enabled);

                                    let bg_color = if enabled { visuals.bg_fill } else { visuals.weak_bg_fill };
                                    let mut rect = ui.max_rect();
                                    rect.max.x = left_panel_width;
                                    ui.painter().rect_filled(rect, CornerRadius::ZERO, bg_color);

                                    let button_width = left_panel_width
                                        .at_least(MIN_TOUCH_SIZE.x)
                                        .at_most(MIN_TOUCH_SIZE.x * 2.0);
                                    ui.add_sized(Vec2::new(button_width, ui.available_height()), egui::Label::new(tr!(&format!("panel-{}-icon", toggle_definition.name)))
                                        .selectable(false));

                                    if left_panel_width > MIN_TOUCH_SIZE.x * 2.0 {
                                        ui.add(egui::Label::new(tr!(&format!("panel-{}-name", toggle_definition.name))).selectable(false));
                                    }
                                }).response;

                                if response.interact(Sense::click()).clicked() {
                                    let default_mode = ViewMode::Window;

                                    match toggle_state.mode {
                                        ViewMode::Disabled => {
                                            // if it's not enabled, enable it
                                            sender.send(UiCommand::SetPanelMode(*kind, default_mode)).expect("sent");
                                        }

                                        // otherwise, if it's not active, activate it
                                        ViewMode::Tile => {
                                            request_make_visible.replace(*toggle_state);
                                        }
                                        ViewMode::Window => {
                                            request_make_visible.replace(*toggle_state);
                                        }
                                    }
                                }
                            }
                        });

                    match request_make_visible {
                        Some(toggle_state) if toggle_state.mode == ViewMode::Tile => {
                            let tile_id = workspace.tree.tiles.find_pane( &toggle_state.kind).unwrap();
                            workspace.tree.make_active( |candidate_id, _tile | candidate_id == tile_id);
                        }
                        _ => {}
                    }

                    egui::Frame::new()
                        .outer_margin(0.0)
                        .fill(Color32::WHITE)
                        .show(ui, |ui| {
                            ui.set_width(left_panel_width);
                            ui.vertical_centered(|ui| {
                                // TODO use a smaller version of the logo, since egui image resizing isn't great
                                if ui.add_sized(
                                    MIN_TOUCH_SIZE * 2.0,
                                    egui::Button::new(Image::from_bytes("bytes://logo", &LOGO[..]))
                                        .frame(false)
                                ).clicked() {
                                    // TODO show an 'about' modal.

                                    request_workspace_toggle = true;
                                }
                            })
                    });
                });
            });

        egui::CentralPanel::default()
            .frame(Frame::NONE.fill(panel_fill_color))
            .show(ctx, |ui| {
            //
            // Tiles
            //

            let mut state = self.state.as_mut().unwrap().lock().unwrap();
            // reset the flag
            state.tree_behavior.container_is_tabs = false;


            let mut workspaces = self.workspaces.lock().unwrap();
            let mut workspace = workspaces.active();

            workspace.tree.ui(&mut state.tree_behavior, ui);
        });


        //
        // Windows
        //

        let windows = {
            let mut workspaces = self.workspaces.lock().unwrap();
            let workspace = workspaces.active();

            workspace.toggle_states.iter().filter(|candidate| candidate.is_windowed()).cloned().collect::<Vec<_>>()
        };

        for toggle_state in windows.iter() {
            let kind_key = kind_key(&toggle_state.kind);
            let title_key = title_key(kind_key);
            let title = tr!(&title_key);

            let mut open = true;

            let window = egui::Window::new(&title)
                .title_bar(false)
                .open(&mut open)
                .resizable(true)
                .show(ctx, |ui| {
                    ui.vertical(|ui| {
                        if false {
                            trace!("window, layer_id: {:?}, toggle_state: {:?}", ui.layer_id(), toggle_state);
                        }

                        let kind = toggle_state.kind;
                        let mut app_state = self.app_state();
                        let mut ui_state = app_state.ui_state();

                        let mut dragged = false;
                        show_panel_title_and_controls(&kind, title, sender.clone(), ui, false, false, true, &mut dragged);
                        ui.separator();
                        show_panel_content(&kind, ui, &mut ui_state);
                    });

                });

            if let Some(window) = window {
                match request_make_visible {
                    Some(requested_toggle_state) if requested_toggle_state.mode == ViewMode::Window && requested_toggle_state.kind == toggle_state.kind => {
                        trace!("bringing window to front. layer_id: {:?}, toggle_state: {:?}", window.response.layer_id, toggle_state);
                        bring_window_to_front(ctx, window.response.layer_id);
                    }
                    _ => {}
                }
            }

            if open == false {
                self.app_state().command_sender.send(UiCommand::SetPanelMode(toggle_state.kind, ViewMode::Disabled)).expect("sent");
            }
        }

        if request_workspace_toggle {
            let mut workspaces = self.workspaces.lock().unwrap();

            if workspaces.count() == 1 {
                workspaces.clone_active();
            }
            match workspaces.active_index() {
                0 => workspaces.set_active(1).expect("set active"),
                1 => workspaces.set_active(0).expect("set active"),
                _ => unreachable!(),
            };
        }
    }
}

pub struct ToggleDefinition {
    name: &'static str,
    kind: PaneKind,
}

#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Copy, Clone, Debug)]
pub struct ToggleState {
    pub(crate) kind: PaneKind,
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

#[derive(serde::Deserialize, serde::Serialize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewMode {
    Disabled,
    Tile,
    Window,
    //Fullscreen,
    //ViewPort,
}

pub(crate) struct TreeBehavior {
    simplification_options: egui_tiles::SimplificationOptions,
    ui_state: Value<UiState>,
    command_sender: Enqueue<UiCommand>,
    drag: Option<TileId>,
    container_is_tabs: bool,
}

impl TreeBehavior {
    fn new(ui_state: Value<UiState>, command_sender: Enqueue<UiCommand>) -> Self {
        Self {
            simplification_options: SimplificationOptions {
                all_panes_must_have_tabs: true,
                ..SimplificationOptions::default()
            },
            ui_state,
            command_sender,
            drag: None,
            container_is_tabs: false,
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

impl egui_tiles::Behavior<PaneKind> for TreeBehavior {
    fn pane_ui(&mut self, ui: &mut Ui, tile_id: TileId, kind: &mut PaneKind) -> UiResponse {
        let in_tab = self.container_is_tabs;

        let mut ui_state = self.ui_state.lock().unwrap();

        let kind_key = kind_key(&kind);
        let title_key = title_key(kind_key);
        let title = tr!(&title_key);

        let mut dragged = false;


        if !in_tab {
            show_panel_title_and_controls(&kind, title, self.command_sender.clone(), ui, true, true, false, &mut dragged);
            ui.separator();
        }

        show_panel_content(&kind, ui, &mut ui_state);

        dragged |= self.container_is_tabs && matches!(self.drag, Some(drag_id) if drag_id == tile_id);

        // reset for the next container
        self.container_is_tabs = false;

        if dragged {
            println!("drag started: {:?}", tile_id);

            self.drag = None;
            UiResponse::DragStarted
        } else {
            UiResponse::None
        }
    }

    fn tab_title_for_pane(&mut self, pane: &PaneKind) -> WidgetText {
        let kind_key = kind_key(pane);

        let title_key = format!("panel-{}-window-title", kind_key);

        tr!(&title_key).into()
    }

    fn is_tab_closable(&self, _tiles: &Tiles<PaneKind>, _tile_id: TileId) -> bool {
        // We use the X buttons in the tab title bar area instead. This is for a few of reasons:
        // 1. less space consumed when multiple tabs.
        // 2. less visual clutter
        // 3. avoids the left/right arrows for longer when tabs don't fit
        // 4. when tabs don't fit, and arrows appear, when you scroll right and the arrow
        //    under the cursor disappears the 'X' button would be under the cursor and
        //    is easily accidentally clicked when repeated clicking to scroll to the right.
        false
    }

    fn on_tab_close(&mut self, _tiles: &mut Tiles<PaneKind>, _tile_id: TileId) -> bool {

        if let Some(Tile::Pane(kind)) = _tiles.get(_tile_id) {
            self.command_sender.send(UiCommand::ClosePanel(*kind)).expect("sent");
        }
        // always deny, manually handle closing ourselves
        false
    }

    fn top_bar_right_ui(&mut self, _tiles: &Tiles<PaneKind>, _ui: &mut Ui, _tile_id: TileId, _tabs: &Tabs, _scroll_offset: &mut f32) {
        if let Some(tile_id) = _tabs.active {

            if let Some(Tile::Pane(kind)) = _tiles.get(tile_id) {
                let mut dragged = false;
                show_panel_controls(&kind, self.command_sender.clone(), _ui, true, true, false, &mut dragged);

                if dragged{
                    println!("set dragging, from: {:?}", _tile_id);
                    self.drag = Some(tile_id);
                }
            }
        }
        self.container_is_tabs = true;
    }

    fn simplification_options(&self) -> egui_tiles::SimplificationOptions {
        self.simplification_options
    }
}


fn show_panel_title_and_controls(kind: &PaneKind, title: String, sender: Sender<UiCommand>, ui: &mut Ui, show_drag_handle: bool, show_make_window: bool, show_make_tile: bool, dragged: &mut bool) {
    egui::Sides::new().show(ui, |ui| {
        ui.add(egui::Label::new(title).selectable(false));
    }, |ui| {
        show_panel_controls(kind, sender, ui, show_drag_handle, show_make_window, show_make_tile, dragged);
    });
}

fn show_panel_controls(kind: &PaneKind, sender: Sender<UiCommand>, ui: &mut Ui, show_drag_handle: bool, show_make_window: bool, show_make_tile: bool, dragged: &mut bool) {
    let button_width = MIN_TOUCH_SIZE.x;

    let desired = ui
        .fonts_mut(|f| {
            f.layout_no_wrap(
                "🗙🗖🗕✋".to_string(),
                egui::TextStyle::Button.resolve(ui.style()),
                egui::Color32::WHITE,
            )
        })
        .size();

    let button_size = Vec2::new(button_width, ui.spacing().button_padding.y + desired.y + 2.0);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        ui.allocate_space(Vec2::new(0.0, button_size.y));
        if ui.add_sized(button_size, egui::Button::new("🗙")).clicked() {
            sender.send(UiCommand::SetPanelMode(*kind, ViewMode::Disabled)).expect("sent");
        }
        if show_make_tile {
            if ui.add_sized(button_size, egui::Button::new("🗕")).clicked() {
                sender.send(UiCommand::SetPanelMode(*kind, ViewMode::Tile)).expect("sent");
            }
        }
        if show_make_window {
            if ui.add_sized(button_size, egui::Button::new("🗖")).clicked() {
                sender.send(UiCommand::SetPanelMode(*kind, ViewMode::Window)).expect("sent");
            }
        }
        if show_drag_handle {
            if ui.add_sized(button_size, egui::Button::new("✋")
                .sense(Sense::click_and_drag())
            )
                .on_hover_cursor(egui::CursorIcon::Grab)
                .dragged() {
                *dragged = true;
            }
        }

    });
}

fn show_panel_content(kind: &PaneKind, ui: &mut Ui, ui_state: &mut UiState) {
    match kind {
        PaneKind::Camera => ui_state.camera_ui.ui(ui),
        PaneKind::Controls => ui_state.controls_ui.ui(ui),
        PaneKind::Diagnostics => ui_state.diagnostics_ui.ui(ui),
        PaneKind::Plot => ui_state.plot_ui.ui(ui),
        PaneKind::Settings => ui_state.settings_ui.ui(ui),
        PaneKind::Status => ui_state.status_ui.ui(ui),
    }
}