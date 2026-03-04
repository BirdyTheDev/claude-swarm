use super::agent::AgentId;
use super::communication::InterAgentMessage;
use super::message::ClaudeStreamMessage;
use super::task::{Task, TaskId, TaskPriority};

/// Events that flow through the application.
#[derive(Debug, Clone)]
pub enum AppEvent {
    // Terminal events
    Key(crossterm::event::KeyEvent),
    Tick,
    Resize(u16, u16),

    // Agent events
    AgentOutput {
        agent_id: AgentId,
        message: ClaudeStreamMessage,
    },
    AgentTextOutput {
        agent_id: AgentId,
        text: String,
    },
    AgentToolUse {
        agent_id: AgentId,
        tool_name: String,
        tool_id: String,
    },
    AgentCompleted {
        agent_id: AgentId,
        cost_usd: Option<f64>,
    },
    AgentError {
        agent_id: AgentId,
        error: String,
    },
    AgentReady {
        agent_id: AgentId,
    },

    // Task events
    TaskCreated(Task),
    TaskAssigned {
        task_id: TaskId,
        agent_id: AgentId,
    },
    TaskCompleted {
        task_id: TaskId,
        result: String,
    },

    // Communication events
    MessageRouted(InterAgentMessage),

    // Team task events
    TeamTaskUpdate {
        phase: TeamTaskPhase,
        description: String,
    },

    // Telegram notification (for system log)
    TelegramNotify {
        text: String,
    },

    // Telegram pairing completed
    TelegramPaired {
        chat_id: String,
    },

    // Soul updated
    SoulUpdated {
        agent_id: AgentId,
        soul: String,
    },

    // Build verification result
    VerifyResult {
        agent_id: AgentId,
        success: bool,
        output: Option<String>,
    },

    // Telegram query events
    TelegramStatusRequest,
    TelegramCostRequest,

    // Telegram task tracking — route through app so Tasks view is updated
    TelegramTaskPrompt {
        agent_id: AgentId,
        prompt: String,
    },
    TelegramTeamTask {
        description: String,
    },

    // Telegram scheduled tasks
    TelegramSchedule {
        time: String,
        command: String,
    },
    TelegramSchedulesList,

    // System events
    Shutdown,
}

/// Phase of a team task execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TeamTaskPhase {
    Planning,
    Executing,
    Synthesizing,
    Completed,
}

/// Commands sent to the orchestrator actor.
#[derive(Debug, Clone)]
pub enum OrchestratorCommand {
    SpawnAgent {
        id: AgentId,
    },
    SendPrompt {
        agent_id: AgentId,
        prompt: String,
    },
    CreateTask {
        description: String,
        priority: TaskPriority,
        skills: Vec<String>,
    },
    RouteMessage {
        message: InterAgentMessage,
    },
    StopAgent {
        agent_id: AgentId,
    },
    TeamTask {
        description: String,
    },
    Broadcast {
        prompt: String,
    },
    PromptLead {
        prompt: String,
    },
    SetSoul {
        agent_id: AgentId,
        soul: String,
    },
    Shutdown,
}

/// UI display modes / views.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiMode {
    Normal,
    Command,
    Prompt,
    Help,
    TaskInput,
    SettingsEdit,
}

/// Which view tab is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewTab {
    Dashboard,
    AgentDetail,
    Tasks,
    Logs,
    Office,
    Settings,
    Performance,
}

impl ViewTab {
    pub fn title(&self) -> &'static str {
        match self {
            Self::Dashboard => "Dashboard",
            Self::AgentDetail => "Agent Detail",
            Self::Tasks => "Tasks",
            Self::Logs => "Logs",
            Self::Office => "Office",
            Self::Settings => "Settings",
            Self::Performance => "Performance",
        }
    }

    pub fn all() -> &'static [ViewTab] {
        &[
            ViewTab::Dashboard,
            ViewTab::AgentDetail,
            ViewTab::Tasks,
            ViewTab::Logs,
            ViewTab::Office,
            ViewTab::Settings,
            ViewTab::Performance,
        ]
    }
}
