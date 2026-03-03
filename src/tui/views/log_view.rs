use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::config::settings::LogVerbosity;
use crate::tui::theme;
use crate::types::log_entry::{LogCategory, LogEntry};

/// Filter logs based on verbosity level.
fn should_show(entry: &LogEntry, verbosity: LogVerbosity) -> bool {
    match verbosity {
        LogVerbosity::Minimal => matches!(entry.category, LogCategory::Error | LogCategory::Team),
        LogVerbosity::Normal => !matches!(entry.category, LogCategory::Communication),
        LogVerbosity::Detailed => true,
    }
}

fn category_color(cat: LogCategory) -> Color {
    match cat {
        LogCategory::Agent => theme::working(),
        LogCategory::Task => theme::accent(),
        LogCategory::Team => theme::success(),
        LogCategory::Communication => theme::warning(),
        LogCategory::System => theme::accent_dim(),
        LogCategory::Error => theme::error_color(),
    }
}

pub fn render(frame: &mut Frame, area: Rect, logs: &[LogEntry], verbosity: LogVerbosity) {
    let block = Block::default()
        .title(" System Logs ")
        .title_style(theme::title_style())
        .borders(Borders::ALL)
        .border_style(theme::border_style());

    let inner_height = area.height.saturating_sub(2) as usize;

    let filtered: Vec<&LogEntry> = logs
        .iter()
        .filter(|e| should_show(e, verbosity))
        .collect();

    let start = filtered.len().saturating_sub(inner_height);
    let visible: Vec<Line> = filtered[start..]
        .iter()
        .map(|entry| {
            let color = category_color(entry.category);
            Line::from(vec![
                Span::styled(
                    format!("[{}] ", entry.timestamp),
                    theme::dim_style(),
                ),
                Span::styled(
                    format!("{} ", entry.category.icon()),
                    Style::default().fg(color),
                ),
                Span::styled(
                    &entry.message,
                    Style::default().fg(theme::fg()),
                ),
            ])
        })
        .collect();

    let paragraph = Paragraph::new(visible)
        .block(block)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}
