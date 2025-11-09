use std::sync::mpsc::Sender;

use eframe::emath::{NumExt, Pos2, Vec2};
use eframe::epaint::ahash::HashMap;
use eframe::epaint::{Color32, CornerRadius};
use egui::{CollapsingHeader, Context, Frame, Id, Image, Rect, Sense, ThemePreference, Ui, ViewportId, WidgetText};
use egui_i18n::tr;
use egui_mobius::Value;
use egui_mobius::types::{Enqueue, ValueGuard};
use egui_tiles::{ContainerKind, SimplificationOptions, Tabs, Tile, TileId, Tiles, Tree, UiResponse};
use tracing::{debug, info, trace};

use crate::app::{MIN_TOUCH_SIZE, PaneKind, TOGGLE_DEFINITIONS, UiState};
use crate::fps_stats::egui::show_frame_durations;
use crate::fps_stats::{FpsSnapshot, FpsStats};
use crate::ui_commands::{UiCommand, ViewportUiAction, ViewportUiCommand};
use crate::ui_common::egui::bring_window_to_front;
use crate::ui_common::egui_tree::{add_pane_to_root, dump_tiles};
use crate::{LOGO, app};

#[derive(serde::Deserialize, serde::Serialize, Default, Clone)]
#[serde(default)]
pub struct ViewportConfig {
    pub(crate) position: Option<Pos2>,
    pub(crate) inner_size: Option<Vec2>,
}

impl ViewportConfig {
    pub fn update_size_and_position(&mut self, ctx: &Context) {
        let viewport_id = ctx.viewport_id();
        let viewport_frame_number = ctx.cumulative_frame_nr_for(viewport_id);

        let (new_position, new_inner_size) = {
            let maybe_position = ctx.input(|i| i.viewport().outer_rect.map(|it| it.min));
            (maybe_position, Some(ctx.content_rect().size()))
        };
        debug!(
            "viewport: {:?}, frame: {}, position: {:?}, size: {:?}",
            viewport_id, viewport_frame_number, new_position, new_inner_size
        );

        if new_position != self.position {
            debug!(
                "viewport: {:?}, position: [old: {:?}, new: {:?}]",
                viewport_id, self.position, new_position
            );
            self.position = new_position;
        }
        if new_inner_size != self.inner_size {
            debug!(
                "viewport: {:?}, inner_size: [old: {:?}, new: {:?}]",
                viewport_id, self.inner_size, new_inner_size
            );
            self.inner_size = new_inner_size;
        }
    }
}

/// Stores the tree of panes, and the state of each pane (position, size, etc)
///
/// Should be persisted between application restarts, one per viewport.
#[derive(serde::Deserialize, serde::Serialize, Clone)]
#[serde(default)]
pub struct ViewportTreeConfig {
    pub(crate) tree: Tree<PaneKind>,
}

impl Default for ViewportTreeConfig {
    fn default() -> Self {
        let tree = Self::create_tree();

        Self {
            tree,
        }
    }
}

