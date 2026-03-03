use anyhow::Result;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use crate::agent::pool::AgentPool;
use crate::agent::stream_parser::{detect_mentions, parse_stream_message};
use crate::config::SwarmConfig;
use crate::types::agent::{AgentId, AgentStatus};
use crate::types::communication::{InterAgentMessage, MessageContent};
use crate::types::event::{AppEvent, OrchestratorCommand, TeamTaskPhase};
use crate::types::message::ClaudeStreamMessage;

use super::registry::AgentRegistry;
use super::router::MessageRouter;
use super::scheduler::TaskScheduler;
use super::team_task::{self, TeamTaskState};

/// Central orchestrator that manages agents, tasks, and communication.
pub struct Orchestrator {
    config: SwarmConfig,
    registry: AgentRegistry,
    scheduler: TaskScheduler,
    router: MessageRouter,
    pool: AgentPool,
    event_tx: mpsc::Sender<AppEvent>,
    cmd_rx: mpsc::Receiver<OrchestratorCommand>,
    stream_rx: mpsc::Receiver<(AgentId, ClaudeStreamMessage)>,
    /// Active team task being executed (only one at a time).
    active_team_task: Option<TeamTaskState>,
}

impl Orchestrator {
    pub fn new(
        config: SwarmConfig,
        event_tx: mpsc::Sender<AppEvent>,
        cmd_rx: mpsc::Receiver<OrchestratorCommand>,
    ) -> Self {
        let (stream_tx, stream_rx) = mpsc::channel(256);
        Self {
            config,
            registry: AgentRegistry::new(),
            scheduler: TaskScheduler::new(),
            router: MessageRouter::new(),
            pool: AgentPool::new(stream_tx),
            event_tx,
            cmd_rx,
            stream_rx,
            active_team_task: None,
        }
    }

    /// Enable visible terminal windows for agents.
    pub fn set_visible_terminals(&mut self, log_dir: std::path::PathBuf) {
        self.pool.set_visible_terminals(log_dir);
    }

    /// Initialize by registering all configured agents (no process spawned yet).
    pub async fn initialize(&mut self) -> Result<()> {
        let agent_configs = self.config.agent_configs();
        for (id, config) in agent_configs {
            self.registry.register(id.clone(), config.clone());
            self.pool.register_agent(id.clone(), config);
            // Mark agents as idle immediately — they're ready to accept prompts
            self.registry.set_status(&id, AgentStatus::Idle);
            let _ = self
                .event_tx
                .send(AppEvent::AgentReady {
                    agent_id: id,
                })
                .await;
        }
        Ok(())
    }

    /// Run the orchestrator event loop.
    pub async fn run(mut self) -> Result<()> {
        info!("orchestrator starting");

        self.initialize().await?;

        loop {
            tokio::select! {
                Some(cmd) = self.cmd_rx.recv() => {
                    match cmd {
                        OrchestratorCommand::Shutdown => {
                            info!("orchestrator shutting down");
                            let _ = self.event_tx.send(AppEvent::Shutdown).await;
                            break;
                        }
                        _ => self.handle_command(cmd).await,
                    }
                }

                Some((agent_id, msg)) = self.stream_rx.recv() => {
                    self.handle_agent_output(agent_id, msg).await;
                }

                else => break,
            }
        }

        info!("orchestrator stopped");
        Ok(())
    }

    async fn handle_command(&mut self, cmd: OrchestratorCommand) {
        match cmd {
            OrchestratorCommand::SendPrompt { agent_id, prompt } => {
                self.registry.set_status(&agent_id, AgentStatus::Working);
                if let Err(e) = self.pool.send_prompt(&agent_id, &prompt) {
                    error!(agent = %agent_id, "send prompt failed: {e}");
                }
            }
            OrchestratorCommand::CreateTask {
                description,
                priority,
                skills,
            } => {
                let task_id = self.scheduler.create_task(description, priority, skills);
                if let Some(task) = self.scheduler.get(&task_id) {
                    let _ = self
                        .event_tx
                        .send(AppEvent::TaskCreated(task.clone()))
                        .await;
                }
                self.try_assign_tasks().await;
            }
            OrchestratorCommand::RouteMessage { message } => {
                let to = message.to.clone();
                let prompt = self.router.route_message(message.clone());
                let _ = self
                    .event_tx
                    .send(AppEvent::MessageRouted(message))
                    .await;
                self.registry.set_status(&to, AgentStatus::Working);
                if let Err(e) = self.pool.send_prompt(&to, &prompt) {
                    error!(agent = %to, "route message failed: {e}");
                }
            }
            OrchestratorCommand::StopAgent { agent_id } => {
                self.pool.remove_agent(&agent_id);
                self.registry.set_status(&agent_id, AgentStatus::Stopped);
            }
            OrchestratorCommand::TeamTask { description } => {
                self.start_team_task(description).await;
            }
            OrchestratorCommand::SpawnAgent { .. } => {}
            OrchestratorCommand::Shutdown => {}
        }
    }

