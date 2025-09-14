use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};
use crate::ui::ansi::ansi_spans;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};


pub fn pane_constraints(app: &crate::App) -> [Constraint; 3] {
  let (mut p, mut c, mut r) = (30u16, 40u16, 30u16);
  if let Some(panes) = app.config.ui.panes.as_ref() {
    p = panes.parent;
    c = panes.current;
    r = panes.preview;
  }
  let total = p.saturating_add(c).saturating_add(r);
  if total == 0 {
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
) {
  f.render_widget(Clear, area);
  let mut block = Block::default().borders(Borders::ALL);
  if let Some(th) = app.config.ui.theme.as_ref() {
    if let Some(bg) = th.pane_bg.as_ref().and_then(|s| crate::ui::colors::parse_color(s)) {
      block = block.style(Style::default().bg(bg));
    }
    if let Some(bfg) = th.border_fg.as_ref().and_then(|s| crate::ui::colors::parse_color(s)) {
      block = block.border_style(Style::default().fg(bfg));
    }
  }
  // Draw block and compute inner content area
  f.render_widget(block.clone(), area);
  let inner = block.inner(area);
  let inner_width = inner.width;
  let fmt = app.config.ui.row.clone().unwrap_or_default();
  // List area (full inner area; no per-pane header)
  let list_area = Rect { x: inner.x, y: inner.y, width: inner.width, height: inner.height };
  let items: Vec<ListItem> = app
    .parent_entries
    .iter()
    .map(|e| build_row_item(app, &fmt, e, inner_width))
    .collect();
  let mut list = List::new(items);
  if let Some(th) = app.config.ui.theme.as_ref() {
    if let Some(fg) = th.item_fg.as_ref().and_then(|s| crate::ui::colors::parse_color(s)) {
      list = list.style(Style::default().fg(fg));
    }
    if let Some(bg) = th.item_bg.as_ref().and_then(|s| crate::ui::colors::parse_color(s)) {
      list = list.style(Style::default().bg(bg));
    }
  }
  f.render_widget(list, list_area);
}

pub fn draw_current_panel(
  f: &mut ratatui::Frame,
  area: Rect,
  app: &mut crate::App,
) {
  f.render_widget(Clear, area);
  let mut block = Block::default().borders(Borders::ALL);
  if let Some(th) = app.config.ui.theme.as_ref() {
    if let Some(bg) = th.pane_bg.as_ref().and_then(|s| crate::ui::colors::parse_color(s)) {
      block = block.style(Style::default().bg(bg));
    }
    if let Some(bfg) = th.border_fg.as_ref().and_then(|s| crate::ui::colors::parse_color(s)) {
      block = block.border_style(Style::default().fg(bfg));
    }
  }
  // Draw block and compute inner content area
  f.render_widget(block.clone(), area);
  let inner = block.inner(area);
  let inner_width = inner.width;
  let fmt = app.config.ui.row.clone().unwrap_or_default();
  let highlight_symbol = "▶ ";
  let hl_w = UnicodeWidthStr::width(highlight_symbol) as u16;
  let avail_width = inner_width.saturating_sub(hl_w);
  let items: Vec<ListItem> = app
    .current_entries
    .iter()
    .map(|e| build_row_item(app, &fmt, e, avail_width))
    .collect();

  // List area (full inner area; no per-pane header)
  let list_area = Rect { x: inner.x, y: inner.y, width: inner.width, height: inner.height };
  let mut list = List::new(items).highlight_symbol(highlight_symbol);
  if let Some(th) = app.config.ui.theme.as_ref() {
    let mut hl = Style::default();
    if let Some(fg) = th.selected_item_fg.as_ref().and_then(|s| crate::ui::colors::parse_color(s)) {
      hl = hl.fg(fg);
    }
    if let Some(bg) = th.selected_item_bg.as_ref().and_then(|s| crate::ui::colors::parse_color(s)) {
      hl = hl.bg(bg);
    }
    list = list.highlight_style(hl.add_modifier(Modifier::BOLD));
    if let Some(fg) = th.item_fg.as_ref().and_then(|s| crate::ui::colors::parse_color(s)) {
      list = list.style(Style::default().fg(fg));
    }
    if let Some(bg) = th.item_bg.as_ref().and_then(|s| crate::ui::colors::parse_color(s)) {
      list = list.style(Style::default().bg(bg));
    }
  } else {
    list = list.highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
  }

  f.render_stateful_widget(list, list_area, &mut app.list_state);
}

pub fn draw_error_bar(
  f: &mut ratatui::Frame,
  area: Rect,
  msg: &str,
) {
  let layout = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Min(0), Constraint::Length(1)])
    .split(area);
  let bar = layout[1];
  let text = Line::from(Span::styled(
    msg.to_string(),
    Style::default()
      .fg(Color::Black)
      .bg(Color::Red)
      .add_modifier(Modifier::BOLD),
  ));
  let para = Paragraph::new(text);
  f.render_widget(Clear, bar);
  f.render_widget(para, bar);
}

