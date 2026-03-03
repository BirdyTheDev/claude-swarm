use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::tui::theme;
use crate::types::agent::AgentState;

/// Render a mini panel showing recent agent output.
pub fn render_mini(frame: &mut Frame, area: Rect, agent: &AgentState) {
    let status_style = theme::agent_status_style(&agent.status);
    let title = format!(" {} [{}] ", agent.config.name, agent.status);

    let block = Block::default()
        .title(Span::styled(title, status_style))
        .borders(Borders::ALL)
        .border_style(theme::border_style());

    // Show last N lines that fit in the area
    let inner_height = area.height.saturating_sub(2) as usize;
    let start = agent.output_lines.len().saturating_sub(inner_height);
    let visible_lines: Vec<Line> = agent.output_lines[start..]
        .iter()
        .map(|l| Line::from(l.as_str()))
        .collect();

    let paragraph = Paragraph::new(visible_lines)
        .block(block)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

/// Render a full scrollable output panel for the focused agent.
pub fn render_full(
    frame: &mut Frame,
    area: Rect,
    agent: &AgentState,
    scroll_offset: u16,
) {
    let status_style = theme::agent_status_style(&agent.status);
    let role_info = if agent.config.role.is_empty() {
        String::new()
    } else {
        format!(" - {}", agent.config.role)
    };
    let title = format!(
        " {}{} [{}] ",
        agent.config.name, role_info, agent.status
    );

    let block = Block::default()
        .title(Span::styled(title, status_style))
        .borders(Borders::ALL)
        .border_style(theme::border_style());

    let lines: Vec<Line> = agent
        .output_lines
        .iter()
        .map(|l| Line::from(l.as_str()))
        .collect();

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll_offset, 0));

    frame.render_widget(paragraph, area);
}