    /// Start a 3-phase team task.
    async fn start_team_task(&mut self, description: String) {
        if self.active_team_task.is_some() {
            warn!("team task already in progress, ignoring new request");
            return;
        }

        // Find lead agent
        let lead_id = self.config.lead_agent_id();

        info!(task = %description, lead = %lead_id, "starting team task - phase 1: planning");

        let _ = self
            .event_tx
            .send(AppEvent::TeamTaskUpdate {
                phase: TeamTaskPhase::Planning,
                description: description.clone(),
            })
            .await;

        // Build agent roster (exclude lead — lead plans & synthesizes, doesn't execute)
        let agent_names: Vec<String> = self
            .registry
            .ordered_ids()
            .iter()
            .filter(|id| **id != lead_id)
            .map(|id| id.0.clone())
            .collect();

        let planning_prompt = format!(
            "You are the lead agent coordinating a team task.\n\
            \n\
            TASK: {description}\n\
            \n\
            Available team members (DO NOT assign to yourself): {agent_list}\n\
            \n\
            Break this task into subtasks for your team members. \
            For each subtask, use EXACTLY this format on its own line:\n\
            SUBTASK @agent_name: description of what they should do\n\
            \n\
            IMPORTANT: Only use the SUBTASK @name: format. Do NOT use @mentions outside of SUBTASK lines. \
            Only assign to agents listed above. Do NOT assign subtasks to yourself. \
            Be specific about what each agent should do.",
            agent_list = agent_names.join(", ")
        );

        let state = TeamTaskState::new(description, lead_id.clone());
        self.active_team_task = Some(state);

        self.registry.set_status(&lead_id, AgentStatus::Working);
        if let Err(e) = self.pool.send_prompt(&lead_id, &planning_prompt) {
            error!(agent = %lead_id, "team task planning prompt failed: {e}");
            self.active_team_task = None;
        }
    }

