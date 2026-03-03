/// Category of a log entry, determines icon and filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogCategory {
    Agent,
    Task,
    Team,
    Communication,
    System,
    Error,
}

impl LogCategory {
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Agent => "🤖",
            Self::Task => "📋",
            Self::Team => "👥",
            Self::Communication => "💬",
            Self::System => "⚙",
            Self::Error => "❌",
        }
    }
}

/// A structured log entry with category, timestamp, and detail.
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub category: LogCategory,
    pub message: String,
    pub detail: Option<String>,
}

impl LogEntry {
    pub fn new(category: LogCategory, message: String) -> Self {
        let timestamp = chrono::Utc::now().format("%H:%M:%S").to_string();
        Self {
            timestamp,
            category,
            message,
            detail: None,
        }
    }

    pub fn with_detail(mut self, detail: String) -> Self {
        self.detail = Some(detail);
        self
    }

    /// Format as a single display line: `[HH:MM:SS] [icon] message`
    pub fn display(&self) -> String {
        format!("[{}] {} {}", self.timestamp, self.category.icon(), self.message)
    }
}
