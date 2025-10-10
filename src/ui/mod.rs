pub mod ansi;
pub mod colors;
pub mod format;
pub mod overlays;
pub mod panes;
pub mod preview;
pub mod row;
pub mod template;

use ratatui::{
  layout::{
    Alignment,
    Constraint,
    Direction,
    Layout,
    Rect,
  },
  widgets::Paragraph,
};
#[cfg(unix)]
use std::collections::HashMap;
#[cfg(unix)]
use std::sync::{
  OnceLock,
  RwLock,
};
use unicode_width::UnicodeWidthStr;

pub fn draw(
  f: &mut ratatui::Frame,
  app: &mut crate::App,
)
{
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
  crate::ui::preview::draw_preview_panel(f, chunks[2], app);

  // which-key overlay (draw last so it appears on top)
  match app.overlay
  {
    crate::app::Overlay::WhichKey { .. } =>
    {
      panes::draw_whichkey_panel(f, f.area(), app);
    }
    crate::app::Overlay::CommandPane(_) =>
    {
      panes::draw_command_pane(f, f.area(), app);
    }
    crate::app::Overlay::Messages =>
    {
      panes::draw_messages_panel(f, f.area(), app);
    }
    crate::app::Overlay::Output { .. } =>
    {
      panes::draw_output_panel(f, f.area(), app);
    }
    crate::app::Overlay::Prompt(_) =>
    {
      panes::draw_prompt_panel(f, f.area(), app);
    }
    crate::app::Overlay::Confirm(_) =>
    {
      panes::draw_confirm_panel(f, f.area(), app);
    }
    crate::app::Overlay::ThemePicker(_) =>
    {
      panes::draw_theme_picker_panel(f, f.area(), app);
    }
    crate::app::Overlay::None =>
    {}
  }
}

