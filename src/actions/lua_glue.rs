//! Bridges Lua actions with the Rust application state.
//!
//! Provides helper wiring so Lua callbacks can mutate configuration, trigger
//! side-effects, and receive contextual data while running inside the embedded
//! Lua VM.

use std::{
  io,
  io::{
    Write,
    stdin,
    stdout,
  },
  time::Instant,
};

use crossterm::{
  execute,
  terminal::{
    EnterAlternateScreen,
    LeaveAlternateScreen,
    disable_raw_mode,
    enable_raw_mode,
  },
};
use mlua::{
  Lua,
  Table,
  Value,
};

use super::effects::{
  ActionEffects,
  parse_effects_from_lua,
};
use crate::{
  app::App,
  trace,
};

/// Execute the Lua action identified by `idx` against the provided app.
///
/// Returns the lightweight [`ActionEffects`](super::effects::ActionEffects)
/// produced during the call plus an optional configuration overlay to merge
/// back into the runtime configuration.
pub fn call_lua_action(
  app: &mut App,
  idx: usize,
) -> io::Result<(ActionEffects, Option<crate::config_data::ConfigData>)>
{
  let (engine, funcs) = match app.lua.as_ref()
  {
    Some(lua) => (&lua.engine, &lua.actions),
    None => return Ok((ActionEffects::default(), None)),
  };
  if idx >= funcs.len()
  {
    return Ok((ActionEffects::default(), None));
  }

  let lua = engine.lua();
  let func = lua
    .registry_value::<mlua::Function>(&funcs[idx])
    .map_err(|e| io::Error::other(format!("lua fn lookup: {e}")))?;

  // Build config snapshot (mutable by Lua)
  let cfg_tbl = crate::config_data::to_lua_config_table(lua, app)
    .map_err(|e| io::Error::other(format!("build config tbl: {e}")))?;

  // Build lsv helpers table
  let lsv_tbl = build_lsv_helpers(lua, &cfg_tbl, app)?;

  trace::log(format!("[lua] calling action idx={}...", idx));
  let started = Instant::now();
  let ret_val: Value = func.call((lsv_tbl, cfg_tbl.clone())).map_err(|e| {
    trace::log(format!("[lua] action idx={} error: {}", idx, e));
    io::Error::other(format!("lua fn: {e}"))
  })?;
  trace::log(format!(
    "[lua] action idx={} ok in {}ms",
    idx,
    started.elapsed().as_millis()
  ));

  // Prefer merging any returned partial table into the full snapshot to
  // avoid losing required fields expected by the validator. This makes
  // it safe for actions to return only the fields they changed.
  let candidate_tbl = match ret_val
  {
    Value::Table(t) => merge_tables(lua, &cfg_tbl, &t)
      .map_err(|e| io::Error::other(format!("merge: {}", e)))?,
    _ => cfg_tbl,
  };

  // Parse lightweight effects first
  let mut fx = parse_effects_from_lua(&candidate_tbl);
  // Fallback: if output not parsed but keys exist, populate
  if fx.output.is_none()
  {
    if let Ok(text) = candidate_tbl.get::<mlua::String>("output_text")
    {
      let text_s = match text.to_str() { Ok(v) => v.to_string(), Err(_) => String::new() };
      let title_s = match candidate_tbl.get::<mlua::String>("output_title")
      {
        Ok(s) => match s.to_str() { Ok(v) => v.to_string(), Err(_) => String::from("Output") },
        Err(_) => String::from("Output"),
      };
      fx.output = Some((title_s, text_s));
    }
  }

  // Optionally parse a full Config overlay (ui changes, etc.)
  let overlay = crate::config_data::from_lua_config_table(candidate_tbl).ok();
  Ok((fx, overlay))
}

