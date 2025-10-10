use ratatui::{
  layout::Rect,
  style::{
    Color,
    Modifier,
    Style,
  },
  text::{
    Line,
    Span,
  },
  widgets::{
    Block,
    Borders,
    Clear,
    Paragraph,
    Wrap,
  },
};

pub fn draw_prompt_panel(
  f: &mut ratatui::Frame,
  area: Rect,
  app: &crate::App,
)
{
  let state = match app.overlay
  {
    crate::app::Overlay::Prompt(ref s) => s.as_ref(),
    _ => return,
  };

  let (width, height) = if let Some(m) = app.config.ui.modals.as_ref()
  {
    let w = (area.width.saturating_mul(m.prompt.width_pct.clamp(10, 100))
      / 100)
      .max(20);
    let h = (area.height.saturating_mul(m.prompt.height_pct.clamp(10, 100))
      / 100)
      .max(5);
    (w, h)
  }
  else
  {
    (50, 5)
  };

  let popup = Rect::new(
    area.x + area.width.saturating_sub(width) / 2,
    area.y + area.height.saturating_sub(height) / 2,
    width,
    height,
  );
  f.render_widget(Clear, popup);

  let mut pane_bg = None;
  let mut border_fg = None;
  let mut title_fg = Color::Yellow;
  let mut title_bg = None;
  if let Some(th) = app.config.ui.theme.as_ref()
  {
    pane_bg =
      th.pane_bg.as_ref().and_then(|s| crate::ui::colors::parse_color(s));
    border_fg =
      th.border_fg.as_ref().and_then(|s| crate::ui::colors::parse_color(s));
    if let Some(tf) =
      th.title_fg.as_ref().and_then(|s| crate::ui::colors::parse_color(s))
    {
      title_fg = tf;
    }
    title_bg =
      th.title_bg.as_ref().and_then(|s| crate::ui::colors::parse_color(s));
  }

  let mut block = Block::default().borders(Borders::ALL);
  if let Some(bg) = pane_bg
  {
    block = block.style(Style::default().bg(bg));
  }
  if let Some(bfg) = border_fg
  {
    block = block.border_style(Style::default().fg(bfg));
  }
  let mut title_style =
    Style::default().fg(title_fg).add_modifier(Modifier::BOLD);
  if let Some(tb) = title_bg
  {
    title_style = title_style.bg(tb);
  }
  block = block.title(Span::styled(state.title.clone(), title_style));
  let inner = block.inner(popup);
  f.render_widget(block, popup);
  // Display the current input as the editable line
  let lines: Vec<Line> =
    vec![Line::from(""), Line::from(Span::raw(state.input.clone()))];
  let para = Paragraph::new(lines).wrap(Wrap { trim: true });
  f.render_widget(para, inner);
}
