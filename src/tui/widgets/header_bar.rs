use ratatui::prelude::*;
use ratatui::widgets::Tabs;

use crate::tui::theme;
use crate::types::event::ViewTab;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    swarm_name: &str,
    active_tab: ViewTab,
) {
    let tab_titles: Vec<Line> = ViewTab::all()
        .iter()
        .map(|t| Line::from(format!(" {} ", t.title())))
        .collect();

    let active_index = ViewTab::all()
        .iter()
        .position(|t| *t == active_tab)
        .unwrap_or(0);

    let tabs = Tabs::new(tab_titles)
        .select(active_index)
        .style(theme::header_style())
        .highlight_style(theme::title_style())
        .divider(Span::styled("│", theme::dim_style()));

    // Render swarm name + tabs
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(swarm_name.len() as u16 + 4), Constraint::Min(10)])
        .split(area);

    let name = ratatui::widgets::Paragraph::new(
        Line::from(vec![
            Span::styled(" ⬡ ", Style::default().fg(theme::accent())),
            Span::styled(swarm_name, theme::bold_style()),
            Span::raw(" "),
        ])
    ).style(theme::header_style());

    frame.render_widget(name, chunks[0]);
    frame.render_widget(tabs, chunks[1]);
}
