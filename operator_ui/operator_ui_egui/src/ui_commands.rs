use egui::{Context, ThemePreference};
use egui_mobius::Value;
use tracing::trace;
use crate::app::AppState;
use crate::config::Config;
use crate::task::Task;

#[derive(Debug, Clone)]
pub enum UiCommand {
    #[allow(dead_code)]
    None,
    LangageChanged(String),
    ThemeChanged(ThemePreference),
}

pub fn handle_command(
    command: UiCommand,
    app_state: Value<AppState>,
    config: Value<Config>,
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
    }
}