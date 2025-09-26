use std::{
  path::Path,
  process::Command,
};

use ratatui::{
  layout::Rect,
  style::{
    Color,
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
    Paragraph,
    Wrap,
  },
};

use crate::ui::ansi::ansi_spans;

pub fn draw_preview_panel(
  f: &mut ratatui::Frame,
  area: Rect,
  app: &mut crate::App,
)
{
  // Clear area to prevent artifacts when content shrinks or lines are shorter
  f.render_widget(Clear, area);
  // Try dynamic preview via Lua previewer with simple caching to avoid
  // re-running when unchanged
  let mut dynamic_lines: Option<Vec<String>> = None;
  if let Some(sel) = app.selected_entry()
  {
    if !sel.is_dir
    {
      let key = (sel.path.clone(), area.width, area.height);
      if app.preview.cache_key.as_ref() == Some(&key)
      {
        dynamic_lines = app.preview.cache_lines.clone();
      }
      else
      {
        dynamic_lines =
          run_previewer(app, &sel.path, area, app.config.ui.preview_lines);
        app.preview.cache_key = Some(key);
        app.preview.cache_lines = dynamic_lines.clone();
      }
    }
    else
    {
      app.preview.cache_key = None;
      app.preview.cache_lines = None;
    }
  }
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

  // If a directory is selected, draw entries using the same row format as other
  // panes
  let text: Vec<Line> = if let Some(sel) = app.selected_entry()
  {
    if sel.is_dir
    {
      let block_inner = block.inner(area);
      let inner_w = block_inner.width;
      let fmt = app.config.ui.row.clone().unwrap_or_default();
      let list = app.read_dir_sorted(&sel.path).unwrap_or_default();
      let limit = app.config.ui.preview_lines.min(list.len());
      list
        .into_iter()
        .take(limit)
        .map(|e| crate::ui::panes::build_row_line(app, &fmt, &e, inner_w))
        .collect()
    }
    else if let Some(lines) = dynamic_lines.as_ref()
    {
      if lines.is_empty()
      {
        vec![Line::from(Span::styled(
          "<no selection>",
          Style::default().fg(Color::DarkGray),
        ))]
      }
      else
      {
        lines.iter().map(|l| Line::from(ansi_spans(l))).collect()
      }
    }
    else if app.preview.static_lines.is_empty()
    {
      vec![Line::from(Span::styled(
        "<no selection>",
        Style::default().fg(Color::DarkGray),
      ))]
    }
    else
    {
      app
        .preview
        .static_lines
        .iter()
        .map(|l| Line::from(ansi_spans(l)))
        .collect()
    }
  }
  else if let Some(lines) = dynamic_lines.as_ref()
  {
    if lines.is_empty()
    {
      vec![Line::from(Span::styled(
        "<no selection>",
        Style::default().fg(Color::DarkGray),
      ))]
    }
    else
    {
      lines.iter().map(|l| Line::from(ansi_spans(l))).collect()
    }
  }
  else if app.preview.static_lines.is_empty()
  {
    vec![Line::from(Span::styled(
      "<no selection>",
      Style::default().fg(Color::DarkGray),
    ))]
  }
  else
  {
    app.preview.static_lines.iter().map(|l| Line::from(ansi_spans(l))).collect()
  };

  let mut para = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
  if let Some(th) = app.config.ui.theme.as_ref()
  {
    let mut st = Style::default();
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
    para = para.style(st);
  }
  f.render_widget(para, area);
}

