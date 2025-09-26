  use crate::ui::ansi::ansi_spans;
use ratatui::{
  layout::{
    Alignment,
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
  text::{
    Line,
    Span,
  },
  widgets::{
    Block,
    Borders,
    Clear,
    List,
    ListItem,
    ListState,
    Paragraph,
    Wrap,
  },
};
use unicode_width::{
  UnicodeWidthStr,
};

pub fn pane_constraints(app: &crate::App) -> [Constraint; 3]
{
  let (mut p, mut c, mut r) = (30u16, 40u16, 30u16);
  if let Some(panes) = app.config.ui.panes.as_ref()
  {
    p = panes.parent;
    c = panes.current;
    r = panes.preview;
  }
  let total = p.saturating_add(c).saturating_add(r);
  if total == 0
  {
    return [
      Constraint::Percentage(30),
      Constraint::Percentage(40),
      Constraint::Percentage(30),
    ];
  }
  let p_norm = (p as u32 * 100 / total as u32) as u16;
  let c_norm = (c as u32 * 100 / total as u32) as u16;
  let r_norm = 100u16.saturating_sub(p_norm).saturating_sub(c_norm);
  [
    Constraint::Percentage(p_norm),
    Constraint::Percentage(c_norm),
    Constraint::Percentage(r_norm),
  ]
}

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
  // Draw block and compute inner content area
  f.render_widget(block.clone(), area);
  let inner = block.inner(area);
  let inner_width = inner.width;
  let fmt = app.config.ui.row.clone().unwrap_or_default();
  // List area (full inner area; no per-pane header)
  let list_area = Rect {
    x:      inner.x,
    y:      inner.y,
    width:  inner.width,
    height: inner.height,
  };
  let items: Vec<ListItem> = app
    .parent_entries
    .iter()
    .map(|e| build_row_item(app, &fmt, e, inner_width))
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
  // Draw block and compute inner content area
  f.render_widget(block.clone(), area);
  let inner = block.inner(area);
  let inner_width = inner.width;
  let fmt = app.config.ui.row.clone().unwrap_or_default();
  // Remove the left arrow indicator; no extra gutter space
  let highlight_symbol = "";
  let avail_width = inner_width;
  let items: Vec<ListItem> = app
    .current_entries
    .iter()
    .map(|e| build_row_item(app, &fmt, e, avail_width))
    .collect();

  // List area (full inner area; no per-pane header)
  let list_area = Rect {
    x:      inner.x,
    y:      inner.y,
    width:  inner.width,
    height: inner.height,
  };
  let mut list = List::new(items).highlight_symbol(highlight_symbol);
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

pub fn draw_whichkey_panel(
  f: &mut ratatui::Frame,
  area: Rect,
  app: &crate::App,
)
{
  // Build lookup: last mapping wins for duplicate sequences
  use std::collections::HashMap;
  let mut map: HashMap<&str, (&str, &str)> = HashMap::new();
  for km in &app.keys.maps
  {
    let label = km.description.as_deref().unwrap_or(km.action.as_str());
    map.insert(km.sequence.as_str(), (km.sequence.as_str(), label));
  }

  let prefix = match app.overlay
  {
    crate::app::Overlay::WhichKey { ref prefix } => prefix.as_str(),
    _ => "",
  };
  // Bucket by next-prefix (prefix + next char)
  let mut buckets: HashMap<String, Vec<(&str, &str)>> = HashMap::new();
  for (seq, (_, label)) in map.into_iter()
  {
    if seq.starts_with(prefix) && seq.len() > prefix.len()
    {
      let mut np = String::with_capacity(prefix.len() + 1);
      np.push_str(prefix);
      if let Some(ch) = seq.chars().nth(prefix.chars().count())
      {
        np.push(ch);
      }
      else
      {
        continue;
      }
      buckets.entry(np).or_default().push((seq, label));
    }
    else if seq == prefix
    {
      // exact match binding at current prefix (unlikely); treat as its own
      // bucket
      buckets.entry(seq.to_string()).or_default().push((seq, label));
    }
  }

  // Flatten into display entries, sorted alphabetically by key/prefix
  #[derive(Clone)]
  struct Entry
  {
    left:     String,
    right:    String,
    is_group: bool,
  }
  let mut entries: Vec<Entry> = Vec::new();
  let mut keys: Vec<String> = buckets.keys().cloned().collect();
  keys.sort();
  for k in keys
  {
    let list = buckets.get(&k).unwrap();
    // Determine if single binding exactly equals this next-prefix
    let mut exact_only = false;
    if list.len() == 1
    {
      let (seq, _label) = list[0];
      if seq == k
      {
        exact_only = true;
      }
    }
    if exact_only
    {
      let (_seq, label) = list[0];
      entries.push(Entry {
        left:     k.clone(),
        right:    label.to_string(),
        is_group: false,
      });
    }
    else
    {
      let n = list.len();
      let label = if n == 1
      {
        "(1 binding)".to_string()
      }
      else
      {
        format!("({} bindings)", n)
      };
      entries.push(Entry {
        left:     k.clone(),
        right:    label,
        is_group: true,
      });
    }
  }

  // If no matches, nothing to draw
  if entries.is_empty()
  {
    return;
  }

  // Layout: multiple columns as needed
  let title_str = if prefix.is_empty()
  {
    "Keys".to_string()
  }
  else
  {
    format!("Keys: prefix '{}'", prefix)
  };
  let mut block = Block::default().borders(Borders::ALL).title(Span::styled(
    title_str,
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

  // Compute desired height: 20% of screen by default, expand if columns
  // overflow width
  let inner_width = area.width.saturating_sub(2) as usize; // account for borders
  let mut rows = ((area.height as u32 * 20) / 100) as u16;
  if rows < 3
  {
    rows = 3;
  }
  if rows + 2 > area.height
  {
    rows = area.height.saturating_sub(2);
  }
  if rows == 0
  {
    rows = 1;
  }

  // Function to compute column widths for a given row count
  let compute_widths = |row_count: usize| -> (Vec<usize>, usize, usize) {
    let rows_usize = row_count.max(1);
    let cols = entries.len().div_ceil(rows_usize).max(1);
    let mut col_widths = vec![0usize; cols];
    for (c, width) in col_widths.iter_mut().enumerate()
    {
      let mut w = 0usize;
      for r in 0..rows_usize
      {
        let idx = c * rows_usize + r;
        if idx >= entries.len()
        {
          break;
        }
        let e = &entries[idx];
        let cell = format!("{}  {}", e.left, e.right);
        let cw = UnicodeWidthStr::width(cell.as_str());
        if cw > w
        {
          w = cw;
        }
      }
      *width = w + 2; // inter-column gap
    }
    let total: usize = col_widths.iter().sum();
    (col_widths, total, cols)
  };

  let mut rows_usize = rows as usize;
  let (mut col_widths, mut total_width, _) = compute_widths(rows_usize);
  while total_width > inner_width && (rows_usize as u16) + 2 < area.height
  {
    // Increase rows to reduce columns, then recompute
    rows_usize += 1;
    let (new_widths, new_total, _) = compute_widths(rows_usize);
    col_widths = new_widths;
    total_width = new_total;
  }

  // Build lines row-wise using final rows_usize
  let mut lines: Vec<Line> = Vec::new();
  for r in 0..rows_usize
  {
    let mut spans: Vec<Span> = Vec::new();
    let mut consumed_any = false;
    for (c, col_width) in col_widths.iter().enumerate()
    {
      let idx = c * rows_usize + r;
      if idx >= entries.len()
      {
        continue;
      }
      consumed_any = true;
      let e = &entries[idx];
      let left_style =
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
      let right_style = if e.is_group
      {
        Style::default().fg(Color::DarkGray)
      }
      else
      {
        Style::default().fg(Color::Gray)
      };
      let cell = format!("{}  {}", e.left, e.right);
      let cw = UnicodeWidthStr::width(cell.as_str());
      spans.push(Span::styled(e.left.clone(), left_style));
      spans.push(Span::raw("  "));
      spans.push(Span::styled(e.right.clone(), right_style));
      // pad to column width
      let pad = (*col_width).saturating_sub(cw);
      if pad > 0
      {
        let max_pad = 4096usize;
        spans.push(Span::raw(" ".repeat(std::cmp::min(pad, max_pad))));
      }
    }
    if consumed_any
    {
      lines.push(Line::from(spans));
    }
  }

  // Render at bottom as before
  let panel_height = (rows_usize as u16).saturating_add(2).min(area.height);
  let layout = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Min(0), Constraint::Length(panel_height)])
    .split(area);
  let panel = layout[1];
  f.render_widget(Clear, panel);
  let para = Paragraph::new(lines).block(block);
  f.render_widget(para, panel);
}

pub fn draw_messages_panel(
  f: &mut ratatui::Frame,
  area: Rect,
  app: &crate::App,
)
{
  // Bottom area: start with 20% height, expand up to fit all messages but cap
  // at 50%
  let min_h = ((area.height as u32 * 20) / 100).max(3) as u16;
  let max_h = ((area.height as u32 * 50) / 100).max(min_h as u32) as u16;
  // Determine needed height (messages + borders)
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

  // Render messages newest last (bottom) for natural reading; we will take the
  // last panel_h-2 items
  let avail_rows = panel_h.saturating_sub(2) as usize;
  let start = app.recent_messages.len().saturating_sub(avail_rows);
  let slice = &app.recent_messages[start..];
  let mut lines: Vec<Line> = Vec::new();
  for m in slice
  {
    lines.push(Line::from(Span::styled(
      m.clone(),
      Style::default().fg(Color::Gray),
    )));
  }
  let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
  f.render_widget(para, panel);
}

pub fn draw_output_panel(
  f: &mut ratatui::Frame,
  area: Rect,
  app: &crate::App,
)
{
  let (title, lines): (String, Vec<String>) = match app.overlay.clone()
  {
    crate::app::Overlay::Output { title, lines } => (title, lines),
    _ => (String::new(), Vec::new()),
  };
  let min_h = ((area.height as u32 * 20) / 100).max(3) as u16;
  let max_h = ((area.height as u32 * 60) / 100).max(min_h as u32) as u16;
  let needed = (lines.len() as u16).saturating_add(2).max(3);
  let panel_h = needed.min(max_h).max(min_h).min(area.height);

  let mut block = Block::default().borders(Borders::ALL).title(Span::styled(
    title,
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
  let start = lines.len().saturating_sub(avail_rows);
  let slice = &lines[start..];
  let mut lines: Vec<Line> = Vec::new();
  for m in slice
  {
    lines.push(Line::from(ansi_spans(m)));
  }
  let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
  f.render_widget(para, panel);
}

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

  // Popup centered (configurable)
  let (width, height) = if let Some(m) = app.config.ui.modals.as_ref()
  {
    let w = (area.width.saturating_mul(m.prompt.width_pct.clamp(10, 100)) / 100)
      .max(30);
    let h = (area.height.saturating_mul(m.prompt.height_pct.clamp(10, 100)) / 100)
      .max(5);
    (w, h)
  }
  else
  {
    (area.width.saturating_sub(area.width / 3).max(30), 5u16)
  };
  let popup = Rect::new(
    area.x + area.width.saturating_sub(width) / 2,
    area.y + area.height.saturating_sub(height) / 2,
    width,
    height,
  );
  f.render_widget(Clear, popup);

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
  let title_style = Style::default()
    .fg(Color::Yellow)
    .add_modifier(Modifier::BOLD);
  block = block.title(Span::styled(state.title.clone(), title_style));

  let inner = block.inner(popup);
  f.render_widget(block, popup);

  let lines: Vec<Line> = vec![
    Line::from(""),
    Line::from(Span::raw(state.input.clone())),
  ];
  let para = Paragraph::new(lines).wrap(Wrap { trim: false });
  f.render_widget(para, inner);

  // Place terminal cursor at the logical input cursor position
  let pre = if state.cursor <= state.input.len()
  {
    &state.input[..state.cursor]
  }
  else
  {
    state.input.as_str()
  };
  let mut xoff = UnicodeWidthStr::width(pre) as u16;
  if xoff >= inner.width { xoff = inner.width.saturating_sub(1); }
  let cur_x = inner.x.saturating_add(xoff);
  let cur_y = inner.y.saturating_add(1); // second line (after blank)
  f.set_cursor_position((cur_x, cur_y));
}

pub fn draw_confirm_panel(
  f: &mut ratatui::Frame,
  area: Rect,
  app: &crate::App,
)
{
  let state = match app.overlay
  {
    crate::app::Overlay::Confirm(ref s) => s.as_ref(),
    _ => return,
  };
  let (width, height) = if let Some(m) = app.config.ui.modals.as_ref()
  {
    let w = (area.width.saturating_mul(m.confirm.width_pct.clamp(10, 100)) / 100)
      .max(30);
    let h = (area.height.saturating_mul(m.confirm.height_pct.clamp(10, 100)) / 100)
      .max(5);
    (w, h)
  }
  else
  {
    (area.width.saturating_sub(area.width / 3).max(30), 5u16)
  };
  let popup = Rect::new(
    area.x + area.width.saturating_sub(width) / 2,
    area.y + area.height.saturating_sub(height) / 2,
    width,
    height,
  );
  f.render_widget(Clear, popup);
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
  let title_style =
    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD);
  block = block.title(Span::styled(state.title.clone(), title_style));
  let inner = block.inner(popup);
  f.render_widget(block, popup);
  let lines: Vec<Line> = vec![
    Line::from(""),
    Line::from(Span::raw(state.question.clone())),
  ];
  let para = Paragraph::new(lines).wrap(Wrap { trim: true });
  f.render_widget(para, inner);
}

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
  let (popup_width, popup_height) = if let Some(m) = app.config.ui.modals.as_ref()
  {
    let w = (area.width.saturating_mul(m.theme.width_pct.clamp(10, 100)) / 100)
      .max(20);
    let h = (area.height.saturating_mul(m.theme.height_pct.clamp(10, 100)) / 100)
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
    .map(|entry| ListItem::new(Line::from(entry.name.clone())))
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
      .alignment(Alignment::Center);
    f.render_widget(hint, info_area);
  }
}

