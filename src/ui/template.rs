use crate::app::App;
use ratatui::{
  style::{Modifier, Style},
  text::Span,
};

#[derive(Clone, Default)]
pub struct HeaderSide
{
  pub text:  String,
  pub spans: Vec<Span<'static>>,
}

/// Render a header side using the configured template and runtime context.
/// Unknown placeholders are logged via trace for troubleshooting.
pub fn format_header_side(
  app: &App,
  tpl_opt: Option<&String>,
) -> HeaderSide
{
  // Extract placeholder names like {foo}
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
  let cwd_s = app.get_cwd_path().display().to_string();
  let sel_opt = app.selected_entry();
  let current_file = sel_opt
    .as_ref()
    .map(|e| e.path.display().to_string())
    .unwrap_or_else(|| cwd_s.clone());
  let current_file_dir = sel_opt
    .as_ref()
    .map(|e| e.path.parent().unwrap_or(app.get_cwd_path().as_path()).display().to_string())
    .unwrap_or_else(|| cwd_s.clone());
  let owner = sel_opt
    .as_ref()
    .map(|e| super::owner_string(&e.path))
    .unwrap_or_else(|| String::from("-"));
  let perms = sel_opt
    .as_ref()
    .map(|e| super::panes::permissions_string(e))
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
        match app.get_display_mode()
        {
          crate::app::DisplayMode::Friendly => super::panes::human_size(e.size),
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
  let name_now = sel_opt
    .as_ref()
    .and_then(|e| e.path.file_name().and_then(|s| s.to_str()).map(|s| s.to_string()))
    .unwrap_or_default();
  let date_fmt_binding = app.get_date_format();
  let date_fmt = date_fmt_binding.as_deref().unwrap_or("%Y-%m-%d %H:%M");
  let ctime_s = sel_opt
    .as_ref()
    .and_then(|e| e.ctime)
    .map(|t| super::panes::format_time_abs(t, date_fmt))
    .unwrap_or_else(|| String::from("-"));
  let mtime_s = sel_opt
    .as_ref()
    .and_then(|e| e.mtime)
    .map(|t| super::panes::format_time_abs(t, date_fmt))
    .unwrap_or_else(|| String::from("-"));

  let tpl = tpl_opt.cloned().unwrap_or_default();

  let allowed = [
    "date",
    "time",
    "cwd",
    "current_file",
    "current_file_dir",
    "current_file_name",
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
    if !allowed.contains(&ph.as_str())
    {
      crate::trace::log(format!("[header] unknown placeholder '{{{}}}'", ph));
    }
  }

  // Helper to resolve placeholder value
  let value_for = |name: &str| -> String {
    match name
    {
      "date" => date_s.clone(),
      "time" => time_s.clone(),
      "cwd" => cwd_s.clone(),
      "current_file" => current_file.clone(),
      "current_file_dir" => current_file_dir.clone(),
      "current_file_name" => name_now.clone(),
      "username" => username.clone(),
      "hostname" => hostname.clone(),
      "current_file_permissions" => perms.clone(),
      "current_file_size" => size_s.clone(),
      "current_file_ctime" => ctime_s.clone(),
      "current_file_mtime" => mtime_s.clone(),
      "current_file_extension" => ext.clone(),
      "owner" => owner.clone(),
      _ => String::new(),
    }
  };

  // Parse a modifier string like "fg=red;bg=black;style=italic/bold"
  fn style_from_mods(mods: &str) -> Style
  {
    let mut st = Style::default();
    for part in mods.split(';')
    {
      let mut it = part.splitn(2, '=');
      let key = it.next().unwrap_or("").trim().to_ascii_lowercase();
      let val = it.next().unwrap_or("").trim();
      if key.is_empty() || val.is_empty()
      {
        continue;
      }
      match key.as_str()
      {
        "fg" =>
        {
          if let Some(c) = crate::ui::colors::parse_color(val)
          {
            st = st.fg(c);
          }
        }
        "bg" =>
        {
          if let Some(c) = crate::ui::colors::parse_color(val)
          {
            st = st.bg(c);
          }
        }
        "style" =>
        {
          for tok in val.split(&['/', ','][..])
          {
            match tok.trim().to_ascii_lowercase().as_str()
            {
              "bold" => st = st.add_modifier(Modifier::BOLD),
              "italic" => st = st.add_modifier(Modifier::ITALIC),
              "underline" | "underlined" =>
              {
                st = st.add_modifier(Modifier::UNDERLINED)
              }
              _ => {}
            }
          }
        }
        _ => {}
      }
    }
    st
  }

  // Walk template and build plain text + styled spans
  let mut out = HeaderSide::default();
  let bytes = tpl.as_bytes();
  let mut i = 0usize;
  let mut seg_start = 0usize;
  while i < bytes.len()
  {
    if bytes[i] == b'{' && tpl[i + 1..].contains('}')
    {
      // flush previous plain segment
      if seg_start < i && let Some(seg) = tpl.get(seg_start..i)
      {
        out.text.push_str(seg);
        out.spans.push(Span::raw(seg.to_string()));
      }
      // find end
      if let Some(rel) = tpl[i + 1..].find('}')
      {
        let end = i + 1 + rel + 1;
        let token = &tpl[i + 1..end - 1];
        let (name, mods) = match token.split_once('|')
        {
          Some((n, m)) => (n.trim(), Some(m.trim())),
          None => (token.trim(), None),
        };
        let allowed = [
          "date",
          "time",
          "cwd",
          "current_file",
          "current_file_dir",
          "current_file_name",
          "username",
          "hostname",
          "current_file_permissions",
          "current_file_size",
          "current_file_ctime",
          "current_file_mtime",
          "current_file_extension",
          "owner",
        ];
        if allowed.contains(&name)
        {
          let val = value_for(name);
          out.text.push_str(&val);
          let mut span = Span::raw(val);
          if let Some(m) = mods
          {
            let st = style_from_mods(m);
            span = Span::styled(span.content.clone().into_owned(), st);
          }
          out.spans.push(span);
        }
        else
        {
          crate::trace::log(format!("[header] unknown placeholder '{{{}}}'", token));
          // pass through literally
          let lit = format!("{{{}}}", token);
          out.text.push_str(&lit);
          out.spans.push(Span::raw(lit));
        }
        i = end;
        seg_start = i;
        continue;
      }
    }
    let ch = tpl[i..].chars().next().unwrap();
    i += ch.len_utf8();
  }
  if seg_start < tpl.len()
    && let Some(seg) = tpl.get(seg_start..)
  {
    out.text.push_str(seg);
    out.spans.push(Span::raw(seg.to_string()));
  }
  out
}
