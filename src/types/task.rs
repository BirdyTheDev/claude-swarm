use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

use super::agent::AgentId;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TaskId(pub String);

impl TaskId {
    pub fn new() -> Self {
        Self(ulid::Ulid::new().to_string())
    }
}

impl fmt::Display for TaskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.0[self.0.len().saturating_sub(6)..])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TaskStatus {
    Pending,
    Assigned,
    InProgress,
    Completed,
    Failed,
}

impl fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Assigned => write!(f, "assigned"),
            Self::InProgress => write!(f, "in-progress"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
        }
    }
}

impl TaskStatus {
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Pending => "○",
            Self::Assigned => "◐",
            Self::InProgress => "▶",
            Self::Completed => "✓",
            Self::Failed => "✗",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TaskPriority {
    Low,
    Normal,
    High,
    Critical,
}

impl fmt::Display for TaskPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Normal => write!(f, "normal"),
            Self::High => write!(f, "high"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub description: String,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub required_skills: Vec<String>,
    pub assigned_to: Option<AgentId>,
    pub dependencies: Vec<TaskId>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub result: Option<String>,
}

impl Task {
    pub fn new(description: String, priority: TaskPriority, required_skills: Vec<String>) -> Self {
        Self {
            id: TaskId::new(),
            description,
            status: TaskStatus::Pending,
            priority,
            required_skills,
            assigned_to: None,
            dependencies: Vec::new(),
            created_at: Utc::now(),
            completed_at: None,
            result: None,
        }
    }

    pub fn is_ready(&self) -> bool {
        self.status == TaskStatus::Pending && self.dependencies.is_empty()
    }

    pub fn assign(&mut self, agent: AgentId) {
        self.assigned_to = Some(agent);
        self.status = TaskStatus::Assigned;
    }

    pub fn start(&mut self) {
        self.status = TaskStatus::InProgress;
    }

    pub fn complete(&mut self, result: String) {
        self.status = TaskStatus::Completed;
        self.completed_at = Some(Utc::now());
        self.result = Some(result);
    }

    pub fn fail(&mut self, reason: String) {
        self.status = TaskStatus::Failed;
        self.completed_at = Some(Utc::now());
        self.result = Some(reason);
    }
}
