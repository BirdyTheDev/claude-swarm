use claude_swarm::agent::stream_parser::parse_stream_message;
use claude_swarm::types::agent::AgentId;
use claude_swarm::types::event::AppEvent;
use claude_swarm::types::message::ClaudeStreamMessage;

#[test]
fn test_parse_fixture_file() {
    let content = std::fs::read_to_string("tests/fixtures/sample_stream.ndjson").unwrap();
    let agent_id = AgentId::new("test-agent");
    let mut all_events = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(msg) = ClaudeStreamMessage::parse(line) {
            let events = parse_stream_message(&agent_id, &msg);
            all_events.extend(events);
        }
    }

    // Should have: AgentReady, TextOutput, ToolUse, TextOutput, AgentCompleted
    assert_eq!(all_events.len(), 5);

    assert!(matches!(all_events[0], AppEvent::AgentReady { .. }));

    match &all_events[1] {
        AppEvent::AgentTextOutput { text, .. } => {
            assert!(text.contains("help you with that task"));
        }
        other => panic!("expected AgentTextOutput, got: {other:?}"),
    }

    match &all_events[2] {
        AppEvent::AgentToolUse { tool_name, .. } => {
            assert_eq!(tool_name, "Read");
        }
        other => panic!("expected AgentToolUse, got: {other:?}"),
    }

    match &all_events[3] {
        AppEvent::AgentTextOutput { text, .. } => {
            assert!(text.contains("analysis"));
        }
        other => panic!("expected AgentTextOutput, got: {other:?}"),
    }

    match &all_events[4] {
        AppEvent::AgentCompleted { cost_usd, .. } => {
            assert_eq!(*cost_usd, Some(0.0342));
        }
        other => panic!("expected AgentCompleted, got: {other:?}"),
    }
}

#[test]
fn test_unknown_message_types_dont_crash() {
    let agent_id = AgentId::new("test");
    let msg = ClaudeStreamMessage::parse(
        r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed"}}"#,
    );
    assert!(msg.is_some());
    let events = parse_stream_message(&agent_id, &msg.unwrap());
    assert_eq!(events.len(), 0);
}
