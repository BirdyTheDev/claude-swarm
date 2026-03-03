use ratatui::prelude::*;

use crate::tui::widgets::task_table;
use crate::types::task::Task;

pub fn render(
    frame: &mut Frame,
    area: Rect,
    tasks: &[&Task],
    selected: usize,
) {
    task_table::render(frame, area, tasks, selected);
}