pub(crate) fn human_size(bytes: u64) -> String
{
  const UNITS: [&str; 7] = ["B", "KB", "MB", "GB", "TB", "PB", "EB"];
  let mut val = bytes as f64;
  let mut idx = 0usize;
  while val >= 1024.0 && idx + 1 < UNITS.len()
  {
    val /= 1024.0;
    idx += 1;
  }
  if idx == 0
  {
    format!("{} {}", bytes, UNITS[idx])
  }
  else
  {
    format!("{:.1} {}", val, UNITS[idx])
  }
}

pub(crate) fn format_time_abs(
  t: std::time::SystemTime,
  fmt: &str,
) -> String
{
  use chrono::{
    DateTime,
    Local,
  };
  // Convert to local DateTime
  let dt: DateTime<Local> = DateTime::from(t);
  dt.format(fmt).to_string()
}

fn format_time_ago(t: std::time::SystemTime) -> String
{
  let now = std::time::SystemTime::now();
  match now.duration_since(t)
  {
    Ok(d) =>
    {
      let secs = d.as_secs();
      if secs < 60
      {
        format!("{}s ago", secs)
      }
      else if secs < 3600
      {
        format!("{}m ago", secs / 60)
      }
      else if secs < 86400
      {
        format!("{}h ago", secs / 3600)
      }
      else if secs < 86400 * 30
      {
        format!("{}d ago", secs / 86400)
      }
      else if secs < 86400 * 365
      {
        format!("{}mo ago", secs / (86400 * 30))
      }
      else
      {
        format!("{}y ago", secs / (86400 * 365))
      }
    }
    Err(_) => "just now".to_string(),
  }
}

