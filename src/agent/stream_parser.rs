use regex::Regex;
use std::sync::LazyLock;

use crate::types::agent::AgentId;
use crate::types::event::AppEvent;
use crate::types::message::{ClaudeStreamMessage, ContentBlock};

static MENTION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"@(\w+):\s*(.+)").unwrap());

/// Detect @agent_name: message patterns in text output.
/// Returns Vec of (mentioned_agent_name, message_text).
/// Skips lines that are SUBTASK directives (handled by team_task parser).
pub fn detect_mentions(text: &str) -> Vec<(String, String)> {
    let mut mentions = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        // Skip SUBTASK lines — those are handled by the team task parser
        if trimmed.starts_with("SUBTASK") {
            continue;
        }
        for cap in MENTION_RE.captures_iter(trimmed) {
            let agent_name = cap[1].to_string();
            let message = cap[2].trim().to_string();
            if !message.is_empty() {
                mentions.push((agent_name, message));
            }
        }
    }
    mentions
}

/// Converts a ClaudeStreamMessage into zero or more AppEvents.
pub fn parse_stream_message(agent_id: &AgentId, msg: &ClaudeStreamMessage) -> Vec<AppEvent> {
    let mut events = Vec::new();

    match msg {
        ClaudeStreamMessage::System(sys) => {
            if sys.subtype == "init" {
                events.push(AppEvent::AgentReady {
                    agent_id: agent_id.clone(),
                });
            }
        }
        ClaudeStreamMessage::Assistant(asst) => {
            if let Some(ref content) = asst.message {
                for block in &content.content {
                    match block {
                        ContentBlock::Text { text } => {
                            events.push(AppEvent::AgentTextOutput {
                                agent_id: agent_id.clone(),
                                text: text.clone(),
                            });
                        }
                        ContentBlock::ToolUse { id, name, .. } => {
                            events.push(AppEvent::AgentToolUse {
                                agent_id: agent_id.clone(),
                                tool_name: name.clone(),
                                tool_id: id.clone(),
                            });
                        }
                        ContentBlock::ToolResult { .. } => {}
                    }
                }
            }
        }
        ClaudeStreamMessage::Result(result) => {
            events.push(AppEvent::AgentCompleted {
                agent_id: agent_id.clone(),
                cost_usd: result.total_cost_usd,
            });
        }
        ClaudeStreamMessage::User(_) => {}
        ClaudeStreamMessage::Unknown { .. } => {}
    }

    events
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_system_init() {
        let agent_id = AgentId::new("test");
        let msg = ClaudeStreamMessage::parse(
            r#"{"type":"system","subtype":"init","session_id":"abc","tools":["Read","Write"]}"#,
        )
        .unwrap();
        let events = parse_stream_message(&agent_id, &msg);
        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], AppEvent::AgentReady { .. }));
    }

    #[test]
    fn test_parse_assistant_text() {
        let agent_id = AgentId::new("test");
        let msg = ClaudeStreamMessage::parse(
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"Hello world"}]}}"#,
        )
        .unwrap();
        let events = parse_stream_message(&agent_id, &msg);
        assert_eq!(events.len(), 1);
        match &events[0] {
            AppEvent::AgentTextOutput { text, .. } => assert_eq!(text, "Hello world"),
            _ => panic!("expected AgentTextOutput"),
        }
    }

    #[test]
    fn test_parse_assistant_tool_use() {
        let agent_id = AgentId::new("test");
        let msg = ClaudeStreamMessage::parse(
            r#"{"type":"assistant","message":{"content":[{"type":"tool_use","id":"tu_1","name":"Read","input":{"file_path":"/tmp/x"}}]}}"#,
        )
        .unwrap();
        let events = parse_stream_message(&agent_id, &msg);
        assert_eq!(events.len(), 1);
        match &events[0] {
            AppEvent::AgentToolUse { tool_name, .. } => assert_eq!(tool_name, "Read"),
            _ => panic!("expected AgentToolUse"),
        }
    }

    #[test]
    fn test_parse_result_with_total_cost() {
        let agent_id = AgentId::new("test");
        let msg = ClaudeStreamMessage::parse(
            r#"{"type":"result","subtype":"success","total_cost_usd":0.05,"duration_ms":1234}"#,
        )
        .unwrap();
        let events = parse_stream_message(&agent_id, &msg);
        assert_eq!(events.len(), 1);
        match &events[0] {
            AppEvent::AgentCompleted { cost_usd, .. } => {
                assert_eq!(*cost_usd, Some(0.05));
            }
            _ => panic!("expected AgentCompleted"),
        }
    }

    #[test]
    fn test_parse_unknown_type() {
        let agent_id = AgentId::new("test");
        let msg = ClaudeStreamMessage::parse(
            r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed"}}"#,
        )
        .unwrap();
        let events = parse_stream_message(&agent_id, &msg);
        assert_eq!(events.len(), 0); // Unknown types produce no events
    }
}
