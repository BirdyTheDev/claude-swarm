use ratatui::prelude::*;

use crate::tui::layout;
use crate::tui::widgets::{agent_list, agent_panel};
use crate::types::agent::AgentState;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    agents: &[&AgentState],
    selected: usize,
    scroll_offset: u16,
) {
    let (list_area, mini_area, focus_area) = layout::dashboard_layout(area);

    // Left sidebar: agent list
    agent_list::render(frame, list_area, agents, selected);

    // Middle: mini panels for non-focused agents
    let other_agents: Vec<&&AgentState> = agents
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != selected)
        .map(|(_, a)| a)
        .collect();

    if !other_agents.is_empty() {
        let mini_chunks = layout::split_vertical(mini_area, other_agents.len().min(6));
        for (i, chunk) in mini_chunks.iter().enumerate() {
            if i < other_agents.len() {
                agent_panel::render_mini(frame, *chunk, other_agents[i]);
            }
        }
    }

    // Right: focused agent output (scrollable)
    if let Some(agent) = agents.get(selected) {
        agent_panel::render_full(frame, focus_area, agent, scroll_offset);
    }
}
