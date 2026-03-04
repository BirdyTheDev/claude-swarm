use claude_swarm::orchestrator::registry::AgentRegistry;
use claude_swarm::orchestrator::router::MessageRouter;
use claude_swarm::orchestrator::scheduler::TaskScheduler;
use claude_swarm::types::agent::{AgentConfig, AgentId, AgentStatus, PermissionMode};
use claude_swarm::types::task::TaskPriority;

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
fn test_registry_crud() {
    let mut reg = AgentRegistry::new();

    let id1 = AgentId::new("agent-1");
    let id2 = AgentId::new("agent-2");

    reg.register(id1.clone(), test_config("agent-1", vec!["coding"]));
    reg.register(id2.clone(), test_config("agent-2", vec!["testing"]));

    assert_eq!(reg.len(), 2);
    assert!(reg.get(&id1).is_some());
    assert!(reg.get(&id2).is_some());

    reg.set_status(&id1, AgentStatus::Idle);
    assert_eq!(reg.get(&id1).unwrap().status, AgentStatus::Idle);
}

#[test]
fn test_scheduler_skill_matching() {
    let mut reg = AgentRegistry::new();
    let arch = AgentId::new("architect");
    let dev = AgentId::new("developer");

    reg.register(
        arch.clone(),
        test_config("architect", vec!["planning", "architecture"]),
    );
    reg.register(
        dev.clone(),
        test_config("developer", vec!["coding", "testing"]),
    );
    reg.set_status(&arch, AgentStatus::Idle);
    reg.set_status(&dev, AgentStatus::Idle);

    let mut scheduler = TaskScheduler::new();

    // Task requiring planning should go to architect
    scheduler.create_task(
        "Design the system".to_string(),
        TaskPriority::High,
        vec!["planning".to_string()],
    );

    let assignment = scheduler.find_assignment(&reg);
    assert!(assignment.is_some());
    let (_, assigned_agent) = assignment.unwrap();
    assert_eq!(assigned_agent.0, "architect");
}

#[test]
fn test_scheduler_task_lifecycle() {
    let mut scheduler = TaskScheduler::new();
    let agent_id = AgentId::new("dev");

    let task_id = scheduler.create_task(
        "Implement feature X".to_string(),
        TaskPriority::Normal,
        vec![],
    );

    assert_eq!(scheduler.pending_count(), 1);
    assert_eq!(scheduler.active_count(), 0);

    scheduler.assign_task(&task_id, agent_id);
    assert_eq!(scheduler.pending_count(), 0);
    assert_eq!(scheduler.active_count(), 1);

    scheduler.start_task(&task_id);
    assert_eq!(scheduler.active_count(), 1);

    scheduler.complete_task(&task_id, "Feature implemented".to_string());
    assert_eq!(scheduler.active_count(), 0);

    let task = scheduler.get(&task_id).unwrap();
    assert_eq!(task.result.as_deref(), Some("Feature implemented"));
}

#[test]
fn test_router_message_formatting() {
    let mut router = MessageRouter::new();
    let msg = MessageRouter::create_text_message(
        AgentId::new("architect"),
        AgentId::new("developer"),
        "Please implement the login endpoint".to_string(),
    );

    let prompt = router.route_message(msg);
    assert!(prompt.contains("[Message from agent 'architect']"));
    assert!(prompt.contains("Please implement the login endpoint"));
    assert!(prompt.contains("[End message]"));
    assert_eq!(router.message_count(), 1);
}

#[test]
fn test_config_parsing() {
    let toml_str = r#"
name = "test-swarm"
description = "Test swarm"

[[agent]]
name = "lead"
role = "Lead"
system_prompt = "You are the lead."
skills = ["planning"]
is_lead = true

[[agent]]
name = "worker"
role = "Worker"
system_prompt = "You are a worker."
skills = ["coding"]
permission_mode = "full-auto"
"#;

    let config: claude_swarm::config::SwarmConfig = toml::from_str(toml_str).unwrap();
    assert_eq!(config.name, "test-swarm");
    assert_eq!(config.agent.len(), 2);

    let lead_id = config.lead_agent_id();
    assert_eq!(lead_id.0, "lead");

    let agent_configs = config.agent_configs();
    assert_eq!(agent_configs.len(), 2);

    let (_, worker_cfg) = agent_configs.iter().find(|(id, _)| id.0 == "worker").unwrap();
    assert_eq!(
        worker_cfg.permission_mode,
        claude_swarm::types::agent::PermissionMode::BypassPermissions
    );
}
