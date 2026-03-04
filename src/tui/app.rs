use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::io::Write;
use tokio::sync::mpsc;

use crate::config::settings::Settings;
use crate::telegram::TelegramNotification;
use crate::tui::i18n::{self, Messages};
use crate::tui::views::office_view::OfficeState;
use crate::tui::widgets::command_input::InputState;
use crate::types::agent::{AgentId, AgentState, AgentStatus};
use crate::types::communication::InterAgentMessage;
use crate::types::event::{AppEvent, OrchestratorCommand, TeamTaskPhase, UiMode, ViewTab};
use crate::types::log_entry::{LogCategory, LogEntry};
use crate::types::task::{Task, TaskPriority};

/// A task scheduled to fire at a specific time.
#[derive(Debug, Clone)]
pub struct ScheduledTask {
    pub fire_at: chrono::DateTime<chrono::Utc>,
    pub command: String,
    pub fired: bool,
}

/// Application state for the TUI.
pub struct App {
    pub swarm_name: String,
    pub running: bool,
    pub mode: UiMode,
    pub active_tab: ViewTab,

    // Agent state (mirrored from orchestrator)
    pub agents: Vec<AgentState>,
    pub selected_agent: usize,
    pub scroll_offset: u16,

    // Task state
    pub tasks: Vec<Task>,
    pub selected_task: usize,

    // Communication
    pub messages: Vec<InterAgentMessage>,

    // Input
    pub input_state: InputState,

    // Logs
    pub system_logs: Vec<LogEntry>,

    // Office view state
    pub office_state: OfficeState,

    // Team task tracking
    pub team_task_phase: Option<TeamTaskPhase>,

    // Persistent conversation log file
    pub conversation_log: Option<std::fs::File>,

    // Settings
    pub settings: Settings,

    // Settings view state
    pub settings_selected: usize,
    pub settings_editing: bool,
    pub settings_edit_buffer: String,

    // Channel to send commands to orchestrator
    pub cmd_tx: mpsc::Sender<OrchestratorCommand>,

    // Channel to send Telegram notifications
    pub telegram_notify_tx: Option<mpsc::Sender<TelegramNotification>>,

    // Telegram pairing code (shown in settings view while pairing)
    pub telegram_pairing_code: Option<String>,

    // Event channel for async tasks (e.g., verify results)
    pub event_tx: mpsc::Sender<AppEvent>,

    // Build verification retry counters
    pub verify_retries: std::collections::HashMap<AgentId, u32>,

    // Performance metrics
    pub started_at: std::time::Instant,
    pub total_messages_routed: usize,
    pub sysinfo: sysinfo::System,

    // Scheduled tasks
    pub scheduled_tasks: Vec<ScheduledTask>,
}

impl App {
    pub fn new(
        swarm_name: String,
        cmd_tx: mpsc::Sender<OrchestratorCommand>,
        settings: Settings,
        event_tx: mpsc::Sender<AppEvent>,
    ) -> Self {
        let history_size = settings.input_history_size;
        Self {
            swarm_name,
            running: true,
            mode: UiMode::Normal,
            active_tab: ViewTab::Dashboard,
            agents: Vec::new(),
            selected_agent: 0,
            scroll_offset: 0,
            tasks: Vec::new(),
            selected_task: 0,
            messages: Vec::new(),
            input_state: InputState::new(history_size),
            system_logs: Vec::new(),
            office_state: OfficeState::new(),
            team_task_phase: None,
            conversation_log: None,
            settings,
            settings_selected: 0,
            settings_editing: false,
            settings_edit_buffer: String::new(),
            cmd_tx,
            telegram_notify_tx: None,
            telegram_pairing_code: None,
            event_tx,
            verify_retries: std::collections::HashMap::new(),
            started_at: std::time::Instant::now(),
            total_messages_routed: 0,
            sysinfo: sysinfo::System::new_all(),
            scheduled_tasks: Vec::new(),
        }
    }