    /// Transition team task from Planning to Executing phase.
    async fn team_task_execute(&mut self) {
        let task_state = match self.active_team_task.as_mut() {
            Some(s) if s.phase == team_task::TeamTaskPhase::Planning => s,
            _ => return,
        };

        // Get the lead agent's recent output to parse subtasks
        let lead_id = task_state.lead_agent.clone();
        let output = self
            .registry
            .get(&lead_id)
            .map(|s| s.output_lines.join("\n"))
            .unwrap_or_default();

        // Build list of known non-lead agent names for flexible parsing
        let known_agents: Vec<String> = self
            .registry
            .ordered_ids()
            .iter()
            .filter(|id| **id != lead_id)
            .map(|id| id.0.clone())
            .collect();
        let subtasks = team_task::parse_subtask_plan(&output, &known_agents);

        if subtasks.is_empty() {
            warn!("lead agent produced no subtasks, falling back to broadcast");
            // Fallback: send the original description to all non-lead agents
            let description = task_state.description.clone();
            let agent_ids: Vec<AgentId> = self
                .registry
                .ordered_ids()
                .iter()
                .filter(|id| **id != lead_id)
                .cloned()
                .collect();

            task_state.phase = team_task::TeamTaskPhase::Executing;
            for agent_id in &agent_ids {
                task_state.pending_agents.push(agent_id.clone());
            }

            let _ = self
                .event_tx
                .send(AppEvent::TeamTaskUpdate {
                    phase: TeamTaskPhase::Executing,
                    description: format!("Broadcast to {} agents", agent_ids.len()),
                })
                .await;

            for agent_id in agent_ids {
                let prompt = format!(
                    "Team task assigned to you:\n{}\n\nComplete your part based on your role. Be thorough.",
                    description
                );
                self.registry.set_status(&agent_id, AgentStatus::Working);
                if let Err(e) = self.pool.send_prompt(&agent_id, &prompt) {
                    error!(agent = %agent_id, "team task subtask prompt failed: {e}");
                }
            }
            return;
        }

        info!(
            count = subtasks.len(),
            "team task - phase 2: executing subtasks"
        );

        task_state.subtasks = subtasks.clone();
        task_state.phase = team_task::TeamTaskPhase::Executing;

        let _ = self
            .event_tx
            .send(AppEvent::TeamTaskUpdate {
                phase: TeamTaskPhase::Executing,
                description: format!("{} subtasks dispatched", subtasks.len()),
            })
            .await;

        let lead_id = task_state.lead_agent.clone();
        for subtask in &subtasks {
            let agent_id = AgentId::new(&subtask.agent_name);

            // Skip lead agent — lead plans & synthesizes, doesn't execute subtasks
            if agent_id == lead_id {
                warn!(agent = %agent_id, "skipping subtask for lead agent");
                continue;
            }

            // Check if agent exists
            if self.registry.get(&agent_id).is_none() {
                warn!(agent = %agent_id, "subtask target agent not found, skipping");
                continue;
            }

            task_state.pending_agents.push(agent_id.clone());

            let prompt = format!(
                "Team subtask assigned to you:\n{}\n\nComplete this thoroughly and report your findings.",
                subtask.description
            );

            // Emit MessageRouted so Office view shows meeting
            let msg = InterAgentMessage::new(
                lead_id.clone(),
                agent_id.clone(),
                MessageContent::Text(subtask.description.clone()),
            );
            let _ = self.event_tx.send(AppEvent::MessageRouted(msg)).await;

            self.registry.set_status(&agent_id, AgentStatus::Working);
            if let Err(e) = self.pool.send_prompt(&agent_id, &prompt) {
                error!(agent = %agent_id, "subtask prompt failed: {e}");
            }
        }

        // If no valid subtask agents after filtering, go straight to synthesis
        let should_synthesize = self
            .active_team_task
            .as_ref()
            .map(|s| s.pending_agents.is_empty())
            .unwrap_or(false);
        if should_synthesize {
            info!("no valid subtask agents, skipping to synthesis");
            self.team_task_synthesize().await;
        }
    }

    /// Transition team task from Executing to Synthesizing phase.
    async fn team_task_synthesize(&mut self) {
        let task_state = match self.active_team_task.as_mut() {
            Some(s) if s.phase == team_task::TeamTaskPhase::Executing => s,
            _ => return,
        };

        info!("team task - phase 3: synthesizing");

        task_state.phase = team_task::TeamTaskPhase::Synthesizing;
        let lead_id = task_state.lead_agent.clone();
        let result_agents: Vec<String> = task_state.results.keys().cloned().collect();
        let synthesis_prompt = task_state.build_synthesis_prompt();

        // Emit MessageRouted for each result → lead (shows meeting in Office)
        for agent_name in &result_agents {
            let msg = InterAgentMessage::new(
                AgentId::new(agent_name),
                lead_id.clone(),
                MessageContent::Text(format!("Sending results to lead for synthesis")),
            );
            let _ = self.event_tx.send(AppEvent::MessageRouted(msg)).await;
        }

        let _ = self
            .event_tx
            .send(AppEvent::TeamTaskUpdate {
                phase: TeamTaskPhase::Synthesizing,
                description: format!("Lead synthesizing {} results", result_agents.len()),
            })
            .await;

        self.registry.set_status(&lead_id, AgentStatus::Working);
        if let Err(e) = self.pool.send_prompt(&lead_id, &synthesis_prompt) {
            error!(agent = %lead_id, "synthesis prompt failed: {e}");
        }
    }

    /// Complete the team task.
    async fn team_task_complete(&mut self) {
        info!("team task completed");
        let _ = self
            .event_tx
            .send(AppEvent::TeamTaskUpdate {
                phase: TeamTaskPhase::Completed,
                description: "Team task finished".to_string(),
            })
            .await;
        self.active_team_task = None;
    }

