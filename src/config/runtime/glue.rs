//! Bridges Lua actions with the Rust application state.
//!
//! Provides helper wiring so Lua callbacks can mutate configuration, trigger
//! side-effects, and receive contextual data while running inside the embedded
//! Lua VM.

use std::{
  io,
  io::stdout,
  time::Instant,
};

use crossterm::terminal::{
  EnterAlternateScreen,
  LeaveAlternateScreen,
  disable_raw_mode,
  enable_raw_mode,
};
use mlua::{
  Lua,
  Table,
  Value,
};

use crate::{
  actions::effects::{
    ActionEffects,
    parse_effects_from_lua,
  },
  app::App,
  trace,
};

/// Execute the Lua action identified by `idx` against the provided app.
///
/// Returns the lightweight
/// [`ActionEffects`](crate::actions::effects::ActionEffects) produced during
/// the call plus an optional configuration overlay to merge back into the
/// runtime configuration.
pub fn call_lua_action(
  app: &mut App,
  idx: usize,
) -> io::Result<(ActionEffects, Option<crate::config::runtime::data::ConfigData>)>
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
  let cfg_tbl = crate::config::runtime::data::to_lua_config_table(lua, app)
    .map_err(|e| io::Error::other(format!("build config tbl: {e}")))?;
  let cfg_tbl_copy = cfg_tbl.clone();

  // Build lsv helpers table
  let lsv_tbl = build_lsv_helpers(lua, &cfg_tbl, app)?;

  trace::log(format!("[lua] calling action idx={}...", idx));
  let started = Instant::now();
  let ret_val: Value = func.call((lsv_tbl, cfg_tbl.clone())).map_err(|e| {
    let bt = std::backtrace::Backtrace::force_capture();
    trace::log(format!("[lua] action idx={} error: {}", idx, e));
    trace::log(format!("[lua] backtrace:\n{}", bt));
    io::Error::other(format!("lua fn: {e}"))
  })?;
  trace::log(format!(
    "[lua] action idx={} ok in {}ms",
    idx,
    started.elapsed().as_millis()
  ));

  // Prefer merging any returned partial table into the full snapshot
  let candidate_tbl = match ret_val
  {
    Value::Table(t) => merge_tables(lua, &cfg_tbl, &t)
      .map_err(|e| io::Error::other(format!("merge: {}", e)))?,
    _ => cfg_tbl,
  };

  // Parse lightweight effects first
  let mut fx = parse_effects_from_lua(&candidate_tbl);
  // Fallback: read from original cfg table if helper mutated it
  if fx.output.is_none()
    && let Ok(text) = cfg_tbl_copy.get::<String>("output_text")
  {
    let title = cfg_tbl_copy
      .get::<String>("output_title")
      .unwrap_or_else(|_| String::from("Output"));
    fx.output = Some((title, text));
  }

  // Optionally parse a full Config overlay (ui changes, etc.)
  let overlay =
    crate::config::runtime::data::from_lua_config_table(candidate_tbl).ok();
  Ok((fx, overlay))
}

