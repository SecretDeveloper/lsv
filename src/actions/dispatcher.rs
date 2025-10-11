//! Central action dispatcher used by both the binary and integration tests.
//!
//! Accepts action strings (optionally `;`-separated sequences), routes
//! `run_lua:<idx>` entries to the Lua runtime glue, and executes
//! built-in actions parsed by [`internal`](super::internal).
//! Lua side-effects and configuration overlays are applied immediately.
use std::io;

use crate::app::App;

use super::{
  apply::{
    apply_config_overlay,
    apply_effects,
  },
  internal::{
    execute_internal_action,
    parse_internal_action,
  },
  // moved under config::runtime::glue
};
use crate::{
  config::runtime::glue::call_lua_action,
  trace,
};

/// Parse and execute an action string.
///
/// Returns `Ok(true)` when at least one action ran successfully. Supports
/// multiple actions separated by `;`, Lua actions via `run_lua:<idx>`, and
/// internal actions parsed by [`internal`](super::internal).
pub fn dispatch_action(
  app: &mut App,
  action: &str,
) -> io::Result<bool>
{
  // Support multiple commands separated by ';'
  let parts: Vec<&str> =
    action.split(';').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
  if parts.len() > 1
  {
    let mut any = false;
    for p in parts
    {
      trace::log(format!("[dispatch] action='{}'", p));
      if dispatch_action(app, p)?
      {
        any = true;
      }
      if app.should_quit
      {
        break;
      }
    }
    return Ok(any);
  }

  // Lua action index
  if let Some(rest) = action.strip_prefix("run_lua:")
  {
    trace::log(format!("[dispatch] action='{}'", action));
    if let Ok(idx) = rest.parse::<usize>()
    {
      if let Some(lua) = app.lua.as_ref()
        && idx < lua.actions.len()
      {
        let (fx, overlay) = call_lua_action(app, idx)?;
        apply_effects(app, fx);
        if let Some(data) = overlay
        {
          apply_config_overlay(app, &data);
        }
        return Ok(true);
      }
      return Ok(false);
    }
    else
    {
      trace::log(format!("[dispatch] bad lua index in '{}'", action));
    }
  }

  // Internal action
  if let Some(int) = parse_internal_action(action)
  {
    trace::log(format!("[dispatch] action='{}'", action));
    if let Some(fx) = super::internal::internal_effects(app, &int)
    {
      apply_effects(app, fx);
    }
    else
    {
      execute_internal_action(app, int);
    }
    return Ok(true);
  }
  Ok(false)
}
