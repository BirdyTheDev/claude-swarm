use anyhow::Result;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tracing::info;

use super::handle::AgentHandle;
use crate::types::agent::{AgentConfig, AgentId};
use crate::types::message::ClaudeStreamMessage;

/// Collection of agent handles.
pub struct AgentPool {
    agents: HashMap<AgentId, AgentHandle>,
    output_tx: mpsc::Sender<(AgentId, ClaudeStreamMessage)>,
    /// If set, agents will open visible Terminal.app windows.
    visible_terminals: bool,
    /// Directory for log files when using visible terminals.
    log_dir: Option<PathBuf>,
}

impl AgentPool {
    pub fn new(output_tx: mpsc::Sender<(AgentId, ClaudeStreamMessage)>) -> Self {
        Self {
            agents: HashMap::new(),
            output_tx,
            visible_terminals: false,
            log_dir: None,
        }
    }

    /// Enable visible terminal windows for all agents.
    pub fn set_visible_terminals(&mut self, log_dir: PathBuf) {
        self.visible_terminals = true;
        self.log_dir = Some(log_dir);
    }

    pub fn register_agent(&mut self, id: AgentId, config: AgentConfig) {
        info!(agent = %id, role = %config.role, "registering agent");
        let handle = AgentHandle::new(id.clone(), config, self.output_tx.clone());
        let handle = if self.visible_terminals {
            if let Some(ref dir) = self.log_dir {
                handle.with_visible_terminal(dir.clone())
            } else {
                handle
            }
        } else {
            handle
        };
        self.agents.insert(id, handle);
    }

    pub fn send_prompt(&self, agent_id: &AgentId, prompt: &str) -> Result<()> {
        if let Some(handle) = self.agents.get(agent_id) {
            handle.send_prompt(prompt.to_string());
            Ok(())
        } else {
            anyhow::bail!("agent '{}' not found in pool", agent_id)
        }
    }

    pub fn remove_agent(&mut self, agent_id: &AgentId) {
        self.agents.remove(agent_id);
    }

    pub fn get(&self, agent_id: &AgentId) -> Option<&AgentHandle> {
        self.agents.get(agent_id)
    }

    pub fn agent_ids(&self) -> Vec<AgentId> {
        self.agents.keys().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.agents.len()
    }

    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }
}
