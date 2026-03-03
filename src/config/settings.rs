use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    En,
    Tr,
}

impl Language {
    pub fn next(&self) -> Self {
        match self {
            Self::En => Self::Tr,
            Self::Tr => Self::En,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::En => "English",
            Self::Tr => "Türkçe",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeName {
    Dark,
    Light,
    HighContrast,
}

impl ThemeName {
    pub fn next(&self) -> Self {
        match self {
            Self::Dark => Self::Light,
            Self::Light => Self::HighContrast,
            Self::HighContrast => Self::Dark,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Dark => "Dark",
            Self::Light => "Light",
            Self::HighContrast => "High Contrast",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogVerbosity {
    Minimal,
    Normal,
    Detailed,
}

impl LogVerbosity {
    pub fn next(&self) -> Self {
        match self {
            Self::Minimal => Self::Normal,
            Self::Normal => Self::Detailed,
            Self::Detailed => Self::Minimal,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Minimal => "Minimal",
            Self::Normal => "Normal",
            Self::Detailed => "Detailed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub language: Language,
    pub theme: ThemeName,
    pub log_verbosity: LogVerbosity,
    pub terminal_app: String,
    pub input_history_size: usize,
    pub meeting_timeout_secs: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: Language::En,
            theme: ThemeName::Dark,
            log_verbosity: LogVerbosity::Normal,
            terminal_app: "Terminal".to_string(),
            input_history_size: 50,
            meeting_timeout_secs: 10,
        }
    }
}

impl Settings {
    fn config_dir() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".claude-swarm"))
    }

    fn config_path() -> Option<PathBuf> {
        Self::config_dir().map(|d| d.join("settings.toml"))
    }

    /// Load settings from `~/.claude-swarm/settings.toml`.
    /// Creates the file with defaults if it doesn't exist.
    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };

        if !path.exists() {
            let settings = Self::default();
            let _ = settings.save();
            return settings;
        }

        match std::fs::read_to_string(&path) {
            Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save settings to `~/.claude-swarm/settings.toml`.
    pub fn save(&self) -> Result<(), String> {
        let Some(dir) = Self::config_dir() else {
            return Err("Cannot determine home directory".to_string());
        };
        std::fs::create_dir_all(&dir)
            .map_err(|e| format!("Cannot create config dir: {e}"))?;

        let path = dir.join("settings.toml");
        let content = toml::to_string_pretty(self)
            .map_err(|e| format!("Cannot serialize settings: {e}"))?;
        std::fs::write(&path, content)
            .map_err(|e| format!("Cannot write settings: {e}"))?;
        Ok(())
    }
}
