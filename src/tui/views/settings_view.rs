use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::config::settings::Settings;
use crate::tui::theme;

struct SettingRow {
    label: &'static str,
    value: String,
    editable_text: bool,
}

pub fn render(
    frame: &mut Frame,
    area: Rect,
    settings: &Settings,
    selected: usize,
    editing: bool,
    edit_buffer: &str,
) {
    let block = Block::default()
        .title(" Settings ")
        .title_style(theme::title_style())
        .borders(Borders::ALL)
        .border_style(theme::border_style());

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = vec![
        SettingRow {
            label: "Language",
            value: settings.language.label().to_string(),
            editable_text: false,
        },
        SettingRow {
            label: "Theme",
            value: settings.theme.label().to_string(),
            editable_text: false,
        },
        SettingRow {
            label: "Log Verbosity",
            value: settings.log_verbosity.label().to_string(),
            editable_text: false,
        },
        SettingRow {
            label: "Terminal App",
            value: settings.terminal_app.clone(),
            editable_text: true,
        },
        SettingRow {
            label: "History Size",
            value: settings.input_history_size.to_string(),
            editable_text: true,
        },
        SettingRow {
            label: "Meeting Timeout",
            value: format!("{}s", settings.meeting_timeout_secs),
            editable_text: true,
        },
    ];

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(""));

    for (i, row) in rows.iter().enumerate() {
        let is_selected = i == selected;
        let marker = if is_selected { "▸ " } else { "  " };

        let value_display = if editing && is_selected {
            format!("[{}▏]", edit_buffer)
        } else if row.editable_text {
            format!("[{}]", row.value)
        } else {
            format!("[{}]", row.value)
        };

        let label_style = if is_selected {
            Style::default().fg(theme::accent()).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme::fg())
        };

        let value_style = if editing && is_selected {
            Style::default().fg(theme::warning())
        } else if is_selected {
            Style::default().fg(theme::accent())
        } else {
            theme::dim_style()
        };

        lines.push(Line::from(vec![
            Span::styled(marker, label_style),
            Span::styled(format!("{:<18}", row.label), label_style),
            Span::styled(value_display, value_style),
        ]));
        lines.push(Line::from(""));
    }

    // Save hint
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  ", Style::default()),
        Span::styled(
            "Enter: toggle/edit  s: save  j/k: navigate",
            theme::dim_style(),
        ),
    ]));

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, inner);
}
