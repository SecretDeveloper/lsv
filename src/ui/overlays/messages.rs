use ratatui::{
  layout::{
    Constraint,
    Direction,
    Layout,
    Rect,
  },
  style::{
    Color,
    Modifier,
    Style,
  },
  text::Span,
  widgets::{
    Block,
    Borders,
    Clear,
    Paragraph,
    Wrap,
  },
};

pub fn draw_messages_panel(
  f: &mut ratatui::Frame,
  area: Rect,
  app: &crate::App,
)
{
  let min_h = ((area.height as u32 * 20) / 100).max(3) as u16;
  let max_h = ((area.height as u32 * 50) / 100).max(min_h as u32) as u16;
  let needed = (app.recent_messages.len() as u16).saturating_add(2).max(3);
  let panel_h = needed.min(max_h).max(min_h).min(area.height);

  let mut block = Block::default().borders(Borders::ALL).title(Span::styled(
    "Messages",
    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
  ));
  if let Some(th) = app.config.ui.theme.as_ref()
  {
    if let Some(bg) =
      th.pane_bg.as_ref().and_then(|s| crate::ui::colors::parse_color(s))
    {
      block = block.style(Style::default().bg(bg));
    }
    if let Some(bfg) =
      th.border_fg.as_ref().and_then(|s| crate::ui::colors::parse_color(s))
    {
      block = block.border_style(Style::default().fg(bfg));
    }
  }
  let layout = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Min(0), Constraint::Length(panel_h)])
    .split(area);
  let panel = layout[1];
  f.render_widget(Clear, panel);

  let avail_rows = panel_h.saturating_sub(2) as usize;
  let start = app.recent_messages.len().saturating_sub(avail_rows);
  let slice = &app.recent_messages[start..];
  let mut lines: Vec<ratatui::text::Line> = Vec::new();
  for m in slice
  {
    lines.push(ratatui::text::Line::from(Span::styled(
      m.clone(),
      Style::default().fg(Color::Gray),
    )));
  }
  let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
  f.render_widget(para, panel);
}