impl ViewportTreeConfig {
    pub(crate) fn update_tree(&mut self, viewport_id: ViewportId, toggle_states: &[ToggleState]) {
        if self.tree.is_empty() {
            self.tree = Self::create_tree();
        }

        for toggle_state in toggle_states.iter() {
            if !matches!(toggle_state.mode, ViewMode::Tile(candidate_viewport_id) if candidate_viewport_id == viewport_id)
            {
                // not a tile, or not for this viewport
                continue;
            }

            // is there a tile for this one?
            let is_open = self.tree.tiles.iter().any(
                |(_tile_id, tile_kind)| matches!(tile_kind, Tile::Pane(pane_kind) if *pane_kind == toggle_state.kind),
            );

            if !is_open {
                debug!("tree:");
                let root = self.tree.root();
                dump_tiles(&mut self.tree.tiles, root);

                add_pane_to_root(&mut self.tree, toggle_state.kind, ContainerKind::Tabs);
            }
        }

        // now deal with existing tiles that should be closed
        let tiles_to_close = self.tree.tiles.iter().filter_map(|(tile_id, tile)| {
            let should_close = toggle_states.iter().any(|candidate| {
                let is_tile_for_this_viewport = matches!(candidate.mode, ViewMode::Tile(candidate_viewport_id) if candidate_viewport_id == viewport_id);
                let same_kind_of_pane = matches!(tile, Tile::Pane(kind) if *kind == candidate.kind);
                let result = !is_tile_for_this_viewport && same_kind_of_pane;
                if false {
                    trace!("should close?. tile {:?}: is_tile_for_this_viewport: {} same_kind_of_pane: {} = {}", tile_id, is_tile_for_this_viewport, same_kind_of_pane, result);
                }

                result
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
        let mut tiles = Tiles::default();

        let root_tabs = vec![];
        let root = tiles.insert_grid_tile(root_tabs);

        let tree = Tree::new("tile_tree", root, tiles);

        tree
    }
}

pub struct ViewportState {
    pub(crate) command_sender: Enqueue<UiCommand>,
    pub(crate) id: ViewportId,
    pub(crate) viewport_actions: Vec<ViewportAction>,
    pub(crate) tree_behavior: TreeBehavior,
    pub(crate) workspaces: Value<Workspaces>,
    pub(crate) context: Option<egui::Context>,
    pub(crate) ui_state: Value<UiState>,

    fps_stats: FpsStats,
    fps_snapshot: Option<FpsSnapshot>,
    frame_number: u64,
}

#[derive(Debug, Copy, Clone)]
pub enum ViewportAction {
    RepositionWindow(PaneKind, Pos2, Vec2),
}

impl ViewportState {
    pub fn new(
        id: ViewportId,
        command_sender: Enqueue<UiCommand>,
        ui_state: Value<UiState>,
        workspaces: Value<Workspaces>,
    ) -> Self {
        workspaces
            .lock()
            .unwrap()
            .ensure_viewport(id);

        Self {
            command_sender: command_sender.clone(),
            id,
            tree_behavior: TreeBehavior::new(ui_state.clone(), command_sender, id),
            viewport_actions: Default::default(),
            workspaces,
            context: None,
            ui_state,

            fps_stats: FpsStats::new(300),
            fps_snapshot: None,
            frame_number: 0,
        }
    }

    pub fn init(&mut self) {
        let mut workspaces = self.workspaces.lock().unwrap();
        let workspace = workspaces.active();

        let actions = Self::build_window_reposition_actions(&workspace.toggle_states, self.id);
        self.viewport_actions.extend(actions);
    }

    pub fn update(&mut self, command: ViewportUiCommand) -> Option<ViewportUiAction> {
        match command {
            ViewportUiCommand::SetPanelMode(kind, mode) => {
                let mut workspaces = self.workspaces.lock().unwrap();
                let mut workspace = workspaces.active();

                if let Some(toggle_state) = workspace
                    .toggle_states
                    .iter_mut()
                    .find(|candidate| candidate.kind == kind)
                {
                    let new_mode_is_window =
                        matches!(mode, ViewMode::Window(mode_view_port) if mode_view_port == self.id);
                    let old_mode_is_window = matches!(toggle_state.mode, ViewMode::Window(_));

                    if !old_mode_is_window && new_mode_is_window {
                        if let (Some(window_position), Some(window_size)) =
                            (toggle_state.window_position, toggle_state.window_size)
                        {
                            self.viewport_actions
                                .push(ViewportAction::RepositionWindow(
                                    toggle_state.kind,
                                    window_position,
                                    window_size,
                                ));
                        }
                    }
                    toggle_state.mode = mode;
                }

                None
            }
            ViewportUiCommand::ClosePanel(kind) => {
                let mut workspaces = self.workspaces.lock().unwrap();
                let mut workspace = workspaces.active();

                if let Some(toggle_state) = workspace
                    .toggle_states
                    .iter_mut()
                    .find(|candidate| candidate.kind == kind)
                {
                    match toggle_state.mode {
                        ViewMode::Tile(viewport_id) if viewport_id == self.id => {
                            toggle_state.mode = ViewMode::Disabled;
                        }
                        _ => unreachable!(),
                    }
                }

                None
            }
            ViewportUiCommand::WorkspaceChanged(_index) => {
                let mut workspaces = self.workspaces.lock().unwrap();
                let workspace = workspaces.active();

                let actions = Self::build_window_reposition_actions(&workspace.toggle_states, self.id);
                self.viewport_actions.extend(actions);

                None
            }
        }
    }

    fn build_window_reposition_actions(toggle_states: &[ToggleState], viewport_id: ViewportId) -> Vec<ViewportAction> {
        toggle_states
            .iter()
            .filter_map(|toggle_state| match toggle_state {
                ToggleState {
                    mode: ViewMode::Window(id),
                    window_position: Some(window_position),
                    window_size: Some(window_size),
                    ..
                } if *id == viewport_id => Some(ViewportAction::RepositionWindow(
                    toggle_state.kind,
                    *window_position,
                    *window_size,
                )),
                _ => None,
            })
            .collect::<Vec<_>>()
    }

    pub fn ui(&mut self, ctx: &egui::Context) {
        let now = std::time::Instant::now();

        self.fps_snapshot = self.fps_stats.update(now);
        self.frame_number = ctx.cumulative_frame_nr();

        let ui_id = Id::from(self.id);

        if self.context.is_none() {
            self.context.replace(ctx.clone());
        }

        if ctx.input(|i| i.viewport().close_requested()) {
            info!("Viewport close requested. viewport: {:?}", self.id);
            self.command_sender
                .send(UiCommand::CloseViewport(self.id))
                .expect("sent");
        }

        {
            let mut workspaces = self.workspaces.lock().unwrap();
            let mut workspace = workspaces.active();

            // temporarily remove to satisfy the borrow checker
            let mut workspace_viewport_config = workspace
                .viewport_tree_configs
                .remove(&self.id)
                .unwrap();

            // TODO or workspace changed
            if self.frame_number == 0 {
                let actions = workspace
                    .toggle_states
                    .iter()
                    .filter_map(|toggle_state| match toggle_state {
                        ToggleState {
                            mode: ViewMode::Window(toggle_viewport_id),
                            window_position: Some(window_position),
                            window_size: Some(window_size),
                            ..
                        } if *toggle_viewport_id == self.id => Some(ViewportAction::RepositionWindow(
                            toggle_state.kind,
                            *window_position,
                            *window_size,
                        )),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                debug!("actions: {:?}", actions);
                self.viewport_actions.extend(actions);
            }

            workspace_viewport_config.update_tree(self.id, workspace.toggle_states.as_slice());

            workspace
                .viewport_tree_configs
                .insert(self.id, workspace_viewport_config);
        }

        let mut request_workspace_toggle = false;

        let sender = self.command_sender.clone();

        if self.id == ViewportId::ROOT {
            egui::TopBottomPanel::top(ui_id.with("top_panel")).show(ctx, |ui| {
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
                                        if ui
                                            .add(egui::Button::selectable(
                                                other_language.eq(&language),
                                                tr!(&format_language_key(&other_language)),
                                            ))
                                            .clicked()
                                        {
                                            sender
                                                .send(UiCommand::LanguageChanged(other_language.clone()))
                                                .expect("sent");
                                        }
                                    }
                                });
                        },
                    );
                });
            });
        }

