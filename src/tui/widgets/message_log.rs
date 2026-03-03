use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem};

use crate::tui::theme;
use crate::types::communication::InterAgentMessage;

pub fn render(frame: &mut Frame, area: Rect, messages: &[InterAgentMessage]) {
    let items: Vec<ListItem> = messages
        .iter()
        .rev()
        .take(area.height.saturating_sub(2) as usize)
        .map(|msg| {
            let time = msg.timestamp.format("%H:%M:%S");
            let summary = msg.content.summary();
            let content = Line::from(vec![
                Span::styled(format!("[{time}] "), theme::dim_style()),
                Span::styled(format!("{}", msg.from), Style::default().fg(theme::accent())),
                Span::styled(" → ", theme::dim_style()),
                Span::styled(format!("{}", msg.to), Style::default().fg(theme::success())),
                Span::raw(format!(": {summary}")),
            ]);
            ListItem::new(content)
        })
        .collect();

    let block = Block::default()
        .title(" Messages ")
        .title_style(theme::title_style())
        .borders(Borders::ALL)
        .border_style(theme::border_style());

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}
