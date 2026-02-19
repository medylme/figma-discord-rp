use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::PathBuf};

pub const FALLBACK_IMAGE: &str = "defaulticon";

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum AppName {
    #[default]
    Figma,
    FigmaDesktop,
    Custom(String),
}

pub const STATE_ENTRIES: &[(&str, &str)] = &[
    ("design", "Designing"),
    ("whiteboard", "Whiteboarding"),
    ("slides", "Presenting"),
    ("sites", "Building a Site"),
    ("buzz", "Buzzing"),
    ("make", "Making"),
    ("dev_mode", "Dev Mode"),
    ("other", "Working"),
    ("idle", "Idle"),
];

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ImageOverride {
    pub enabled: bool,
    pub image_url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub default_image: String,
    pub image_overrides: HashMap<String, ImageOverride>,
    pub hide_filename: bool,
    pub disable_idle: bool,
    #[serde(default)]
    pub app_name: AppName,
}

impl Default for Settings {
    fn default() -> Self {
        let overrides = STATE_ENTRIES
            .iter()
            .map(|(key, _)| (key.to_string(), ImageOverride::default()))
            .collect();
        Self {
            default_image: String::new(),
            image_overrides: overrides,
            hide_filename: false,
            disable_idle: false,
            app_name: AppName::default(),
        }
    }
}

impl Settings {
    pub fn resolved_app_name(&self) -> &str {
        match &self.app_name {
            AppName::Figma => "Figma",
            AppName::FigmaDesktop => "Figma Desktop",
            AppName::Custom(s) if s.is_empty() => "Figma",
            AppName::Custom(s) => s,
        }
    }
}

impl Settings {
    pub fn image_url_for_state(&self, state_key: &str) -> &str {
        if let Some(ov) = self.image_overrides.get(state_key)
            && ov.enabled
            && !ov.image_url.is_empty()
        {
            return &ov.image_url;
        }
        if self.default_image.is_empty() {
            FALLBACK_IMAGE
        } else {
            &self.default_image
        }
    }

    fn path() -> PathBuf {
        let config = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        config.join("dyl-figma-discord-rp").join("settings.json")
    }

    pub fn load() -> Self {
        let path = Self::path();
        match fs::read_to_string(&path) {
            Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(data) = serde_json::to_string_pretty(self) {
            let _ = fs::write(&path, data);
        }
    }
}
