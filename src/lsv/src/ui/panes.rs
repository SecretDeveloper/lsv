use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

pub fn panel_title<'a>(
  label: &'a str,
  path: Option<&std::path::Path>,
) -> Line<'a> {
  let path_str = path
    .map(|p| p.to_string_lossy().to_string())
    .unwrap_or_else(|| String::from("<root>"));
  Line::from(vec![
    Span::styled(
      label,
      Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    ),
    Span::raw("  "),
    Span::styled(path_str, Style::default().fg(Color::Gray)),
  ])
}

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
  // Collect matching keymaps for the current prefix
  let prefix = &app.whichkey_prefix;
  let mut entries: Vec<(String, String)> = Vec::new();
  for km in &app.keymaps {
    if km.sequence.starts_with(prefix) {
      let label = km
        .description
        .as_ref()
        .cloned()
        .unwrap_or_else(|| km.action.clone());
      entries.push((km.sequence.clone(), label));
    }
  }
  // If toggled via '?' with empty prefix, just show all entries
  // Limit number of rows
  let max_rows: usize = 12;
  if entries.len() > max_rows {
    entries.truncate(max_rows);
  }

  let title_str = if prefix.is_empty() {
    "Keys".to_string()
  } else {
    format!("Keys: prefix '{}'", prefix)
  };
  let block = Block::default()
    .borders(Borders::ALL)
    .title(Span::styled(
      title_str,
      Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    ));

  let items: Vec<ListItem> = entries
    .into_iter()
    .map(|(seq, label)| {
      let line = Line::from(vec![
        Span::styled(seq, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(label, Style::default().fg(Color::Gray)),
      ]);
      ListItem::new(line)
    })
    .collect();

  // Panel height: items + borders + one padding row
  let height = (items.len() as u16).saturating_add(2).max(3).min(area.height);

  // Place at bottom
  let layout = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Min(0), Constraint::Length(height)])
    .split(area);
  let panel = layout[1];

  f.render_widget(Clear, panel);
  let list = List::new(items).block(block);
  f.render_widget(list, panel);
}

fn human_size(bytes: u64) -> String {
  const UNITS: [&str; 7] = ["B", "KB", "MB", "GB", "TB", "PB", "EB"];
  let mut val = bytes as f64;
  let mut idx = 0usize;
  while val >= 1024.0 && idx + 1 < UNITS.len() {
    val /= 1024.0;
    idx += 1;
  }
  if idx == 0 { format!("{} {}", bytes, UNITS[idx]) } else { format!("{:.1} {}", val, UNITS[idx]) }
}

fn format_time_abs(t: std::time::SystemTime, fmt: &str) -> String {
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
  let mid_s = replace_placeholders(&fmt.middle, &icon_val, &name_val, &info_val, &perms_val);
  let right_s = replace_placeholders(&fmt.right, &icon_val, &name_val, &info_val, &perms_val);
  // Compose with truncation: [icon][left] [middle centered] [right aligned]
  let mut spans: Vec<Span> = Vec::new();
  let total = inner_width as i32;
  let icon_w = UnicodeWidthStr::width(icon_s.as_str()) as i32;
  let left_w = UnicodeWidthStr::width(left_s.as_str()) as i32;
  let mut current_w = 0i32;

  if !icon_s.is_empty() {
    spans.push(Span::styled(icon_s.clone(), base_style));
    current_w += icon_w;
  }
  if !left_s.is_empty() {
    spans.push(Span::styled(left_s.clone(), base_style));
    current_w += left_w;
  }

  // Compute available space for middle + gap + right
  let mut right_txt = right_s.clone();
  let mut right_w = UnicodeWidthStr::width(right_txt.as_str()) as i32;
  let mut mid_txt = mid_s.clone();
  let mut mid_w = UnicodeWidthStr::width(mid_txt.as_str()) as i32;

  let space_for_mid_and_gap_and_right = (total - current_w).max(0);
  // Reserve at least 1 space before right if any content
  if right_w > 0 && space_for_mid_and_gap_and_right > 0 {
    // First, try to fit right; if not, truncate right after dropping mid
    // Step 1: drop or truncate middle if needed
    let middle_space = space_for_mid_and_gap_and_right.saturating_sub(1);
    if mid_w > middle_space {
      // truncate middle to available space (can be zero)
      mid_txt = truncate_to_width(&mid_txt, middle_space as usize);
      mid_w = UnicodeWidthStr::width(mid_txt.as_str()) as i32;
    }
    // Recompute remaining for right (with 1 space)
    let remaining_for_right = space_for_mid_and_gap_and_right - mid_w - 1;
    if remaining_for_right < right_w {
      right_txt = truncate_tail_to_width(&right_txt, remaining_for_right.max(0) as usize);
      right_w = UnicodeWidthStr::width(right_txt.as_str()) as i32;
    }

    // After truncation, if still no room for right, drop middle entirely
    let mut remaining = space_for_mid_and_gap_and_right - mid_w - 1;
    if remaining < right_w {
      mid_txt.clear();
      mid_w = 0;
      remaining = space_for_mid_and_gap_and_right - 1;
      if remaining < right_w {
        right_txt = truncate_tail_to_width(&right_txt, remaining.max(0) as usize);
        right_w = UnicodeWidthStr::width(right_txt.as_str()) as i32;
      }
    }

    // Now place middle centered within middle_space and pad to right
    let middle_space = space_for_mid_and_gap_and_right - 1 - right_w;
    if mid_w > 0 && middle_space > 0 {
      let mut mid_start = current_w + (middle_space - mid_w) / 2;
      if mid_start < current_w { mid_start = current_w; }
      let pad_before_mid = (mid_start - current_w) as usize;
      if pad_before_mid > 0 { spans.push(Span::styled(" ".repeat(pad_before_mid), base_style)); }
      spans.push(Span::styled(mid_txt.clone(), base_style));
      current_w = mid_start + mid_w;
    }

    // Pad up to where right should start; allow zero gap if exact fit
    let pad_before_right = (total - right_w - current_w).max(0) as usize;
    if pad_before_right > 0 { spans.push(Span::styled(" ".repeat(pad_before_right), base_style)); }
    if right_w > 0 {
      let mut s = Style::default().fg(Color::Gray);
      if let Some(th) = app.config.ui.theme.as_ref() {
        if let Some(fg) = th.info_fg.as_ref().and_then(|v| crate::ui::colors::parse_color(v)) {
          s = s.fg(fg);
        }
      }
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

fn permissions_string(e: &crate::app::DirEntryInfo) -> String {
  #[cfg(unix)]
  {
    use std::os::unix::fs::PermissionsExt;
    let mut s = String::new();
    s.push(if e.is_dir { 'd' } else { '-' });
    if let Ok(meta) = std::fs::metadata(&e.path) {
      let mode = meta.permissions().mode();
      let classes = [(mode >> 6) & 0o7, (mode >> 3) & 0o7, mode & 0o7];
      for c in classes { s.push_str(rwx(c as u8)); }
    } else {
      s.push_str("?????????");
    }
    s
  }
  #[cfg(not(unix))]
  {
    String::new()
  }
}

#[cfg(unix)]
fn rwx(bits: u8) -> &'static str {
  match bits & 0o7 {
    0o0 => "---",
    0o1 => "--x",
    0o2 => "-w-",
    0o3 => "-wx",
    0o4 => "r--",
    0o5 => "r-x",
    0o6 => "rw-",
    _ => "rwx",
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
