use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::tui::theme;
use crate::types::agent::{AgentState, AgentStatus};
use crate::types::communication::InterAgentMessage;

/// State tracking for meeting room animations.
pub struct MeetingEntry {
    pub agent_a: String,
    pub agent_b: String,
    pub topic: String,
    pub expires_at: std::time::Instant,
}

pub struct OfficeState {
    pub meetings: Vec<MeetingEntry>,
}

impl OfficeState {
    pub fn new() -> Self {
        Self {
            meetings: Vec::new(),
        }
    }

    /// Record a new meeting between two agents.
    pub fn add_meeting(&mut self, from: &str, to: &str, topic: &str) {
        // Remove existing meetings involving either agent
        self.meetings
            .retain(|m| m.agent_a != from && m.agent_b != from && m.agent_a != to && m.agent_b != to);
        self.meetings.push(MeetingEntry {
            agent_a: from.to_string(),
            agent_b: to.to_string(),
            topic: {
                let chars: Vec<char> = topic.chars().collect();
                if chars.len() > 40 {
                    let truncated: String = chars[..37].iter().collect();
                    format!("{}...", truncated)
                } else {
                    topic.to_string()
                }
            },
            expires_at: std::time::Instant::now() + std::time::Duration::from_secs(10),
        });
    }

    /// Expire old meetings, returns names of agents that left meetings.
    pub fn tick(&mut self) -> Vec<String> {
        let now = std::time::Instant::now();
        let mut freed = Vec::new();
        self.meetings.retain(|m| {
            if now >= m.expires_at {
                freed.push(m.agent_a.clone());
                freed.push(m.agent_b.clone());
                false
            } else {
                true
            }
        });
        freed
    }

    /// Check if an agent is currently in a meeting.
    pub fn is_in_meeting(&self, agent_name: &str) -> bool {
        self.meetings
            .iter()
            .any(|m| m.agent_a == agent_name || m.agent_b == agent_name)
    }
}

pub fn render(
    frame: &mut Frame,
    area: Rect,
    agents: &[&AgentState],
    office_state: &OfficeState,
    recent_messages: &[InterAgentMessage],
) {
    let outer_block = Block::default()
        .title(" Office ")
        .title_style(theme::title_style())
        .borders(Borders::ALL)
        .border_style(theme::border_style());

    let inner = outer_block.inner(area);
    frame.render_widget(outer_block, area);

    // Split into: cubicles area, meeting room, message log
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(6),     // cubicles
            Constraint::Length(7),  // meeting room
            Constraint::Length(4),  // recent messages
        ])
        .split(inner);

    // --- Cubicles ---
    render_cubicles(frame, chunks[0], agents, office_state);

    // --- Meeting Room ---
    render_meeting_room(frame, chunks[1], office_state);

    // --- Recent Messages ---
    render_message_log(frame, chunks[2], recent_messages);
}