    /// Get the current i18n messages based on settings.
    pub fn msgs(&self) -> &'static Messages {
        Messages::for_lang(&self.settings.language)
    }

    /// Handle an AppEvent, updating state accordingly.
    pub async fn handle_event(&mut self, event: AppEvent) {
        match event {
            AppEvent::Key(key) => self.handle_key(key).await,
            AppEvent::Tick => {
                // Refresh system info for performance metrics
                self.sysinfo.refresh_memory();
                self.sysinfo.refresh_processes();

                // Expire meetings in office view
                let freed = self.office_state.tick();
                for name in freed {
                    let id = AgentId::new(&name);
                    // Restore to Idle if they were InMeeting
                    if let Some(agent) = self.find_agent_mut(&id) {
                        if agent.status == AgentStatus::InMeeting {
                            agent.status = AgentStatus::Idle;
                        }
                    }
                }

                // Fire scheduled tasks whose time has come
                let now = chrono::Utc::now();
                let mut to_fire: Vec<String> = Vec::new();
                for st in &mut self.scheduled_tasks {
                    if !st.fired && now >= st.fire_at {
                        st.fired = true;
                        to_fire.push(st.command.clone());
                    }
                }
                for cmd in to_fire {
                    self.notify_telegram(&format!("Firing scheduled: {cmd}"));
                    self.execute_telegram_command(&cmd).await;
                }
                // Remove fired tasks
                self.scheduled_tasks.retain(|st| !st.fired);
            }
            AppEvent::Resize(_, _) => {}

            AppEvent::AgentReady { agent_id } => {
                self.update_agent_status(&agent_id, AgentStatus::Idle);
                let msg = i18n::fmt(self.msgs().agent_ready, &[agent_id.as_ref()]);
                self.log_entry(LogCategory::Agent, msg);
            }
            AppEvent::AgentTextOutput { agent_id, text } => {
                self.log_agent_output(&agent_id, &text);
                if let Some(agent) = self.find_agent_mut(&agent_id) {
                    agent.append_output(text);
                    agent.status = AgentStatus::Working;
                }
            }
            AppEvent::AgentToolUse {
                agent_id,
                tool_name,
                ..
            } => {
                self.log_agent_output(&agent_id, &format!("[tool: {tool_name}]"));
                if let Some(agent) = self.find_agent_mut(&agent_id) {
                    agent.append_output(format!("[tool: {tool_name}]"));
                    agent.status = AgentStatus::Working;
                }
            }
            AppEvent::AgentCompleted {
                agent_id,
                cost_usd,
            } => {
                if let Some(agent) = self.find_agent_mut(&agent_id) {
                    agent.status = AgentStatus::Idle;
                    if let Some(cost) = cost_usd {
                        agent.usage.cost_usd += cost;
                    }
                }
                // Mark any in-progress task for this agent as completed
                let task_desc = if let Some(task) = self.tasks.iter_mut().rev().find(|t| {
                    t.assigned_to.as_ref().map(|a| a == &agent_id).unwrap_or(false)
                        && t.status == crate::types::task::TaskStatus::InProgress
                }) {
                    let desc = task.description.clone();
                    task.complete("Done".to_string());
                    // Trigger auto README for :t tasks
                    if self.settings.auto_readme {
                        self.trigger_readme_generation(&desc);
                    }
                    Some(desc)
                } else {
                    None
                };
                let msg = i18n::fmt(self.msgs().agent_completed, &[agent_id.as_ref()]);
                self.log_entry(LogCategory::Agent, msg.clone());
                self.notify_telegram(&format!(
                    "Agent '{}' completed{}",
                    agent_id.as_ref(),
                    task_desc.map(|d| format!(": {d}")).unwrap_or_default()
                ));
                // Auto-verify build if enabled
                if self.settings.auto_verify {
                    let retries = self.verify_retries.get(&agent_id).copied().unwrap_or(0);
                    if retries < self.settings.max_verify_retries {
                        let cmd = if self.settings.verify_command.is_empty() {
                            detect_verify_command()
                        } else {
                            Some(self.settings.verify_command.clone())
                        };
                        if let Some(verify_cmd) = cmd {
                            let ev_tx = self.event_tx.clone();
                            let aid = agent_id.clone();
                            tokio::spawn(async move {
                                let output = tokio::process::Command::new("sh")
                                    .arg("-c")
                                    .arg(&verify_cmd)
                                    .output()
                                    .await;
                                match output {
                                    Ok(result) => {
                                        let success = result.status.success();
                                        let out = if success {
                                            None
                                        } else {
                                            let stderr = String::from_utf8_lossy(&result.stderr);
                                            let stdout = String::from_utf8_lossy(&result.stdout);
                                            Some(format!("{}\n{}", stdout, stderr))
                                        };
                                        let _ = ev_tx.send(AppEvent::VerifyResult {
                                            agent_id: aid,
                                            success,
                                            output: out,
                                        }).await;
                                    }
                                    Err(e) => {
                                        let _ = ev_tx.send(AppEvent::VerifyResult {
                                            agent_id: aid,
                                            success: false,
                                            output: Some(format!("Failed to run verify command: {e}")),
                                        }).await;
                                    }
                                }
                            });
                        }
                    }
                }
            }
            AppEvent::AgentError { agent_id, error } => {
                self.update_agent_status(&agent_id, AgentStatus::Failed);
                let friendly = humanize_error(&error);
                let msg = i18n::fmt(self.msgs().agent_error, &[agent_id.as_ref(), &friendly]);
                self.log_entry(LogCategory::Error, msg.clone());
                self.notify_telegram(&msg);
            }
            AppEvent::AgentOutput { .. } => {}

            AppEvent::TaskCreated(task) => {
                let msg = i18n::fmt(self.msgs().task_created, &[&task.description]);
                self.log_entry(LogCategory::Task, msg);
                self.tasks.push(task);
            }
            AppEvent::TaskAssigned {
                task_id,
                agent_id,
            } => {
                if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task_id) {
                    task.assign(agent_id.clone());
                }
                let msg = i18n::fmt(self.msgs().task_assigned, &[&task_id.to_string(), agent_id.as_ref()]);
                self.log_entry(LogCategory::Task, msg);
            }
            AppEvent::TaskCompleted { task_id, result } => {
                if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task_id) {
                    task.complete(result);
                }
                let msg = i18n::fmt(self.msgs().task_completed, &[&task_id.to_string()]);
                self.log_entry(LogCategory::Task, msg);
            }

            AppEvent::MessageRouted(msg) => {
                self.total_messages_routed += 1;
                let summary = msg.content.summary();
                let log_msg = i18n::fmt(
                    self.msgs().message_routed,
                    &[msg.from.as_ref(), msg.to.as_ref(), &summary],
                );
                self.log_entry(LogCategory::Communication, log_msg);
                // Track meeting in office view
                let topic = msg.content.summary();
                self.office_state.add_meeting(
                    msg.from.as_ref(),
                    msg.to.as_ref(),
                    &topic,
                );
                // Update agent statuses to InMeeting
                let from_id = msg.from.clone();
                let to_id = msg.to.clone();
                if let Some(agent) = self.find_agent_mut(&from_id) {
                    agent.status = AgentStatus::InMeeting;
                }
                if let Some(agent) = self.find_agent_mut(&to_id) {
                    agent.status = AgentStatus::InMeeting;
                }
                self.messages.push(msg);
            }

            AppEvent::TeamTaskUpdate { phase, description } => {
                self.team_task_phase = Some(phase.clone());
                let msgs = self.msgs();
                let phase_name = match phase {
                    TeamTaskPhase::Planning => msgs.team_planning,
                    TeamTaskPhase::Executing => msgs.team_executing,
                    TeamTaskPhase::Synthesizing => msgs.team_synthesizing,
                    TeamTaskPhase::Completed => msgs.team_completed,
                };
                let log_msg = format!("[{}]: {}", phase_name, description);
                self.log_entry(LogCategory::Team, log_msg);
                if phase == TeamTaskPhase::Completed {
                    self.team_task_phase = None;
                    // Mark the team task as completed in the Tasks view
                    if let Some(task) = self.tasks.iter_mut().rev().find(|t| {
                        t.description.starts_with("[Team]")
                            && t.status != crate::types::task::TaskStatus::Completed
                    }) {
                        task.complete(description.clone());
                    }
                    self.notify_telegram(&format!("Team task completed: {description}"));
                    // Trigger auto README for team tasks
                    if self.settings.auto_readme {
                        self.trigger_readme_generation(&description);
                    }
                }
            }

            AppEvent::TelegramNotify { text } => {
                self.log_entry(LogCategory::System, format!("[Telegram] {text}"));
            }

            AppEvent::SoulUpdated { agent_id, soul } => {
                if let Some(agent) = self.find_agent_mut(&agent_id) {
                    agent.config.soul = soul.clone();
                }
                let preview = if soul.len() > 50 {
                    format!("{}...", &soul[..50])
                } else {
                    soul
                };
                let msg = format!("Soul updated for '{}': {}", agent_id.as_ref(), preview);
                self.log_entry(LogCategory::System, msg.clone());
                self.notify_telegram(&msg);
            }

            AppEvent::VerifyResult { agent_id, success, output } => {
                if success {
                    self.verify_retries.remove(&agent_id);
                    let msg = format!("Build verification passed for '{}'", agent_id.as_ref());
                    self.log_entry(LogCategory::System, msg);
                } else {
                    let retries = self.verify_retries.entry(agent_id.clone()).or_insert(0);
                    *retries += 1;
                    let max = self.settings.max_verify_retries;
                    if *retries <= max {
                        let error_output = output.unwrap_or_else(|| "Unknown error".to_string());
                        let prompt = format!(
                            "Build verification failed. Please fix the errors and try again.\n\nError output:\n{}",
                            error_output
                        );
                        let _ = self.cmd_tx.try_send(OrchestratorCommand::SendPrompt {
                            agent_id: agent_id.clone(),
                            prompt,
                        });
                        let msg = format!(
                            "Build verification failed for '{}' (retry {}/{})",
                            agent_id.as_ref(), retries, max
                        );
                        self.log_entry(LogCategory::System, msg);
                    } else {
                        let msg = format!(
                            "Build verification failed for '{}' — max retries ({}) reached, stopping",
                            agent_id.as_ref(), max
                        );
                        self.log_entry(LogCategory::Error, msg.clone());
                        self.notify_telegram(&msg);
                        self.verify_retries.remove(&agent_id);
                    }
                }
            }

            AppEvent::TelegramStatusRequest => {
                let mut lines = Vec::new();
                for agent in &self.agents {
                    let icon = agent.status.icon();
                    lines.push(format!(
                        "{} {} [{}] — {}",
                        icon, agent.config.name, agent.status, agent.config.role
                    ));
                }
                let status_text = if lines.is_empty() {
                    "No agents running.".to_string()
                } else {
                    lines.join("\n")
                };
                self.notify_telegram(&status_text);
            }

            AppEvent::TelegramCostRequest => {
                let mut lines = Vec::new();
                let mut total_cost = 0.0;
                let mut total_input = 0u64;
                let mut total_output = 0u64;
                for agent in &self.agents {
                    total_cost += agent.usage.cost_usd;
                    total_input += agent.usage.input_tokens;
                    total_output += agent.usage.output_tokens;
                    lines.push(format!(
                        "{}: ${:.4} (in: {} out: {})",
                        agent.config.name,
                        agent.usage.cost_usd,
                        agent.usage.input_tokens,
                        agent.usage.output_tokens
                    ));
                }
                lines.push(format!(
                    "\nTotal: ${:.4} (in: {} out: {})",
                    total_cost, total_input, total_output
                ));
                self.notify_telegram(&lines.join("\n"));
            }

            AppEvent::TelegramTaskPrompt { agent_id, prompt } => {
                // Create task entry + send to orchestrator (mirrors TUI `:t` behavior)
                let mut task = Task::new(
                    prompt.clone(),
                    TaskPriority::Normal,
                    Vec::new(),
                );
                task.assign(agent_id.clone());
                task.start();
                let msg = i18n::fmt(self.msgs().task_created, &[&task.description]);
                self.log_entry(LogCategory::Task, msg);
                self.tasks.push(task);
                let _ = self.cmd_tx.send(OrchestratorCommand::SendPrompt {
                    agent_id,
                    prompt,
                }).await;
            }

            AppEvent::TelegramTeamTask { description } => {
                // Create team task entry + send to orchestrator (mirrors TUI `:tt` behavior)
                let mut task = Task::new(
                    format!("[Team] {}", &description),
                    TaskPriority::High,
                    Vec::new(),
                );
                task.start();
                let msg = i18n::fmt(self.msgs().task_created, &[&task.description]);
                self.log_entry(LogCategory::Task, msg);
                self.tasks.push(task);
                let _ = self.cmd_tx.send(OrchestratorCommand::TeamTask {
                    description: description.clone(),
                }).await;
                let msg = i18n::fmt(self.msgs().team_task_initiated, &[&description]);
                self.log_entry(LogCategory::Team, msg);
            }

            AppEvent::TelegramSchedule { time, command } => {
                match parse_schedule_time(&time) {
                    Some(fire_at) => {
                        let local_time = fire_at.with_timezone(&chrono::Local);
                        let time_str = local_time.format("%H:%M:%S").to_string();
                        self.scheduled_tasks.push(ScheduledTask {
                            fire_at,
                            command: command.clone(),
                            fired: false,
                        });
                        let msg = format!("Scheduled at {time_str}: {command}");
                        self.log_entry(LogCategory::System, msg.clone());
                        self.notify_telegram(&msg);
                    }
                    None => {
                        let msg = format!("Invalid time format: {time} (use HH:MM or HH:MM:SS)");
                        self.log_entry(LogCategory::Error, msg.clone());
                        self.notify_telegram(&msg);
                    }
                }
            }

            AppEvent::TelegramSchedulesList => {
                let pending: Vec<&ScheduledTask> = self.scheduled_tasks.iter().filter(|s| !s.fired).collect();
                let msg = if pending.is_empty() {
                    "No pending schedules.".to_string()
                } else {
                    let lines: Vec<String> = pending.iter().map(|s| {
                        let local = s.fire_at.with_timezone(&chrono::Local);
                        format!("{} — {}", local.format("%H:%M:%S"), s.command)
                    }).collect();
                    format!("Pending schedules:\n{}", lines.join("\n"))
                };
                self.notify_telegram(&msg);
            }

            AppEvent::TelegramPaired { chat_id } => {
                self.settings.telegram_chat_id = chat_id.clone();
                let _ = self.settings.save();
                self.telegram_pairing_code = None;
                let msg = i18n::fmt(self.msgs().telegram_paired, &[&chat_id]);
                self.log_entry(LogCategory::System, format!("[Telegram] {msg}"));
            }

            AppEvent::Shutdown => {
                self.running = false;
            }
        }
    }

    async fn handle_key(&mut self, key: KeyEvent) {
        match self.mode {
            UiMode::Normal => self.handle_normal_key(key).await,
            UiMode::Command | UiMode::Prompt | UiMode::TaskInput => {
                self.handle_input_key(key).await
            }
            UiMode::Help => {
                // Any key closes help
                self.mode = UiMode::Normal;
            }
            UiMode::SettingsEdit => {
                self.handle_settings_key(key).await;
            }
        }
    }

    async fn handle_normal_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') => {
                let _ = self.cmd_tx.send(OrchestratorCommand::Shutdown).await;
            }
            KeyCode::Esc => {
                let _ = self.cmd_tx.send(OrchestratorCommand::Shutdown).await;
            }

            // Navigation
            KeyCode::Char('j') | KeyCode::Down => {
                if self.active_tab == ViewTab::Settings {
                    self.settings_selected = (self.settings_selected + 1).min(12);
                } else if !self.agents.is_empty() {
                    self.selected_agent = (self.selected_agent + 1) % self.agents.len();
                    self.scroll_offset = 0;
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.active_tab == ViewTab::Settings {
                    self.settings_selected = self.settings_selected.saturating_sub(1);
                } else if !self.agents.is_empty() {
                    self.selected_agent = if self.selected_agent == 0 {
                        self.agents.len() - 1
                    } else {
                        self.selected_agent - 1
                    };
                    self.scroll_offset = 0;
                }
            }

            // View tabs
            KeyCode::Char('1') => {
                self.active_tab = ViewTab::Dashboard;
            }
            KeyCode::Char('2') => {
                self.active_tab = ViewTab::AgentDetail;
            }
            KeyCode::Char('3') => {
                self.active_tab = ViewTab::Tasks;
            }
            KeyCode::Char('4') => {
                self.active_tab = ViewTab::Logs;
            }
            KeyCode::Char('5') => {
                self.active_tab = ViewTab::Office;
            }
            KeyCode::Char('6') => {
                self.active_tab = ViewTab::Settings;
            }
            KeyCode::Char('7') => {
                self.active_tab = ViewTab::Performance;
            }
            KeyCode::Tab => {
                let tabs = ViewTab::all();
                let idx = tabs
                    .iter()
                    .position(|t| *t == self.active_tab)
                    .unwrap_or(0);
                self.active_tab = tabs[(idx + 1) % tabs.len()];
            }

            // Enter on settings view toggles values
            KeyCode::Enter => {
                if self.active_tab == ViewTab::Settings {
                    self.toggle_setting();
                } else {
                    self.active_tab = ViewTab::AgentDetail;
                }
            }

            // Save settings
            KeyCode::Char('s') if self.active_tab == ViewTab::Settings => {
                self.save_settings();
            }

            // Mode switches
            KeyCode::Char(':') => {
                self.mode = UiMode::Command;
                self.input_state.clear();
            }
            KeyCode::Char('p') if self.active_tab != ViewTab::Settings => {
                self.mode = UiMode::Prompt;
                self.input_state.clear();
            }
            KeyCode::Char('?') => {
                self.mode = UiMode::Help;
            }

            // Scrolling
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.scroll_offset = self.scroll_offset.saturating_add(10);
            }
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.scroll_offset = self.scroll_offset.saturating_sub(10);
            }
            KeyCode::Char('g') if self.active_tab != ViewTab::Settings => {
                self.scroll_offset = 0;
            }
            KeyCode::Char('G') if self.active_tab != ViewTab::Settings => {
                if let Some(agent) = self.agents.get(self.selected_agent) {
                    self.scroll_offset = agent.output_lines.len().saturating_sub(20) as u16;
                }
            }

            _ => {}
        }
    }

    async fn handle_input_key(&mut self, key: KeyEvent) {
        let is_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        match key.code {
            KeyCode::Esc => {
                self.mode = UiMode::Normal;
                self.input_state.clear();
            }
            // Ctrl+Enter → submit
            KeyCode::Enter if is_ctrl => {
                let input = self.input_state.submit();
                let mode = self.mode;
                self.mode = UiMode::Normal;

                if !input.is_empty() {
                    match mode {
                        UiMode::Command => self.execute_command(&input).await,
                        UiMode::Prompt => self.send_prompt(&input).await,
                        UiMode::TaskInput => self.create_task(&input).await,
                        _ => {}
                    }
                }
            }
            // Enter in Command mode → submit (single-line commands)
            KeyCode::Enter if self.mode == UiMode::Command => {
                let input = self.input_state.submit();
                self.mode = UiMode::Normal;
                if !input.is_empty() {
                    self.execute_command(&input).await;
                }
            }
            // Enter → new line (in Prompt/TaskInput)
            KeyCode::Enter => {
                self.input_state.new_line();
            }
            KeyCode::Backspace => {
                self.input_state.backspace();
            }
            KeyCode::Delete => {
                self.input_state.delete();
            }
            KeyCode::Left => {
                self.input_state.move_left();
            }
            KeyCode::Right => {
                self.input_state.move_right();
            }
            // Ctrl+Up/Down → history navigate
            KeyCode::Up if is_ctrl => {
                self.input_state.history_prev();
            }
            KeyCode::Down if is_ctrl => {
                self.input_state.history_next();
            }
            // Up/Down → move cursor between lines (or history if single line)
            KeyCode::Up => {
                if self.input_state.line_count() == 1 {
                    self.input_state.history_prev();
                } else {
                    self.input_state.move_up();
                }
            }
            KeyCode::Down => {
                if self.input_state.line_count() == 1 {
                    self.input_state.history_next();
                } else {
                    self.input_state.move_down();
                }
            }
            KeyCode::Home => {
                self.input_state.home();
            }
            KeyCode::End => {
                self.input_state.end();
            }
            KeyCode::Char(c) => {
                self.input_state.insert_char(c);
            }
            _ => {}
        }
    }

    async fn handle_settings_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.settings_editing = false;
                self.mode = UiMode::Normal;
            }
            KeyCode::Enter => {
                // Apply edit buffer to the setting
                if self.settings_editing {
                    self.apply_settings_edit();
                    self.settings_editing = false;
                    self.mode = UiMode::Normal;
                }
            }
            KeyCode::Backspace if self.settings_editing => {
                self.settings_edit_buffer.pop();
            }
            KeyCode::Char(c) if self.settings_editing => {
                self.settings_edit_buffer.push(c);
            }
            _ => {}
        }
    }

    fn toggle_setting(&mut self) {
        match self.settings_selected {
            0 => {
                // Language toggle
                self.settings.language = self.settings.language.next();
            }
            1 => {
                // Theme toggle
                self.settings.theme = self.settings.theme.next();
                crate::tui::theme::set_palette(self.settings.theme);
            }
            2 => {
                // Log verbosity toggle
                self.settings.log_verbosity = self.settings.log_verbosity.next();
            }
            3 => {
                // Terminal app - enter edit mode
                self.settings_editing = true;
                self.settings_edit_buffer = self.settings.terminal_app.clone();
                self.mode = UiMode::SettingsEdit;
            }
            4 => {
                // History size - enter edit mode
                self.settings_editing = true;
                self.settings_edit_buffer = self.settings.input_history_size.to_string();
                self.mode = UiMode::SettingsEdit;
            }
            5 => {
                // Meeting timeout - enter edit mode
                self.settings_editing = true;
                self.settings_edit_buffer = self.settings.meeting_timeout_secs.to_string();
                self.mode = UiMode::SettingsEdit;
            }
            6 => {
                // Auto README toggle
                self.settings.auto_readme = !self.settings.auto_readme;
            }
            7 => {
                // Auto Verify toggle
                self.settings.auto_verify = !self.settings.auto_verify;
            }
            8 => {
                // Verify Command - enter edit mode
                self.settings_editing = true;
                self.settings_edit_buffer = self.settings.verify_command.clone();
                self.mode = UiMode::SettingsEdit;
            }
            9 => {
                // Max Retries - enter edit mode
                self.settings_editing = true;
                self.settings_edit_buffer = self.settings.max_verify_retries.to_string();
                self.mode = UiMode::SettingsEdit;
            }
            10 => {
                // Telegram toggle
                self.settings.telegram_enabled = !self.settings.telegram_enabled;
            }
            11 => {
                // TG Bot Token - enter edit mode
                self.settings_editing = true;
                self.settings_edit_buffer = self.settings.telegram_bot_token.clone();
                self.mode = UiMode::SettingsEdit;
            }
            // 12 = TG Chat ID — read-only (managed by pairing)
            _ => {}
        }
    }

    fn apply_settings_edit(&mut self) {
        match self.settings_selected {
            3 => {
                self.settings.terminal_app = self.settings_edit_buffer.clone();
            }
            4 => {
                if let Ok(v) = self.settings_edit_buffer.parse::<usize>() {
                    self.settings.input_history_size = v;
                }
            }
            5 => {
                if let Ok(v) = self.settings_edit_buffer.parse::<u64>() {
                    self.settings.meeting_timeout_secs = v;
                }
            }
            8 => {
                // Verify Command
                self.settings.verify_command = self.settings_edit_buffer.clone();
            }
            9 => {
                // Max Retries
                if let Ok(v) = self.settings_edit_buffer.parse::<u32>() {
                    self.settings.max_verify_retries = v.min(10);
                }
            }
            11 => {
                self.settings.telegram_bot_token = self.settings_edit_buffer.clone();
            }
            _ => {}
        }
    }

    fn save_settings(&mut self) {
        match self.settings.save() {
            Ok(()) => {
                let msg = self.msgs().settings_saved.to_string();
                self.log_entry(LogCategory::System, msg);
            }
            Err(e) => {
                let msg = i18n::fmt(self.msgs().settings_save_error, &[&e]);
                self.log_entry(LogCategory::Error, msg);
            }
        }
    }

    async fn execute_command(&mut self, cmd: &str) {
        let parts: Vec<&str> = cmd.splitn(3, ' ').collect();
        match parts.first().copied() {
            Some("q" | "quit") => {
                let _ = self.cmd_tx.send(OrchestratorCommand::Shutdown).await;
            }
            Some("task") => {
                if let Some(desc) = parts.get(1..) {
                    let description = desc.join(" ");
                    self.create_task(&description).await;
                }
            }
            Some("t") => {
                // Send task to selected agent and track it
                if let Some(desc) = parts.get(1..) {
                    let prompt = desc.join(" ");
                    if !prompt.is_empty() {
                        let agent_name = self
                            .agents
                            .get(self.selected_agent)
                            .map(|a| a.id.0.clone())
                            .unwrap_or_default();
                        let mut task = Task::new(
                            prompt.clone(),
                            TaskPriority::Normal,
                            Vec::new(),
                        );
                        task.assign(AgentId::new(&agent_name));
                        task.start();
                        let msg = i18n::fmt(self.msgs().task_created, &[&task.description]);
                        self.log_entry(LogCategory::Task, msg);
                        self.tasks.push(task);
                        self.send_prompt(&prompt).await;
                    }
                }
            }
            Some("send") => {
                if parts.len() >= 3 {
                    let agent_name = parts[1];
                    let message = parts[2];
                    self.send_inter_agent_message(agent_name, message).await;
                } else {
                    self.log_entry(LogCategory::System, self.msgs().usage_send.to_string());
                }
            }
            Some("stop") => {
                if let Some(agent_name) = parts.get(1) {
                    let id = AgentId::new(agent_name);
                    let _ = self
                        .cmd_tx
                        .send(OrchestratorCommand::StopAgent { agent_id: id })
                        .await;
                }
            }
            Some("broadcast" | "bc") => {
                if let Some(msg) = parts.get(1..) {
                    let prompt = msg.join(" ");
                    if prompt.is_empty() {
                        self.log_entry(LogCategory::System, self.msgs().usage_broadcast.to_string());
                    } else {
                        self.broadcast_prompt(&prompt).await;
                    }
                }
            }
            Some("teamtask" | "tt") => {
                if let Some(desc) = parts.get(1..) {
                    let description = desc.join(" ");
                    if description.is_empty() {
                        self.log_entry(LogCategory::System, self.msgs().usage_teamtask.to_string());
                    } else {
                        // Track team task in Tasks view
                        let mut task = Task::new(
                            format!("[Team] {}", &description),
                            TaskPriority::High,
                            Vec::new(),
                        );
                        task.start();
                        let msg = i18n::fmt(self.msgs().task_created, &[&task.description]);
                        self.log_entry(LogCategory::Task, msg);
                        self.tasks.push(task);

                        let _ = self
                            .cmd_tx
                            .send(OrchestratorCommand::TeamTask {
                                description: description.clone(),
                            })
                            .await;
                        let msg = i18n::fmt(self.msgs().team_task_initiated, &[&description]);
                        self.log_entry(LogCategory::Team, msg);
                    }
                }
            }
            Some(unknown) => {
                let msg = i18n::fmt(self.msgs().unknown_command, &[unknown]);
                self.log_entry(LogCategory::System, msg);
            }
            None => {}
        }
    }

    async fn send_prompt(&mut self, prompt: &str) {
        if let Some(agent) = self.agents.get(self.selected_agent) {
            let agent_id = agent.id.clone();
            let _ = self
                .cmd_tx
                .send(OrchestratorCommand::SendPrompt {
                    agent_id,
                    prompt: prompt.to_string(),
                })
                .await;
        }
    }

    async fn create_task(&mut self, description: &str) {
        let _ = self
            .cmd_tx
            .send(OrchestratorCommand::CreateTask {
                description: description.to_string(),
                priority: TaskPriority::Normal,
                skills: Vec::new(),
            })
            .await;
    }

    async fn broadcast_prompt(&mut self, prompt: &str) {
        let agent_ids: Vec<AgentId> = self.agents.iter().map(|a| a.id.clone()).collect();
        let count = agent_ids.len();
        for agent_id in agent_ids {
            let _ = self
                .cmd_tx
                .send(OrchestratorCommand::SendPrompt {
                    agent_id,
                    prompt: prompt.to_string(),
                })
                .await;
        }
        let msg = i18n::fmt(self.msgs().broadcast_sent, &[&count.to_string()]);
        self.log_entry(LogCategory::Communication, msg);
    }

    async fn send_inter_agent_message(&mut self, agent_name: &str, text: &str) {
        if let Some(from_agent) = self.agents.get(self.selected_agent) {
            let from = from_agent.id.clone();
            let to = AgentId::new(agent_name);
            let message = crate::orchestrator::router::MessageRouter::create_text_message(
                from,
                to,
                text.to_string(),
            );
            let _ = self
                .cmd_tx
                .send(OrchestratorCommand::RouteMessage { message })
                .await;
        }
    }

    /// Send a notification to Telegram if the bridge is active.
    fn notify_telegram(&self, text: &str) {
        if let Some(ref tx) = self.telegram_notify_tx {
            let _ = tx.try_send(TelegramNotification::Text(text.to_string()));
        }
    }

    /// Trigger README.md generation via the lead agent.
    fn trigger_readme_generation(&mut self, task_result: &str) {
        let lead = match self.agents.iter().find(|a| a.config.is_lead) {
            Some(a) => a.id.clone(),
            None => return,
        };
        let prompt = format!(
            "The following task has been completed:\n\n{task_result}\n\n\
            Based on the work done, please write or update the project's README.md file. \
            Make it tutorial-style with clear sections: Overview, Features, Installation, Usage, and Examples. \
            Write the file directly using your tools."
        );
        let cmd = OrchestratorCommand::SendPrompt {
            agent_id: lead,
            prompt,
        };
        let _ = self.cmd_tx.try_send(cmd);
        let msg = self.msgs().auto_readme_generating.to_string();
        self.log_entry(LogCategory::System, msg);
    }

    fn find_agent_mut(&mut self, id: &AgentId) -> Option<&mut AgentState> {
        self.agents.iter_mut().find(|a| a.id == *id)
    }

    fn update_agent_status(&mut self, id: &AgentId, status: AgentStatus) {
        if let Some(agent) = self.find_agent_mut(id) {
            agent.status = status;
        }
    }

    /// Add a structured log entry.
    fn log_entry(&mut self, category: LogCategory, message: String) {
        let entry = LogEntry::new(category, message);
        let display = entry.display();
        self.system_logs.push(entry);
        self.write_to_conversation_log(&display);
    }

    /// Write agent output to conversation log (not to system_logs).
    fn log_agent_output(&mut self, agent_id: &AgentId, text: &str) {
        let timestamp = chrono::Utc::now().format("%H:%M:%S");
        let line = format!("[{timestamp}] [{agent_id}] {text}");
        self.write_to_conversation_log(&line);
    }

    /// Write a line to the conversation log. Disables logging on write error.
    fn write_to_conversation_log(&mut self, line: &str) {
        let failed = if let Some(ref mut f) = self.conversation_log {
            if let Err(e) = writeln!(f, "{}", line) {
                let msg = format!("[WARN] Conversation log write failed: {e} — disabling log");
                self.system_logs.push(LogEntry::new(LogCategory::Error, msg));
                true
            } else {
                let _ = f.flush();
                false
            }
        } else {
            false
        };
        if failed {
            self.conversation_log = None;
        }
    }

    /// Set the conversation log file path.
    pub fn set_conversation_log(&mut self, path: &std::path::Path) {
        match std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            Ok(mut f) => {
                let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S");
                let _ = writeln!(f, "\n=== Session started at {timestamp} ===\n");
                self.conversation_log = Some(f);
            }
            Err(e) => {
                let msg = format!("Cannot open conversation log: {e}");
                self.system_logs.push(LogEntry::new(LogCategory::Error, msg));
            }
        }
    }

    // Convenience getters for rendering

    pub fn agent_states(&self) -> Vec<&AgentState> {
        self.agents.iter().collect()
    }

    pub fn total_cost(&self) -> f64 {
        self.agents.iter().map(|a| a.usage.cost_usd).sum()
    }

    pub fn pending_tasks(&self) -> usize {
        self.tasks
            .iter()
            .filter(|t| t.status == crate::types::task::TaskStatus::Pending)
            .count()
    }

    pub fn active_tasks(&self) -> usize {
        self.tasks
            .iter()
            .filter(|t| {
                matches!(
                    t.status,
                    crate::types::task::TaskStatus::Assigned
                        | crate::types::task::TaskStatus::InProgress
                )
            })
            .count()
    }

    /// Initialize agent states from config.
    pub fn init_agents(&mut self, agents: Vec<AgentState>) {
        self.agents = agents;
    }

    /// Execute a command string as if it came from Telegram (used by scheduler).
    async fn execute_telegram_command(&mut self, cmd: &str) {
        let trimmed = cmd.trim();

        if let Some(rest) = trimmed.strip_prefix(":t ") {
            let parts: Vec<&str> = rest.splitn(2, ' ').collect();
            if parts.len() == 2 {
                let agent_id = AgentId::new(parts[0]);
                let prompt = parts[1].to_string();
                let mut task = Task::new(
                    prompt.clone(),
                    TaskPriority::Normal,
                    Vec::new(),
                );
                task.assign(agent_id.clone());
                task.start();
                let msg = i18n::fmt(self.msgs().task_created, &[&task.description]);
                self.log_entry(LogCategory::Task, msg);
                self.tasks.push(task);
                let _ = self.cmd_tx.send(OrchestratorCommand::SendPrompt {
                    agent_id,
                    prompt,
                }).await;
                return;
            }
        }

        if let Some(rest) = trimmed.strip_prefix(":tt ") {
            let description = rest.to_string();
            let mut task = Task::new(
                format!("[Team] {}", &description),
                TaskPriority::High,
                Vec::new(),
            );
            task.start();
            let msg = i18n::fmt(self.msgs().task_created, &[&task.description]);
            self.log_entry(LogCategory::Task, msg);
            self.tasks.push(task);
            let _ = self.cmd_tx.send(OrchestratorCommand::TeamTask {
                description,
            }).await;
            return;
        }

        if let Some(rest) = trimmed.strip_prefix(":bc ") {
            let _ = self.cmd_tx.send(OrchestratorCommand::Broadcast {
                prompt: rest.to_string(),
            }).await;
            return;
        }

        if let Some(rest) = trimmed.strip_prefix(":stop ") {
            let _ = self.cmd_tx.send(OrchestratorCommand::StopAgent {
                agent_id: AgentId::new(rest.trim()),
            }).await;
            return;
        }

        if trimmed == ":q" {
            let _ = self.cmd_tx.send(OrchestratorCommand::Shutdown).await;
            return;
        }

        // Default: send to lead agent
        let _ = self.cmd_tx.send(OrchestratorCommand::PromptLead {
            prompt: trimmed.to_string(),
        }).await;
    }
}

