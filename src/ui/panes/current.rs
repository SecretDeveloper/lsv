use ratatui::{
  layout::Rect,
  style::{
    Color,
    Modifier,
    Style,
  },
  widgets::{
    Block,
    Borders,
    Clear,
    List,
    ListItem,
  },
};

pub fn draw_current_panel(
  f: &mut ratatui::Frame,
  area: Rect,
  app: &mut crate::App,
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
  let fmt = app.config.ui.row.clone().unwrap_or_default();
  let items: Vec<ListItem> = app
    .current_entries
    .iter()
    .map(|e| {
      ListItem::new(crate::ui::row::build_row_line(app, &fmt, e, inner.width))
    })
    .collect();

  let list_area = Rect {
    x:      inner.x,
    y:      inner.y,
    width:  inner.width,
    height: inner.height,
  };
  let mut list = List::new(items).highlight_symbol("");
  if let Some(th) = app.config.ui.theme.as_ref()
  {
    let mut hl = Style::default();
    if let Some(fg) = th
      .selected_item_fg
      .as_ref()
      .and_then(|s| crate::ui::colors::parse_color(s))
    {
      hl = hl.fg(fg);
    }
    if let Some(bg) = th
      .selected_item_bg
      .as_ref()
      .and_then(|s| crate::ui::colors::parse_color(s))
    {
      hl = hl.bg(bg);
    }
    list = list.highlight_style(hl.add_modifier(Modifier::BOLD));
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
  else
  {
    list = list.highlight_style(
      Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    );
  }

  f.render_stateful_widget(list, list_area, &mut app.list_state);
}