fn format_info(
  app: &crate::App,
  e: &crate::app::DirEntryInfo,
) -> Option<String>
{
  use crate::app::InfoMode;
  let fmt = app.config.ui.date_format.as_deref().unwrap_or("%Y-%m-%d %H:%M");
  match app.info_mode
  {
    InfoMode::None => None,
    InfoMode::Size =>
    {
      if e.is_dir
      {
        None
      }
      else
      {
        Some(match app.display_mode
        {
          crate::app::DisplayMode::Friendly => human_size(e.size),
          crate::app::DisplayMode::Absolute => format!("{} B", e.size),
        })
      }
    }
    InfoMode::Created => match app.display_mode
    {
      crate::app::DisplayMode::Absolute =>
      {
        e.ctime.map(|t| format_time_abs(t, fmt))
      }
      crate::app::DisplayMode::Friendly => e.ctime.map(format_time_ago),
    },
    InfoMode::Modified => match app.display_mode
    {
      crate::app::DisplayMode::Absolute =>
      {
        e.mtime.map(|t| format_time_abs(t, fmt))
      }
      crate::app::DisplayMode::Friendly => e.mtime.map(format_time_ago),
    },
  }
}

// no per-pane header; info label is shown in top title line

pub fn build_row_line(
  app: &crate::App,
  _fmt: &crate::config::UiRowFormat,
  e: &crate::app::DirEntryInfo,
  inner_width: u16,
) -> Line<'static>
{
  let base_style = entry_style(app, e);
  // Left selection bar column (fixed width so text doesn't shift)
  let mut spans: Vec<Span> = Vec::new();
  // Base selection bar style from theme or fallback
  let mut bar_style = Style::default().fg(Color::Cyan);
  if let Some(th) = app.config.ui.theme.as_ref()
  {
    if let Some(fg) = th.selection_bar_fg.as_ref().and_then(|s| crate::ui::colors::parse_color(s))
    {
      bar_style = bar_style.fg(fg);
    }
    else if let Some(fg) = th.border_fg.as_ref().and_then(|s| crate::ui::colors::parse_color(s))
    {
      bar_style = bar_style.fg(fg);
    }
  }
  // Make the indicator wider (3x the previous bar width). Keep width fixed
  // for both selected and unselected rows to prevent text shifting.
  // Compose in order: {selected} {icon} {name} {info} with right-aligned info
  let marker = if e.is_dir { "/" } else { "" };
  let name_val = format!("{}{}", e.name, marker);
  let icon_val = compute_icon(app, e);
  let info_val = format_info(app, e).unwrap_or_default();

  // Determine clipboard state color override
  let mut sel_style = bar_style;
  if let Some(cb) = app.clipboard.as_ref()
    && cb.items.iter().any(|p| p == &e.path)
    && let Some(th) = app.config.ui.theme.as_ref()
  {
      match cb.op
      {
        crate::app::ClipboardOp::Copy =>
        {
          if let Some(fg) = th.selection_bar_copy_fg.as_ref().and_then(|s| crate::ui::colors::parse_color(s))
          {
            sel_style = Style::default().fg(fg);
          }
          else
          {
            sel_style = Style::default().fg(Color::Green);
          }
        }
        crate::app::ClipboardOp::Move =>
        {
          if let Some(fg) = th.selection_bar_move_fg.as_ref().and_then(|s| crate::ui::colors::parse_color(s))
          {
            sel_style = Style::default().fg(fg);
          }
          else
          {
            sel_style = Style::default().fg(Color::Yellow);
          }
        }
      }
  }

  // Left/Right composition with truncation and right alignment
  let sel_char = if app.selected.contains(&e.path) { "█" } else { " " };
  let mut left_rest = format!(" {} {}", icon_val, name_val);
  let mut right_txt = info_val;
  let total_w = inner_width as usize;

  // Truncate right first if it exceeds total width
  let mut right_w = UnicodeWidthStr::width(right_txt.as_str());
  if right_w > total_w
  {
    right_txt = truncate_with_tilde(&right_txt, total_w);
    right_w = UnicodeWidthStr::width(right_txt.as_str());
  }

  // Render selection marker if space permits
  let sel_w = UnicodeWidthStr::width(sel_char);
  let mut left_allowed = total_w.saturating_sub(right_w);
  let mut rendered_left_w = 0usize;
  if left_allowed >= sel_w && sel_w > 0
  {
    spans.push(Span::styled(sel_char.to_string(), sel_style));
    rendered_left_w += sel_w;
    left_allowed -= sel_w;
  }
  // Truncate left_rest to fit allowed width
  if left_allowed > 0
  {
    let lr_w = UnicodeWidthStr::width(left_rest.as_str());
    if lr_w > left_allowed
    {
      left_rest = truncate_with_tilde(&left_rest, left_allowed);
    }
    rendered_left_w += UnicodeWidthStr::width(left_rest.as_str());
    if !left_rest.is_empty()
    {
      spans.push(Span::styled(left_rest, base_style));
    }
  }

  // Pad so that right text ends at the right edge
  let total_rendered = rendered_left_w + right_w;
  let space = total_w.saturating_sub(total_rendered);
  if space > 0
  {
    let max_pad = 4096usize;
    spans.push(Span::styled(
      " ".repeat(std::cmp::min(space, max_pad)),
      base_style,
    ));
  }
  if right_w > 0
  {
    let mut s = Style::default().fg(Color::Gray);
    if let Some(th) = app.config.ui.theme.as_ref()
      && let Some(fg) =
        th.info_fg.as_ref().and_then(|v| crate::ui::colors::parse_color(v))
    {
      s = s.fg(fg);
    }
    spans.push(Span::styled(right_txt, s));
  }

  Line::from(spans)
}


