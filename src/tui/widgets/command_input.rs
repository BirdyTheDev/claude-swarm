use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::tui::theme;
use crate::types::event::UiMode;

/// Multi-line input state with history.
pub struct InputState {
    pub lines: Vec<String>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub scroll_row: usize,
    pub history: Vec<String>,
    pub history_idx: Option<usize>,
    pub max_history: usize,
}

impl InputState {
    pub fn new(max_history: usize) -> Self {
        Self {
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            scroll_row: 0,
            history: Vec::new(),
            history_idx: None,
            max_history,
        }
    }

    /// Get the full input as a single string (lines joined with \n).
    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    /// Number of content lines.
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Calculate the visible height needed (including borders).
    /// min 3, max 10.
    pub fn visible_height(&self) -> u16 {
        let content_lines = self.line_count() as u16;
        let with_border = content_lines + 2; // top + bottom border
        with_border.max(3).min(10)
    }

    /// Insert a character at the cursor position.
    pub fn insert_char(&mut self, c: char) {
        self.history_idx = None;
        if self.cursor_row < self.lines.len() {
            let line = &mut self.lines[self.cursor_row];
            let byte_idx = char_to_byte_idx(line, self.cursor_col);
            line.insert(byte_idx, c);
            self.cursor_col += 1;
        }
    }

    /// Insert a new line at cursor (Enter key).
    pub fn new_line(&mut self) {
        self.history_idx = None;
        if self.cursor_row < self.lines.len() {
            let current = &self.lines[self.cursor_row];
            let byte_idx = char_to_byte_idx(current, self.cursor_col);
            let rest = current[byte_idx..].to_string();
            self.lines[self.cursor_row] = current[..byte_idx].to_string();
            self.cursor_row += 1;
            self.lines.insert(self.cursor_row, rest);
            self.cursor_col = 0;
        }
    }

    /// Delete the character before the cursor (Backspace).
    pub fn backspace(&mut self) {
        self.history_idx = None;
        if self.cursor_col > 0 {
            let line = &mut self.lines[self.cursor_row];
            let byte_idx = char_to_byte_idx(line, self.cursor_col - 1);
            let end_byte = char_to_byte_idx(line, self.cursor_col);
            line.replace_range(byte_idx..end_byte, "");
            self.cursor_col -= 1;
        } else if self.cursor_row > 0 {
            // Merge with previous line
            let current = self.lines.remove(self.cursor_row);
            self.cursor_row -= 1;
            let prev_char_len = self.lines[self.cursor_row].chars().count();
            self.lines[self.cursor_row].push_str(&current);
            self.cursor_col = prev_char_len;
        }
    }

    /// Delete the character at the cursor (Delete key).
    pub fn delete(&mut self) {
        self.history_idx = None;
        let line_char_len = self.lines[self.cursor_row].chars().count();
        if self.cursor_col < line_char_len {
            let line = &mut self.lines[self.cursor_row];
            let byte_idx = char_to_byte_idx(line, self.cursor_col);
            let end_byte = char_to_byte_idx(line, self.cursor_col + 1);
            line.replace_range(byte_idx..end_byte, "");
        } else if self.cursor_row + 1 < self.lines.len() {
            // Merge next line into current
            let next = self.lines.remove(self.cursor_row + 1);
            self.lines[self.cursor_row].push_str(&next);
        }
    }

