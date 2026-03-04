use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemeName {
    #[default]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LogVerbosity {
    Minimal,
    #[default]
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
    #[serde(default)]
    pub language: Language,
    #[serde(default)]
    pub theme: ThemeName,
    #[serde(default)]
    pub log_verbosity: LogVerbosity,
    #[serde(default = "default_terminal_app")]
    pub terminal_app: String,
    #[serde(default = "default_history_size")]
    pub input_history_size: usize,
    #[serde(default = "default_meeting_timeout")]
    pub meeting_timeout_secs: u64,
    #[serde(default)]
    pub auto_readme: bool,
    #[serde(default)]
    pub auto_verify: bool,
    #[serde(default)]
    pub verify_command: String,
    #[serde(default = "default_max_verify_retries")]
    pub max_verify_retries: u32,
    #[serde(default)]
    pub telegram_enabled: bool,
    #[serde(default)]
    pub telegram_bot_token: String,
    #[serde(default)]
    pub telegram_chat_id: String,
}

fn default_terminal_app() -> String {
    "Terminal".to_string()
}

fn default_history_size() -> usize {
    50
}

fn default_meeting_timeout() -> u64 {
    10
}

fn default_max_verify_retries() -> u32 {
    3
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            language: Language::En,
            theme: ThemeName::Dark,
            log_verbosity: LogVerbosity::Normal,
            terminal_app: default_terminal_app(),
            input_history_size: default_history_size(),
            meeting_timeout_secs: default_meeting_timeout(),
            auto_readme: false,
            auto_verify: false,
            verify_command: String::new(),
            max_verify_retries: default_max_verify_retries(),
            telegram_enabled: false,
            telegram_bot_token: String::new(),
            telegram_chat_id: String::new(),
        }
    }
}

impl Settings {
    /// Returns the bot token with all but the last 4 characters masked.
    pub fn masked_bot_token(&self) -> String {
        let token = &self.telegram_bot_token;
        if token.len() <= 4 {
            return "****".to_string();
        }
        let visible = &token[token.len() - 4..];
        format!("****{visible}")
    }

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
