use tracing::info;

use crate::types::agent::AgentId;
use crate::types::communication::{InterAgentMessage, MessageContent};

/// Manages inter-agent message routing.
pub struct MessageRouter {
    /// Log of all routed messages
    message_log: Vec<InterAgentMessage>,
}

impl MessageRouter {
    pub fn new() -> Self {
        Self {
            message_log: Vec::new(),
        }
    }

    /// Create and log a message. Returns the formatted prompt string to inject.
    pub fn route_message(&mut self, message: InterAgentMessage) -> String {
        info!(
            from = %message.from,
            to = %message.to,
            "routing inter-agent message"
        );
        let prompt = message.format_for_recipient();
        self.message_log.push(message);
        prompt
    }

    /// Create a text message between agents.
    pub fn create_text_message(from: AgentId, to: AgentId, text: String) -> InterAgentMessage {
        InterAgentMessage::new(from, to, MessageContent::Text(text))
    }

    pub fn message_log(&self) -> &[InterAgentMessage] {
        &self.message_log
    }

    pub fn message_count(&self) -> usize {
        self.message_log.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_message() {
        let mut router = MessageRouter::new();
        let msg = MessageRouter::create_text_message(
            AgentId::new("arch"),
            AgentId::new("dev"),
            "Review the auth module".to_string(),
        );
        let prompt = router.route_message(msg);
        assert!(prompt.contains("[Message from agent 'arch']"));
        assert!(prompt.contains("Review the auth module"));
        assert!(prompt.contains("[End message]"));
        assert_eq!(router.message_count(), 1);
    }
}
