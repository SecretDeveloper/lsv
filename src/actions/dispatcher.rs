// Central action dispatcher.
//
// Accepts action strings, supports ';' separated sequences, routes
// `run_lua:<idx>` to Lua via lua_glue, and executes native internal
// actions parsed by `internal`. Effects and optional config overlays
// returned from Lua are applied immediately.
use std::io;

use crate::app::App;

use super::{internal::{parse_internal_action, execute_internal_action}, apply::{apply_effects, apply_config_overlay}, lua_glue::call_lua_action};

/// Parse and execute an action string.
/// Supports multiple actions separated by ';', Lua actions via `run_lua:<idx>`,
/// and internal actions parsed by `internal`.
pub fn dispatch_action(app: &mut App, action: &str) -> io::Result<bool> {
  // Support multiple commands separated by ';'
  let parts: Vec<&str> = action.split(';').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
  if parts.len() > 1 {
    let mut any = false;
    for p in parts {
      if dispatch_action(app, p)? { any = true; }
      if app.should_quit { break; }
    }
    return Ok(any);
  }

  // Lua action index
  if let Some(rest) = action.strip_prefix("run_lua:") {
    if let Ok(idx) = rest.parse::<usize>() {
      if let (Some(_), Some(funcs)) = (app.lua_engine.as_ref(), app.lua_action_fns.as_ref()) {
        if idx < funcs.len() {
          let (fx, overlay) = call_lua_action(app, idx)?;
          apply_effects(app, fx);
          if let Some(data) = overlay { apply_config_overlay(app, &data); }
          return Ok(true);
        }
      }
      return Ok(false);
    }
  }

  // Internal action
  if let Some(int) = parse_internal_action(action) {
    execute_internal_action(app, int);
    return Ok(true);
  }
  Ok(false)
}
