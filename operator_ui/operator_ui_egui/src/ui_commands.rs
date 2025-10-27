use egui::{Context, ThemePreference};
use egui_mobius::Value;
use tracing::trace;
use crate::app::{AppState, PaneKind, PersistentAppState, ViewMode};
use crate::config::Config;
use crate::task::Task;

#[derive(Debug, Clone)]
pub enum UiCommand {
    #[allow(dead_code)]
    None,
    LangageChanged(String),
    ThemeChanged(ThemePreference),
    SetPanelMode(PaneKind, ViewMode),
    ClosePanel(PaneKind),
}

pub fn handle_command(
    command: UiCommand,
    _app_state: Value<AppState>,
    config: Value<Config>,
    persistent_app_state: Value<PersistentAppState>,
    ui_context: Context,
) -> Task<UiCommand> {
    trace!("Handling command: {:?}", command);

    match command {
        UiCommand::None => Task::none(),
        UiCommand::LangageChanged(language) => {
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
        UiCommand::SetPanelMode(kind, mode) => {
            let mut state = persistent_app_state.lock().unwrap();

            if let Some(toggle_state) = state.toggle_states.iter_mut().find(|candidate|candidate.kind == kind) {
                toggle_state.mode = mode;
            }

            Task::none()
        }
        UiCommand::ClosePanel(kind) => {
            let mut state = persistent_app_state.lock().unwrap();

            if let Some(toggle_state) = state.toggle_states.iter_mut().find(|candidate|candidate.kind == kind) {
                match toggle_state.mode {
                    ViewMode::Tile => {
                        toggle_state.mode = ViewMode::Disabled;
                    }
                    _ => unreachable!()
                }
            }

            Task::none()
        }
    }
}