fn build_lsv_helpers(
  lua: &Lua,
  cfg_tbl: &Table,
  app: &App,
) -> io::Result<Table>
{
  let tbl = lua.create_table().map_err(|e| io::Error::other(e.to_string()))?;

  // (UI helpers removed)
  // Selection and prompts
  build_selection_helpers(lua, &tbl, cfg_tbl)?;
  // Clipboard helpers
  build_clipboard_helpers(lua, &tbl, cfg_tbl)?;
  // Process helpers are inlined below

  // Selection snapshot helper: return selected file paths as a Lua array
  let selected_paths_snapshot: Vec<String> =
    app.selected.iter().map(|p| p.to_string_lossy().to_string()).collect();
  let get_selected_paths_fn = lua
    .create_function(move |lua, ()| {
      let t = lua.create_table()?;
      for (i, s) in selected_paths_snapshot.iter().enumerate()
      {
        t.set((i + 1) as i64, s.clone())?;
      }
      Ok(t)
    })
    .map_err(|e| io::Error::other(e.to_string()))?;
  tbl
    .set("get_selected_paths", get_selected_paths_fn)
    .map_err(|e| io::Error::other(e.to_string()))?;

  // lsv.quote(s)
  let quote_fn = lua
    .create_function(|_, s: String| {
      #[cfg(windows)]
      {
        Ok(format!("\"{}\"", s.replace('"', "\"\"")))
      }
      #[cfg(not(windows))]
      {
        Ok(format!("'{}'", s.replace('\'', "'\\''")))
      }
    })
    .map_err(|e| io::Error::other(e.to_string()))?;
  tbl.set("quote", quote_fn).map_err(|e| io::Error::other(e.to_string()))?;

  // lsv.get_os_name()
  let os_fn = lua
    .create_function(|_, ()| Ok(std::env::consts::OS.to_string()))
    .map_err(|e| io::Error::other(e.to_string()))?;
  tbl.set("get_os_name", os_fn).map_err(|e| io::Error::other(e.to_string()))?;

  // getenv(name, default?) -> string|nil
  let getenv_fn = lua
    .create_function(|_, (name, default): (String, Option<String>)| {
      Ok(std::env::var(&name).ok().or(default))
    })
    .map_err(|e| io::Error::other(e.to_string()))?;
  tbl.set("getenv", getenv_fn).map_err(|e| io::Error::other(e.to_string()))?;

  // force_redraw(): request a full rerender
  let cfg_ref_redraw = cfg_tbl.clone();
  let force_redraw_fn = lua
    .create_function(move |_, ()| {
      let _ = cfg_ref_redraw.set("redraw", true);
      Ok(true)
    })
    .map_err(|e| io::Error::other(e.to_string()))?;
  tbl
    .set("force_redraw", force_redraw_fn)
    .map_err(|e| io::Error::other(e.to_string()))?;

  // clear_messages(): clear the UI message list
  let cfg_ref_cmsg = cfg_tbl.clone();
  let clear_messages_fn = lua
    .create_function(move |_, ()| {
      let _ = cfg_ref_cmsg.set("clear_messages", true);
      Ok(true)
    })
    .map_err(|e| io::Error::other(e.to_string()))?;
  tbl
    .set("clear_messages", clear_messages_fn)
    .map_err(|e| io::Error::other(e.to_string()))?;

  // set_theme_by_name(name)
  let cfg_ref_settheme = cfg_tbl.clone();
  let set_theme_by_name_fn = lua
    .create_function(move |_, name: String| {
      let n = name.trim();
      if !n.is_empty()
      {
        let _ = cfg_ref_settheme.set("theme_set_name", n);
      }
      Ok(true)
    })
    .map_err(|e| io::Error::other(e.to_string()))?;
  tbl
    .set("set_theme_by_name", set_theme_by_name_fn)
    .map_err(|e| io::Error::other(e.to_string()))?;

  // show_message(text)
  let cfg_ref_msg = cfg_tbl.clone();
  let show_message_fn = lua
    .create_function(move |_, text: String| {
      let _ = cfg_ref_msg.set("message_text", text);
      Ok(true)
    })
    .map_err(|e| io::Error::other(e.to_string()))?;
  tbl
    .set("show_message", show_message_fn)
    .map_err(|e| io::Error::other(e.to_string()))?;

  // show_error(text)
  let cfg_ref_err = cfg_tbl.clone();
  let show_error_fn = lua
    .create_function(move |_, text: String| {
      let _ = cfg_ref_err.set("error_text", text);
      let _ = cfg_ref_err.set("messages", "show");
      Ok(true)
    })
    .map_err(|e| io::Error::other(e.to_string()))?;
  tbl
    .set("show_error", show_error_fn)
    .map_err(|e| io::Error::other(e.to_string()))?;

  // trace(text)
  let trace_fn = lua
    .create_function(|_, text: String| {
      crate::trace::log(text);
      Ok(true)
    })
    .map_err(|e| io::Error::other(e.to_string()))?;
  tbl.set("trace", trace_fn).map_err(|e| io::Error::other(e.to_string()))?;

  // quit(): set quit flag in effects
  let cfg_ref_quit = cfg_tbl.clone();
  let quit_fn = lua
    .create_function(move |_, ()| {
      let _ = cfg_ref_quit.set("quit", true);
      Ok(true)
    })
    .map_err(|e| io::Error::other(e.to_string()))?;
  tbl.set("quit", quit_fn).map_err(|e| io::Error::other(e.to_string()))?;

  // display_output(body, title?)
  let cfg_ref_out = cfg_tbl.clone();
  let display_output_fn = lua
    .create_function(move |_, (body, title_opt): (String, Option<String>)| {
      let _ = cfg_ref_out.set("output_text", body);
      if let Some(t) = title_opt
      {
        let _ = cfg_ref_out.set("output_title", t);
      }
      let _ = cfg_ref_out.set("output", "show");
      Ok(true)
    })
    .map_err(|e| io::Error::other(e.to_string()))?;
  tbl
    .set("display_output", display_output_fn)
    .map_err(|e| io::Error::other(e.to_string()))?;

  // Clipboard helpers
  let cfg_ref_cp = cfg_tbl.clone();
  let copy_selection_fn = lua
    .create_function(move |_, ()| {
      let _ = cfg_ref_cp.set("clipboard", "copy_arm");
      Ok(true)
    })
    .map_err(|e| io::Error::other(e.to_string()))?;
  tbl
    .set("copy_selection", copy_selection_fn)
    .map_err(|e| io::Error::other(e.to_string()))?;

  let cfg_ref_mv = cfg_tbl.clone();
  let move_selection_fn = lua
    .create_function(move |_, ()| {
      let _ = cfg_ref_mv.set("clipboard", "move_arm");
      Ok(true)
    })
    .map_err(|e| io::Error::other(e.to_string()))?;
  tbl
    .set("move_selection", move_selection_fn)
    .map_err(|e| io::Error::other(e.to_string()))?;

  let cfg_ref_ps = cfg_tbl.clone();
  let paste_clipboard_fn = lua
    .create_function(move |_, ()| {
      let _ = cfg_ref_ps.set("clipboard", "paste");
      Ok(true)
    })
    .map_err(|e| io::Error::other(e.to_string()))?;
  tbl
    .set("paste_clipboard", paste_clipboard_fn)
    .map_err(|e| io::Error::other(e.to_string()))?;

  let cfg_ref_cc = cfg_tbl.clone();
  let clear_clipboard_fn = lua
    .create_function(move |_, ()| {
      let _ = cfg_ref_cc.set("clipboard", "clear");
      Ok(true)
    })
    .map_err(|e| io::Error::other(e.to_string()))?;
  tbl
    .set("clear_clipboard", clear_clipboard_fn)
    .map_err(|e| io::Error::other(e.to_string()))?;

  // os_run (captured)
  let cfg_ref5 = cfg_tbl.clone();
  let cwd_str = app.cwd.to_string_lossy().to_string();
  let cwd_capture = cwd_str.clone();

  let os_run_fn = lua
    .create_function(move |_, cmd: String| {
      trace::log(format!("[os_run] cwd='{}' cmd='{}'", cwd_capture, cmd));
      #[cfg(windows)]
      let mut command = {
        let mut c = std::process::Command::new("cmd");
        c.arg("/C").arg(&cmd);
        c
      };
      #[cfg(not(windows))]
      let mut command = {
        let mut c = std::process::Command::new("sh");
        c.arg("-lc").arg(&cmd);
        c
      };
      let out = command.current_dir(&cwd_capture).output();
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
          let bytes = buf.len();
          let success = output.status.success();
          trace::log(format!(
            "[os_run] exit={:?} bytes_out={}",
            output.status.code(),
            bytes
          ));
          if bytes > 0 || !success
          {
            let text = String::from_utf8_lossy(&buf).to_string();
            let title = format!("$ {}", cmd);
            let _ = cfg_ref5.set("output_text", text);
            let _ = cfg_ref5.set("output_title", title);
          }
          else
          {
            let _ = cfg_ref5.set("message_text", format!("$ {}", cmd));
          }
          Ok(true)
        }
        Err(e) =>
        {
          trace::log(format!("[os_run] error: {}", e));
          let text = format!("<error: {}>", e);
          let title = format!("$ {}", cmd);
          let _ = cfg_ref5.set("output_text", text);
          let _ = cfg_ref5.set("output_title", title);
          Ok(true)
        }
      }
    })
    .map_err(|e| io::Error::other(e.to_string()))?;
  tbl.set("os_run", os_run_fn).map_err(|e| io::Error::other(e.to_string()))?;

  // os_run_interactive
  let cfg_ref_i = cfg_tbl.clone();
  let cwd_str_i = cwd_str.clone();
  let os_run_interactive_fn = lua
    .create_function(move |_, cmd: String| {
      let title = format!("$ {}", cmd);
      let _ = cfg_ref_i.set("output_title", title);
      #[cfg(windows)]
      let program = "cmd";
      #[cfg(windows)]
      let args: &[&str] = &["/C", &cmd];
      #[cfg(not(windows))]
      let program = "sh";
      #[cfg(not(windows))]
      let args: &[&str] = &["-lc", &cmd];
      // leave tui
      disable_raw_mode().ok();
      let _ = crossterm::execute!(stdout(), LeaveAlternateScreen);
      // run
      let status = std::process::Command::new(program)
        .args(args)
        .current_dir(&cwd_str_i)
        .status();
      // re-enter tui
      enable_raw_mode().ok();
      let _ = crossterm::execute!(stdout(), EnterAlternateScreen);
      let text = match status
      {
        Ok(s) => format!("exit status: {:?}", s.code()),
        Err(e) => format!("<error: {}>", e),
      };
      let _ = cfg_ref_i.set("output_text", text);
      Ok(true)
    })
    .map_err(|e| io::Error::other(e.to_string()))?;
  tbl
    .set("os_run_interactive", os_run_interactive_fn)
    .map_err(|e| io::Error::other(e.to_string()))?;

  // preview helpers
  build_preview_helpers(lua, &tbl)?;
  Ok(tbl)
}

