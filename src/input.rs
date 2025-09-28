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

  if app.is_theme_picker_active()
  {
    match key.code
    {
      KeyCode::Esc =>
      {
        app.cancel_theme_picker();
      }
      KeyCode::Enter =>
      {
        app.confirm_theme_picker();
      }
      KeyCode::Up | KeyCode::Char('k') =>
      {
        app.theme_picker_move(-1);
      }
      KeyCode::Down | KeyCode::Char('j') =>
      {
        app.theme_picker_move(1);
      }
      KeyCode::PageUp =>
      {
        app.theme_picker_move(-5);
      }
      KeyCode::PageDown =>
      {
        app.theme_picker_move(5);
      }
      _ =>
      {}
    }
    return Ok(false);
  }

  // Prompt overlay input handling
  if let crate::app::Overlay::Prompt(ref mut st_box) = app.overlay
  {
    use crossterm::event::KeyEventKind;
    if key.kind != KeyEventKind::Press
    {
      return Ok(false);
    }
    let st = st_box.as_mut();
    match key.code
    {
      KeyCode::Esc =>
      {
        app.overlay = crate::app::Overlay::None;
        app.force_full_redraw = true;
      }
      KeyCode::Enter =>
      {
        // Submit
        match st.kind
        {
          crate::app::PromptKind::AddEntry =>
          {
            let name = st.input.trim();
            if !name.is_empty()
            {
              let path = app.cwd.join(name);
              if name.ends_with('/') || name.ends_with('\u{2f}')
              {
                let _ = std::fs::create_dir_all(&path);
              }
              else
              {
                let _ = std::fs::OpenOptions::new()
                  .create_new(true)
                  .write(true)
                  .open(&path);
              }
              app.refresh_lists();
            }
          }
          crate::app::PromptKind::RenameEntry { ref from } =>
          {
            let new_name = st.input.trim();
            if !new_name.is_empty()
            {
              let dest = app.cwd.join(new_name);
              if std::fs::rename(from, &dest).is_ok()
              {
                // Keep item selected after rename (update selection to new
                // path)
                if app.selected.remove(from)
                {
                  app.selected.insert(dest.clone());
                }
              }
              app.refresh_lists();
            }
          }
          crate::app::PromptKind::RenameMany {
            ref items,
            ref pre,
            ref suf,
          } =>
          {
            let tpl = st.input.trim().to_string();
            // Require exactly one {}
            if let Some(pos) = tpl.find("{}")
              && tpl.matches("{}").count() == 1
            {
              let (new_pre, new_suf) =
                (tpl[..pos].to_string(), tpl[pos + 2..].to_string());
              for p in items.iter()
              {
                if let Some(name_os) = p.file_name()
                  && let Some(name) = name_os.to_str()
                {
                  // Extract variable segment using original pre/suf
                  let var = name
                    .strip_prefix(pre.as_str())
                    .unwrap_or(name)
                    .strip_suffix(suf.as_str())
                    .unwrap_or(name);
                  let new_name = format!("{}{}{}", new_pre, var, new_suf);
                  let dst = app.cwd.join(new_name);
                  if std::fs::rename(p, &dst).is_ok() && app.selected.remove(p)
                  {
                    app.selected.insert(dst.clone());
                  }
                }
              }
              app.refresh_lists();
            }
            else
            {
              app.add_message(
                "Rename: template must contain exactly one {} placeholder",
              );
            }
          }
        }
        app.overlay = crate::app::Overlay::None;
        app.force_full_redraw = true;
      }
      KeyCode::Backspace =>
      {
        if st.cursor > 0 && st.cursor <= st.input.len()
        {
          st.input.remove(st.cursor - 1);
          st.cursor -= 1;
          app.force_full_redraw = true;
        }
      }
      KeyCode::Left =>
      {
        if st.cursor > 0
        {
          st.cursor -= 1;
          app.force_full_redraw = true;
        }
      }
      KeyCode::Right =>
      {
        if st.cursor < st.input.len()
        {
          st.cursor += 1;
          app.force_full_redraw = true;
        }
      }
      KeyCode::Home =>
      {
        st.cursor = 0;
        app.force_full_redraw = true;
      }
      KeyCode::End =>
      {
        st.cursor = st.input.len();
        app.force_full_redraw = true;
      }
      KeyCode::Char(ch) =>
      {
        if !key.modifiers.contains(KeyModifiers::CONTROL)
          && !key.modifiers.contains(KeyModifiers::ALT)
          && !key.modifiers.contains(KeyModifiers::SUPER)
        {
          st.input.insert(st.cursor, ch);
          st.cursor += ch.len_utf8();
          app.force_full_redraw = true;
        }
      }
      _ =>
      {}
    }
    return Ok(false);
  }

  // Command pane (search input)
  if let crate::app::Overlay::CommandPane(ref mut st_box) = app.overlay
  {
    let st = st_box.as_mut();
    let mut live_update: Option<String> = None;
    match key.code
    {
      KeyCode::Esc =>
      {
        app.overlay = crate::app::Overlay::None;
      }
      KeyCode::Enter =>
      {
        let pat = st.input.trim().to_string();
        if !pat.is_empty()
        {
          app.search_query = Some(pat);
        }
        app.overlay = crate::app::Overlay::None;
      }
      KeyCode::Backspace =>
      {
        if st.cursor > 0 && st.cursor <= st.input.len()
        {
          st.input.remove(st.cursor - 1);
          st.cursor -= 1;
          live_update = Some(st.input.clone());
          // incremental update handled via search_live
        }
      }
      KeyCode::Left =>
      {
        if st.cursor > 0
        {
          st.cursor -= 1;
          // incremental update handled via search_live
        }
      }
      KeyCode::Right =>
      {
        if st.cursor < st.input.len()
        {
          st.cursor += 1;
          app.force_full_redraw = true;
        }
      }
      KeyCode::Home =>
      {
        st.cursor = 0;
        app.force_full_redraw = true;
      }
      KeyCode::End =>
      {
        st.cursor = st.input.len();
        app.force_full_redraw = true;
      }
      KeyCode::Char(ch) =>
      {
        if !key.modifiers.contains(KeyModifiers::CONTROL)
          && !key.modifiers.contains(KeyModifiers::ALT)
          && !key.modifiers.contains(KeyModifiers::SUPER)
        {
          st.input.insert(st.cursor, ch);
          st.cursor += ch.len_utf8();
          live_update = Some(st.input.clone());
          app.force_full_redraw = true;
        }
      }
      _ =>
      {}
    }
    if let Some(s) = live_update
    {
      app.update_search_live(&s);
    }
    return Ok(false);
  }

  // Confirm overlay input handling (y/n)
  if let crate::app::Overlay::Confirm(ref mut st_box) = app.overlay
  {
    use crossterm::event::KeyEventKind;
    if key.kind != KeyEventKind::Press
    {
      return Ok(false);
    }
    let st = st_box.as_ref();
    enum Act
    {
      None,
      DeleteAll,
    }
    let mut act = Act::None;
    match key.code
    {
      KeyCode::Esc =>
      {
        crate::trace::log("[confirm] ESC -> cancel");
        act = Act::None;
      }
      KeyCode::Enter =>
      {
        // ENTER only confirms if default_yes
        if st.default_yes
        {
          act = Act::DeleteAll;
        }
      }
      KeyCode::Char('y') | KeyCode::Char('Y') =>
      {
        act = Act::DeleteAll;
      }
      KeyCode::Char('n') | KeyCode::Char('N') =>
      {
        crate::trace::log("[confirm] key='n' -> cancel");
        act = Act::None;
      }
      _ =>
      {}
    }
    // Drop borrow before mutating app
    let kind = st.kind.clone();
    app.overlay = crate::app::Overlay::None;
    app.force_full_redraw = true;
    if let (Act::DeleteAll, crate::app::ConfirmKind::DeleteSelected(list)) =
      (act, &kind)
    {
      for p in list.iter()
      {
        app.perform_delete_path(p);
      }
    }
    return Ok(false);
  }

  // First, try dynamic key mappings with simple sequence support
  // Quick toggle of which-key help
  if let KeyCode::Char('?') = key.code
  {
    app.overlay = match app.overlay
    {
      crate::app::Overlay::WhichKey { .. } => crate::app::Overlay::None,
      _ => crate::app::Overlay::WhichKey { prefix: app.keys.pending.clone() },
    };
    return Ok(false);
  }

  if let KeyCode::Char(ch) = key.code
  {
    // Allow modifier combinations; build token string for sequence matching
    {
      let now = std::time::Instant::now();
      // reset pending_seq on timeout
      if app.config.keys.sequence_timeout_ms > 0
        && let Some(last) = app.keys.last_at
      {
        let timeout =
          std::time::Duration::from_millis(app.config.keys.sequence_timeout_ms);
        if now.duration_since(last) > timeout
        {
          app.keys.pending.clear();
        }
      }
      app.keys.last_at = Some(now);

      // Build token
      let tok = crate::keymap::build_token(ch, key.modifiers);
      app.keys.pending.push_str(&tok);
      let seq = app.keys.pending.clone();

      if let Some(action) = app.keys.lookup.get(seq.as_str()).cloned()
      {
        // exact match
        app.keys.pending.clear();
        if matches!(app.overlay, crate::app::Overlay::WhichKey { .. })
        {
          app.overlay = crate::app::Overlay::None;
        }
        if crate::actions::dispatch_action(app, &action).unwrap_or(false)
        {
          if app.should_quit
          {
            return Ok(true);
          }
          return Ok(false);
        }
      }
      else if app.keys.prefixes.contains(&seq)
      {
        // keep gathering keys
        app.overlay = crate::app::Overlay::WhichKey { prefix: seq };
        return Ok(false);
      }
      else
      {
        // no sequence match; clear pending and exit this path (case-sensitive)
        app.keys.pending.clear();
        if matches!(app.overlay, crate::app::Overlay::WhichKey { .. })
        {
          app.overlay = crate::app::Overlay::None;
        }
      }
    }
  }
  match (key.code, key.modifiers)
  {
    (KeyCode::Char('q'), _) => return Ok(true),
    (KeyCode::Esc, _mods) =>
    {
      // If a mapping exists for <Esc>, dispatch it first
      let esc_seq = String::from("<Esc>");
      if let Some(action) = app.keys.lookup.get(esc_seq.as_str()).cloned()
      {
        let _ = crate::actions::dispatch_action(app, &action);
        if app.should_quit
        {
          return Ok(true);
        }
      }
      // cancel pending sequences and which-key
      app.keys.pending.clear();
      app.overlay = crate::app::Overlay::None;
      return Ok(false);
    }
    (KeyCode::Up, _) | (KeyCode::Char('k'), _) =>
    {
      if let Some(sel) = app.list_state.selected()
        && sel > 0
      {
        app.list_state.select(Some(sel - 1));
        app.refresh_preview();
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
        && entry.is_dir
      {
        app.cwd = entry.path.clone();
        app.refresh_lists();
        if app.current_entries.is_empty()
        {
          app.list_state.select(None);
        }
        else
        {
          app.list_state.select(Some(0));
        }
        app.refresh_preview();
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
          && let Some(idx) =
            app.current_entries.iter().position(|e| e.name == name)
        {
          app.list_state.select(Some(idx));
        }
        app.refresh_preview();
      }
    }
    _ =>
    {}
  }
  Ok(false)
}
