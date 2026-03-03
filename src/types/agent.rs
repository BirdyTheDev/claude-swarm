use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AgentId(pub String);

impl AgentId {
    pub fn new(name: &str) -> Self {
        Self(name.to_string())
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for AgentId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PermissionMode {
    Default,
    Plan,
    AcceptEdits,
    BypassPermissions,
}

impl fmt::Display for PermissionMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Default => write!(f, "default"),
            Self::Plan => write!(f, "plan"),
            Self::AcceptEdits => write!(f, "acceptEdits"),
            Self::BypassPermissions => write!(f, "bypassPermissions"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub role: String,
    pub system_prompt: String,
    pub model: Option<String>,
    pub skills: Vec<String>,
    pub allowed_tools: Vec<String>,
    pub permission_mode: PermissionMode,
    pub max_turns: Option<u32>,
    pub max_budget_usd: Option<f64>,
    pub is_lead: bool,
}

impl AgentConfig {
    /// Build CLI args for a one-shot `-p "prompt"` invocation.
    pub fn to_cli_args(&self, prompt: &str, session_id: Option<&str>) -> Vec<String> {
        let mut args = vec![
            "-p".to_string(),
            "--output-format".to_string(),
            "stream-json".to_string(),
            "--verbose".to_string(),
        ];

        // Resume existing session for follow-up prompts
        if let Some(sid) = session_id {
            args.push("--resume".to_string());
            args.push(sid.to_string());
        }

        if !self.system_prompt.is_empty() {
            args.push("--system-prompt".to_string());
            args.push(self.system_prompt.clone());
        }

        if let Some(ref model) = self.model {
            args.push("--model".to_string());
            args.push(model.clone());
        }

        if !self.allowed_tools.is_empty() {
            args.push("--allowedTools".to_string());
            args.push(self.allowed_tools.join(","));
        }

        match self.permission_mode {
            PermissionMode::Default => {}
            PermissionMode::Plan => {
                args.push("--permission-mode".to_string());
                args.push("plan".to_string());
            }
            PermissionMode::AcceptEdits => {
                args.push("--permission-mode".to_string());
                args.push("acceptEdits".to_string());
            }
            PermissionMode::BypassPermissions => {
                args.push("--permission-mode".to_string());
                args.push("bypassPermissions".to_string());
            }
        }

        if let Some(budget) = self.max_budget_usd {
            args.push("--max-budget-usd".to_string());
            args.push(budget.to_string());
        }

        // Prompt must be the last positional argument
        args.push(prompt.to_string());

        args
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentStatus {
    Starting,
    Idle,
    Working,
    Waiting,
    Completed,
    Failed,
    Stopped,
    InMeeting,
}

impl fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Starting => write!(f, "starting"),
            Self::Idle => write!(f, "idle"),
            Self::Working => write!(f, "working"),
            Self::Waiting => write!(f, "waiting"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Stopped => write!(f, "stopped"),
            Self::InMeeting => write!(f, "in meeting"),
        }
    }
}

impl AgentStatus {
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Starting => "◔",
            Self::Idle => "●",
            Self::Working => "▶",
            Self::Waiting => "⏸",
            Self::Completed => "✓",
            Self::Failed => "✗",
            Self::Stopped => "■",
            Self::InMeeting => "⊕",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_creation_tokens: u64,
    pub cost_usd: f64,
}

impl TokenUsage {
    pub fn add_turn(&mut self, input: u64, output: u64, cache_read: u64, cache_creation: u64) {
        self.input_tokens += input;
        self.output_tokens += output;
        self.cache_read_tokens += cache_read;
        self.cache_creation_tokens += cache_creation;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub id: AgentId,
    pub config: AgentConfig,
    pub status: AgentStatus,
    pub usage: TokenUsage,
    pub output_lines: Vec<String>,
    pub current_task: Option<super::task::TaskId>,
    pub started_at: DateTime<Utc>,
    pub last_activity: DateTime<Utc>,
}

impl AgentState {
    pub fn new(id: AgentId, config: AgentConfig) -> Self {
        let now = Utc::now();
        Self {
            id,
            config,
            status: AgentStatus::Starting,
            usage: TokenUsage::default(),
            output_lines: Vec::new(),
            current_task: None,
            started_at: now,
            last_activity: now,
        }
    }

    pub fn append_output(&mut self, line: String) {
        self.output_lines.push(line);
        self.last_activity = Utc::now();
    }
}