// No command placeholder expansion here; build arguments from Lua config/ctx.

fn build_preview_helpers(
  lua: &Lua,
  out: &Table,
) -> io::Result<()>
{
  let max_fn = lua
    .create_function(|_, (a, b): (u64, u64)| Ok(std::cmp::max(a, b)))
    .map_err(|e| io::Error::other(e.to_string()))?;
  out.set("math_max", max_fn).map_err(|e| io::Error::other(e.to_string()))?;
  Ok(())
}

fn build_clipboard_helpers(
  lua: &Lua,
  out: &Table,
  cfg_tbl: &Table,
) -> io::Result<()>
{
  // delete_selected(): delete confirmation
  let cfg_ref_del = cfg_tbl.clone();
  let delete_selected_fn = lua
    .create_function(move |_, ()| {
      let _ = cfg_ref_del.set("confirm", "delete_selected");
      Ok(true)
    })
    .map_err(|e| io::Error::other(e.to_string()))?;
  out
    .set("delete_selected", delete_selected_fn)
    .map_err(|e| io::Error::other(e.to_string()))?;
  Ok(())
}

fn build_selection_helpers(
  lua: &Lua,
  out: &Table,
  cfg_tbl: &Table,
) -> io::Result<()>
{
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
  out
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
  out
    .set("select_last_item", select_last_fn)
    .map_err(|e| io::Error::other(e.to_string()))?;
  Ok(())
}

fn merge_tables(
  lua: &Lua,
  base: &Table,
  overlay: &Table,
) -> mlua::Result<Table>
{
  let out = lua.create_table()?;
  for pair in base.pairs::<mlua::Value, mlua::Value>()
  {
    let (k, v) = pair?;
    out.set(k, v)?;
  }
  for pair in overlay.pairs::<mlua::Value, mlua::Value>()
  {
    let (k, v) = pair?;
    out.set(k, v)?;
  }
  Ok(out)
}
