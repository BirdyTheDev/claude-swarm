use ratatui::prelude::*;

/// Compute the top-level layout: header, main area, status bar.
pub fn main_layout(area: Rect) -> (Rect, Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),  // header
            Constraint::Min(5),    // main content
            Constraint::Length(1), // status bar
        ])
        .split(area);

    (chunks[0], chunks[1], chunks[2])
}

/// Compute the 3-column dashboard layout within the main area.
pub fn dashboard_layout(area: Rect) -> (Rect, Rect, Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(22),    // agent list sidebar
            Constraint::Percentage(35), // mini panels
            Constraint::Min(30),       // focused output
        ])
        .split(area);

    (chunks[0], chunks[1], chunks[2])
}

/// Split area vertically into N equal chunks (for mini agent panels).
pub fn split_vertical(area: Rect, n: usize) -> Vec<Rect> {
    if n == 0 {
        return vec![];
    }
    let constraints: Vec<Constraint> = (0..n)
        .map(|_| Constraint::Ratio(1, n as u32))
        .collect();
    Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area)
        .to_vec()
}

/// Layout with input area at bottom.
/// `input_height` is the total height for the input widget (including borders).
pub fn with_input(area: Rect, input_height: u16) -> (Rect, Rect) {
    let h = input_height.max(3).min(area.height.saturating_sub(3));
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(h)])
        .split(area);
    (chunks[0], chunks[1])
}