    /// Move cursor left.
    pub fn move_left(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].chars().count();
        }
    }

    /// Move cursor right.
    pub fn move_right(&mut self) {
        let line_len = self.lines[self.cursor_row].chars().count();
        if self.cursor_col < line_len {
            self.cursor_col += 1;
        } else if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            self.cursor_col = 0;
        }
    }

    /// Move cursor up one line.
    pub fn move_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            let line_len = self.lines[self.cursor_row].chars().count();
            self.cursor_col = self.cursor_col.min(line_len);
        }
    }

    /// Move cursor down one line.
    pub fn move_down(&mut self) {
        if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            let line_len = self.lines[self.cursor_row].chars().count();
            self.cursor_col = self.cursor_col.min(line_len);
        }
    }

    /// Move to beginning of line.
    pub fn home(&mut self) {
        self.cursor_col = 0;
    }

    /// Move to end of line.
    pub fn end(&mut self) {
        self.cursor_col = self.lines[self.cursor_row].chars().count();
    }

    /// Navigate to previous history entry.
    pub fn history_prev(&mut self) {
        if self.history.is_empty() {
            return;
        }
        let idx = match self.history_idx {
            None => {
                // Save current input before entering history
                self.history.len() - 1
            }
            Some(i) if i > 0 => i - 1,
            Some(_) => return,
        };
        self.history_idx = Some(idx);
        let entry = self.history[idx].clone();
        self.set_text(&entry);
    }

    /// Navigate to next history entry.
    pub fn history_next(&mut self) {
        let Some(idx) = self.history_idx else {
            return;
        };
        if idx + 1 < self.history.len() {
            self.history_idx = Some(idx + 1);
            let entry = self.history[idx + 1].clone();
            self.set_text(&entry);
        } else {
            // Back to empty
            self.history_idx = None;
            self.clear();
        }
    }

    /// Submit: returns the text and adds to history, then clears.
    pub fn submit(&mut self) -> String {
        let text = self.text();
        if !text.is_empty() {
            self.history.push(text.clone());
            if self.history.len() > self.max_history {
                self.history.remove(0);
            }
        }
        self.clear();
        text
    }

    /// Clear all input.
    pub fn clear(&mut self) {
        self.lines = vec![String::new()];
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.scroll_row = 0;
        self.history_idx = None;
    }

    /// Set text from string (may contain newlines).
    fn set_text(&mut self, text: &str) {
        self.lines = text.lines().map(|l| l.to_string()).collect();
        if self.lines.is_empty() {
            self.lines.push(String::new());
        }
        self.cursor_row = self.lines.len() - 1;
        self.cursor_col = self.lines[self.cursor_row].chars().count();
        self.scroll_row = 0;
    }

    /// Adjust scroll to keep cursor visible within viewport_height lines.
    pub fn adjust_scroll(&mut self, viewport_height: usize) {
        if viewport_height == 0 {
            return;
        }
        if self.cursor_row < self.scroll_row {
            self.scroll_row = self.cursor_row;
        } else if self.cursor_row >= self.scroll_row + viewport_height {
            self.scroll_row = self.cursor_row - viewport_height + 1;
        }
    }
}

/// Convert char index to byte index in a string.
fn char_to_byte_idx(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(i, _)| i)
        .unwrap_or(s.len())
}

