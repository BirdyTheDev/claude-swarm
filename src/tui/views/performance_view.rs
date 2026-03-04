use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table, Wrap};
use crate::tui::theme;
use crate::types::agent::AgentState;
use crate::tui::app::App;

pub fn render(frame: &mut Frame, area: Rect, agents: &[&AgentState], app: &App) {
    let block = Block::default()
        .title(" Performance ")
        .title_style(theme::title_style())
        .borders(Borders::ALL)
        .border_style(theme::border_style());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // 4-section layout: Process Resources, Aggregate Stats, Token Usage, Swarm Health
    let sections = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(7),
            Constraint::Length(5),
            Constraint::Min(8),
            Constraint::Length(5),
        ])
        .split(inner);

    render_process_section(frame, sections[0], app);
    render_aggregate_stats_section(frame, sections[1], agents, app);
    render_token_usage_section(frame, sections[2], agents);
    render_swarm_health_section(frame, sections[3], agents, app);
}

fn render_process_section(frame: &mut Frame, area: Rect, app: &App) {
    let title_block = Block::default()
        .title(" Process Resources ")
        .title_style(theme::title_style())
        .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT);

    let own_pid = sysinfo::get_current_pid().ok();

    let mut own_mem: u64 = 0;
    let mut own_cpu: f32 = 0.0;
    let mut claude_count: usize = 0;
    let mut claude_mem: u64 = 0;
    let mut claude_cpu: f32 = 0.0;

    for (pid, process) in app.sysinfo.processes() {
        let is_own = own_pid.map(|p| p == *pid).unwrap_or(false);

        if is_own {
            own_mem = process.memory();
            own_cpu = process.cpu_usage();
            continue;
        }

        // Check if it's a claude CLI process
        let name = process.name();
        let is_claude = name == "claude"
            || process.cmd().iter().any(|arg| arg.contains("claude"));

        if is_claude {
            claude_count += 1;
            claude_mem += process.memory();
            claude_cpu += process.cpu_usage();
        }
    }

    let own_mem_mb = own_mem as f64 / (1024.0 * 1024.0);
    let claude_mem_mb = claude_mem as f64 / (1024.0 * 1024.0);
    let total_mem_mb = own_mem_mb + claude_mem_mb;
    let total_cpu = own_cpu + claude_cpu;

    let text = vec![
        Line::from(vec![
            Span::styled("claude-swarm  ", Style::default().fg(theme::success()).add_modifier(Modifier::BOLD)),
            Span::styled("Mem: ", Style::default().fg(theme::accent())),
            Span::raw(format!("{:.1} MB", own_mem_mb)),
            Span::raw("  "),
            Span::styled("CPU: ", Style::default().fg(theme::accent())),
            Span::raw(format!("{:.1}%", own_cpu)),
        ]),
        Line::from(vec![
            Span::styled(
                format!("claude CLI x{:<3}", claude_count),
                Style::default().fg(theme::working()).add_modifier(Modifier::BOLD),
            ),
            Span::styled("Mem: ", Style::default().fg(theme::accent())),
            Span::raw(format!("{:.1} MB", claude_mem_mb)),
            Span::raw("  "),
            Span::styled("CPU: ", Style::default().fg(theme::accent())),
            Span::raw(format!("{:.1}%", claude_cpu)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Total         ", Style::default().fg(theme::accent()).add_modifier(Modifier::BOLD)),
            Span::styled("Mem: ", Style::default().fg(theme::accent())),
            Span::raw(format!("{:.1} MB", total_mem_mb)),
            Span::raw("  "),
            Span::styled("CPU: ", Style::default().fg(theme::accent())),
            Span::raw(format!("{:.1}%", total_cpu)),
            Span::raw("  "),
            Span::styled("Procs: ", Style::default().fg(theme::accent())),
            Span::raw(format!("{}", 1 + claude_count)),
        ]),
    ];

    let paragraph = Paragraph::new(text)
        .block(title_block)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

fn render_aggregate_stats_section(frame: &mut Frame, area: Rect, agents: &[&AgentState], app: &App) {
    let title_block = Block::default()
        .title(" Aggregate Stats ")
        .title_style(theme::title_style())
        .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT);

    let total_tokens: u64 = agents.iter().map(|a| a.usage.input_tokens + a.usage.output_tokens).sum();
    let total_cost: f64 = agents.iter().map(|a| a.usage.cost_usd).sum();

    let uptime_secs = std::time::Instant::now().duration_since(app.started_at).as_secs_f64();
    let cost_per_hr = if uptime_secs > 0.0 {
        total_cost / (uptime_secs / 3600.0)
    } else {
        0.0
    };

    let agent_count = agents.len().max(1) as f64;
    let avg_tokens = total_tokens as f64 / agent_count;

    let completed = app.tasks.iter().filter(|t| t.status == crate::types::task::TaskStatus::Completed).count();
    let failed = app.tasks.iter().filter(|t| t.status == crate::types::task::TaskStatus::Failed).count();
    let total_done = completed + failed;
    let success_rate = if total_done > 0 {
        (completed as f64 / total_done as f64) * 100.0
    } else {
        100.0
    };

    let text = vec![
        Line::from(vec![
            Span::styled("Tokens: ", Style::default().fg(theme::accent()).add_modifier(Modifier::BOLD)),
            Span::raw(format!("{}", total_tokens)),
            Span::raw("  "),
            Span::styled("Cost: ", Style::default().fg(theme::accent()).add_modifier(Modifier::BOLD)),
            Span::styled(format!("${:.4}", total_cost), Style::default().fg(theme::warning())),
            Span::raw("  "),
            Span::styled("Cost/hr: ", Style::default().fg(theme::accent()).add_modifier(Modifier::BOLD)),
            Span::styled(format!("${:.4}", cost_per_hr), Style::default().fg(theme::warning())),
        ]),
        Line::from(vec![
            Span::styled("Avg tokens/agent: ", Style::default().fg(theme::accent()).add_modifier(Modifier::BOLD)),
            Span::raw(format!("{:.0}", avg_tokens)),
            Span::raw("  "),
            Span::styled("Tasks: ", Style::default().fg(theme::accent()).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{} done", completed), Style::default().fg(theme::success())),
            Span::raw(" / "),
            Span::styled(format!("{} failed", failed), Style::default().fg(theme::error_color())),
            Span::raw("  "),
            Span::styled("Success: ", Style::default().fg(theme::accent()).add_modifier(Modifier::BOLD)),
            Span::raw(format!("{:.0}%", success_rate)),
        ]),
    ];

    let paragraph = Paragraph::new(text)
        .block(title_block)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

fn render_token_usage_section(frame: &mut Frame, area: Rect, agents: &[&AgentState]) {
    let title_block = Block::default()
        .title(" Token Usage per Agent ")
        .title_style(theme::title_style())
        .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT);

    let headers = Row::new(vec!["Agent", "Input", "Output", "Cache Read", "Cost"])
        .style(theme::header_style())
        .bottom_margin(1);

    let rows: Vec<Row> = agents
        .iter()
        .map(|agent| {
            let agent_name = Span::styled(
                &agent.id.0,
                Style::default().fg(theme::success()),
            );
            let input = Span::raw(agent.usage.input_tokens.to_string());
            let output = Span::raw(agent.usage.output_tokens.to_string());
            let cache = Span::raw(agent.usage.cache_read_tokens.to_string());
            let cost = Span::styled(
                format!("${:.4}", agent.usage.cost_usd),
                Style::default().fg(theme::warning()),
            );

            Row::new(vec![agent_name, input, output, cache, cost])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ],
    )
    .header(headers)
    .block(title_block);

    frame.render_widget(table, area);
}

fn render_swarm_health_section(frame: &mut Frame, area: Rect, agents: &[&AgentState], app: &App) {
    let title_block = Block::default()
        .title(" Swarm Health ")
        .title_style(theme::title_style())
        .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT | Borders::BOTTOM);

    let uptime = std::time::Instant::now().duration_since(app.started_at);
    let uptime_secs = uptime.as_secs();
    let hours = uptime_secs / 3600;
    let minutes = (uptime_secs % 3600) / 60;
    let seconds = uptime_secs % 60;

    let idle_count = agents.iter().filter(|a| a.status == crate::types::agent::AgentStatus::Idle).count();
    let working_count = agents.iter().filter(|a| a.status == crate::types::agent::AgentStatus::Working).count();

    let task_summary = if app.pending_tasks() > 0 {
        format!(
            "Pending: {}, Active: {}, Completed: {}",
            app.pending_tasks(),
            app.active_tasks(),
            app.tasks.len().saturating_sub(app.pending_tasks() + app.active_tasks())
        )
    } else {
        format!(
            "Active: {}, Completed: {}",
            app.active_tasks(),
            app.tasks.len().saturating_sub(app.active_tasks())
        )
    };

    let health_text = vec![
        Line::from(vec![
            Span::styled("Uptime: ", Style::default().fg(theme::accent()).add_modifier(Modifier::BOLD)),
            Span::raw(format!("{}h {}m {}s", hours, minutes, seconds)),
            Span::raw("  "),
            Span::styled("Messages: ", Style::default().fg(theme::accent()).add_modifier(Modifier::BOLD)),
            Span::raw(format!("{}", app.total_messages_routed)),
            Span::raw("  "),
            Span::styled("Agents: ", Style::default().fg(theme::accent()).add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("{} idle", idle_count),
                Style::default().fg(theme::success()),
            ),
            Span::raw(" / "),
            Span::styled(
                format!("{} working", working_count),
                Style::default().fg(theme::working()),
            ),
        ]),
        Line::from(vec![
            Span::styled("Tasks: ", Style::default().fg(theme::accent()).add_modifier(Modifier::BOLD)),
            Span::raw(task_summary),
        ]),
    ];

    let paragraph = Paragraph::new(health_text)
        .block(title_block)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}
