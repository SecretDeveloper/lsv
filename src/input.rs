//! Input handling for keyboard events.

use crate::app::App;
use std::io;

use crossterm::event::{
  KeyCode,
  KeyEvent,
  KeyEventKind,
  KeyModifiers,
};

/// Accept a terminal key event and mutate the [`App`] accordingly.
///
/// Returns `Ok(true)` when the caller should exit. Multi-key sequences are
/// resolved via the keymap; unrecognised keys fall back to built-in
/// navigation behaviour.
pub fn handle_key(
  app: &mut App,
  key: KeyEvent,
) -> io::Result<bool>
{
  // Ignore key release/repeat events to avoid double-processing (esp. on
  // Windows)
  if key.kind != KeyEventKind::Press
  {
    return Ok(false);
  }

  // First, try dynamic key mappings with simple sequence support
  // Quick toggle of which-key help
  if let KeyCode::Char('?') = key.code
  {
    if app.show_whichkey
    {
      app.show_whichkey = false;
      app.whichkey_prefix.clear();
    }
    else
    {
      app.show_whichkey = true;
      // Show for current pending prefix if any; otherwise root
      app.whichkey_prefix = app.pending_seq.clone();
    }
    return Ok(false);
  }

  if let KeyCode::Char(ch) = key.code
  {
    // Allow plain or SHIFT-modified letters; ignore Ctrl/Alt/Super
    let disallowed = key.modifiers.contains(KeyModifiers::CONTROL)
      || key.modifiers.contains(KeyModifiers::ALT)
      || key.modifiers.contains(KeyModifiers::SUPER);
    if !disallowed
    {
      let now = std::time::Instant::now();
      // reset pending_seq on timeout
      if app.config.keys.sequence_timeout_ms > 0
      {
        if let Some(last) = app.last_seq_time
        {
          let timeout = std::time::Duration::from_millis(
            app.config.keys.sequence_timeout_ms,
          );
          if now.duration_since(last) > timeout
          {
            app.pending_seq.clear();
          }
        }
      }
      app.last_seq_time = Some(now);

      app.pending_seq.push(ch);
      let seq = app.pending_seq.clone();

      if let Some(action) = app.keymap_lookup.get(seq.as_str()).cloned()
      {
        // exact match
        app.pending_seq.clear();
        app.show_whichkey = false;
        app.whichkey_prefix.clear();
        if crate::actions::dispatch_action(app, &action).unwrap_or(false)
        {
          if app.should_quit
          {
            return Ok(true);
          }
          return Ok(false);
        }
      }
      else if app.prefix_set.contains(&seq)
      {
        // keep gathering keys
        app.show_whichkey = true;
        app.whichkey_prefix = seq;
        return Ok(false);
      }
      else
      {
        // no sequence match, try single-key fallbacks (normalize case variants)
        app.pending_seq.clear();
        app.show_whichkey = false;
        app.whichkey_prefix.clear();
        let mut tried = std::collections::HashSet::new();
        for k in [
          ch.to_string(),
          ch.to_ascii_lowercase().to_string(),
          ch.to_ascii_uppercase().to_string(),
        ]
        {
          if !tried.insert(k.clone())
          {
            continue;
          }
          if let Some(action) = app.keymap_lookup.get(k.as_str()).cloned()
          {
            if crate::actions::dispatch_action(app, &action).unwrap_or(false)
            {
              if app.should_quit
              {
                return Ok(true);
              }
              return Ok(false);
            }
          }
        }
      }
    }
  }
  match (key.code, key.modifiers)
  {
    (KeyCode::Char('q'), _) => return Ok(true),
    (KeyCode::Esc, _) =>
    {
      // cancel pending sequences and which-key
      app.pending_seq.clear();
      app.show_whichkey = false;
      app.whichkey_prefix.clear();
      app.show_messages = false;
      app.show_output = false;
      return Ok(false);
    }
    (KeyCode::Up, _) | (KeyCode::Char('k'), _) =>
    {
      if let Some(sel) = app.list_state.selected()
      {
        if sel > 0
        {
          app.list_state.select(Some(sel - 1));
          app.refresh_preview();
        }
      }
    }
    (KeyCode::Down, _) | (KeyCode::Char('j'), _) =>
    {
      if let Some(sel) = app.list_state.selected()
      {
        if sel + 1 < app.current_entries.len()
        {
          app.list_state.select(Some(sel + 1));
          app.refresh_preview();
        }
      }
      else if !app.current_entries.is_empty()
      {
        app.list_state.select(Some(0));
        app.refresh_preview();
      }
    }
    (KeyCode::Enter, _) | (KeyCode::Right, _) =>
    {
      if let Some(entry) = app.selected_entry()
      {
        if entry.is_dir
        {
          app.cwd = entry.path.clone();
          app.refresh_lists();
          app.refresh_preview();
        }
      }
    }
    (KeyCode::Backspace, _)
    | (KeyCode::Left, _)
    | (KeyCode::Char('h'), KeyModifiers::NONE) =>
    {
      if let Some(parent) = app.cwd.parent()
      {
        // Remember the directory name we are leaving so we can reselect it
        let just_left =
          app.cwd.file_name().map(|s| s.to_string_lossy().to_string());
        app.cwd = parent.to_path_buf();
        app.refresh_lists();
        if let Some(name) = just_left
        {
          if let Some(idx) =
            app.current_entries.iter().position(|e| e.name == name)
          {
            app.list_state.select(Some(idx));
          }
        }
        app.refresh_preview();
      }
    }
    _ =>
    {}
  }
  Ok(false)
}
