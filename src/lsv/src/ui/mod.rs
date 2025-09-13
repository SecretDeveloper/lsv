pub mod ansi;
pub mod panes;

use ratatui::layout::{Direction, Layout, Constraint, Alignment, Rect};
use ratatui::widgets::Paragraph;
use unicode_width::UnicodeWidthStr;

pub fn draw(
  f: &mut ratatui::Frame,
  app: &mut crate::App,
) {
  // Split top header (1 row) and content
  let full = f.area();
  let vchunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Length(1), Constraint::Min(1)])
    .split(full);

  draw_header(f, vchunks[0], app);

  let constraints = panes::pane_constraints(app);
  let chunks = Layout::default()
    .direction(Direction::Horizontal)
    .constraints(constraints)
    .split(vchunks[1]);

  panes::draw_parent_panel(f, chunks[0], app);
  panes::draw_current_panel(f, chunks[1], app);
  crate::preview::draw_preview_panel(f, chunks[2], app);

  if let Some(msg) = &app.status_error {
    panes::draw_error_bar(f, f.area(), msg);
  }

  // which-key overlay (draw last so it appears on top)
  if app.show_whichkey {
    panes::draw_whichkey_panel(f, f.area(), app);
  }
}

fn draw_header(f: &mut ratatui::Frame, area: Rect, app: &crate::App) {
  // Left: {user}@{host}:{current_dir}
  let user = whoami::username();
  let host = whoami::hostname();
  let left_full = format!("{}@{}:{}", user, host, app.cwd.display());

  // Right: {sort_type}:{display_type} plus current info field label
  let sort = crate::enums::sort_key_to_str(app.sort_key);
  let disp = crate::enums::display_mode_to_str(app.display_mode);
  let info_label = crate::enums::info_mode_to_str(app.info_mode);
  let right_full = if let Some(lbl) = info_label { format!("{}:{}:{}", sort, disp, lbl) } else { format!("{}:{}", sort, disp) };

  let total = area.width as usize;
  let right_w = UnicodeWidthStr::width(right_full.as_str());
  let left_max = total.saturating_sub(right_w + 1);
  let left = truncate_to_width(&left_full, left_max);

  // Draw left and right in the same row using two aligned paragraphs
  let style = ratatui::style::Style::default().fg(ratatui::style::Color::Gray);
  let left_p = Paragraph::new(left).alignment(Alignment::Left).style(style);
  let right_p = Paragraph::new(right_full).alignment(Alignment::Right).style(style);
  f.render_widget(left_p, area);
  f.render_widget(right_p, area);
}

fn truncate_to_width(s: &str, max_w: usize) -> String {
  if max_w == 0 { return String::new(); }
  let mut out = String::new();
  let mut w = 0usize;
  for ch in s.chars() {
    let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
    if w + cw > max_w { break; }
    out.push(ch);
    w += cw;
  }
  out
}
