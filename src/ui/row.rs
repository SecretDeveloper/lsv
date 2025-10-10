use ratatui::{
  style::{
    Color,
    Style,
  },
  text::{
    Line,
    Span,
  },
};
use unicode_width::UnicodeWidthStr;

pub fn build_row_line(
  app: &crate::App,
  _fmt: &crate::config::UiRowFormat,
  e: &crate::app::DirEntryInfo,
  inner_width: u16,
) -> Line<'static>
{
  let base_style = entry_style(app, e);
  let mut spans: Vec<Span> = Vec::new();

  let mut bar_style = Style::default().fg(Color::Cyan);
  if let Some(th) = app.config.ui.theme.as_ref()
  {
    if let Some(fg) = th
      .selection_bar_fg
      .as_ref()
      .and_then(|s| crate::ui::colors::parse_color(s))
    {
      bar_style = bar_style.fg(fg);
    }
    else if let Some(fg) =
      th.border_fg.as_ref().and_then(|s| crate::ui::colors::parse_color(s))
    {
      bar_style = bar_style.fg(fg);
    }
  }

  let marker = if e.is_dir { "/" } else { "" };
  let name_val = format!("{}{}", e.name, marker);
  let icon_val = compute_icon(app, e);
  let info_val = format_info(app, e).unwrap_or_default();

  let mut sel_style = bar_style;
  if let Some(cb) = app.clipboard.as_ref()
    && cb.items.iter().any(|p| p == &e.path)
    && let Some(th) = app.config.ui.theme.as_ref()
  {
    match cb.op
    {
      crate::app::ClipboardOp::Copy =>
      {
        if let Some(fg) = th
          .selection_bar_copy_fg
          .as_ref()
          .and_then(|s| crate::ui::colors::parse_color(s))
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
        if let Some(fg) = th
          .selection_bar_move_fg
          .as_ref()
          .and_then(|s| crate::ui::colors::parse_color(s))
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

  let sel = app.selected.contains(&e.path);
  let indicator = if sel { "┃" } else { " " };
  spans.push(Span::styled(indicator.to_string(), sel_style));
  spans.push(Span::raw(" "));

  let mut left_txt = String::new();
  if !icon_val.is_empty()
  {
    left_txt.push_str(&icon_val);
    left_txt.push(' ');
  }
  left_txt.push_str(&name_val);

  let right_txt = info_val;
  let tw = inner_width as usize;
  let left_fixed = 2usize;
  let total_w = tw.saturating_sub(2);

  let mut rendered_left_w = left_fixed;

  let mut left_rest = left_txt;
  let right_w = UnicodeWidthStr::width(right_txt.as_str());
  let left_allowed = total_w.saturating_sub(right_w);

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
  app: &crate::App,
  e: &crate::app::DirEntryInfo,
) -> String
{
  let ic = &app.config.icons;
  if !ic.enabled
  {
    return String::new();
  }
  if e.is_dir
  {
    let name_lc = e.name.to_lowercase();
    if let Some(sym) = ic.folders.get(&name_lc)
    {
      return sym.clone();
    }
    return ic.default_dir.clone().unwrap_or_else(|| "📁".to_string());
  }
  let ext = e
    .path
    .extension()
    .and_then(|s| s.to_str())
    .map(|s| s.to_lowercase())
    .unwrap_or_default();
  if !ext.is_empty()
    && let Some(sym) = ic.extensions.get(&ext)
  {
    return sym.clone();
  }
  ic.default_file.clone().unwrap_or_else(|| "📄".to_string())
}

fn truncate_with_tilde(
  s: &str,
  max_w: usize,
) -> String
{
  if max_w == 0
  {
    return String::new();
  }
  let w = UnicodeWidthStr::width(s);
  if w <= max_w
  {
    return s.to_string();
  }
  if max_w == 1
  {
    return "~".to_string();
  }
  let mut out = String::new();
  let mut used = 0usize;
  for ch in s.chars()
  {
    let cw = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
    if used + cw + 1 > max_w
    {
      break;
    }
    out.push(ch);
    used += cw;
  }
  out.push('~');
  out
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
pub fn permissions_string(e: &crate::app::DirEntryInfo) -> String
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
  s.push(if mode & 0o400 != 0 { 'r' } else { '-' });
  s.push(if mode & 0o200 != 0 { 'w' } else { '-' });
  s.push(match (mode & 0o100 != 0, mode & 0o4000 != 0)
  {
    (true, true) => 's',
    (false, true) => 'S',
    (true, false) => 'x',
    (false, false) => '-',
  });
  s.push(if mode & 0o040 != 0 { 'r' } else { '-' });
  s.push(if mode & 0o020 != 0 { 'w' } else { '-' });
  s.push(match (mode & 0o010 != 0, mode & 0o2000 != 0)
  {
    (true, true) => 's',
    (false, true) => 'S',
    (true, false) => 'x',
    (false, false) => '-',
  });
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

#[cfg(not(unix))]
pub fn permissions_string(_e: &crate::app::DirEntryInfo) -> String
{
  "---------".to_string()
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

#[cfg(not(unix))]
fn is_executable(_path: &std::path::Path) -> bool
{
  false
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
          crate::app::DisplayMode::Friendly =>
          {
            crate::ui::format::human_size(e.size)
          }
          crate::app::DisplayMode::Absolute => format!("{} B", e.size),
        })
      }
    }
    InfoMode::Created => match app.display_mode
    {
      crate::app::DisplayMode::Absolute =>
      {
        e.ctime.map(|t| crate::ui::format::format_time_abs(t, fmt))
      }
      crate::app::DisplayMode::Friendly =>
      {
        e.ctime.map(crate::ui::format::format_time_ago)
      }
    },
    InfoMode::Modified => match app.display_mode
    {
      crate::app::DisplayMode::Absolute =>
      {
        e.mtime.map(|t| crate::ui::format::format_time_abs(t, fmt))
      }
      crate::app::DisplayMode::Friendly =>
      {
        e.mtime.map(crate::ui::format::format_time_ago)
      }
    },
  }
}