        let panel_fill_color = ctx.style().visuals.panel_fill;
        let side_panel_fill_color = panel_fill_color.gamma_multiply(0.9);

        let mut request_make_visible: Option<ToggleState> = None;

        egui::SidePanel::left(ui_id.with("left_panel"))
            .min_width(MIN_TOUCH_SIZE.x * 2.0)
            .max_width(200.0)
            .resizable(true)
            .frame(Frame::NONE.fill(side_panel_fill_color))
            .show(ctx, |ui| {
                let left_panel_width = ui.available_size_before_wrap().x;
                ui.vertical(|ui| {
                    let mut workspaces = self.workspaces.lock().unwrap();
                    let mut workspace = workspaces.active();
                    let mut workspace_viewport_tree_config = workspace.viewport_tree_configs.remove(&self.id).unwrap();

                    egui::ScrollArea::both()
                        // FIXME the 4.0 is a guess at the height of a separator and margins and such
                        .max_height(ui.available_height() - ((MIN_TOUCH_SIZE.y * 2.0) + 2.0))
                        .auto_shrink([false, false])
                        .min_scrolled_width(MIN_TOUCH_SIZE.x)
                        .show(ui, |ui| {

                            CollapsingHeader::new("Stats")
                                .show_unindented(ui, |ui|{
                                    ui.label(format!("Frame: {}", self.frame_number));
                                    if let Some(snapshot) = &self.fps_snapshot {
                                        ui.label(format!(
                                            "FPS: {:.1} (min {:.1}, max {:.1}, avg {:.1})",
                                            snapshot.latest,
                                            snapshot.min,
                                            snapshot.max,
                                            snapshot.avg
                                        ));

                                        show_frame_durations(ui, &self.fps_stats);
                                    }
                                });

                            for kind in workspace.left_toggles.iter() {
                                let toggle_definition = TOGGLE_DEFINITIONS.iter().find(|candidate| candidate.kind == *kind).unwrap();

                                let toggle_state = workspace.toggle_states.iter().find(|candidate| candidate.kind == *kind).unwrap();

                                #[derive(Debug, Clone, Copy, PartialEq, Eq)]
                                enum ToggleStatus {
                                    Disabled,
                                    WindowThisViewport,
                                    WindowOtherViewport,
                                    TileThisViewport,
                                    TileOtherViewport,
                                }
                                let toggle_status = match toggle_state.mode {
                                    ViewMode::Disabled => ToggleStatus::Disabled,
                                    ViewMode::Window(viewport) if viewport == self.id => ToggleStatus::WindowThisViewport,
                                    ViewMode::Window(_) => ToggleStatus::WindowOtherViewport,
                                    ViewMode::Tile(viewport) if viewport == self.id => ToggleStatus::TileThisViewport,
                                    ViewMode::Tile(_)  => ToggleStatus::TileOtherViewport,
                                };

                                let response = ui.horizontal(|ui| {
                                    ui.set_width(left_panel_width);
                                    ui.set_height(MIN_TOUCH_SIZE.y * 2.0);

                                    let visuals = ui.style().interact_selectable(&ui.response(), toggle_status != ToggleStatus::Disabled);

                                    let bg_color = match toggle_status {
                                        ToggleStatus::Disabled => { visuals.weak_bg_fill }
                                        //ToggleStatus::WindowThisViewport | ToggleStatus::TileThisViewport => { visuals.bg_fill }
                                        ToggleStatus::WindowThisViewport => { visuals.bg_fill }
                                        ToggleStatus::TileThisViewport => { visuals.bg_fill.gamma_multiply(0.8) }
                                        //ToggleStatus::WindowOtherViewport | ToggleStatus::TileOtherViewport => { visuals.bg_fill.gamma_multiply(0.3) }
                                        ToggleStatus::WindowOtherViewport => { visuals.bg_fill.gamma_multiply(0.4) }
                                        ToggleStatus::TileOtherViewport => { visuals.bg_fill.gamma_multiply(0.2) }
                                    };

                                    let mut rect = ui.max_rect();
                                    rect.max.x = left_panel_width;
                                    ui.painter().rect_filled(rect, CornerRadius::ZERO, bg_color);

                                    let button_width = left_panel_width
                                        .at_least(MIN_TOUCH_SIZE.x)
                                        .at_most(MIN_TOUCH_SIZE.x * 2.0);
                                    ui.add_sized(Vec2::new(button_width, ui.available_height()), egui::Label::new(tr!(&format!("panel-{}-icon", toggle_definition.key)))
                                        .selectable(false));

                                    if left_panel_width > MIN_TOUCH_SIZE.x * 2.0 {
                                        ui.add(egui::Label::new(tr!(&format!("panel-{}-name", toggle_definition.key))).selectable(false));
                                    }
                                }).response;

                                let window_mode = ViewMode::Window(self.id);

                                let interaction = response.interact(Sense::click());
                                if interaction.double_clicked() {
                                    sender.send(UiCommand::ViewportUiCommand(self.id, ViewportUiCommand::SetPanelMode(*kind, window_mode))).expect("sent");
                                } else if interaction.clicked() {

                                    match toggle_state.mode {
                                        ViewMode::Disabled => {
                                            // if it's not enabled, make it a window
                                            sender.send(UiCommand::ViewportUiCommand(self.id, ViewportUiCommand::SetPanelMode(*kind, window_mode))).expect("sent");
                                        }

                                        // otherwise, if it's not active, activate it
                                        ViewMode::Tile(viewport_id) if viewport_id == self.id => {
                                            request_make_visible.replace(*toggle_state);
                                        }
                                        ViewMode::Window(viewport_id) if viewport_id == self.id => {
                                            request_make_visible.replace(*toggle_state);
                                        }
                                        _ => {
                                            // on a different viewport

                                            // TODO maybe show send a message to the other viewport and have it activate the tile/window.
                                        }
                                    }
                                }
                            }
                        });

                    match request_make_visible {
                        Some(toggle_state) if matches!(toggle_state.mode, ViewMode::Tile(r_viewport_id) if r_viewport_id == self.id) => {

                            let tile_id = workspace_viewport_tree_config.tree.tiles.find_pane( &toggle_state.kind).unwrap();
                            workspace_viewport_tree_config.tree.make_active( |candidate_id, _tile | candidate_id == tile_id);


                        }
                        _ => {}
                    }

                    workspace.viewport_tree_configs.insert(self.id, workspace_viewport_tree_config);

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

                // reset the flag
                self.tree_behavior.container_is_tabs = false;

                let mut workspaces = self.workspaces.lock().unwrap();
                let mut workspace = workspaces.active();

                let workspace_viewport_config = workspace
                    .viewport_tree_configs
                    .get_mut(&self.id)
                    .unwrap();

                workspace_viewport_config
                    .tree
                    .ui(&mut self.tree_behavior, ui);
            });

        //
        // Windows
        //

        let windows = {
            let mut workspaces = self.workspaces.lock().unwrap();
            let workspace = workspaces.active();

            workspace.toggle_states.iter().cloned().enumerate().filter(|(_index, candidate)| {
                matches!(candidate, ToggleState { mode: ViewMode::Window(candidate_viewport_id), .. } if *candidate_viewport_id == self.id)
            }).collect::<Vec<_>>()
        };

        for (toggle_index, toggle_state) in windows.into_iter() {
            let kind_key = kind_key(&toggle_state.kind);
            let title_i18n_key = title_i18n_key(kind_key);
            let title = tr!(&title_i18n_key);

            let mut applicable_actions: Vec<ViewportAction> = vec![];

            // HACK: windows-in-wrong-positions-hack
            // wait till the UI settles before applying actions.  Without this hack when the app
            // first starts up the window might not be big enough to show the windows in the right positions.
            // Currently, it seems that waiting 1 frame is enough, but this is fragile and may break in the future.
            if self.frame_number >= 1 {
                self.viewport_actions
                    .retain(|candidate| {
                        let steal = match candidate {
                            ViewportAction::RepositionWindow(kind, _, _) if *kind == toggle_state.kind => true,
                            _ => false,
                        };

                        applicable_actions.push(*candidate);

                        !steal
                    });
            } else {
                ctx.request_repaint();
            }

            let mut dump_position = false;

            let style = &ctx.style();

            let window_id = ui_id.with(kind_key);
            ctx.memory(|memory| {
                if let Some(rect) = memory.area_rect(window_id) {
                    // IMPORTANT we can't just use the `toggle_state` from the loop iterator.
                    // a) because it was cloned
                    // b) because a message could have been sent to the app to change the window into a tile
                    // So, we need to update the real one.

                    let mut workspaces = self.workspaces.lock().unwrap();
                    let mut workspace = workspaces.active();

                    workspace.toggle_states[toggle_index]
                        .window_position
                        .replace(rect.min);

                    let window_size = rect.size()
                        - style.spacing.window_margin.sum()
                        //- style.visuals.window_shadow.margin().right_bottom() / 2.0
                        - Vec2::splat(style.visuals.window_stroke.width * 2.0);
                    workspace.toggle_states[toggle_index]
                        .window_size
                        .replace(window_size);
                }
            });

            let mut window = egui::Window::new(&title)
                .id(window_id)
                // .frame(Frame::NONE)
                .frame(
                    Frame::new()
                        .fill(style.visuals.panel_fill)
                        .inner_margin(style.spacing.window_margin)
                        .stroke(style.visuals.window_stroke)
                        .shadow(style.visuals.window_shadow)
                        .corner_radius(style.visuals.window_corner_radius),
                )
                .title_bar(false);

            for applicable_action in applicable_actions {
                match applicable_action {
                    ViewportAction::RepositionWindow(kind, position, size) if kind == toggle_state.kind => {
                        debug!(
                            "repositioning window. kind: {:?}, rect: {:?}",
                            kind,
                            Rect::from_min_size(position, size)
                        );
                        dump_position = true;
                        window = window
                            .current_pos(position)
                            .fixed_size(size)
                    }
                    _ => {}
                }
            }

            let window = window.resizable(true).show(ctx, |ui| {
                ui.vertical(|ui| {
                    if false {
                        trace!(
                            "window, layer_id: {:?}, toggle_state: {:?}",
                            ui.layer_id(),
                            toggle_state
                        );
                    }

                    let kind = toggle_state.kind;
                    let mut ui_state = self.ui_state.lock().unwrap();

                    let mut dragged = false;
                    let result = show_panel_title_and_controls(
                        self.id,
                        &kind,
                        title,
                        sender.clone(),
                        ui,
                        false,
                        false,
                        true,
                        &mut dragged,
                        |ui, button_size| {
                            ui.add_sized(button_size, egui::Button::new("?"))
                                .clicked()
                        },
                    );
                    ui.separator();
                    app::show_panel_content(&kind, ui, &mut ui_state);
                    result
                })
                .inner
            });

            if let Some(window) = window {
                match request_make_visible {
                    Some(requested_toggle_state)
                        if requested_toggle_state.mode == ViewMode::Window(self.id)
                            && requested_toggle_state.kind == toggle_state.kind =>
                    {
                        trace!(
                            "bringing window to front. layer_id: {:?}, toggle_state: {:?}",
                            window.response.layer_id, toggle_state
                        );
                        bring_window_to_front(ctx, window.response.layer_id);
                    }
                    _ => {}
                }

                if dump_position || window.inner.unwrap() {
                    debug!(
                        "saving window rect. kind: {:?}, rect: {:?}",
                        toggle_state.kind, window.response.rect
                    );
                }
            }
        }

        if request_workspace_toggle {
            let mut workspaces = self.workspaces.lock().unwrap();

            if workspaces.count() == 1 {
                workspaces.clone_active();
            }
            let index = match workspaces.active_index() {
                0 => 1,
                1 => 0,
                _ => unreachable!(),
            };

            self.command_sender
                .send(UiCommand::ChangeWorkspace(index))
                .expect("sent");
        }
    }
}

pub struct ToggleDefinition {
    pub key: &'static str,
    pub kind: PaneKind,
}

#[derive(serde::Deserialize, serde::Serialize, Copy, Clone, Debug)]
pub struct ToggleState {
    pub(crate) kind: PaneKind,
    pub(crate) mode: ViewMode,
    pub(crate) window_position: Option<Pos2>,
    pub(crate) window_size: Option<Vec2>,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewMode {
    Disabled,
    Tile(ViewportId),
    Window(ViewportId),
}

pub(crate) struct TreeBehavior {
    simplification_options: SimplificationOptions,
    ui_state: Value<UiState>,
    command_sender: Enqueue<UiCommand>,
    drag: Option<TileId>,
    container_is_tabs: bool,
    viewport_id: ViewportId,
}

impl TreeBehavior {
    fn new(ui_state: Value<UiState>, command_sender: Enqueue<UiCommand>, viewport_id: ViewportId) -> Self {
        Self {
            simplification_options: SimplificationOptions {
                all_panes_must_have_tabs: true,
                ..SimplificationOptions::default()
            },
            ui_state,
            command_sender,
            drag: None,
            container_is_tabs: false,
            viewport_id,
        }
    }
}

impl egui_tiles::Behavior<PaneKind> for TreeBehavior {
    fn pane_ui(&mut self, ui: &mut Ui, tile_id: TileId, kind: &mut PaneKind) -> UiResponse {
        let in_tab = self.container_is_tabs;

        let mut ui_state = self.ui_state.lock().unwrap();

        let kind_key = kind_key(&kind);
        let title_i18n_key = title_i18n_key(kind_key);
        let title = tr!(&title_i18n_key);

        let mut dragged = false;

        if !in_tab {
            show_panel_title_and_controls(
                self.viewport_id,
                &kind,
                title,
                self.command_sender.clone(),
                ui,
                true,
                true,
                false,
                &mut dragged,
                |_, _| (),
            );
            ui.separator();
        }

        app::show_panel_content(&kind, ui, &mut ui_state);

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

        let title_i18n_key = format!("panel-{}-window-title", kind_key);

        tr!(&title_i18n_key).into()
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
            self.command_sender
                .send(UiCommand::ViewportUiCommand(
                    self.viewport_id,
                    ViewportUiCommand::ClosePanel(*kind),
                ))
                .expect("sent");
        }
        // always deny, manually handle closing ourselves
        false
    }