fn render_cubicles(
    frame: &mut Frame,
    area: Rect,
    agents: &[&AgentState],
    office_state: &OfficeState,
) {
    if agents.is_empty() {
        return;
    }

    // Calculate cubicle layout: fit as many per row as possible
    let cubicle_width = 18u16;
    let cols = (area.width / cubicle_width).max(1) as usize;
    let rows = (agents.len() + cols - 1) / cols;

    let row_height = 5u16;
    let row_constraints: Vec<Constraint> = (0..rows)
        .map(|_| Constraint::Length(row_height))
        .collect();

    let row_areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints(row_constraints)
        .split(area);

    for (i, agent) in agents.iter().enumerate() {
        let row = i / cols;
        let col = i % cols;

        if row >= row_areas.len() {
            break;
        }

        let x = row_areas[row].x + (col as u16) * cubicle_width;
        let w = cubicle_width.min(row_areas[row].right().saturating_sub(x));
        if w < 4 {
            continue;
        }
        let cubicle_area = Rect::new(x, row_areas[row].y, w, row_height.min(row_areas[row].height));

        let in_meeting = office_state.is_in_meeting(agent.id.as_ref());
        let effective_status = if in_meeting {
            AgentStatus::InMeeting
        } else {
            agent.status
        };

        let status_text = match effective_status {
            AgentStatus::Working => ">_ typing...",
            AgentStatus::Idle => "zzz",
            AgentStatus::InMeeting => ">>meeting<<",
            AgentStatus::Starting => "booting...",
            AgentStatus::Waiting => "...",
            AgentStatus::Completed => "done",
            AgentStatus::Failed => "ERROR",
            AgentStatus::Stopped => "offline",
        };

        let name = &agent.id.0;
        let display_name: String = if name.len() > (w as usize - 4) {
            name.chars().take((w as usize).saturating_sub(4)).collect()
        } else {
            name.to_string()
        };

        let status_style = theme::agent_status_style(&effective_status);

        let block = Block::default()
            .title(format!(" {} ", display_name))
            .title_style(Style::default().fg(theme::fg()).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(status_style);

        let inner_area = block.inner(cubicle_area);
        frame.render_widget(block, cubicle_area);

        if inner_area.height >= 2 {
            // Status label
            let status_label = match effective_status {
                AgentStatus::Working => "[working]",
                AgentStatus::Idle => "[idle]",
                AgentStatus::InMeeting => "[meeting]",
                AgentStatus::Starting => "[starting]",
                AgentStatus::Waiting => "[waiting]",
                AgentStatus::Completed => "[done]",
                AgentStatus::Failed => "[failed]",
                AgentStatus::Stopped => "[stopped]",
            };

            let lines = vec![
                Line::from(Span::styled(status_label, status_style)),
                Line::from(Span::styled(
                    format!(" {}", status_text),
                    theme::dim_style(),
                )),
            ];
            let p = Paragraph::new(lines);
            frame.render_widget(p, inner_area);
        } else if inner_area.height >= 1 {
            let p = Paragraph::new(Span::styled(status_text, status_style));
            frame.render_widget(p, inner_area);
        }
    }
}

fn render_meeting_room(frame: &mut Frame, area: Rect, office_state: &OfficeState) {
    let block = Block::default()
        .title(" Meeting Room ")
        .title_style(theme::title_style())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::accent()));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if office_state.meetings.is_empty() {
        let empty = Paragraph::new(Span::styled(
            "  (empty - no active meetings)",
            theme::dim_style(),
        ));
        frame.render_widget(empty, inner);
        return;
    }

    let mut lines = Vec::new();
    for meeting in &office_state.meetings {
        lines.push(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(&meeting.agent_a, Style::default().fg(theme::accent()).add_modifier(Modifier::BOLD)),
            Span::styled(" <-> ", Style::default().fg(theme::fg())),
            Span::styled(&meeting.agent_b, Style::default().fg(theme::accent()).add_modifier(Modifier::BOLD)),
        ]));
        lines.push(Line::from(Span::styled(
            format!("  \"{}\"", meeting.topic),
            theme::dim_style(),
        )));
    }

    let p = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(p, inner);
}

fn render_message_log(frame: &mut Frame, area: Rect, messages: &[InterAgentMessage]) {
    let block = Block::default()
        .title(" Recent ")
        .title_style(theme::dim_style())
        .borders(Borders::TOP)
        .border_style(theme::border_style());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Show last N messages that fit
    let max_lines = inner.height as usize;
    let start = messages.len().saturating_sub(max_lines);
    let mut lines = Vec::new();
    for msg in &messages[start..] {
        let summary = msg.content.summary();
        let max_chars = (inner.width as usize).saturating_sub(20);
        let chars: Vec<char> = summary.chars().collect();
        let display: String = if chars.len() > max_chars {
            let trunc: String = chars[..max_chars.saturating_sub(3)].iter().collect();
            format!("{}...", trunc)
        } else {
            summary
        };
        lines.push(Line::from(vec![
            Span::styled(
                format!("{}", msg.from),
                Style::default().fg(theme::accent()),
            ),
            Span::styled(" -> ", theme::dim_style()),
            Span::styled(
                format!("{}", msg.to),
                Style::default().fg(theme::accent()),
            ),
            Span::styled(format!(": {}", display), Style::default().fg(theme::fg())),
        ]));
    }

    let p = Paragraph::new(lines);
    frame.render_widget(p, inner);
}
