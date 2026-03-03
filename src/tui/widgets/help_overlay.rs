use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

use crate::tui::theme;

const HELP_TEXT: &str = "\
Navigation:
  j / ↓        Select next agent
  k / ↑        Select previous agent
  1            Dashboard view
  2            Agent detail view
  3            Task view
  4            Log view
  5            Office view
  6            Settings
  Tab          Next view tab

Agent Control:
  p            Send prompt to selected agent
  Enter        Focus selected agent (detail view)

Commands:
  :            Enter command mode
  :t <desc>    Send task to selected agent
  :tt <desc>   Team task - all agents collaborate
  :task <desc> Create a new task (auto-assign)
  :bc <msg>    Broadcast same prompt to all
  :send <agent> <msg>  Send inter-agent msg
  :stop <agent>        Stop an agent
  :quit / :q   Quit

Scrolling:
  Ctrl-d       Scroll down
  Ctrl-u       Scroll up
  g            Scroll to top
  G            Scroll to bottom

Settings (tab 6):
  j/k          Navigate settings
  Enter        Toggle / edit value
  s            Save settings

Other:
  ?            Toggle this help
  q / Esc      Quit / Cancel";

pub fn render(frame: &mut Frame, area: Rect) {
    // Center the help popup
    let popup_width = 52.min(area.width.saturating_sub(4));
    let popup_height = 34.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(popup_width)) / 2;
    let y = (area.height.saturating_sub(popup_height)) / 2;
    let popup_area = Rect::new(x, y, popup_width, popup_height);

    frame.render_widget(Clear, popup_area);

    let block = Block::default()
        .title(" Help ")
        .title_style(theme::title_style())
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::accent()));

    let paragraph = Paragraph::new(HELP_TEXT)
        .block(block)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, popup_area);
}