    fn top_bar_right_ui(
        &mut self,
        _tiles: &Tiles<PaneKind>,
        _ui: &mut Ui,
        _tile_id: TileId,
        _tabs: &Tabs,
        _scroll_offset: &mut f32,
    ) {
        if let Some(tile_id) = _tabs.active {
            if let Some(Tile::Pane(kind)) = _tiles.get(tile_id) {
                let mut dragged = false;
                show_panel_controls(
                    self.viewport_id,
                    &kind,
                    self.command_sender.clone(),
                    _ui,
                    true,
                    true,
                    false,
                    &mut dragged,
                    |_, _| (),
                );

                if dragged {
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

fn show_panel_title_and_controls<T>(
    viewport_id: ViewportId,
    kind: &PaneKind,
    title: String,
    sender: Sender<UiCommand>,
    ui: &mut Ui,
    show_drag_handle: bool,
    show_make_window: bool,
    show_make_tile: bool,
    dragged: &mut bool,
    controls_ui: impl FnOnce(&mut Ui, Vec2) -> T,
) -> T {
    let (_, result) = egui::Sides::new().show(
        ui,
        |ui| {
            ui.add(egui::Label::new(title).selectable(false));
        },
        |ui| {
            show_panel_controls(
                viewport_id,
                kind,
                sender,
                ui,
                show_drag_handle,
                show_make_window,
                show_make_tile,
                dragged,
                controls_ui,
            )
        },
    );

    result
}

fn show_panel_controls<T>(
    viewport_id: ViewportId,
    kind: &PaneKind,
    sender: Sender<UiCommand>,
    ui: &mut Ui,
    show_drag_handle: bool,
    show_make_window: bool,
    show_make_tile: bool,
    dragged: &mut bool,
    controls_ui: impl FnOnce(&mut Ui, Vec2) -> T,
) -> T {
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
        if ui
            .add_sized(button_size, egui::Button::new("🗙"))
            .clicked()
        {
            sender
                .send(UiCommand::ViewportUiCommand(
                    viewport_id,
                    ViewportUiCommand::SetPanelMode(*kind, ViewMode::Disabled),
                ))
                .expect("sent");
        }
        if show_make_tile {
            if ui
                .add_sized(button_size, egui::Button::new("🗕"))
                .clicked()
            {
                sender
                    .send(UiCommand::ViewportUiCommand(
                        viewport_id,
                        ViewportUiCommand::SetPanelMode(*kind, ViewMode::Tile(viewport_id)),
                    ))
                    .expect("sent");
            }
        }
        if show_make_window {
            if ui
                .add_sized(button_size, egui::Button::new("🗖"))
                .clicked()
            {
                sender
                    .send(UiCommand::ViewportUiCommand(
                        viewport_id,
                        ViewportUiCommand::SetPanelMode(*kind, ViewMode::Window(viewport_id)),
                    ))
                    .expect("sent");
            }
        }
        if show_drag_handle {
            if ui
                .add_sized(button_size, egui::Button::new("✋").sense(Sense::click_and_drag()))
                .on_hover_cursor(egui::CursorIcon::Grab)
                .dragged()
            {
                *dragged = true;
            }
        }
        let result = controls_ui(ui, button_size);

        result
    })
    .inner
}

/// Persisted
#[derive(serde::Deserialize, serde::Serialize, Clone)]
#[serde(default)]
pub struct WorkspaceConfig {
    pub(crate) toggle_states: Vec<ToggleState>,
    pub(crate) left_toggles: Vec<PaneKind>,
    pub(crate) viewport_tree_configs: HashMap<ViewportId, ViewportTreeConfig>,
    pub(crate) viewport_configs: HashMap<ViewportId, ViewportConfig>,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        let left_toggles = TOGGLE_DEFINITIONS
            .iter()
            .map(|candidate| candidate.kind)
            .collect::<Vec<_>>();

        let toggle_states = vec![
            ToggleState {
                mode: ViewMode::Tile(ViewportId::ROOT),
                kind: PaneKind::Camera,
                window_position: None,
                window_size: None,
            },
            ToggleState {
                mode: ViewMode::Tile(ViewportId::ROOT),
                kind: PaneKind::Controls,
                window_position: None,
                window_size: None,
            },
            ToggleState {
                mode: ViewMode::Window(ViewportId::ROOT),
                kind: PaneKind::Diagnostics,
                window_position: None,
                window_size: None,
            },
            ToggleState {
                mode: ViewMode::Disabled,
                kind: PaneKind::Plot,
                window_position: None,
                window_size: None,
            },
            ToggleState {
                mode: ViewMode::Window(ViewportId::ROOT),
                kind: PaneKind::Settings,
                window_position: None,
                window_size: None,
            },
            ToggleState {
                mode: ViewMode::Tile(ViewportId::ROOT),
                kind: PaneKind::Status,
                window_position: None,
                window_size: None,
            },
        ];

        Self {
            left_toggles,
            toggle_states,
            viewport_tree_configs: Default::default(),
            viewport_configs: Default::default(),
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Workspaces {
    workspaces: Vec<Value<WorkspaceConfig>>,
    active_workspace: usize,
}

impl Default for Workspaces {
    fn default() -> Self {
        Self {
            workspaces: vec![Value::new(WorkspaceConfig::default())],
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
    pub fn ensure_viewport(&mut self, viewport_id: ViewportId) {
        for workspace in self.workspaces.iter_mut() {
            let mut workspace = workspace.lock().unwrap();

            if !workspace
                .viewport_tree_configs
                .contains_key(&viewport_id)
            {
                workspace
                    .viewport_tree_configs
                    .insert(viewport_id, ViewportTreeConfig::default());
            }
            if !workspace
                .viewport_configs
                .contains_key(&viewport_id)
            {
                workspace
                    .viewport_configs
                    .insert(viewport_id, ViewportConfig::default());
            }
        }
    }

    pub fn active(&mut self) -> ValueGuard<'_, WorkspaceConfig> {
        self.workspaces[self.active_workspace]
            .lock()
            .unwrap()
    }

    pub fn active_index(&mut self) -> usize {
        self.active_workspace
    }

    pub fn set_active(&mut self, index: usize) -> Result<(), WorkspaceError> {
        if index >= self.workspaces.len() {
            return Err(WorkspaceError::InvalidWorkspaceIndex);
        }
        if index == self.active_workspace {
            return Err(WorkspaceError::AlreadyActive);
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

    pub fn remove(&mut self, index: usize) -> Result<(), WorkspaceError> {
        if index >= self.workspaces.len() {
            return Err(WorkspaceError::InvalidWorkspaceIndex);
        }
        if index == self.active_workspace {
            return Err(WorkspaceError::CannotRemoveActiveWorkspace);
        }

        self.workspaces.remove(index);

        Ok(())
    }

    pub fn count(&self) -> usize {
        self.workspaces.len()
    }
}

pub(crate) fn kind_key(kind: &PaneKind) -> &str {
    TOGGLE_DEFINITIONS
        .iter()
        .find_map(|candidate| {
            if candidate.kind == *kind {
                Some(candidate.key)
            } else {
                None
            }
        })
        .unwrap()
}

//
// i18n
//

pub(crate) fn title_i18n_key(kind_key: &str) -> String {
    format!("panel-{}-window-title", kind_key)
}