pub fn draw_whichkey_panel(
  f: &mut ratatui::Frame,
  area: Rect,
  app: &crate::App,
) {
  // Build lookup: last mapping wins for duplicate sequences
  use std::collections::HashMap;
  let mut map: HashMap<&str, (&str, &str)> = HashMap::new();
  for km in &app.keymaps {
    let label = km.description.as_deref().unwrap_or(km.action.as_str());
    map.insert(km.sequence.as_str(), (km.sequence.as_str(), label));
  }

  let prefix = app.whichkey_prefix.as_str();
  // Bucket by next-prefix (prefix + next char)
  let mut buckets: HashMap<String, Vec<(&str, &str)>> = HashMap::new();
  for (seq, (_, label)) in map.into_iter() {
    if seq.starts_with(prefix) && seq.len() > prefix.len() {
      let mut np = String::with_capacity(prefix.len() + 1);
      np.push_str(prefix);
      if let Some(ch) = seq.chars().nth(prefix.chars().count()) {
        np.push(ch);
      } else {
        continue;
      }
      buckets.entry(np).or_default().push((seq, label));
    } else if seq == prefix {
      // exact match binding at current prefix (unlikely); treat as its own bucket
      buckets.entry(seq.to_string()).or_default().push((seq, label));
    }
  }

  // Flatten into display entries, sorted alphabetically by key/prefix
  #[derive(Clone)]
  struct Entry { left: String, right: String, is_group: bool }
  let mut entries: Vec<Entry> = Vec::new();
  let mut keys: Vec<String> = buckets.keys().cloned().collect();
  keys.sort();
  for k in keys {
    let list = buckets.get(&k).unwrap();
    // Determine if single binding exactly equals this next-prefix
    let mut exact_only = false;
    if list.len() == 1 {
      let (seq, _label) = list[0];
      if seq == k { exact_only = true; }
    }
    if exact_only {
      let (_seq, label) = list[0];
      entries.push(Entry { left: k.clone(), right: label.to_string(), is_group: false });
    } else {
      let n = list.len();
      let label = if n == 1 { "(1 binding)".to_string() } else { format!("({} bindings)", n) };
      entries.push(Entry { left: k.clone(), right: label, is_group: true });
    }
  }

  // If no matches, nothing to draw
  if entries.is_empty() { return; }

  // Layout: multiple columns as needed
  let title_str = if prefix.is_empty() { "Keys".to_string() } else { format!("Keys: prefix '{}'", prefix) };
  let mut block = Block::default().borders(Borders::ALL).title(Span::styled(
    title_str,
    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
  ));
  if let Some(th) = app.config.ui.theme.as_ref() {
    if let Some(bg) = th.pane_bg.as_ref().and_then(|s| crate::ui::colors::parse_color(s)) { block = block.style(Style::default().bg(bg)); }
    if let Some(bfg) = th.border_fg.as_ref().and_then(|s| crate::ui::colors::parse_color(s)) { block = block.border_style(Style::default().fg(bfg)); }
  }

  // Compute desired height: 20% of screen by default, expand if columns overflow width
  let inner_width = area.width.saturating_sub(2) as usize; // account for borders
  let mut rows = ((area.height as u32 * 20) / 100) as u16;
  if rows < 3 { rows = 3; }
  if rows + 2 > area.height { rows = area.height.saturating_sub(2); }
  if rows == 0 { rows = 1; }

  // Function to compute column widths for a given row count
  let compute_widths = |row_count: usize| -> (Vec<usize>, usize, usize) {
    let rows_usize = row_count.max(1);
    let cols = ((entries.len() + rows_usize - 1) / rows_usize).max(1);
    let mut col_widths = vec![0usize; cols];
    for c in 0..cols {
      let mut w = 0usize;
      for r in 0..rows_usize {
        let idx = c * rows_usize + r;
        if idx >= entries.len() { break; }
        let e = &entries[idx];
        let cell = format!("{}  {}", e.left, e.right);
        let cw = UnicodeWidthStr::width(cell.as_str());
        if cw > w { w = cw; }
      }
      col_widths[c] = w + 2; // inter-column gap
    }
    let total: usize = col_widths.iter().sum();
    (col_widths, total, cols)
  };

  let mut rows_usize = rows as usize;
  let (mut col_widths, mut total_width, mut cols) = compute_widths(rows_usize);
  while total_width > inner_width && (rows_usize as u16) + 2 < area.height {
    // Increase rows to reduce columns, then recompute
    rows_usize += 1;
    let res = compute_widths(rows_usize);
    col_widths = res.0; total_width = res.1; cols = res.2;
  }

  // Build lines row-wise using final rows_usize
  let mut lines: Vec<Line> = Vec::new();
  for r in 0..rows_usize {
    let mut spans: Vec<Span> = Vec::new();
    let mut consumed_any = false;
    for c in 0..cols {
      let idx = c * rows_usize + r;
      if idx >= entries.len() { continue; }
      consumed_any = true;
      let e = &entries[idx];
      let left_style = Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
      let right_style = if e.is_group { Style::default().fg(Color::DarkGray) } else { Style::default().fg(Color::Gray) };
      let cell = format!("{}  {}", e.left, e.right);
      let cw = UnicodeWidthStr::width(cell.as_str());
      spans.push(Span::styled(e.left.clone(), left_style));
      spans.push(Span::raw("  "));
      spans.push(Span::styled(e.right.clone(), right_style));
      // pad to column width
      let pad = col_widths[c].saturating_sub(cw) as usize;
      if pad > 0 { spans.push(Span::raw(" ".repeat(pad))); }
    }
    if consumed_any { lines.push(Line::from(spans)); }
  }

  // Render at bottom as before
  let panel_height = (rows_usize as u16).saturating_add(2).min(area.height);
  let layout = Layout::default().direction(Direction::Vertical).constraints([
    Constraint::Min(0), Constraint::Length(panel_height)
  ]).split(area);
  let panel = layout[1];
  f.render_widget(Clear, panel);
  let para = Paragraph::new(lines).block(block);
  f.render_widget(para, panel);
}

