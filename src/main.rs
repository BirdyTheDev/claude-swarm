use anyhow::{Context, Result};
use clap::Parser;
use tokio::sync::mpsc;
use tracing::info;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

use claude_swarm::config::{CliArgs, Settings, SwarmConfig};
use claude_swarm::orchestrator::Orchestrator;
use claude_swarm::tui::app::App;
use claude_swarm::tui::event_handler::EventHandler;
use claude_swarm::tui::terminal;
use claude_swarm::tui::layout;
use claude_swarm::tui::theme;
use claude_swarm::tui::widgets::{command_input, header_bar, help_overlay, status_bar};
use claude_swarm::tui::views::{agent_detail, dashboard, log_view, office_view, settings_view, task_view};
use claude_swarm::types::agent::AgentState;
use claude_swarm::types::event::{AppEvent, OrchestratorCommand, UiMode, ViewTab};

fn setup_logging(args: &CliArgs) -> Result<WorkerGuard> {
    let file = std::fs::File::create(&args.log_file)
        .with_context(|| format!("creating log file: {}", args.log_file.display()))?;
    let (non_blocking, guard) = tracing_appender::non_blocking(file);

    let filter = EnvFilter::try_new(&args.log_level).unwrap_or_else(|e| {
        eprintln!(
            "warning: invalid log level '{}' ({}), falling back to 'info'",
            args.log_level, e
        );
        EnvFilter::new("info")
    });

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(non_blocking)
        .with_ansi(false)
        .init();

    Ok(guard)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = CliArgs::parse();

    // Setup file logging (must keep guard alive)
    let _log_guard = setup_logging(&args)?;

    info!("claude-swarm starting");

    // Check claude CLI is available
    which::which("claude").context(
        "claude CLI not found in PATH. Install it from https://docs.anthropic.com/en/docs/claude-code",
    )?;

    // Load config
    let mut config = SwarmConfig::load(&args.config)?;
    if let Some(ref agent_names) = args.agents {
        config.filter_agents(agent_names);
    }

    info!(
        name = %config.name,
        agents = config.agent.len(),
        "config loaded"
    );

    // Create channels
    let (event_tx, mut event_rx) = mpsc::channel::<AppEvent>(512);
    let (cmd_tx, cmd_rx) = mpsc::channel::<OrchestratorCommand>(256);

    // Initialize agent states for TUI
    let agent_states: Vec<AgentState> = config
        .agent_configs()
        .into_iter()
        .map(|(id, cfg)| AgentState::new(id, cfg))
        .collect();

    // Load user settings and initialize theme
    let settings = Settings::load();
    theme::set_palette(settings.theme);

    // Create app
    let swarm_name = config.name.clone();
    let mut app = App::new(swarm_name, cmd_tx.clone(), settings);
    app.init_agents(agent_states);

    // Setup persistent conversation log
    let conversation_log_path = std::path::PathBuf::from("claude-swarm-conversation.log");
    app.set_conversation_log(&conversation_log_path);
    info!(path = %conversation_log_path.display(), "conversation log enabled");

    // Spawn orchestrator
    let mut orchestrator = Orchestrator::new(config, event_tx.clone(), cmd_rx);

    // Setup visible terminals if requested
    if args.visible_terminals {
        let log_dir = std::path::PathBuf::from(format!(
            "/tmp/claude-swarm-{}/",
            app.swarm_name.replace(' ', "-")
        ));
        std::fs::create_dir_all(&log_dir)
            .with_context(|| format!("creating log dir: {}", log_dir.display()))?;
        info!(dir = %log_dir.display(), "visible terminals enabled");
        orchestrator.set_visible_terminals(log_dir);
    }

    let orch_handle = tokio::spawn(async move {
        if let Err(e) = orchestrator.run().await {
            tracing::error!("orchestrator error: {e}");
        }
    });

    // Spawn health check server
    tokio::spawn(async {
        if let Err(e) = claude_swarm::health::serve().await {
            tracing::warn!("health server failed: {e}");
        }
    });

    // Spawn event handler
    let event_handler = EventHandler::new(event_tx.clone(), args.tick_rate);
    let _evt_handle = event_handler.spawn();

    // Send initial prompt if provided
    if let Some(prompt) = args.prompt {
        // Find lead agent
        if let Some(lead) = app.agents.iter().find(|a| a.config.is_lead) {
            let _ = cmd_tx
                .send(OrchestratorCommand::SendPrompt {
                    agent_id: lead.id.clone(),
                    prompt,
                })
                .await;
        }
    }

    // Setup terminal
    let mut tui = terminal::setup()?;

    // Main render + event loop
    while app.running {
        // Render
        tui.draw(|frame| {
            let (header_area, main_area, status_area) = layout::main_layout(frame.area());

            // Header
            header_bar::render(frame, header_area, &app.swarm_name, app.active_tab);

            // Main content area (may include input bar)
            let content_area = if matches!(
                app.mode,
                UiMode::Command | UiMode::Prompt | UiMode::TaskInput
            ) {
                let input_height = app.input_state.visible_height();
                let (content, input_area) = layout::with_input(main_area, input_height);
                command_input::render(
                    frame,
                    input_area,
                    app.mode,
                    &app.input_state,
                );
                content
            } else {
                main_area
            };

            // View content
            let agents = app.agent_states();
            match app.active_tab {
                ViewTab::Dashboard => {
                    dashboard::render(
                        frame,
                        content_area,
                        &agents,
                        app.selected_agent,
                        app.scroll_offset,
                    );
                }
                ViewTab::AgentDetail => {
                    if let Some(agent) = agents.get(app.selected_agent) {
                        agent_detail::render(
                            frame,
                            content_area,
                            agent,
                            app.scroll_offset,
                        );
                    }
                }
                ViewTab::Tasks => {
                    let tasks: Vec<&_> = app.tasks.iter().collect();
                    task_view::render(frame, content_area, &tasks, app.selected_task);
                }
                ViewTab::Logs => {
                    log_view::render(
                        frame,
                        content_area,
                        &app.system_logs,
                        app.settings.log_verbosity,
                    );
                }
                ViewTab::Office => {
                    office_view::render(
                        frame,
                        content_area,
                        &agents,
                        &app.office_state,
                        &app.messages,
                    );
                }
                ViewTab::Settings => {
                    settings_view::render(
                        frame,
                        content_area,
                        &app.settings,
                        app.settings_selected,
                        app.settings_editing,
                        &app.settings_edit_buffer,
                    );
                }
            }

            // Status bar
            status_bar::render(
                frame,
                status_area,
                app.mode,
                app.agents.len(),
                app.total_cost(),
                app.pending_tasks(),
                app.active_tasks(),
            );

            // Help overlay
            if app.mode == UiMode::Help {
                help_overlay::render(frame, frame.area());
            }
        })?;

        // Wait for at least one event, then drain all pending events
        if let Some(event) = event_rx.recv().await {
            app.handle_event(event).await;
            // Drain remaining pending events without blocking
            while let Ok(event) = event_rx.try_recv() {
                app.handle_event(event).await;
                if !app.running {
                    break;
                }
            }
        } else {
            break;
        }
    }

    // Cleanup
    terminal::restore()?;

    // Wait for orchestrator to finish
    let _ = orch_handle.await;

    info!("claude-swarm stopped");
    Ok(())
}