fn build_lsv_helpers(
  lua: &Lua,
  cfg_tbl: &Table,
  app: &App,
) -> io::Result<Table>
{
  let tbl = lua.create_table().map_err(|e| io::Error::other(e.to_string()))?;

  // select_item(index)
  let cfg_ref = cfg_tbl.clone();
  let select_item_fn = lua
    .create_function(move |lua, idx: i64| {
      let ctx: Table = match cfg_ref.get("context")
      {
        Ok(t) => t,
        Err(_) =>
        {
          let t = lua.create_table()?;
          cfg_ref.set("context", t.clone())?;
          t
        }
      };
      let i = if idx < 0 { 0 } else { idx as u64 };
      ctx.set("selected_index", i)?;
      Ok(true)
    })
    .map_err(|e| io::Error::other(e.to_string()))?;
  tbl
    .set("select_item", select_item_fn)
    .map_err(|e| io::Error::other(e.to_string()))?;

  // select_last_item()
  let cfg_ref2 = cfg_tbl.clone();
  let select_last_fn = lua
    .create_function(move |_, ()| {
      if let Ok(ctx) = cfg_ref2.get::<Table>("context")
      {
        let len = ctx.get::<u64>("current_len").unwrap_or(0);
        if len > 0
        {
          ctx.set("selected_index", len - 1)?;
        }
      }
      Ok(true)
    })
    .map_err(|e| io::Error::other(e.to_string()))?;
  tbl
    .set("select_last_item", select_last_fn)
    .map_err(|e| io::Error::other(e.to_string()))?;

  // quit()
  let cfg_ref3 = cfg_tbl.clone();
  let quit_fn = lua
    .create_function(move |_, ()| {
      cfg_ref3.set("quit", true)?;
      Ok(true)
    })
    .map_err(|e| io::Error::other(e.to_string()))?;
  tbl.set("quit", quit_fn).map_err(|e| io::Error::other(e.to_string()))?;

  // prompt(message, default?)
  let prompt_fn = lua
    .create_function(move |_, (msg, def): (String, Option<String>)| {
      let mut out = stdout();
      write!(out, "{}", msg)?;
      out.flush()?;
      let mut input = String::new();
      stdin().read_line(&mut input)?;
      let input = input.trim_end().to_string();
      Ok(if input.is_empty() { def.unwrap_or_default() } else { input })
    })
    .map_err(|e| io::Error::other(e.to_string()))?;
  tbl.set("prompt", prompt_fn).map_err(|e| io::Error::other(e.to_string()))?;

  // display_output(text, title?)
  let cfg_ref4 = cfg_tbl.clone();
  let display_output_fn = lua
    .create_function(move |_, (text, title): (String, Option<String>)| {
      cfg_ref4.set("output_text", text)?;
      if let Some(t) = title
      {
        cfg_ref4.set("output_title", t)?;
      }
      Ok(true)
    })
    .map_err(|e| io::Error::other(e.to_string()))?;
  tbl
    .set("display_output", display_output_fn)
    .map_err(|e| io::Error::other(e.to_string()))?;

  let cfg_ref6 = cfg_tbl.clone();
  let open_theme_picker_fn = lua
    .create_function(move |_, ()| {
      cfg_ref6.set("theme_picker", "open")?;
      Ok(true)
    })
    .map_err(|e| io::Error::other(e.to_string()))?;
  tbl
    .set("open_theme_picker", open_theme_picker_fn)
    .map_err(|e| io::Error::other(e.to_string()))?;

  // delete_selected(): request delete confirmation (respects ui.confirm_delete)
  let cfg_ref_del = cfg_tbl.clone();
  let delete_selected_fn = lua
    .create_function(move |_, ()| {
      let _ = cfg_ref_del.set("confirm", "delete");
      Ok(true)
    })
    .map_err(|e| io::Error::other(e.to_string()))?;
  tbl
    .set("delete_selected", delete_selected_fn)
    .map_err(|e| io::Error::other(e.to_string()))?;

  // add_entry(): open add-entry prompt overlay
  let cfg_ref_add = cfg_tbl.clone();
  let add_entry_fn = lua
    .create_function(move |_, ()| {
      let _ = cfg_ref_add.set("prompt", "add_entry");
      Ok(true)
    })
    .map_err(|e| io::Error::other(e.to_string()))?;
  tbl
    .set("add_entry", add_entry_fn)
    .map_err(|e| io::Error::other(e.to_string()))?;

  // show_messages(): toggle messages overlay
  let cfg_ref_msg = cfg_tbl.clone();
  let show_messages_fn = lua
    .create_function(move |_, ()| {
      let _ = cfg_ref_msg.set("messages", "toggle");
      Ok(true)
    })
    .map_err(|e| io::Error::other(e.to_string()))?;
  tbl
    .set("show_messages", show_messages_fn)
    .map_err(|e| io::Error::other(e.to_string()))?;

  // rename_item(): open rename prompt
  let cfg_ref_ren = cfg_tbl.clone();
  let rename_item_fn = lua
    .create_function(move |_, ()| {
      let _ = cfg_ref_ren.set("prompt", "rename_entry");
      Ok(true)
    })
    .map_err(|e| io::Error::other(e.to_string()))?;
  tbl
    .set("rename_item", rename_item_fn)
    .map_err(|e| io::Error::other(e.to_string()))?;

  // Note: we only add unknown-function guard on config-time lsv to avoid
  // interfering with action helpers and tests here.

  // os_run (captured)
  let cfg_ref5 = cfg_tbl.clone();
  let cwd_str = app.cwd.to_string_lossy().to_string();
  let sel_path = app
    .selected_entry()
    .map(|e| e.path.clone())
    .unwrap_or_else(|| app.cwd.clone());
  let sel_dir = sel_path.parent().unwrap_or(&app.cwd).to_path_buf();
  let path_str = sel_path.to_string_lossy().to_string();
  let dir_str = sel_dir.to_string_lossy().to_string();
  let name_str = sel_path
    .file_name()
    .map(|s| s.to_string_lossy().to_string())
    .unwrap_or_default();

  let cwd_capture = cwd_str.clone();
  let path_capture = path_str.clone();
  let dir_capture = dir_str.clone();
  let name_capture = name_str.clone();

  let os_run_fn = lua
    .create_function(move |lua, cmd: String| {
      let rendered =
        render_cmd(&cmd, &path_capture, &dir_capture, &name_capture);
      trace::log(format!(
        "[os_run] cwd='{}' cmd='{}' rendered='{}' LSV_PATH='{}' LSV_DIR='{}' \
         LSV_NAME='{}'",
        cwd_capture, cmd, rendered, path_capture, dir_capture, name_capture
      ));
      let out = std::process::Command::new("sh")
        .arg("-lc")
        .arg(&cmd)
        .current_dir(&cwd_capture)
        .env("LSV_PATH", &path_capture)
        .env("LSV_DIR", &dir_capture)
        .env("LSV_NAME", &name_capture)
        .output();
      match out
      {
        Ok(output) =>
        {
          let mut buf = Vec::new();
          buf.extend_from_slice(&output.stdout);
          if !output.stderr.is_empty()
          {
            buf.push(b'\n');
            buf.extend_from_slice(&output.stderr);
          }
          let text = String::from_utf8_lossy(&buf).to_string();
          let title = format!("$ {}", cmd);
          let _ = cfg_ref5.set("output_text", text.clone());
          let _ = cfg_ref5.set("output_title", title.clone());
          trace::log(format!(
            "[os_run] exit={:?} bytes_out={}",
            output.status.code(),
            buf.len()
          ));
          let ret = lua.create_table()?;
          ret.set("output_text", text)?;
          ret.set("output_title", title)?;
          Ok(mlua::Value::Table(ret))
        }
        Err(e) =>
        {
          trace::log(format!("[os_run] error: {}", e));
          let text = format!("<error: {}>", e);
          let title = format!("$ {}", cmd);
          let _ = cfg_ref5.set("output_text", text.clone());
          let _ = cfg_ref5.set("output_title", title.clone());
          let ret = lua.create_table()?;
          ret.set("output_text", text)?;
          ret.set("output_title", title)?;
          Ok(mlua::Value::Table(ret))
        }
      }
    })
    .map_err(|e| io::Error::other(e.to_string()))?;
  tbl.set("os_run", os_run_fn).map_err(|e| io::Error::other(e.to_string()))?;

  // os_run_interactive
  let cfg_ref_i = cfg_tbl.clone();
  let cwd_str_i = cwd_str.clone();
  let path_str_i = path_str.clone();
  let dir_str_i = dir_str.clone();
  let name_str_i = name_str.clone();
  let os_run_interactive_fn = lua
    .create_function(move |_, cmd: String| {
      let rendered = render_cmd(&cmd, &path_str_i, &dir_str_i, &name_str_i);
      trace::log(format!(
        "[os_run_interactive] cwd='{}' cmd='{}' rendered='{}' LSV_PATH='{}' \
         LSV_DIR='{}' LSV_NAME='{}'",
        cwd_str_i, cmd, rendered, path_str_i, dir_str_i, name_str_i
      ));
      let _ = disable_raw_mode();
      let mut out = std::io::stdout();
      let _ = execute!(out, LeaveAlternateScreen);
      let status = std::process::Command::new("sh")
        .arg("-lc")
        .arg(&cmd)
        .current_dir(&cwd_str_i)
        .env("LSV_PATH", &path_str_i)
        .env("LSV_DIR", &dir_str_i)
        .env("LSV_NAME", &name_str_i)
        .status();
      let _ = enable_raw_mode();
      let mut out2 = std::io::stdout();
      let _ = execute!(out2, EnterAlternateScreen);
      let _ = cfg_ref_i.set("redraw", true);
      match status
      {
        Ok(st) =>
        {
          if !st.success()
          {
            let code = st.code().unwrap_or(-1);
            let _ = cfg_ref_i.set(
              "output_text",
              format!("<interactive exit {}> $ {}", code, cmd),
            );
            let _ = cfg_ref_i.set("output_title", String::from("Output"));
          }
        }
        Err(e) =>
        {
          let _ = cfg_ref_i.set(
            "output_text",
            format!("<interactive error: {}> $ {}", e, cmd),
          );
          let _ = cfg_ref_i.set("output_title", String::from("Output"));
        }
      }
      Ok(true)
    })
    .map_err(|e| io::Error::other(e.to_string()))?;
  tbl
    .set("os_run_interactive", os_run_interactive_fn)
    .map_err(|e| io::Error::other(e.to_string()))?;

  Ok(tbl)
}

