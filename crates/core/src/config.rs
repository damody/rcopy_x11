use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub database_path: PathBuf,
    pub max_history_items: usize,
    pub auto_paste: bool,
    pub paste_command: String,
    pub capture_text: bool,
    pub capture_html: bool,
    pub capture_images: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            database_path: PathBuf::from("rcopy.db"),
            max_history_items: 10_000,
            auto_paste: true,
            paste_command: "wtype".to_string(),
            capture_text: true,
            capture_html: true,
            capture_images: true,
        }
    }
}

impl AppConfig {
    pub fn from_toml(input: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_defaults_capture_text_html_and_images() {
        let config = AppConfig::default();
        assert!(config.capture_text);
        assert!(config.capture_html);
        assert!(config.capture_images);
        assert!(config.auto_paste);
        assert_eq!(config.paste_command, "wtype");
    }

    #[test]
    fn partial_toml_override_preserves_default_fields() {
        let config = AppConfig::from_toml("auto_paste = false").expect("valid partial config");

        assert!(!config.auto_paste);
        assert_eq!(config.database_path, AppConfig::default().database_path);
        assert_eq!(
            config.max_history_items,
            AppConfig::default().max_history_items
        );
        assert_eq!(config.paste_command, AppConfig::default().paste_command);
        assert_eq!(config.capture_text, AppConfig::default().capture_text);
        assert_eq!(config.capture_html, AppConfig::default().capture_html);
        assert_eq!(config.capture_images, AppConfig::default().capture_images);
    }
}
