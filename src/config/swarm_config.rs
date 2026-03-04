use anyhow::{Context, Result, bail};
use serde::Deserialize;
use std::path::Path;

use crate::types::agent::{AgentConfig, AgentId, PermissionMode};

#[derive(Debug, Deserialize)]
pub struct SwarmConfig {
    #[serde(default = "default_swarm_name")]
    pub name: String,

    #[serde(default)]
    pub description: String,

    pub agent: Vec<AgentToml>,
}

fn default_swarm_name() -> String {
    "claude-swarm".to_string()
}

#[derive(Debug, Deserialize)]
pub struct AgentToml {
    pub name: String,

    #[serde(default)]
    pub role: String,

    #[serde(default)]
    pub system_prompt: String,

    #[serde(default)]
    pub soul: String,

    #[serde(default)]
    pub model: Option<String>,

    #[serde(default)]
    pub skills: Vec<String>,

    #[serde(default)]
    pub allowed_tools: Vec<String>,

    #[serde(default = "default_permission_mode")]
    pub permission_mode: String,

    #[serde(default)]
    pub max_turns: Option<u32>,

    #[serde(default)]
    pub max_budget_usd: Option<f64>,

    #[serde(default)]
    pub is_lead: bool,
}

fn default_permission_mode() -> String {
    "default".to_string()
}

impl SwarmConfig {
    pub fn load(path: &Path) -> Result<Self> {
        let content =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let config: SwarmConfig =
            toml::from_str(&content).with_context(|| format!("parsing {}", path.display()))?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> Result<()> {
        if self.agent.is_empty() {
            bail!("config must define at least one [[agent]]");
        }

        let lead_count = self.agent.iter().filter(|a| a.is_lead).count();
        if lead_count == 0 {
            bail!("config must have exactly one lead agent (is_lead = true)");
        }
        if lead_count > 1 {
            bail!("config must have exactly one lead agent, found {lead_count}");
        }

        let mut names = std::collections::HashSet::new();
        for agent in &self.agent {
            if agent.name.is_empty() {
                bail!("agent name cannot be empty");
            }
            if !names.insert(&agent.name) {
                bail!("duplicate agent name: '{}'", agent.name);
            }
        }

        Ok(())
    }

    pub fn agent_configs(&self) -> Vec<(AgentId, AgentConfig)> {
        self.agent
            .iter()
            .map(|a| {
                let perm = match a.permission_mode.as_str() {
                    "plan" => PermissionMode::Plan,
                    "acceptEdits" | "accept-edits" => PermissionMode::AcceptEdits,
                    "bypassPermissions" | "bypass-permissions" | "full-auto" => {
                        PermissionMode::BypassPermissions
                    }
                    _ => PermissionMode::Default,
                };

                let id = AgentId::new(&a.name);
                let config = AgentConfig {
                    name: a.name.clone(),
                    role: a.role.clone(),
                    system_prompt: a.system_prompt.clone(),
                    soul: a.soul.clone(),
                    model: a.model.clone(),
                    skills: a.skills.clone(),
                    allowed_tools: a.allowed_tools.clone(),
                    permission_mode: perm,
                    max_turns: a.max_turns,
                    max_budget_usd: a.max_budget_usd,
                    is_lead: a.is_lead,
                };
                (id, config)
            })
            .collect()
    }

    pub fn lead_agent_id(&self) -> AgentId {
        // Safe: validate() ensures exactly one lead exists before this is callable
        let lead = self
            .agent
            .iter()
            .find(|a| a.is_lead)
            .expect("BUG: no lead agent after validation");
        AgentId::new(&lead.name)
    }

    /// Filter to only specified agent names (if provided)
    pub fn filter_agents(&mut self, names: &[String]) {
        if names.is_empty() {
            return;
        }
        // Always keep the lead agent
        self.agent
            .retain(|a| a.is_lead || names.iter().any(|n| n == &a.name));
    }
}
