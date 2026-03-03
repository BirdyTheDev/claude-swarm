use ratatui::prelude::*;
use ratatui::widgets::Paragraph;

use crate::tui::theme;
use crate::types::event::UiMode;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    mode: UiMode,
    agent_count: usize,
    total_cost: f64,
    task_pending: usize,
    task_active: usize,
) {
    let mode_str = match mode {
        UiMode::Normal => "NORMAL",
        UiMode::Command => "COMMAND",
        UiMode::Prompt => "PROMPT",
        UiMode::Help => "HELP",
        UiMode::TaskInput => "TASK",
        UiMode::SettingsEdit => "EDIT",
    };

    let content = Line::from(vec![
        Span::styled(format!(" {mode_str} "), theme::mode_style()),
        Span::styled(" | ", theme::dim_style()),
        Span::styled(format!("agents: {agent_count}"), Style::default()),
        Span::styled(" | ", theme::dim_style()),
        Span::styled(
            format!("cost: ${total_cost:.2}"),
            if total_cost > 5.0 {
                Style::default().fg(theme::warning())
            } else {
                Style::default()
            },
        ),
        Span::styled(" | ", theme::dim_style()),
        Span::styled(
            format!("tasks: {task_pending} pending, {task_active} active"),
            Style::default(),
        ),
        Span::styled(" | ", theme::dim_style()),
        Span::styled(
            "j/k:nav  p:prompt  :cmd  ?:help  q:quit",
            theme::dim_style(),
        ),
    ]);

    let bar = Paragraph::new(content).style(theme::status_bar_style());
    frame.render_widget(bar, area);
}