pub fn draw_messages_panel(
  f: &mut ratatui::Frame,
  area: Rect,
  app: &crate::App,
) {
  // Bottom area: start with 20% height, expand up to fit all messages but cap at 50%
  let min_h = ((area.height as u32 * 20) / 100).max(3) as u16;
  let max_h = ((area.height as u32 * 50) / 100).max(min_h as u32) as u16;
  // Determine needed height (messages + borders)
  let needed = (app.recent_messages.len() as u16).saturating_add(2).max(3);
  let panel_h = needed.min(max_h).max(min_h).min(area.height);

  let mut block = Block::default()
    .borders(Borders::ALL)
    .title(Span::styled(
      "Messages",
      Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    ));
  if let Some(th) = app.config.ui.theme.as_ref() {
    if let Some(bg) = th.pane_bg.as_ref().and_then(|s| crate::ui::colors::parse_color(s)) { block = block.style(Style::default().bg(bg)); }
    if let Some(bfg) = th.border_fg.as_ref().and_then(|s| crate::ui::colors::parse_color(s)) { block = block.border_style(Style::default().fg(bfg)); }
  }
  let layout = Layout::default().direction(Direction::Vertical).constraints([
    Constraint::Min(0), Constraint::Length(panel_h)
  ]).split(area);
  let panel = layout[1];
  f.render_widget(Clear, panel);

  // Render messages newest last (bottom) for natural reading; we will take the last panel_h-2 items
  let avail_rows = panel_h.saturating_sub(2) as usize;
  let start = app.recent_messages.len().saturating_sub(avail_rows);
  let slice = &app.recent_messages[start..];
  let mut lines: Vec<Line> = Vec::new();
  for m in slice {
    lines.push(Line::from(Span::styled(m.clone(), Style::default().fg(Color::Gray))));
  }
  let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
  f.render_widget(para, panel);
}

