use std::collections::HashMap;

use crate::types::agent::{AgentConfig, AgentId, AgentState, AgentStatus};

/// Tracks agent state for all agents in the swarm.
pub struct AgentRegistry {
    agents: HashMap<AgentId, AgentState>,
    /// Ordered list of agent IDs for consistent display order
    order: Vec<AgentId>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            agents: HashMap::new(),
            order: Vec::new(),
        }
    }

    pub fn register(&mut self, id: AgentId, config: AgentConfig) {
        let state = AgentState::new(id.clone(), config);
        self.agents.insert(id.clone(), state);
        if !self.order.contains(&id) {
            self.order.push(id);
        }
    }

    pub fn get(&self, id: &AgentId) -> Option<&AgentState> {
        self.agents.get(id)
    }

    pub fn get_mut(&mut self, id: &AgentId) -> Option<&mut AgentState> {
        self.agents.get_mut(id)
    }

    pub fn set_status(&mut self, id: &AgentId, status: AgentStatus) {
        if let Some(state) = self.agents.get_mut(id) {
            state.status = status;
        }
    }

    pub fn ordered_ids(&self) -> &[AgentId] {
        &self.order
    }

    pub fn all_states(&self) -> Vec<&AgentState> {
        self.order
            .iter()
            .filter_map(|id| self.agents.get(id))
            .collect()
    }

    pub fn agents_with_skill(&self, skill: &str) -> Vec<&AgentState> {
        self.agents
            .values()
            .filter(|state| state.config.skills.iter().any(|s| s == skill))
            .collect()
    }

    pub fn idle_agents(&self) -> Vec<&AgentState> {
        self.agents
            .values()
            .filter(|state| state.status == AgentStatus::Idle)
            .collect()
    }

    pub fn total_cost(&self) -> f64 {
        self.agents.values().map(|s| s.usage.cost_usd).sum()
    }

    pub fn len(&self) -> usize {
        self.agents.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::agent::PermissionMode;

    fn test_config(name: &str, skills: Vec<&str>) -> AgentConfig {
        AgentConfig {
            name: name.to_string(),
            role: "test".to_string(),
            system_prompt: String::new(),
            soul: String::new(),
            model: None,
            skills: skills.into_iter().map(String::from).collect(),
            allowed_tools: Vec::new(),
            permission_mode: PermissionMode::Default,
            max_turns: None,
            max_budget_usd: None,
            is_lead: false,
        }
    }

    #[test]
    fn test_register_and_get() {
        let mut reg = AgentRegistry::new();
        let id = AgentId::new("arch");
        reg.register(id.clone(), test_config("arch", vec!["planning"]));
        assert!(reg.get(&id).is_some());
        assert_eq!(reg.len(), 1);
    }

    #[test]
    fn test_skill_filter() {
        let mut reg = AgentRegistry::new();
        reg.register(
            AgentId::new("arch"),
            test_config("arch", vec!["planning", "architecture"]),
        );
        reg.register(
            AgentId::new("dev"),
            test_config("dev", vec!["coding", "testing"]),
        );
        let planners = reg.agents_with_skill("planning");
        assert_eq!(planners.len(), 1);
        assert_eq!(planners[0].id.0, "arch");
    }

    #[test]
    fn test_ordered_ids() {
        let mut reg = AgentRegistry::new();
        reg.register(AgentId::new("a"), test_config("a", vec![]));
        reg.register(AgentId::new("b"), test_config("b", vec![]));
        reg.register(AgentId::new("c"), test_config("c", vec![]));
        let ids: Vec<&str> = reg.ordered_ids().iter().map(|id| id.0.as_str()).collect();
        assert_eq!(ids, vec!["a", "b", "c"]);
    }
}
