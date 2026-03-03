use ratatui::prelude::*;

use crate::tui::widgets::agent_panel;
use crate::types::agent::AgentState;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    agent: &AgentState,
    scroll_offset: u16,
) {
    agent_panel::render_full(frame, area, agent, scroll_offset);
}