/// Parse "HH:MM" or "HH:MM:SS" in local time into a UTC DateTime. If the time is in the past today, schedule for tomorrow.
fn parse_schedule_time(time_str: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    let parts: Vec<&str> = time_str.split(':').collect();
    let (hour, minute, second) = match parts.len() {
        2 => {
            let h = parts[0].parse::<u32>().ok()?;
            let m = parts[1].parse::<u32>().ok()?;
            (h, m, 0)
        }
        3 => {
            let h = parts[0].parse::<u32>().ok()?;
            let m = parts[1].parse::<u32>().ok()?;
            let s = parts[2].parse::<u32>().ok()?;
            (h, m, s)
        }
        _ => return None,
    };

    if hour > 23 || minute > 59 || second > 59 {
        return None;
    }

    let now_local = chrono::Local::now();
    let today = now_local.date_naive();
    let time = chrono::NaiveTime::from_hms_opt(hour, minute, second)?;
    let dt = today.and_time(time);

    // Attach local timezone, then convert to UTC
    let candidate = dt.and_local_timezone(chrono::Local).single()?;
    let candidate_utc = candidate.with_timezone(&chrono::Utc);

    if candidate_utc <= chrono::Utc::now() {
        // Schedule for tomorrow
        let tomorrow = today + chrono::Duration::days(1);
        let dt2 = tomorrow.and_time(time);
        let tomorrow_local = dt2.and_local_timezone(chrono::Local).single()?;
        Some(tomorrow_local.with_timezone(&chrono::Utc))
    } else {
        Some(candidate_utc)
    }
}