fn render_cmd(
  cmd: &str,
  path: &str,
  dir: &str,
  name: &str,
) -> String
{
  // Best-effort env substitution for common patterns without running a shell
  let mut s = cmd.to_string();
  // ${VAR} first
  s = s
    .replace("${LSV_PATH}", path)
    .replace("${LSV_DIR}", dir)
    .replace("${LSV_NAME}", name);
  // Then $VAR
  s = s
    .replace("$LSV_PATH", path)
    .replace("$LSV_DIR", dir)
    .replace("$LSV_NAME", name);
  s
}

fn merge_tables(
  lua: &Lua,
  base: &Table,
  overlay: &Table,
) -> mlua::Result<Table>
{
  let out = lua.create_table()?;
  // copy base first
  for pair in base.clone().pairs::<Value, Value>()
  {
    let (k, v) = pair?;
    out.set(k, v)?;
  }
  // overlay keys
  for pair in overlay.clone().pairs::<Value, Value>()
  {
    let (k, v) = pair?;
    match (&k, &v)
    {
      (Value::String(ks), Value::Table(ot)) =>
      {
        // recursive merge for nested tables when base has a table
        if let Ok(bt) = out.get::<Table>(ks.as_bytes())
        {
          let merged = merge_tables(lua, &bt, ot)?;
          out.set(ks.as_bytes(), merged)?;
        }
        else
        {
          out.set(ks.as_bytes(), v)?;
        }
      }
      _ =>
      {
        out.set(k, v)?;
      }
    }
  }
  Ok(out)
}
