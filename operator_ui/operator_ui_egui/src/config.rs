#[derive(serde::Deserialize, serde::Serialize, Debug)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct Config {
    pub language_identifier: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            language_identifier: egui_i18n::get_language(),
        }
    }
}