fn draw_header(
  f: &mut ratatui::Frame,
  area: Rect,
  app: &crate::App,
)
{
  // Paint background row based on explicit header_bg or theme title_bg
  if let Some(bg_s) =
    app.config.ui.header_bg.as_ref().or_else(|| {
      app.config.ui.theme.as_ref().and_then(|t| t.title_bg.as_ref())
    })
    && let Some(bg) = crate::ui::colors::parse_color(bg_s)
  {
    let blk = ratatui::widgets::Block::default()
      .style(ratatui::style::Style::default().bg(bg));
    f.render_widget(blk, area);
  }
  // helper removed; header rendering now lives in template::format_header_side
  let _unused = (); // retain function body start for patching
  #[allow(dead_code)]
  fn render_header_side(
    app: &crate::App,
    tpl_opt: Option<&String>,
  ) -> String
  {
    // Validate placeholders against allowed set; log unknowns
    fn placeholders_in(s: &str) -> Vec<String>
    {
      let mut out = Vec::new();
      let mut i = 0;
      let b = s.as_bytes();
      while i < b.len()
      {
        if b[i] == b'{'
          && let Some(j) = s[i + 1..].find('}')
        {
          let end = i + 1 + j + 1;
          let name = &s[i + 1..end - 1];
          if !name.is_empty()
          {
            out.push(name.to_string());
          }
          i = end;
          continue;
        }
        let ch = s[i..].chars().next().unwrap();
        i += ch.len_utf8();
      }
      out
    }

    use chrono::Local;
    let now = Local::now();
    let date_s = now.format("%Y-%m-%d").to_string();
    let time_s = now.format("%H:%M").to_string();
    let username = whoami::username();
    let hostname = whoami::fallible::hostname().unwrap_or_default();
    let cwd_s = app.cwd.display().to_string();
    let sel_opt = app.selected_entry();
    let current_file = sel_opt
      .as_ref()
      .map(|e| e.path.display().to_string())
      .unwrap_or_else(|| cwd_s.clone());
    let owner = sel_opt
      .as_ref()
      .map(|e| owner_string(&e.path))
      .unwrap_or_else(|| String::from("-"));
    let perms = sel_opt
      .as_ref()
      .map(|e| crate::ui::panes::permissions_string(e))
      .unwrap_or_else(|| String::from("---------"));
    let size_s = sel_opt
      .as_ref()
      .map(|e| {
        if e.is_dir
        {
          "-".to_string()
        }
        else
        {
          match app.display_mode
          {
            crate::app::DisplayMode::Friendly =>
            {
              crate::ui::panes::human_size(e.size)
            }
            crate::app::DisplayMode::Absolute => format!("{} B", e.size),
          }
        }
      })
      .unwrap_or_else(|| String::from("-"));
    let ext = sel_opt
      .as_ref()
      .and_then(|e| {
        e.path.extension().and_then(|s| s.to_str()).map(|s| s.to_string())
      })
      .unwrap_or_default();
    let ctime_s = sel_opt
      .as_ref()
      .and_then(|e| e.ctime)
      .map(|t| {
        let fmt =
          app.config.ui.date_format.as_deref().unwrap_or("%Y-%m-%d %H:%M");
        crate::ui::panes::format_time_abs(t, fmt)
      })
      .unwrap_or_else(|| String::from("-"));
    let mtime_s = sel_opt
      .as_ref()
      .and_then(|e| e.mtime)
      .map(|t| {
        let fmt =
          app.config.ui.date_format.as_deref().unwrap_or("%Y-%m-%d %H:%M");
        crate::ui::panes::format_time_abs(t, fmt)
      })
      .unwrap_or_else(|| String::from("-"));

    let tpl = tpl_opt.cloned().unwrap_or_default();

    // Allowed placeholders for header templates
    let allowed = [
      "date",
      "time",
      "cwd",
      "current_file",
      "username",
      "hostname",
      "current_file_permissions",
      "current_file_size",
      "current_file_ctime",
      "current_file_mtime",
      "current_file_extension",
      "owner",
    ];
    for ph in placeholders_in(&tpl)
    {
      if !allowed.iter().any(|&a| a == ph)
      {
        crate::trace::log(format!("[header] unknown placeholder '{{{}}}'", ph));
      }
    }
    tpl
      .replace("{date}", &date_s)
      .replace("{time}", &time_s)
      .replace("{cwd}", &cwd_s)
      .replace("{current_file}", &current_file)
      .replace("{username}", &username)
      .replace("{hostname}", &hostname)
      .replace("{current_file_permissions}", &perms)
      .replace("{current_file_size}", &size_s)
      .replace("{current_file_ctime}", &ctime_s)
      .replace("{current_file_mtime}", &mtime_s)
      .replace("{current_file_extension}", &ext)
      .replace("{owner}", &owner)
  }

  // Prefer user-configured templates; fall back to a sensible default
  let left_tpl = app
    .config
    .ui
    .header_left
    .as_ref()
    .cloned()
    .or_else(|| Some("{username}@{hostname}:{current_file}".to_string()));
  let right_tpl = app.config.ui.header_right.as_ref().cloned().or_else(|| {
    Some(
      "{current_file_size}  {owner}  {current_file_permissions}  \
       {current_file_ctime}"
        .to_string(),
    )
  });

  let left_side = template::format_header_side(app, left_tpl.as_ref());
  let right_side = template::format_header_side(app, right_tpl.as_ref());

  // Compute widths from plain text
  let total = area.width as usize;
  let right_w = UnicodeWidthStr::width(right_side.text.as_str());
  let left_max = total.saturating_sub(right_w + 1);

  // Truncate left spans to fit
  fn truncate_spans_to_width(
    spans: &[ratatui::text::Span<'_>],
    max_w: usize,
  ) -> Vec<ratatui::text::Span<'static>>
  {
    if max_w == 0
    {
      return Vec::new();
    }
    let mut out: Vec<ratatui::text::Span<'static>> = Vec::new();
    let mut used = 0usize;
    for sp in spans
    {
      let s = sp.content.as_ref();
      let mut acc = String::new();
      for ch in s.chars()
      {
        let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if used + cw > max_w
        {
          break;
        }
        used += cw;
        acc.push(ch);
      }
      if !acc.is_empty()
      {
        let st = sp.style;
        out.push(ratatui::text::Span::styled(acc, st));
      }
      if used >= max_w
      {
        break;
      }
    }
    out
  }

  let left_spans = truncate_spans_to_width(&left_side.spans, left_max);

  // Draw left and right in the same row using two aligned paragraphs
  // Apply default title fg/bg to spans only where not explicitly set
  let mut left_spans_final = left_spans;
  let mut right_spans_final: Vec<ratatui::text::Span<'static>> = right_side
    .spans
    .into_iter()
    .map(|s| ratatui::text::Span::styled(s.content.into_owned(), s.style))
    .collect();
  // Apply default fg/bg to spans where not explicitly set
  if let Some(th) = app.config.ui.theme.as_ref()
  {
    // Prefer explicit ui.header_fg if provided, else theme title_fg
    let fg_opt = app
      .config
      .ui
      .header_fg
      .as_ref()
      .and_then(|s| crate::ui::colors::parse_color(s))
      .or_else(|| {
        th.title_fg.as_ref().and_then(|s| crate::ui::colors::parse_color(s))
      });
    if let Some(fg) = fg_opt
    {
      for sp in &mut left_spans_final
      {
        if sp.style.fg.is_none()
        {
          sp.style = sp.style.fg(fg);
        }
      }
      for sp in &mut right_spans_final
      {
        if sp.style.fg.is_none()
        {
          sp.style = sp.style.fg(fg);
        }
      }
    }
    // Prefer explicit ui.header_bg if provided, else theme title_bg
    let bg_opt = app
      .config
      .ui
      .header_bg
      .as_ref()
      .and_then(|s| crate::ui::colors::parse_color(s))
      .or_else(|| {
        th.title_bg.as_ref().and_then(|s| crate::ui::colors::parse_color(s))
      });
    if let Some(bg) = bg_opt
    {
      for sp in &mut left_spans_final
      {
        if sp.style.bg.is_none()
        {
          sp.style = sp.style.bg(bg);
        }
      }
      for sp in &mut right_spans_final
      {
        if sp.style.bg.is_none()
        {
          sp.style = sp.style.bg(bg);
        }
      }
    }
  }

  let left_line = ratatui::text::Line::from(left_spans_final);
  let left_p = Paragraph::new(left_line).alignment(Alignment::Left);

  let right_line = ratatui::text::Line::from(right_spans_final);
  let right_p = Paragraph::new(right_line).alignment(Alignment::Right);
  f.render_widget(left_p, area);
  f.render_widget(right_p, area);
}

#[cfg(unix)]
fn owner_string(path: &std::path::Path) -> String
{
  use std::os::unix::fs::MetadataExt;
  if let Ok(meta) = std::fs::metadata(path)
  {
    let uid = meta.uid();
    let gid = meta.gid();
    let user = lookup_user_name(uid).unwrap_or_else(|| uid.to_string());
    let group = lookup_group_name(gid).unwrap_or_else(|| gid.to_string());
    format!("{}:{}", user, group)
  }
  else
  {
    String::from("-:-")
  }
}

#[cfg(not(unix))]
fn owner_string(_path: &std::path::Path) -> String
{
  String::from("-")
}

#[cfg(unix)]
static UID_CACHE: OnceLock<RwLock<HashMap<u32, String>>> = OnceLock::new();
#[cfg(unix)]
static GID_CACHE: OnceLock<RwLock<HashMap<u32, String>>> = OnceLock::new();

#[cfg(unix)]
fn uid_cache() -> &'static RwLock<HashMap<u32, String>>
{
  UID_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}
#[cfg(unix)]
fn gid_cache() -> &'static RwLock<HashMap<u32, String>>
{
  GID_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

#[cfg(unix)]
fn lookup_user_name(uid: u32) -> Option<String>
{
  // Fast path: check cache
  if let Ok(map) = uid_cache().read()
    && let Some(v) = map.get(&uid)
  {
    return Some(v.clone());
  }
  // Parse /etc/passwd to resolve uid -> name
  let found = if let Ok(text) = std::fs::read_to_string("/etc/passwd")
  {
    text.lines().find_map(|line| {
      if line.trim().is_empty() || line.starts_with('#')
      {
        return None;
      }
      let mut parts = line.split(':');
      let name = parts.next()?;
      let _pw = parts.next();
      let uid_str = parts.next()?;
      if uid_str.parse::<u32>().ok()? == uid
      {
        Some(name.to_string())
      }
      else
      {
        None
      }
    })
  }
  else
  {
    None
  }
  // Fallback: try `id -nu <uid>` on Unix systems where /etc/passwd is not
  // authoritative (e.g., macOS)
  .or_else(|| {
    use std::process::Command;
    let out = Command::new("id").arg("-nu").arg(uid.to_string()).output();
    match out
    {
      Ok(o) if o.status.success() =>
      {
        let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
        if s.is_empty() { None } else { Some(s) }
      }
      _ => None,
    }
  });
  if let Some(ref name) = found
    && let Ok(mut map) = uid_cache().write()
  {
    map.insert(uid, name.clone());
  }
  found
}

#[cfg(unix)]
fn lookup_group_name(gid: u32) -> Option<String>
{
  if let Ok(map) = gid_cache().read()
    && let Some(v) = map.get(&gid)
  {
    return Some(v.clone());
  }
  let found = if let Ok(text) = std::fs::read_to_string("/etc/group")
  {
    text.lines().find_map(|line| {
      if line.trim().is_empty() || line.starts_with('#')
      {
        return None;
      }
      let mut parts = line.split(':');
      let name = parts.next()?;
      let _pw = parts.next();
      let gid_str = parts.next()?;
      if gid_str.parse::<u32>().ok()? == gid
      {
        Some(name.to_string())
      }
      else
      {
        None
      }
    })
  }
  else
  {
    None
  }
  .or_else(|| {
    use std::process::Command;
    let out = Command::new("id").arg("-ng").arg(gid.to_string()).output();
    match out
    {
      Ok(o) if o.status.success() =>
      {
        let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
        if s.is_empty() { None } else { Some(s) }
      }
      _ => None,
    }
  });
  if let Some(ref name) = found
    && let Ok(mut map) = gid_cache().write()
  {
    map.insert(gid, name.clone());
  }
  found
}

#[cfg(unix)]
pub fn clear_owner_cache()
{
  if let Some(lock) = UID_CACHE.get()
    && let Ok(mut m) = lock.write()
  {
    m.clear();
  }
  if let Some(lock) = GID_CACHE.get()
    && let Ok(mut m) = lock.write()
  {
    m.clear();
  }
}

#[cfg(not(unix))]
pub fn clear_owner_cache() {}

// (unused)