pub fn draw_output_panel(
  f: &mut ratatui::Frame,
  area: Rect,
  app: &crate::App,
) {
  let min_h = ((area.height as u32 * 20) / 100).max(3) as u16;
  let max_h = ((area.height as u32 * 60) / 100).max(min_h as u32) as u16;
  let needed = (app.output_lines.len() as u16).saturating_add(2).max(3);
  let panel_h = needed.min(max_h).max(min_h).min(area.height);

  let mut block = Block::default()
    .borders(Borders::ALL)
    .title(Span::styled(
      app.output_title.clone(),
      Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    ));
  if let Some(th) = app.config.ui.theme.as_ref() {
    if let Some(bg) = th.pane_bg.as_ref().and_then(|s| crate::ui::colors::parse_color(s)) { block = block.style(Style::default().bg(bg)); }
    if let Some(bfg) = th.border_fg.as_ref().and_then(|s| crate::ui::colors::parse_color(s)) { block = block.border_style(Style::default().fg(bfg)); }
  }
  let layout = Layout::default().direction(Direction::Vertical).constraints([
    Constraint::Min(0), Constraint::Length(panel_h)
  ]).split(area);
  let panel = layout[1];
  f.render_widget(Clear, panel);

  let avail_rows = panel_h.saturating_sub(2) as usize;
  let start = app.output_lines.len().saturating_sub(avail_rows);
  let slice = &app.output_lines[start..];
  let mut lines: Vec<Line> = Vec::new();
  for m in slice {
    lines.push(Line::from(ansi_spans(m)));
  }
  let para = Paragraph::new(lines).block(block).wrap(Wrap { trim: true });
  f.render_widget(para, panel);
}

pub(crate) fn human_size(bytes: u64) -> String {
  const UNITS: [&str; 7] = ["B", "KB", "MB", "GB", "TB", "PB", "EB"];
  let mut val = bytes as f64;
  let mut idx = 0usize;
  while val >= 1024.0 && idx + 1 < UNITS.len() {
    val /= 1024.0;
    idx += 1;
  }
  if idx == 0 { format!("{} {}", bytes, UNITS[idx]) } else { format!("{:.1} {}", val, UNITS[idx]) }
}

pub(crate) fn format_time_abs(t: std::time::SystemTime, fmt: &str) -> String {
  use chrono::{DateTime, Local};
  // Convert to local DateTime
  let dt: DateTime<Local> = DateTime::from(t);
  dt.format(fmt).to_string()
}

fn format_time_ago(t: std::time::SystemTime) -> String {
  let now = std::time::SystemTime::now();
  match now.duration_since(t) {
    Ok(d) => {
      let secs = d.as_secs();
      if secs < 60 { format!("{}s ago", secs) }
      else if secs < 3600 { format!("{}m ago", secs / 60) }
      else if secs < 86400 { format!("{}h ago", secs / 3600) }
      else if secs < 86400 * 30 { format!("{}d ago", secs / 86400) }
      else if secs < 86400 * 365 { format!("{}mo ago", secs / (86400 * 30)) }
      else { format!("{}y ago", secs / (86400 * 365)) }
    }
    Err(_) => "just now".to_string(),
  }
}

fn format_info(app: &crate::App, e: &crate::app::DirEntryInfo) -> Option<String> {
  use crate::app::InfoMode;
  let fmt = app
    .config
    .ui
    .date_format
    .as_deref()
    .unwrap_or("%Y-%m-%d %H:%M");
  match app.info_mode {
    InfoMode::None => None,
    InfoMode::Size => {
      if e.is_dir { None } else {
        Some(match app.display_mode {
          crate::app::DisplayMode::Friendly => human_size(e.size),
          crate::app::DisplayMode::Absolute => format!("{} B", e.size),
        })
      }
    }
    InfoMode::Created => match app.display_mode {
      crate::app::DisplayMode::Absolute => e.ctime.map(|t| format_time_abs(t, fmt)),
      crate::app::DisplayMode::Friendly => e.ctime.map(format_time_ago),
    },
    InfoMode::Modified => match app.display_mode {
      crate::app::DisplayMode::Absolute => e.mtime.map(|t| format_time_abs(t, fmt)),
      crate::app::DisplayMode::Friendly => e.mtime.map(format_time_ago),
    },
  }
}

// no per-pane header; info label is shown in top title line