/// Auto-detect verify command based on project files in CWD.
fn detect_verify_command() -> Option<String> {
    if std::path::Path::new("Cargo.toml").exists() {
        Some("cargo build && cargo test".to_string())
    } else if std::path::Path::new("package.json").exists() {
        Some("npm run build && npm test".to_string())
    } else if std::path::Path::new("Makefile").exists() {
        Some("make && make test".to_string())
    } else if std::path::Path::new("pyproject.toml").exists() {
        Some("python -m pytest".to_string())
    } else if std::path::Path::new("go.mod").exists() {
        Some("go build ./... && go test ./...".to_string())
    } else {
        None
    }
}

/// Convert raw Rust error strings into human-readable messages.
fn humanize_error(error: &str) -> String {
    // Common OS errors
    if error.contains("os error 2") || error.contains("No such file") {
        return format!("Command not found or file missing ({})", error);
    }
    if error.contains("os error 13") || error.contains("Permission denied") {
        return format!("Permission denied ({})", error);
    }
    if error.contains("os error 98") || error.contains("Address already in use") {
        return format!("Port already in use ({})", error);
    }
    if error.contains("os error 111") || error.contains("Connection refused") {
        return format!("Connection refused ({})", error);
    }
    if error.contains("timed out") || error.contains("Timeout") {
        return "Operation timed out".to_string();
    }
    if error.contains("broken pipe") {
        return "Agent process terminated unexpectedly".to_string();
    }
    // Claude CLI specific
    if error.contains("not found in PATH") {
        return "Claude CLI not found — is it installed?".to_string();
    }
    // Return original if no match, but trim overly long chains
    let chars: Vec<char> = error.chars().collect();
    if chars.len() > 120 {
        let truncated: String = chars[..117].iter().collect();
        format!("{}...", truncated)
    } else {
        error.to_string()
    }
}
