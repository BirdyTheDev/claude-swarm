use std::collections::HashMap;
use tracing::info;

use crate::types::agent::AgentId;
use crate::types::task::{Task, TaskId, TaskPriority, TaskStatus};

use super::registry::AgentRegistry;

/// Manages task queue and assignment.
pub struct TaskScheduler {
    tasks: HashMap<TaskId, Task>,
    /// Ordered list for consistent display
    order: Vec<TaskId>,
}

impl TaskScheduler {
    pub fn new() -> Self {
        Self {
            tasks: HashMap::new(),
            order: Vec::new(),
        }
    }

    pub fn add_task(&mut self, task: Task) -> TaskId {
        let id = task.id.clone();
        info!(task_id = %id, desc = %task.description, "task created");
        self.order.push(id.clone());
        self.tasks.insert(id.clone(), task);
        id
    }

    pub fn create_task(
        &mut self,
        description: String,
        priority: TaskPriority,
        skills: Vec<String>,
    ) -> TaskId {
        let task = Task::new(description, priority, skills);
        self.add_task(task)
    }

    pub fn get(&self, id: &TaskId) -> Option<&Task> {
        self.tasks.get(id)
    }

    pub fn get_mut(&mut self, id: &TaskId) -> Option<&mut Task> {
        self.tasks.get_mut(id)
    }

    /// Find the best available agent for the next pending task.
    pub fn find_assignment(&self, registry: &AgentRegistry) -> Option<(TaskId, AgentId)> {
        // Get pending tasks sorted by priority (highest first)
        let mut pending: Vec<&Task> = self
            .tasks
            .values()
            .filter(|t| t.is_ready())
            .collect();
        pending.sort_by(|a, b| b.priority.cmp(&a.priority));

        for task in pending {
            if let Some(agent_id) = self.find_best_agent(task, registry) {
                return Some((task.id.clone(), agent_id));
            }
        }
        None
    }

    fn find_best_agent(&self, task: &Task, registry: &AgentRegistry) -> Option<AgentId> {
        let idle = registry.idle_agents();
        if idle.is_empty() {
            return None;
        }

        if task.required_skills.is_empty() {
            // Any idle agent will do
            return idle.first().map(|a| a.id.clone());
        }

        // Score agents by skill match
        let mut best: Option<(AgentId, usize)> = None;
        for agent in &idle {
            let score = task
                .required_skills
                .iter()
                .filter(|skill| agent.config.skills.contains(skill))
                .count();
            if score > 0 {
                if best.as_ref().map_or(true, |(_, s)| score > *s) {
                    best = Some((agent.id.clone(), score));
                }
            }
        }

        // Fall back to any idle agent if no skill match
        best.map(|(id, _)| id)
            .or_else(|| idle.first().map(|a| a.id.clone()))
    }

    pub fn assign_task(&mut self, task_id: &TaskId, agent_id: AgentId) {
        if let Some(task) = self.tasks.get_mut(task_id) {
            task.assign(agent_id.clone());
            info!(task_id = %task_id, agent = %agent_id, "task assigned");
        }
    }

    pub fn start_task(&mut self, task_id: &TaskId) {
        if let Some(task) = self.tasks.get_mut(task_id) {
            task.start();
        }
    }

    pub fn complete_task(&mut self, task_id: &TaskId, result: String) {
        if let Some(task) = self.tasks.get_mut(task_id) {
            task.complete(result);
            info!(task_id = %task_id, "task completed");
        }
    }

    pub fn fail_task(&mut self, task_id: &TaskId, reason: String) {
        if let Some(task) = self.tasks.get_mut(task_id) {
            task.fail(reason);
        }
    }

    pub fn all_tasks(&self) -> Vec<&Task> {
        self.order
            .iter()
            .filter_map(|id| self.tasks.get(id))
            .collect()
    }

    pub fn pending_count(&self) -> usize {
        self.tasks
            .values()
            .filter(|t| t.status == TaskStatus::Pending)
            .count()
    }

    pub fn active_count(&self) -> usize {
        self.tasks
            .values()
            .filter(|t| matches!(t.status, TaskStatus::Assigned | TaskStatus::InProgress))
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestrator::registry::AgentRegistry;
    use crate::types::agent::{AgentConfig, AgentStatus, PermissionMode};

    fn test_config(name: &str, skills: Vec<&str>) -> AgentConfig {
        AgentConfig {
            name: name.to_string(),
            role: "test".to_string(),
            system_prompt: String::new(),
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
    fn test_skill_based_assignment() {
        let mut registry = AgentRegistry::new();
        let arch_id = AgentId::new("architect");
        let dev_id = AgentId::new("developer");
        registry.register(arch_id.clone(), test_config("architect", vec!["planning", "architecture"]));
        registry.register(dev_id.clone(), test_config("developer", vec!["coding", "testing"]));
        registry.set_status(&arch_id, AgentStatus::Idle);
        registry.set_status(&dev_id, AgentStatus::Idle);

        let mut scheduler = TaskScheduler::new();
        scheduler.create_task(
            "Plan the system".to_string(),
            TaskPriority::High,
            vec!["planning".to_string()],
        );

        let assignment = scheduler.find_assignment(&registry);
        assert!(assignment.is_some());
        let (_, agent_id) = assignment.unwrap();
        assert_eq!(agent_id.0, "architect");
    }

    #[test]
    fn test_task_lifecycle() {
        let mut scheduler = TaskScheduler::new();
        let task_id = scheduler.create_task(
            "Test task".to_string(),
            TaskPriority::Normal,
            vec![],
        );

        assert_eq!(scheduler.pending_count(), 1);
        scheduler.assign_task(&task_id, AgentId::new("dev"));
        assert_eq!(scheduler.pending_count(), 0);
        scheduler.start_task(&task_id);
        assert_eq!(scheduler.active_count(), 1);
        scheduler.complete_task(&task_id, "done".to_string());
        assert_eq!(scheduler.active_count(), 0);
    }
}
