use anyhow::{Result, anyhow};
use serde::Deserialize;
use std::{fmt, fs, path::PathBuf, time::Instant};
use sysinfo::{Process, ProcessRefreshKind, ProcessesToUpdate, System};

pub const IDLE_THRESHOLD_SECONDS: u64 = 300;

#[derive(Deserialize, Debug, Default)]
struct FigmaSettings {
    #[serde(rename = "zoomStop")]
    _zoom_stop: Option<u32>,
    windows: Option<Vec<FigmaWindow>>,
}

#[derive(Deserialize, Debug)]
struct FigmaWindow {
    tabs: Option<Vec<FigmaTab>>,
    #[serde(rename = "activeTabPath")]
    _active_tab_path: Option<String>,
}

#[derive(Debug, Default, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum EditorType {
    #[default]
    Design,
    Whiteboard,
    Slides,
    Sites,
    #[serde(rename = "cooper")]
    Buzz,
    #[serde(rename = "figmake")]
    Make,
    #[serde(rename = "dev_handoff")]
    DevMode,
    #[serde(other)]
    Other,
}

impl EditorType {
    pub fn key(&self) -> &'static str {
        match self {
            Self::Design => "design",
            Self::Whiteboard => "whiteboard",
            Self::Slides => "slides",
            Self::Sites => "sites",
            Self::Buzz => "buzz",
            Self::Make => "make",
            Self::DevMode => "dev_mode",
            Self::Other => "other",
        }
    }
}

impl fmt::Display for EditorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            EditorType::Design => "Designing",
            EditorType::Whiteboard => "Whiteboarding",
            EditorType::Slides => "Presenting",
            EditorType::Sites => "Building a site",
            EditorType::Buzz => "Buzzing",
            EditorType::Make => "Making",
            EditorType::DevMode => "Dev Mode",
            EditorType::Other => "Working",
        };
        write!(f, "{s}")
    }
}

#[derive(Clone, Deserialize, Debug, Default, PartialEq)]
pub struct FigmaTab {
    pub title: Option<String>,
    #[serde(rename = "editorType")]
    pub editor_type: Option<EditorType>,
    #[serde(rename = "isLibrary")]
    pub is_library: Option<bool>,
    #[serde(rename = "lastViewedAt")]
    pub last_viewed_at: Option<i64>,
}

#[derive(Clone, Debug, Default)]
pub struct FigmaState {
    pub active_tab: Option<FigmaTab>,
    pub last_focused_at: Option<Instant>,
}

impl FigmaState {
    pub fn is_idle(&self) -> bool {
        match self.last_focused_at {
            Some(ts) => ts.elapsed().as_secs() >= IDLE_THRESHOLD_SECONDS,
            None => true,
        }
    }

    pub fn state_key(&self) -> &'static str {
        match &self.active_tab {
            None => "browsing",
            Some(tab) => tab
                .editor_type
                .as_ref()
                .unwrap_or(&EditorType::default())
                .key(),
        }
    }

    pub fn status(&self) -> String {
        match &self.active_tab {
            None => "Browsing".to_string(),
            Some(tab) => tab
                .editor_type
                .as_ref()
                .unwrap_or(&EditorType::default())
                .to_string(),
        }
    }
}

pub fn find_figma_pid() -> Option<u32> {
    let mut sys = System::new();
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    sys.processes()
        .iter()
        .find(|(_, p)| p.name().to_string_lossy().to_lowercase().contains("figma"))
        .map(|(pid, _)| pid.as_u32())
}

pub fn is_figma_focused() -> bool {
    match active_win_pos_rs::get_active_window() {
        Ok(win) => {
            win.app_name.to_lowercase().contains("figma")
                || win.title.to_lowercase().contains("figma")
        }
        Err(_) => false,
    }
}

fn get_figma_settings_path() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        let config_dir = dirs::config_dir().ok_or_else(|| anyhow!("could not find config dir"))?;
        Ok(config_dir.join("Figma").join("settings.json"))
    }

    #[cfg(target_os = "macos")]
    {
        let home_dir = dirs::home_dir().ok_or_else(|| anyhow!("could not find home dir"))?;
        Ok(home_dir
            .join("Library")
            .join("Application Support")
            .join("Figma")
            .join("settings.json"))
    }

    #[cfg(target_os = "linux")]
    {
        let config_dir = dirs::config_dir().ok_or_else(|| anyhow!("could not find config dir"))?;
        Ok(config_dir.join("Figma").join("settings.json"))
    }
}

pub fn scan_figma_active_tab() -> Result<Option<FigmaTab>> {
    let path = get_figma_settings_path()?;

    let raw =
        fs::read_to_string(&path).map_err(|e| anyhow!("failed to read Figma settings: {}", e))?;

    let settings: FigmaSettings =
        serde_json::from_str(&raw).map_err(|e| anyhow!("failed to parse Figma settings: {}", e))?;

    let tab = settings
        .windows
        .unwrap_or_default()
        .iter()
        .flat_map(|w| w.tabs.iter().flatten())
        .max_by_key(|t| t.last_viewed_at)
        .cloned();

    Ok(tab)
}
