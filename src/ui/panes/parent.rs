use ratatui::{
  layout::Rect,
  style::Style,
  widgets::{
    Block,
    Borders,
    Clear,
    List,
    ListItem,
  },
};

pub fn draw_parent_panel(
  f: &mut ratatui::Frame,
  area: Rect,
  app: &crate::App,
)
{
  f.render_widget(Clear, area);
  let mut block = Block::default().borders(Borders::ALL);
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
  f.render_widget(block.clone(), area);
  let inner = block.inner(area);
  let inner_width = inner.width;
  let fmt = app.config.ui.row.clone().unwrap_or_default();
  let list_area = Rect {
    x:      inner.x,
    y:      inner.y,
    width:  inner.width,
    height: inner.height,
  };
  let items: Vec<ListItem> = app
    .parent_entries
    .iter()
    .map(|e| {
      ListItem::new(crate::ui::row::build_row_line(app, &fmt, e, inner_width))
    })
    .collect();
  let mut list = List::new(items);
  if let Some(th) = app.config.ui.theme.as_ref()
  {
    if let Some(fg) =
      th.item_fg.as_ref().and_then(|s| crate::ui::colors::parse_color(s))
    {
      list = list.style(Style::default().fg(fg));
    }
    if let Some(bg) =
      th.item_bg.as_ref().and_then(|s| crate::ui::colors::parse_color(s))
    {
      list = list.style(Style::default().bg(bg));
    }
  }
  f.render_widget(list, list_area);
}
