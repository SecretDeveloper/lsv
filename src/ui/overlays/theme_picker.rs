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
    List,
    ListItem,
    ListState,
    Paragraph,
  },
};
use unicode_width::UnicodeWidthStr;

pub fn draw_theme_picker_panel(
  f: &mut ratatui::Frame,
  area: Rect,
  app: &crate::App,
)
{
  let state = match app.overlay
  {
    crate::app::Overlay::ThemePicker(ref s) => s.as_ref(),
    _ => return,
  };
  if state.entries.is_empty()
  {
    return;
  }

  let max_name_width = state
    .entries
    .iter()
    .map(|e| UnicodeWidthStr::width(e.name.as_str()))
    .max()
    .unwrap_or(0);
  let (popup_width, popup_height) = if let Some(m) =
    app.config.ui.modals.as_ref()
  {
    let w = (area.width.saturating_mul(m.theme.width_pct.clamp(10, 100)) / 100)
      .max(20);
    let h = (area.height.saturating_mul(m.theme.height_pct.clamp(10, 100))
      / 100)
      .max(5);
    (w, h)
  }
  else
  {
    let base_width = (max_name_width as u16).saturating_add(6);
    let desired_width = base_width.max(30);
    let w = desired_width
      .min(area.width.saturating_sub(4).max(20))
      .min(area.width)
      .max(10);
    let entries_len = state.entries.len() as u16;
    let desired_height = entries_len.saturating_add(4);
    let h = desired_height
      .min(area.height.saturating_sub(4).max(6))
      .min(area.height)
      .max(5);
    (w, h)
  };

  let popup = Rect::new(
    area.x + area.width.saturating_sub(popup_width) / 2,
    area.y + area.height.saturating_sub(popup_height) / 2,
    popup_width,
    popup_height,
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
  block = block.title(Span::styled("Select UI Theme", title_style));

  let inner = block.inner(popup);
  f.render_widget(block, popup);
  if inner.width == 0 || inner.height == 0
  {
    return;
  }

  let base_style = app
    .config
    .ui
    .theme
    .as_ref()
    .and_then(|th| th.item_fg.as_ref())
    .and_then(|s| crate::ui::colors::parse_color(s))
    .map(|fg| Style::default().fg(fg))
    .unwrap_or_else(|| Style::default().fg(Color::Gray));

  let mut highlight = Style::default().add_modifier(Modifier::BOLD);
  if let Some(th) = app.config.ui.theme.as_ref()
  {
    if let Some(fg) = th
      .selected_item_fg
      .as_ref()
      .and_then(|s| crate::ui::colors::parse_color(s))
    {
      highlight = highlight.fg(fg);
    }
    if let Some(bg) = th
      .selected_item_bg
      .as_ref()
      .and_then(|s| crate::ui::colors::parse_color(s))
    {
      highlight = highlight.bg(bg);
    }
    else if let Some(bg) =
      th.pane_bg.as_ref().and_then(|s| crate::ui::colors::parse_color(s))
    {
      highlight = highlight.bg(bg);
    }
  }

  let items: Vec<ListItem> = state
    .entries
    .iter()
    .map(|entry| ListItem::new(ratatui::text::Line::from(entry.name.clone())))
    .collect();

  let constraints: Vec<Constraint> = if inner.height > 3
  {
    vec![Constraint::Min(1), Constraint::Length(1)]
  }
  else
  {
    vec![Constraint::Min(1)]
  };
  let chunks = Layout::default()
    .direction(Direction::Vertical)
    .constraints(constraints)
    .split(inner);
  let list_area = chunks[0];

  let mut list_state = ListState::default();
  list_state.select(Some(state.selected));
  let list = List::new(items).style(base_style).highlight_style(highlight);
  f.render_stateful_widget(list, list_area, &mut list_state);

  if chunks.len() > 1
  {
    let info_area = chunks[1];
    let mut info_style = Style::default().fg(Color::DarkGray);
    if let Some(th) = app.config.ui.theme.as_ref()
      && let Some(fg) =
        th.info_fg.as_ref().and_then(|s| crate::ui::colors::parse_color(s))
    {
      info_style = info_style.fg(fg);
    }
    let hint = Paragraph::new("↑/↓ preview  Enter apply  Esc cancel")
      .style(info_style)
      .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(hint, info_area);
  }
}