/// Render the multi-line input widget.
pub fn render(
    frame: &mut Frame,
    area: Rect,
    mode: UiMode,
    input_state: &InputState,
) {
    let (prefix, title) = match mode {
        UiMode::Command => (":", " Command "),
        UiMode::Prompt => ("> ", " Prompt "),
        UiMode::TaskInput => ("task> ", " New Task "),
        _ => return,
    };

    let block = Block::default()
        .title(title)
        .title_style(theme::title_style())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::accent()));

    let inner = block.inner(area);
    let viewport_height = inner.height as usize;
    let inner_width = inner.width as usize;
    let prefix_len = prefix.len();

    // Build lines for rendering
    let mut rendered_lines: Vec<Line> = Vec::new();
    let scroll = input_state.scroll_row;
    let visible_end = (scroll + viewport_height).min(input_state.lines.len());

    for i in scroll..visible_end {
        let line = &input_state.lines[i];
        let line_num = format!("{:>2} ", i + 1);

        let mut spans = Vec::new();
        // Line number (dim)
        spans.push(Span::styled(line_num, theme::dim_style()));

        // Prefix on first visible line only
        if i == 0 {
            spans.push(Span::styled(prefix, theme::mode_style()));
        } else {
            // Pad to align with first line's prefix
            let pad = " ".repeat(prefix_len);
            spans.push(Span::raw(pad));
        }

        // Line content - truncate to viewport
        let available = inner_width.saturating_sub(3 + prefix_len); // 3 for line number "XX "
        let line_chars: Vec<char> = line.chars().collect();
        let display: String = if line_chars.len() > available {
            let trunc: String = line_chars[..available.saturating_sub(1)].iter().collect();
            format!("{}…", trunc)
        } else {
            line.clone()
        };
        spans.push(Span::raw(display));

        rendered_lines.push(Line::from(spans));
    }

    // Scroll indicators
    let has_scroll_up = scroll > 0;
    let has_scroll_down = visible_end < input_state.lines.len();

    if has_scroll_up || has_scroll_down {
        // We already have the lines, just need scroll indicators in the block title
        // We'll add them as suffix
    }

    let title_text = if has_scroll_up && has_scroll_down {
        format!("{} ↑↓", title.trim())
    } else if has_scroll_up {
        format!("{} ↑", title.trim())
    } else if has_scroll_down {
        format!("{} ↓", title.trim())
    } else {
        title.to_string()
    };

    let block = Block::default()
        .title(format!(" {} ", title_text.trim()))
        .title_style(theme::title_style())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::accent()));

    let paragraph = Paragraph::new(rendered_lines).block(block);
    frame.render_widget(paragraph, area);

    // Set cursor position
    let cursor_screen_row = input_state.cursor_row.saturating_sub(scroll);
    if cursor_screen_row < viewport_height {
        let line_num_width = 3u16; // "XX "
        let prefix_width = prefix_len as u16;
        let x = area.x + 1 + line_num_width + prefix_width + input_state.cursor_col as u16;
        let y = area.y + 1 + cursor_screen_row as u16;
        if x < area.right() && y < area.bottom() {
            frame.set_cursor_position((x, y));
        }
    }
}

/// Legacy render for backwards compatibility (single-line mode).
pub fn render_single_line(
    frame: &mut Frame,
    area: Rect,
    mode: UiMode,
    input: &str,
    cursor_pos: usize,
) {
    let (prefix, title) = match mode {
        UiMode::Command => (":", " Command "),
        UiMode::Prompt => ("> ", " Prompt "),
        UiMode::TaskInput => ("task> ", " New Task "),
        _ => return,
    };

    let block = Block::default()
        .title(title)
        .title_style(theme::title_style())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::accent()));

    let inner_width = area.width.saturating_sub(2) as usize;
    let prefix_len = prefix.len();
    let text_viewport = inner_width.saturating_sub(prefix_len);

    let scroll_offset = if cursor_pos < text_viewport {
        0
    } else {
        cursor_pos - text_viewport + 1
    };

    let has_left_overflow = scroll_offset > 0;
    let visible_end = scroll_offset + text_viewport;
    let has_right_overflow = visible_end < input.len();

    let effective_viewport = if has_left_overflow && has_right_overflow {
        text_viewport.saturating_sub(2)
    } else if has_left_overflow || has_right_overflow {
        text_viewport.saturating_sub(1)
    } else {
        text_viewport
    };

    let scroll_offset = if cursor_pos < effective_viewport {
        0
    } else {
        cursor_pos - effective_viewport + 1
    };

    let has_left_overflow = scroll_offset > 0;
    let visible_end = scroll_offset + effective_viewport;
    let has_right_overflow = visible_end < input.len();

    let visible_text: String = input
        .chars()
        .skip(scroll_offset)
        .take(effective_viewport)
        .collect();

    let mut spans = Vec::new();
    spans.push(Span::styled(prefix, theme::mode_style()));

    if has_left_overflow {
        spans.push(Span::styled("<", theme::dim_style()));
    }

    spans.push(Span::raw(visible_text));

    if has_right_overflow {
        spans.push(Span::styled(">", theme::dim_style()));
    }

    let content = Line::from(spans);
    let paragraph = Paragraph::new(content).block(block);
    frame.render_widget(paragraph, area);

    let cursor_in_viewport = cursor_pos - scroll_offset;
    let indicator_offset = if has_left_overflow { 1u16 } else { 0u16 };
    let x = area.x + 1 + prefix_len as u16 + indicator_offset + cursor_in_viewport as u16;
    let y = area.y + 1;
    frame.set_cursor_position((x, y));
}
