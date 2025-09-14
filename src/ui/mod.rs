pub mod ansi;
pub mod panes;
pub mod colors;

use ratatui::layout::{Direction, Layout, Constraint, Alignment, Rect};
use ratatui::widgets::Paragraph;
use unicode_width::UnicodeWidthStr;
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

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
  if app.show_messages {
    panes::draw_messages_panel(f, f.area(), app);
  }
  if app.show_output {
    panes::draw_output_panel(f, f.area(), app);
  }
}

fn draw_header(f: &mut ratatui::Frame, area: Rect, app: &crate::App) {
  // Left: {user}@{host}:{current_dir}
  let user = whoami::username();
  let host = whoami::fallible::hostname().unwrap_or_default();
  let left_full = format!("{}@{}:{}", user, host, app.cwd.display());

  // Right: details for selected entry: perms, owner, size, created
  let right_full = if let Some(sel) = app.selected_entry() {
    let owner = owner_string(&sel.path);
    let size_s = if sel.is_dir { "-".to_string() } else {
      match app.display_mode {
        crate::app::DisplayMode::Friendly => crate::ui::panes::human_size(sel.size),
        crate::app::DisplayMode::Absolute => format!("{} B", sel.size),
      }
    };
    let perms = crate::ui::panes::permissions_string(sel);
    let created_s = if let Some(ct) = sel.ctime {
      let fmt = app.config.ui.date_format.as_deref().unwrap_or("%Y-%m-%d %H:%M");
      crate::ui::panes::format_time_abs(ct, fmt)
    } else { String::from("-") };
    format!("{}  {}  {}  {}", size_s, owner, perms, created_s)
  } else { String::new() };

  let total = area.width as usize;
  let right_w = UnicodeWidthStr::width(right_full.as_str());
  let left_max = total.saturating_sub(right_w + 1);
  let left = truncate_to_width(&left_full, left_max);

  // Draw left and right in the same row using two aligned paragraphs
  let mut style = ratatui::style::Style::default().fg(ratatui::style::Color::Gray);
  if let Some(th) = app.config.ui.theme.as_ref() {
    if let Some(fg) = th.title_fg.as_ref().and_then(|s| crate::ui::colors::parse_color(s)) {
      style = style.fg(fg);
    }
    if let Some(bg) = th.title_bg.as_ref().and_then(|s| crate::ui::colors::parse_color(s)) {
      style = style.bg(bg);
    }
  }
  let left_p = Paragraph::new(left).alignment(Alignment::Left).style(style);
  let right_p = Paragraph::new(right_full).alignment(Alignment::Right).style(style);
  f.render_widget(left_p, area);
  f.render_widget(right_p, area);
}

#[cfg(unix)]
fn owner_string(path: &std::path::Path) -> String {
  use std::os::unix::fs::MetadataExt;
  if let Ok(meta) = std::fs::metadata(path) {
    let uid = meta.uid();
    let gid = meta.gid();
    let user = lookup_user_name(uid).unwrap_or_else(|| uid.to_string());
    let group = lookup_group_name(gid).unwrap_or_else(|| gid.to_string());
    format!("{}:{}", user, group)
  } else { String::from("-:-") }
}

#[cfg(not(unix))]
fn owner_string(_path: &std::path::Path) -> String { String::from("-") }

#[cfg(unix)]
static UID_CACHE: OnceLock<RwLock<HashMap<u32, String>>> = OnceLock::new();
#[cfg(unix)]
static GID_CACHE: OnceLock<RwLock<HashMap<u32, String>>> = OnceLock::new();

#[cfg(unix)]
fn uid_cache() -> &'static RwLock<HashMap<u32, String>> { UID_CACHE.get_or_init(|| RwLock::new(HashMap::new())) }
#[cfg(unix)]
fn gid_cache() -> &'static RwLock<HashMap<u32, String>> { GID_CACHE.get_or_init(|| RwLock::new(HashMap::new())) }

#[cfg(unix)]
fn lookup_user_name(uid: u32) -> Option<String> {
  // Fast path: check cache
  if let Ok(map) = uid_cache().read() { if let Some(v) = map.get(&uid) { return Some(v.clone()); } }
  // Parse /etc/passwd to resolve uid -> name
  let found = if let Ok(text) = std::fs::read_to_string("/etc/passwd") {
    text.lines().find_map(|line| {
      if line.trim().is_empty() || line.starts_with('#') { return None; }
      let mut parts = line.split(':');
      let name = parts.next()?;
      let _pw = parts.next();
      let uid_str = parts.next()?;
      if uid_str.parse::<u32>().ok()? == uid { Some(name.to_string()) } else { None }
    })
  } else { None };
  if let Some(ref name) = found { if let Ok(mut map) = uid_cache().write() { map.insert(uid, name.clone()); } }
  found
}

#[cfg(unix)]
fn lookup_group_name(gid: u32) -> Option<String> {
  if let Ok(map) = gid_cache().read() { if let Some(v) = map.get(&gid) { return Some(v.clone()); } }
  let found = if let Ok(text) = std::fs::read_to_string("/etc/group") {
    text.lines().find_map(|line| {
      if line.trim().is_empty() || line.starts_with('#') { return None; }
      let mut parts = line.split(':');
      let name = parts.next()?;
      let _pw = parts.next();
      let gid_str = parts.next()?;
      if gid_str.parse::<u32>().ok()? == gid { Some(name.to_string()) } else { None }
    })
  } else { None };
  if let Some(ref name) = found { if let Ok(mut map) = gid_cache().write() { map.insert(gid, name.clone()); } }
  found
}

#[cfg(unix)]
pub fn clear_owner_cache() {
  if let Some(lock) = UID_CACHE.get() { if let Ok(mut m) = lock.write() { m.clear(); } }
  if let Some(lock) = GID_CACHE.get() { if let Ok(mut m) = lock.write() { m.clear(); } }
}

#[cfg(not(unix))]
pub fn clear_owner_cache() {}

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