fn compute_icon(
  _app: &crate::App,
  e: &crate::app::DirEntryInfo,
) -> String
{
  // Placeholder icons (to be themed later)
  if e.is_dir { "📁".to_string() } else { "📄".to_string() }
}

fn truncate_with_tilde(s: &str, max_w: usize) -> String
{
  if max_w == 0 { return String::new(); }
  let w = UnicodeWidthStr::width(s);
  if w <= max_w { return s.to_string(); }
  if max_w == 1 { return "~".to_string(); }
  let mut out = String::new();
  let mut used = 0usize;
  for ch in s.chars()
  {
    let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
    if used + cw + 1 > max_w { break; }
    out.push(ch);
    used += cw;
  }
  out.push('~');
  out
}

pub(crate) fn permissions_string(e: &crate::app::DirEntryInfo) -> String
{
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    let mut s = String::new();
    let (type_ch, mode) = if let Ok(meta) = std::fs::metadata(&e.path)
    {
      let ft = meta.file_type();
      let t = if e.is_dir || ft.is_dir() { 'd' } else { '-' };
      (t, meta.permissions().mode())
    }
    else
    {
      ('?', 0)
    };
    s.push(type_ch);
    if mode == 0
    {
      s.push_str("?????????");
      return s;
    }
    // user
    s.push(if mode & 0o400 != 0 { 'r' } else { '-' });
    s.push(if mode & 0o200 != 0 { 'w' } else { '-' });
    s.push(match (mode & 0o100 != 0, mode & 0o4000 != 0)
    {
      (true, true) => 's',
      (false, true) => 'S',
      (true, false) => 'x',
      (false, false) => '-',
    });
    // group
    s.push(if mode & 0o040 != 0 { 'r' } else { '-' });
    s.push(if mode & 0o020 != 0 { 'w' } else { '-' });
    s.push(match (mode & 0o010 != 0, mode & 0o2000 != 0)
    {
      (true, true) => 's',
      (false, true) => 'S',
      (true, false) => 'x',
      (false, false) => '-',
    });
    // other
    s.push(if mode & 0o004 != 0 { 'r' } else { '-' });
    s.push(if mode & 0o002 != 0 { 'w' } else { '-' });
    s.push(match (mode & 0o001 != 0, mode & 0o1000 != 0)
    {
      (true, true) => 't',
      (false, true) => 'T',
      (true, false) => 'x',
      (false, false) => '-',
    });
    s
  }
}

