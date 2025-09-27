use crate::app::App;

/// Render a header side using the configured template and runtime context.
/// Unknown placeholders are logged via trace for troubleshooting.
pub fn format_header_side(
  app: &App,
  tpl_opt: Option<&String>,
) -> String
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