pub fn build_row_line(
  app: &crate::App,
  fmt: &crate::config::UiRowFormat,
  e: &crate::app::DirEntryInfo,
  inner_width: u16,
) -> Line<'static> {
  let base_style = entry_style(app, e);
  let marker = if e.is_dir { "/" } else { "" };
  let name_val = format!("{}{}", e.name, marker);
  let icon_val = compute_icon(app, e);
  let info_val = format_info(app, e).unwrap_or_default();
  let perms_val = permissions_string(e);
  let icon_s = replace_placeholders(&fmt.icon, &icon_val, &name_val, &info_val, &perms_val);
  let left_s = replace_placeholders(&fmt.left, &icon_val, &name_val, &info_val, &perms_val);
  let right_s = replace_placeholders(&fmt.right, &icon_val, &name_val, &info_val, &perms_val);
  // Compose: [icon][left] ... [right aligned]
  let mut spans: Vec<Span> = Vec::new();
  let total = inner_width as i32;

  // If fixed widths configured, fit each cell into its box
  if let Some(rw) = app.config.ui.row_widths.as_ref() {
    let iw = rw.icon as usize;
    let lw = rw.left as usize;
    let rw_w = rw.right as usize;
    if iw + lw + rw_w > 0 && (iw + lw + rw_w) as i32 <= total {
      let icon_cell = fit_cell(&icon_s, iw, false);
      let left_cell = fit_cell(&left_s, lw, false);
      let right_cell = fit_cell(&right_s, rw_w, true);
      if iw > 0 { spans.push(Span::styled(icon_cell, base_style)); }
      if lw > 0 { spans.push(Span::styled(left_cell, base_style)); }
      if rw_w > 0 {
        let mut s = Style::default().fg(Color::Gray);
        if let Some(th) = app.config.ui.theme.as_ref() {
          if let Some(fg) = th.info_fg.as_ref().and_then(|v| crate::ui::colors::parse_color(v)) { s = s.fg(fg); }
        }
        spans.push(Span::styled(right_cell, s));
      }
      return Line::from(spans);
    }
  }

  // Auto layout fallback
  let icon_w = UnicodeWidthStr::width(icon_s.as_str()) as i32;
  let left_w = UnicodeWidthStr::width(left_s.as_str()) as i32;
  let mut current_w = 0i32;
  if !icon_s.is_empty() { spans.push(Span::styled(icon_s.clone(), base_style)); current_w += icon_w; }
  if !left_s.is_empty() { spans.push(Span::styled(left_s.clone(), base_style)); current_w += left_w; }

  let right_txt = right_s.clone();
  let right_w = UnicodeWidthStr::width(right_txt.as_str()) as i32;
  let space = (total - current_w).max(0);

  if space > 0 {
    // Right-align info text in remaining space
    let pad_before_right = space.saturating_sub(right_w) as usize;
    if pad_before_right > 0 { spans.push(Span::styled(" ".repeat(pad_before_right), base_style)); }
    if right_w > 0 {
      let mut s = Style::default().fg(Color::Gray);
      if let Some(th) = app.config.ui.theme.as_ref() { if let Some(fg) = th.info_fg.as_ref().and_then(|v| crate::ui::colors::parse_color(v)) { s = s.fg(fg); } }
      spans.push(Span::styled(right_txt, s));
    }
  }

  Line::from(spans)
}

fn replace_placeholders(tpl: &str, icon: &str, name: &str, info: &str, perms: &str) -> String {
  let mut s = tpl.replace("{icon}", icon);
  s = s.replace("{name}", name);
  s = s.replace("{info}", info);
  s = s.replace("{perms}", perms);
  s
}

fn compute_icon(_app: &crate::App, _e: &crate::app::DirEntryInfo) -> String {
  // Placeholder: integrate actual icon theme later. For now, one space to reserve a column.
  " ".to_string()
}

pub(crate) fn permissions_string(e: &crate::app::DirEntryInfo) -> String {
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    let mut s = String::new();
    let (type_ch, mode) = if let Ok(meta) = std::fs::metadata(&e.path) {
      let ft = meta.file_type();
      let t = if e.is_dir || ft.is_dir() { 'd' } else { '-' };
      (t, meta.permissions().mode())
    } else {
      ('?', 0)
    };
    s.push(type_ch);
    if mode == 0 {
      s.push_str("?????????");
      return s;
    }
    // user
    s.push(if mode & 0o400 != 0 { 'r' } else { '-' });
    s.push(if mode & 0o200 != 0 { 'w' } else { '-' });
    s.push(match (mode & 0o100 != 0, mode & 0o4000 != 0) {
      (true, true) => 's',
      (false, true) => 'S',
      (true, false) => 'x',
      (false, false) => '-',
    });
    // group
    s.push(if mode & 0o040 != 0 { 'r' } else { '-' });
    s.push(if mode & 0o020 != 0 { 'w' } else { '-' });
    s.push(match (mode & 0o010 != 0, mode & 0o2000 != 0) {
      (true, true) => 's',
      (false, true) => 'S',
      (true, false) => 'x',
      (false, false) => '-',
    });
    // other
    s.push(if mode & 0o004 != 0 { 'r' } else { '-' });
    s.push(if mode & 0o002 != 0 { 'w' } else { '-' });
    s.push(match (mode & 0o001 != 0, mode & 0o1000 != 0) {
      (true, true) => 't',
      (false, true) => 'T',
      (true, false) => 'x',
      (false, false) => '-',
    });
    s
  }
  #[cfg(not(unix))]
  {
    String::new()
  }
}

