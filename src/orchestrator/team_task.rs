use std::collections::HashMap;

use crate::types::agent::AgentId;

/// Phases of team task execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TeamTaskPhase {
    /// Lead agent is planning and breaking down the task.
    Planning,
    /// Subtasks are being executed by individual agents.
    Executing,
    /// Lead agent is synthesizing all results.
    Synthesizing,
}

/// A subtask parsed from the lead agent's plan.
#[derive(Debug, Clone)]
pub struct Subtask {
    pub agent_name: String,
    pub description: String,
}

/// Tracks the state of an active team task.
pub struct TeamTaskState {
    pub description: String,
    pub phase: TeamTaskPhase,
    pub lead_agent: AgentId,
    pub subtasks: Vec<Subtask>,
    /// Results collected from agents: agent_name -> result text
    pub results: HashMap<String, String>,
    /// Agents we're still waiting on
    pub pending_agents: Vec<AgentId>,
}

impl TeamTaskState {
    pub fn new(description: String, lead_agent: AgentId) -> Self {
        Self {
            description,
            phase: TeamTaskPhase::Planning,
            lead_agent,
            subtasks: Vec::new(),
            results: HashMap::new(),
            pending_agents: Vec::new(),
        }
    }

    /// Check if all executing agents have completed.
    pub fn all_subtasks_done(&self) -> bool {
        self.pending_agents.is_empty() && !self.subtasks.is_empty()
    }

    /// Record a result from an agent.
    pub fn record_result(&mut self, agent_id: &AgentId, result: String) {
        self.results.insert(agent_id.0.clone(), result);
        self.pending_agents.retain(|a| a != agent_id);
    }

    /// Build the synthesis prompt from all collected results.
    pub fn build_synthesis_prompt(&self) -> String {
        let mut prompt = format!(
            "You are synthesizing the results of a team task.\n\
            Original task: {}\n\n\
            Here are the results from each team member:\n\n",
            self.description
        );

        for (agent_name, result) in &self.results {
            prompt.push_str(&format!(
                "=== Result from '{}' ===\n{}\n\n",
                agent_name, result
            ));
        }

        prompt.push_str(
            "Please synthesize these results into a coherent, comprehensive response. \
            Highlight key findings, resolve any conflicts, and provide a unified summary.",
        );

        prompt
    }
}

/// Parse the lead agent's plan output to extract subtasks.
///
/// Supports multiple formats:
/// ```text
/// SUBTASK @agent_name: description
/// @agent_name: description
/// - agent_name: description
/// agent_name: description (at start of line, if name matches known agents)
/// ```
pub fn parse_subtask_plan(output: &str, known_agents: &[String]) -> Vec<Subtask> {
    let mut subtasks = Vec::new();

    for line in output.lines() {
        let trimmed = line.trim();

        // Format 1: SUBTASK @agent_name: description
        if let Some(rest) = trimmed.strip_prefix("SUBTASK @") {
            if let Some(sub) = parse_agent_colon(rest) {
                subtasks.push(sub);
                continue;
            }
        }

        // Format 2: SUBTASK agent_name: description (no @)
        if let Some(rest) = trimmed.strip_prefix("SUBTASK ") {
            let rest = rest.trim_start_matches('@');
            if let Some(sub) = parse_agent_colon(rest) {
                subtasks.push(sub);
                continue;
            }
        }

        // Format 3: @agent_name: description (at line start or after bullet)
        let stripped = trimmed
            .trim_start_matches('-')
            .trim_start_matches('*')
            .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.')
            .trim();
        if let Some(rest) = stripped.strip_prefix('@') {
            if let Some(sub) = parse_agent_colon(rest) {
                if known_agents.iter().any(|a| a == &sub.agent_name) {
                    subtasks.push(sub);
                    continue;
                }
            }
        }

        // Format 4: **agent_name**: description or agent_name: description
        // Only if the first word matches a known agent name
        let clean = stripped
            .trim_start_matches('*')
            .trim_start_matches('`');
        if let Some(sub) = parse_agent_colon(clean) {
            let name_clean = sub.agent_name.trim_end_matches('*').trim_end_matches('`').to_string();
            if known_agents.iter().any(|a| a == &name_clean) {
                subtasks.push(Subtask {
                    agent_name: name_clean,
                    description: sub.description,
                });
            }
        }
    }

    // Deduplicate by agent name (keep last)
    let mut seen = std::collections::HashSet::new();
    let mut deduped = Vec::new();
    for sub in subtasks.into_iter().rev() {
        if seen.insert(sub.agent_name.clone()) {
            deduped.push(sub);
        }
    }
    deduped.reverse();
    deduped
}