    async fn handle_agent_output(&mut self, agent_id: AgentId, msg: ClaudeStreamMessage) {
        let events = parse_stream_message(&agent_id, &msg);

        for event in events {
            match &event {
                AppEvent::AgentReady { agent_id } => {
                    self.registry.set_status(agent_id, AgentStatus::Working);
                }
                AppEvent::AgentTextOutput { agent_id, text } => {
                    if let Some(state) = self.registry.get_mut(agent_id) {
                        state.append_output(text.clone());
                        state.status = AgentStatus::Working;
                    }

                    // Detect @mentions for automatic routing (disabled during team tasks)
                    let mentions = if self.active_team_task.is_none() {
                        detect_mentions(text)
                    } else {
                        Vec::new()
                    };
                    for (target_name, message_text) in mentions {
                        let target_id = AgentId::new(&target_name);
                        if self.registry.get(&target_id).is_some() {
                            let msg = InterAgentMessage::new(
                                agent_id.clone(),
                                target_id.clone(),
                                MessageContent::Text(message_text),
                            );
                            let prompt = self.router.route_message(msg.clone());
                            let _ = self.event_tx.send(AppEvent::MessageRouted(msg)).await;
                            self.registry.set_status(&target_id, AgentStatus::Working);
                            if let Err(e) = self.pool.send_prompt(&target_id, &prompt) {
                                error!(agent = %target_id, "mention routing failed: {e}");
                            }
                        }
                    }
                }
                AppEvent::AgentToolUse {
                    agent_id,
                    tool_name,
                    ..
                } => {
                    if let Some(state) = self.registry.get_mut(agent_id) {
                        state.append_output(format!("[tool: {tool_name}]"));
                        state.status = AgentStatus::Working;
                    }
                }
                AppEvent::AgentCompleted {
                    agent_id,
                    cost_usd,
                } => {
                    if let Some(state) = self.registry.get_mut(agent_id) {
                        state.status = AgentStatus::Idle;
                        if let Some(cost) = cost_usd {
                            state.usage.cost_usd += cost;
                        }
                    }

                    // Handle team task phase transitions
                    self.handle_team_task_completion(agent_id).await;

                    self.try_assign_tasks().await;
                }
                _ => {}
            }

            let _ = self.event_tx.send(event).await;
        }
    }

    /// Handle agent completion in context of active team task.
    async fn handle_team_task_completion(&mut self, completed_agent: &AgentId) {
        let phase = match &self.active_team_task {
            Some(s) => s.phase.clone(),
            None => return,
        };

        match phase {
            team_task::TeamTaskPhase::Planning => {
                // Lead finished planning — parse and dispatch subtasks
                let lead_id = self
                    .active_team_task
                    .as_ref()
                    .map(|s| s.lead_agent.clone())
                    .unwrap();
                if *completed_agent == lead_id {
                    self.team_task_execute().await;
                }
            }
            team_task::TeamTaskPhase::Executing => {
                // Collect result from completing agent
                let result = self
                    .registry
                    .get(completed_agent)
                    .map(|s| {
                        // Get the last few lines as the result
                        let lines = &s.output_lines;
                        let start = lines.len().saturating_sub(50);
                        lines[start..].join("\n")
                    })
                    .unwrap_or_default();

                if let Some(state) = self.active_team_task.as_mut() {
                    state.record_result(completed_agent, result);
                }

                // Check if all subtasks are done
                let all_done = self
                    .active_team_task
                    .as_ref()
                    .map(|s| s.all_subtasks_done())
                    .unwrap_or(false);

                if all_done {
                    self.team_task_synthesize().await;
                }
            }
            team_task::TeamTaskPhase::Synthesizing => {
                // Lead finished synthesizing — task complete
                let lead_id = self
                    .active_team_task
                    .as_ref()
                    .map(|s| s.lead_agent.clone())
                    .unwrap();
                if *completed_agent == lead_id {
                    self.team_task_complete().await;
                }
            }
        }
    }

    async fn try_assign_tasks(&mut self) {
        while let Some((task_id, agent_id)) = self.scheduler.find_assignment(&self.registry) {
            self.scheduler.assign_task(&task_id, agent_id.clone());

            if let Some(task) = self.scheduler.get(&task_id) {
                let prompt = format!(
                    "Task assigned to you:\n{}\n\nPlease complete this task.",
                    task.description
                );

                self.registry.set_status(&agent_id, AgentStatus::Working);
                if let Err(e) = self.pool.send_prompt(&agent_id, &prompt) {
                    error!(agent = %agent_id, "task prompt failed: {e}");
                } else {
                    self.scheduler.start_task(&task_id);
                    let _ = self
                        .event_tx
                        .send(AppEvent::TaskAssigned {
                            task_id,
                            agent_id,
                        })
                        .await;
                }
            }
        }
    }
}
