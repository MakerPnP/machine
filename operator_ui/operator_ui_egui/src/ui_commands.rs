use egui::{Context, ThemePreference, ViewportId};
use egui_mobius::Value;
use tracing::trace;

use crate::app::{AppState, PaneKind};
use crate::config::Config;
use crate::task::Task;
use crate::workspace::{ViewMode, ViewportState, Workspaces};

#[derive(Debug, Clone)]
pub enum UiCommand {
    #[allow(dead_code)]
    None,
    LanguageChanged(String),
    ThemeChanged(ThemePreference),

    ViewportUiCommand(ViewportId, ViewportUiCommand),
    CloseViewport(ViewportId),
    ChangeWorkspace(usize),
}

#[derive(Debug, Clone)]
pub enum ViewportUiCommand {
    SetPanelMode(PaneKind, ViewMode),
    ClosePanel(PaneKind),

    // internal
    WorkspaceChanged(usize),
}

pub enum ViewportUiAction {
    None,
}

pub fn handle_command(
    command: UiCommand,
    app_state: Value<AppState>,
    config: Value<Config>,
    workspaces: Value<Workspaces>,
    viewports: Value<Vec<Value<ViewportState>>>,
    ui_context: Context,
) -> Task<UiCommand> {
    ui_context.request_repaint();
    {
        let viewports = viewports.lock().unwrap();
        for viewport in viewports.iter() {
            let id = viewport.lock().unwrap().id;
            ui_context.request_repaint_of(id);
        }
    }

    trace!("Handling command: {:?}", command);

    match command {
        UiCommand::None => Task::none(),
        UiCommand::LanguageChanged(language) => {
            egui_i18n::set_language(&language);
            config
                .lock()
                .unwrap()
                .language_identifier = language;
            Task::none()
        }
        UiCommand::ThemeChanged(theme) => {
            ui_context.set_theme(theme);
            Task::none()
        }
        UiCommand::ViewportUiCommand(id, command) => {
            let viewports = viewports.lock().unwrap();
            if let Some(viewport) = viewports
                .iter()
                .find(|candidate| candidate.lock().unwrap().id == id)
            {
                let action = viewport.lock().unwrap().update(command);
                match action {
                    None => Task::none(),
                    Some(ViewportUiAction::None) => Task::none(),
                }
            } else {
                Task::none()
            }
        }
        UiCommand::CloseViewport(id) => {
            let mut viewports = viewports.lock().unwrap();

            viewports.retain(|candidate| candidate.lock().unwrap().id != id);

            Task::none()
        }
        UiCommand::ChangeWorkspace(index) => {
            let app_state = app_state.lock().unwrap();

            let mut workspaces = workspaces.lock().unwrap();
            let viewports = viewports.lock().unwrap();
            if workspaces.set_active(index).is_ok() {
                for viewport in viewports.iter() {
                    let viewport_id = viewport.lock().unwrap().id;
                    app_state
                        .command_sender
                        .send(UiCommand::ViewportUiCommand(
                            viewport_id,
                            ViewportUiCommand::WorkspaceChanged(index),
                        ))
                        .expect("sent");
                }
            }
            Task::none()
        }
    }
}