fn build_row_item(
  app: &crate::App,
  fmt: &crate::config::UiRowFormat,
  e: &crate::app::DirEntryInfo,
  inner_width: u16,
) -> ListItem<'static>
{
  let item = ListItem::new(build_row_line(app, fmt, e, inner_width));
  item.style(entry_style(app, e))
}

fn entry_style(
  app: &crate::App,
  e: &crate::app::DirEntryInfo,
) -> Style
{
  let mut st = Style::default();
  let th = match app.config.ui.theme.as_ref()
  {
    Some(t) => t,
    None => return st,
  };
  // base item colors
  if let Some(fg) =
    th.item_fg.as_ref().and_then(|s| crate::ui::colors::parse_color(s))
  {
    st = st.fg(fg);
  }
  if let Some(bg) =
    th.item_bg.as_ref().and_then(|s| crate::ui::colors::parse_color(s))
  {
    st = st.bg(bg);
  }
  // type overrides
  if e.is_dir
  {
    if let Some(fg) =
      th.dir_fg.as_ref().and_then(|s| crate::ui::colors::parse_color(s))
    {
      st = st.fg(fg);
    }
    if let Some(bg) =
      th.dir_bg.as_ref().and_then(|s| crate::ui::colors::parse_color(s))
    {
      st = st.bg(bg);
    }
  }
  else
  {
    if let Some(fg) =
      th.file_fg.as_ref().and_then(|s| crate::ui::colors::parse_color(s))
    {
      st = st.fg(fg);
    }
    if let Some(bg) =
      th.file_bg.as_ref().and_then(|s| crate::ui::colors::parse_color(s))
    {
      st = st.bg(bg);
    }
    if is_executable(&e.path)
    {
      if let Some(fg) =
        th.exec_fg.as_ref().and_then(|s| crate::ui::colors::parse_color(s))
      {
        st = st.fg(fg);
      }
      if let Some(bg) =
        th.exec_bg.as_ref().and_then(|s| crate::ui::colors::parse_color(s))
      {
        st = st.bg(bg);
      }
    }
  }
  // hidden overrides last
  if e.name.starts_with('.')
  {
    if let Some(fg) =
      th.hidden_fg.as_ref().and_then(|s| crate::ui::colors::parse_color(s))
    {
      st = st.fg(fg);
    }
    if let Some(bg) =
      th.hidden_bg.as_ref().and_then(|s| crate::ui::colors::parse_color(s))
    {
      st = st.bg(bg);
    }
  }
  st
}

#[cfg(unix)]
fn is_executable(path: &std::path::Path) -> bool
{
  use std::os::unix::fs::PermissionsExt;
  if let Ok(meta) = std::fs::metadata(path)
  {
    let mode = meta.permissions().mode();
    return (mode & 0o111) != 0;
  }
  false
}