/// Helper: parse "agent_name: description" from a string.
fn parse_agent_colon(s: &str) -> Option<Subtask> {
    let colon_pos = s.find(':')?;
    let agent_name = s[..colon_pos].trim().to_string();
    let description = s[colon_pos + 1..].trim().to_string();
    if agent_name.is_empty()
        || description.is_empty()
        || agent_name.contains(' ')
        || agent_name.len() > 30
    {
        return None;
    }
    Some(Subtask {
        agent_name,
        description,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn agents() -> Vec<String> {
        vec![
            "developer".to_string(),
            "reviewer".to_string(),
            "architect".to_string(),
        ]
    }

    #[test]
    fn test_parse_subtask_format() {
        let output = "\
Here's my plan:

SUBTASK @developer: Implement the authentication module with JWT tokens
SUBTASK @reviewer: Review the existing codebase for security issues

That covers everything.";

        let subtasks = parse_subtask_plan(output, &agents());
        assert_eq!(subtasks.len(), 2);
        assert_eq!(subtasks[0].agent_name, "developer");
        assert!(subtasks[0].description.contains("authentication"));
        assert_eq!(subtasks[1].agent_name, "reviewer");
    }

    #[test]
    fn test_parse_mention_format() {
        let output = "\
@developer: Implement the auth module
@reviewer: Check for security issues";

        let subtasks = parse_subtask_plan(output, &agents());
        assert_eq!(subtasks.len(), 2);
        assert_eq!(subtasks[0].agent_name, "developer");
        assert_eq!(subtasks[1].agent_name, "reviewer");
    }

    #[test]
    fn test_parse_bold_format() {
        let output = "\
**developer**: Write the code for health endpoint
**reviewer**: Review the implementation";

        let subtasks = parse_subtask_plan(output, &agents());
        assert_eq!(subtasks.len(), 2);
        assert_eq!(subtasks[0].agent_name, "developer");
        assert_eq!(subtasks[1].agent_name, "reviewer");
    }

    #[test]
    fn test_parse_plain_name_format() {
        let output = "\
developer: Read subprocess.rs and find bugs
reviewer: Check app.rs for panics";

        let subtasks = parse_subtask_plan(output, &agents());
        assert_eq!(subtasks.len(), 2);
    }

    #[test]
    fn test_parse_empty_plan() {
        let output = "No subtasks here, just regular text.";
        let subtasks = parse_subtask_plan(output, &agents());
        assert_eq!(subtasks.len(), 0);
    }

    #[test]
    fn test_parse_malformed_subtask() {
        let output = "SUBTASK @: no agent name\nSUBTASK @dev:";
        let subtasks = parse_subtask_plan(output, &agents());
        assert_eq!(subtasks.len(), 0);
    }

    #[test]
    fn test_deduplicates_agents() {
        let output = "\
SUBTASK @developer: First task
@developer: Second task (overrides)
@reviewer: Check stuff";

        let subtasks = parse_subtask_plan(output, &agents());
        // developer appears twice, keep last
        assert_eq!(subtasks.len(), 2);
        assert_eq!(subtasks[0].agent_name, "developer");
        assert!(subtasks[0].description.contains("Second"));
        assert_eq!(subtasks[1].agent_name, "reviewer");
    }
}
