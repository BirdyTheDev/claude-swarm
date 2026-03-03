use serde::{Deserialize, Serialize};

/// Represents a message from the Claude CLI NDJSON stream protocol.
/// Uses untagged deserialization with a manual type field to handle unknown message types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawStreamMessage {
    #[serde(rename = "type")]
    pub msg_type: String,

    #[serde(flatten)]
    pub fields: serde_json::Value,
}

#[derive(Debug, Clone)]
pub enum ClaudeStreamMessage {
    System(SystemMessage),
    Assistant(AssistantMessage),
    User(UserMessage),
    Result(ResultMessage),
    Unknown { msg_type: String },
}

impl ClaudeStreamMessage {
    pub fn parse(line: &str) -> Option<Self> {
        let raw: RawStreamMessage = serde_json::from_str(line).ok()?;
        match raw.msg_type.as_str() {
            "system" => {
                let msg: SystemMessage = serde_json::from_str(line).ok()?;
                Some(Self::System(msg))
            }
            "assistant" => {
                let msg: AssistantMessage = serde_json::from_str(line).ok()?;
                Some(Self::Assistant(msg))
            }
            "user" => {
                let msg: UserMessage = serde_json::from_str(line).ok()?;
                Some(Self::User(msg))
            }
            "result" => {
                let msg: ResultMessage = serde_json::from_str(line).ok()?;
                Some(Self::Result(msg))
            }
            other => Some(Self::Unknown {
                msg_type: other.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMessage {
    #[serde(default)]
    pub subtype: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub tools: Option<Vec<String>>,
    #[serde(default)]
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantMessage {
    #[serde(default)]
    pub subtype: Option<String>,
    #[serde(default)]
    pub message: Option<AssistantContent>,
    #[serde(default)]
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssistantContent {
    #[serde(default)]
    pub content: Vec<ContentBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        #[serde(default)]
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        #[serde(default)]
        content: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserMessage {
    #[serde(default)]
    pub message: Option<UserContent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserContent {
    #[serde(default)]
    pub content: Vec<ContentBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResultMessage {
    #[serde(default)]
    pub subtype: Option<String>,
    #[serde(default)]
    pub result: Option<String>,
    #[serde(default)]
    pub is_error: Option<bool>,
    #[serde(default)]
    pub session_id: Option<String>,
    /// Claude Code uses total_cost_usd
    #[serde(default, alias = "cost_usd")]
    pub total_cost_usd: Option<f64>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub duration_api_ms: Option<u64>,
    #[serde(default)]
    pub num_turns: Option<u32>,
    #[serde(default)]
    pub usage: Option<UsageInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageInfo {
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_read_input_tokens: u64,
    #[serde(default)]
    pub cache_creation_input_tokens: u64,
}
