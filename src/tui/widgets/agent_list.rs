use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};

use crate::tui::theme;
use crate::types::agent::AgentState;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    agents: &[&AgentState],
    selected: usize,
) {
    let items: Vec<ListItem> = agents
        .iter()
        .enumerate()
        .map(|(i, agent)| {
            let status_style = theme::agent_status_style(&agent.status);
            let icon = agent.status.icon();
            let cost = if agent.usage.cost_usd > 0.0 {
                format!(" ${:.2}", agent.usage.cost_usd)
            } else {
                String::new()
            };
            let content = Line::from(vec![
                Span::styled(format!("{icon} "), status_style),
                Span::styled(
                    &agent.config.name,
                    if i == selected {
                        theme::selected_style().add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    },
                ),
                Span::styled(cost, theme::dim_style()),
            ]);
            ListItem::new(content)
        })
        .collect();

    let block = Block::default()
        .title(" Agents ")
        .title_style(theme::title_style())
        .borders(Borders::ALL)
        .border_style(theme::border_style());

    let list = List::new(items)
        .block(block)
        .highlight_style(theme::selected_style());

    let mut state = ListState::default();
    state.select(Some(selected));

    frame.render_stateful_widget(list, area, &mut state);
}