fn run_previewer(
  app: &crate::App,
  path: &Path,
  area: Rect,
  limit: usize,
) -> Option<Vec<String>>
{
  // 1) Lua previewer function (if configured)
  if let Some(lua) = app.lua.as_ref()
    && let (engine, Some(key)) = (&lua.engine, lua.previewer.as_ref())
  {
    let lua = engine.lua();
    if let Ok(func) = lua.registry_value::<mlua::Function>(key)
    {
      let path_str = path.to_string_lossy().to_string();
      let dir_str = path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_string_lossy()
        .to_string();
      let ext =
        path.extension().and_then(|s| s.to_str()).unwrap_or("").to_string();
      let is_binary = file_is_binary(path);
      if let Ok(ctx) = lua.create_table()
      {
        let _ = ctx.set("path", path_str.clone());
        let _ = ctx.set("directory", dir_str.clone());
        let _ = ctx.set("extension", ext);
        let _ = ctx.set("is_binary", is_binary);
        let _ = ctx.set("height", area.height as i64);
        let _ = ctx.set("width", area.width as i64);
        let _ = ctx.set("preview_x", area.x as i64);
        let _ = ctx.set("preview_y", area.y as i64);
        if let Ok(Some(mut cmd)) = func.call::<Option<String>>(ctx)
        {
          let name_str = path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();
          let ext_str =
            path.extension().and_then(|s| s.to_str()).unwrap_or("").to_string();
          cmd = cmd.replace("{path}", &crate::util::shell_escape(&path_str));
          cmd =
            cmd.replace("{directory}", &crate::util::shell_escape(&dir_str));
          cmd = cmd.replace("{dir}", &crate::util::shell_escape(&dir_str));
          cmd = cmd.replace("{name}", &crate::util::shell_escape(&name_str));
          cmd =
            cmd.replace("{extension}", &crate::util::shell_escape(&ext_str));
          let w = area.width.saturating_sub(10);
          let h = area.height.saturating_sub(10);
          cmd = cmd.replace("{width}", &w.to_string());
          cmd = cmd.replace("{height}", &h.to_string());
          cmd = cmd.replace("{preview_x}", &area.x.to_string());
          cmd = cmd.replace("{preview_y}", &area.y.to_string());
          return run_previewer_command(
            &cmd, &dir_str, &path_str, &name_str, limit,
          );
        }
      }
    }
  }

  // No legacy previewer rules; return None to fall back to default head preview
  None
}

fn run_previewer_command(
  cmd: &str,
  dir_str: &str,
  path_str: &str,
  name_str: &str,
  limit: usize,
) -> Option<Vec<String>>
{
  crate::trace::log(format!(
    "[preview] launching shell='{}' cmd='{}' cwd='{}' file='{}'",
    if cfg!(windows) { "cmd" } else { "sh" },
    cmd,
    dir_str,
    path_str
  ));

  #[cfg(windows)]
  let mut command = {
    let mut c = Command::new("cmd");
    c.arg("/C").arg(cmd);
    c
  };

  #[cfg(not(windows))]
  let mut command = {
    let mut c = Command::new("sh");
    c.arg("-lc").arg(cmd);
    c
  };

  match command
    .current_dir(dir_str)
    .env("LSV_PATH", path_str)
    .env("LSV_DIR", dir_str)
    .env("LSV_NAME", name_str)
    .env("FORCE_COLOR", "1")
    .env("CLICOLOR_FORCE", "1")
    .output()
  {
    Ok(out) =>
    {
      let mut buf = Vec::new();
      buf.extend_from_slice(&out.stdout);
      if !out.stderr.is_empty()
      {
        buf.push(b'\n');
        buf.extend_from_slice(&out.stderr);
      }
      let text = String::from_utf8_lossy(&buf).replace('\r', "");
      crate::trace::log(format!(
        "[preview] exit_code={:?} success={} bytes_out={}",
        out.status.code(),
        out.status.success(),
        text.len()
      ));
      if !out.status.success()
      {
        crate::trace::log(format!(
          "[preview] non-zero status running '{}'",
          cmd
        ));
      }
      let mut lines: Vec<String> = Vec::new();
      for l in text.lines()
      {
        lines.push(l.to_string());
        if lines.len() >= limit
        {
          break;
        }
      }
      Some(lines)
    }
    Err(e) =>
    {
      crate::trace::log(format!(
        "[preview] error spawning via {}: {}",
        if cfg!(windows) { "cmd" } else { "sh" },
        e
      ));
      #[cfg(windows)]
      {
        crate::trace::log(
          "[preview] hint: ensure the command is available in cmd.exe or \
           adjust your previewer to use Windows-compatible tooling."
            .to_string(),
        );
      }
      None
    }
  }
}

fn file_is_binary(path: &Path) -> bool
{
  if let Ok(mut f) = std::fs::File::open(path)
  {
    let mut buf = [0u8; 4096];
    if let Ok(n) = std::io::Read::read(&mut f, &mut buf)
    {
      let slice = &buf[..n];
      if slice.contains(&0)
      {
        return true;
      }
      if std::str::from_utf8(slice).is_err()
      {
        return true;
      }
    }
  }
  false
}
