use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Cell, Row, Table};

use crate::tui::theme;
use crate::types::task::Task;

pub fn render(frame: &mut Frame, area: Rect, tasks: &[&Task], selected: usize) {
    let header = Row::new(vec!["ID", "Status", "Priority", "Agent", "Description"])
        .style(theme::bold_style())
        .bottom_margin(1);

    let rows: Vec<Row> = tasks
        .iter()
        .enumerate()
        .map(|(i, task)| {
            let status_style = theme::task_status_style(&task.status);
            let row = Row::new(vec![
                Cell::from(task.id.to_string()),
                Cell::from(format!("{} {}", task.status.icon(), task.status))
                    .style(status_style),
                Cell::from(task.priority.to_string()),
                Cell::from(
                    task.assigned_to
                        .as_ref()
                        .map(|a| a.0.as_str())
                        .unwrap_or("-"),
                ),
                Cell::from(task.description.as_str()),
            ]);
            if i == selected {
                row.style(theme::selected_style())
            } else {
                row
            }
        })
        .collect();

    let block = Block::default()
        .title(" Tasks ")
        .title_style(theme::title_style())
        .borders(Borders::ALL)
        .border_style(theme::border_style());

    let widths = [
        Constraint::Length(8),
        Constraint::Length(14),
        Constraint::Length(10),
        Constraint::Length(12),
        Constraint::Min(20),
    ];

    let table = Table::new(rows, widths)
        .header(header)
        .block(block);

    frame.render_widget(table, area);
}