fn truncate_to_width(s: &str, max_w: usize) -> String {
  if max_w == 0 { return String::new(); }
  let mut out = String::new();
  let mut w = 0usize;
  for ch in s.chars() {
    let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
    if w + cw > max_w { break; }
    out.push(ch);
    w += cw;
  }
  out
}

fn truncate_tail_to_width(s: &str, max_w: usize) -> String {
  if max_w == 0 { return String::new(); }
  let mut out_rev: Vec<char> = Vec::new();
  let mut w = 0usize;
  for ch in s.chars().rev() {
    let cw = UnicodeWidthChar::width(ch).unwrap_or(0);
    if w + cw > max_w { break; }
    out_rev.push(ch);
    w += cw;
  }
  out_rev.into_iter().rev().collect()
}

fn fit_cell(text: &str, width: usize, align_right: bool) -> String {
  if width == 0 { return String::new(); }
  let w = UnicodeWidthStr::width(text);
  if w == width { return text.to_string(); }
  if w < width {
    let pad = " ".repeat(width - w);
    return if align_right { format!("{}{}", pad, text) } else { format!("{}{}", text, pad) };
  }
  if align_right { truncate_tail_to_width(text, width) } else { truncate_to_width(text, width) }
}

fn build_row_item(
  app: &crate::App,
  fmt: &crate::config::UiRowFormat,
  e: &crate::app::DirEntryInfo,
  inner_width: u16,
) -> ListItem<'static> {
  let item = ListItem::new(build_row_line(app, fmt, e, inner_width));
  item.style(entry_style(app, e))
}

fn entry_style(app: &crate::App, e: &crate::app::DirEntryInfo) -> Style {
  let mut st = Style::default();
  let th = match app.config.ui.theme.as_ref() { Some(t) => t, None => return st };
  // base item colors
  if let Some(fg) = th.item_fg.as_ref().and_then(|s| crate::ui::colors::parse_color(s)) { st = st.fg(fg); }
  if let Some(bg) = th.item_bg.as_ref().and_then(|s| crate::ui::colors::parse_color(s)) { st = st.bg(bg); }
  // type overrides
  if e.is_dir {
    if let Some(fg) = th.dir_fg.as_ref().and_then(|s| crate::ui::colors::parse_color(s)) { st = st.fg(fg); }
    if let Some(bg) = th.dir_bg.as_ref().and_then(|s| crate::ui::colors::parse_color(s)) { st = st.bg(bg); }
  } else {
    if let Some(fg) = th.file_fg.as_ref().and_then(|s| crate::ui::colors::parse_color(s)) { st = st.fg(fg); }
    if let Some(bg) = th.file_bg.as_ref().and_then(|s| crate::ui::colors::parse_color(s)) { st = st.bg(bg); }
    if is_executable(&e.path) {
      if let Some(fg) = th.exec_fg.as_ref().and_then(|s| crate::ui::colors::parse_color(s)) { st = st.fg(fg); }
      if let Some(bg) = th.exec_bg.as_ref().and_then(|s| crate::ui::colors::parse_color(s)) { st = st.bg(bg); }
    }
  }
  // hidden overrides last
  if e.name.starts_with('.') {
    if let Some(fg) = th.hidden_fg.as_ref().and_then(|s| crate::ui::colors::parse_color(s)) { st = st.fg(fg); }
    if let Some(bg) = th.hidden_bg.as_ref().and_then(|s| crate::ui::colors::parse_color(s)) { st = st.bg(bg); }
  }
  st
}

#[cfg(unix)]
fn is_executable(path: &std::path::Path) -> bool {
  use std::os::unix::fs::PermissionsExt;
  if let Ok(meta) = std::fs::metadata(path) {
    let mode = meta.permissions().mode();
    return (mode & 0o111) != 0;
  }
  false
}

#[cfg(not(unix))]
fn is_executable(_path: &std::path::Path) -> bool {
  false
}
