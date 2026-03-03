use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::agent::AgentId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContent {
    Text(String),
    TaskResult {
        task_description: String,
        result: String,
    },
    WorkRequest {
        description: String,
        priority: String,
    },
    SharedArtifact {
        name: String,
        content: String,
    },
}

impl MessageContent {
    pub fn as_prompt_injection(&self) -> String {
        match self {
            Self::Text(text) => text.clone(),
            Self::TaskResult {
                task_description,
                result,
            } => {
                format!(
                    "Task completed: {task_description}\nResult: {result}"
                )
            }
            Self::WorkRequest {
                description,
                priority,
            } => {
                format!(
                    "Work request (priority: {priority}): {description}"
                )
            }
            Self::SharedArtifact { name, content } => {
                format!("Shared artifact '{name}':\n{content}")
            }
        }
    }

    pub fn summary(&self) -> String {
        match self {
            Self::Text(t) => {
                let chars: Vec<char> = t.chars().collect();
                if chars.len() > 80 {
                    let truncated: String = chars[..77].iter().collect();
                    format!("{}...", truncated)
                } else {
                    t.clone()
                }
            }
            Self::TaskResult {
                task_description, ..
            } => format!("[TaskResult] {task_description}"),
            Self::WorkRequest { description, .. } => format!("[WorkRequest] {description}"),
            Self::SharedArtifact { name, .. } => format!("[Artifact] {name}"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterAgentMessage {
    pub id: String,
    pub from: AgentId,
    pub to: AgentId,
    pub content: MessageContent,
    pub timestamp: DateTime<Utc>,
}

impl InterAgentMessage {
    pub fn new(from: AgentId, to: AgentId, content: MessageContent) -> Self {
        Self {
            id: ulid::Ulid::new().to_string(),
            from,
            to,
            content,
            timestamp: Utc::now(),
        }
    }

    pub fn format_for_recipient(&self) -> String {
        format!(
            "[Message from agent '{}']\n{}\n[End message]",
            self.from,
            self.content.as_prompt_injection()
        )
    }
}
