//! Core application state, used both by the TUI and integration tests.
//!
//! The [`App`] struct models the in-memory view of the three-pane interface
//! (current directory listing, preview cache, overlays, etc.). The binary owns
//! an instance of `App`, but tests can create their own to simulate navigation
//! or exercise Lua actions.

use std::{
  cmp::min,
  env,
  fs,
  io,
  path::{
    Path,
    PathBuf,
  },
  time::SystemTime,
};

use mlua::RegistryKey;
use ratatui::widgets::ListState;

use crate::{
  actions::SortKey,
  core::fs_ops,
};

#[derive(Debug, Clone)]
/// Runtime state for lsv, including directory listings, preview cache, overlay
/// flags, and configuration.
pub struct DirEntryInfo
{
  pub(crate) name:   String,
  pub(crate) path:   PathBuf,
  pub(crate) is_dir: bool,
  pub(crate) size:   u64,
  pub(crate) mtime:  Option<SystemTime>,
  pub(crate) ctime:  Option<SystemTime>,
}

#[derive(Debug, Clone)]
pub struct ThemePickerEntry
{
  pub name:  String,
  pub path:  PathBuf,
  pub theme: crate::config::UiTheme,
}

#[derive(Debug, Clone)]
pub struct ThemePickerState
{
  pub entries:             Vec<ThemePickerEntry>,
  pub selected:            usize,
  pub original_theme:      Option<crate::config::UiTheme>,
  pub original_theme_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub enum Overlay
{
  None,
  WhichKey
  {
    prefix: String,
  },
  Messages,
  Output
  {
    title: String,
    lines: Vec<String>,
  },
  ThemePicker(Box<ThemePickerState>),
  Prompt(Box<PromptState>),
  Confirm(Box<ConfirmState>),
}

#[derive(Debug, Clone, Default)]
pub struct PreviewState
{
  pub static_lines: Vec<String>,
  pub cache_key:    Option<(std::path::PathBuf, u16, u16)>,
  pub cache_lines:  Option<Vec<String>>,
}

#[derive(Debug, Clone, Default)]
pub struct KeyState
{
  pub maps:     Vec<crate::config::KeyMapping>,
  pub lookup:   std::collections::HashMap<String, String>,
  pub prefixes: std::collections::HashSet<String>,
  pub pending:  String,
  pub last_at:  Option<std::time::Instant>,
}

use crate::keymap::tokenize_sequence;

pub struct LuaRuntime
{
  pub engine:    crate::config::LuaEngine,
  pub previewer: Option<RegistryKey>,
  pub actions:   Vec<RegistryKey>,
}

#[derive(Debug, Clone)]
pub enum PromptKind
{
  AddEntry,
  RenameEntry
  {
    from: std::path::PathBuf,
  },
  RenameMany
  {
    items: Vec<std::path::PathBuf>,
    pre:   String,
    suf:   String,
  },
}

#[derive(Debug, Clone)]
pub struct PromptState
{
  pub title:  String,
  pub input:  String,
  pub cursor: usize,
  pub kind:   PromptKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardOp
{
  Copy,
  Move,
}

#[derive(Debug, Clone)]
pub struct Clipboard
{
  pub op:    ClipboardOp,
  pub items: Vec<std::path::PathBuf>,
}

#[derive(Debug, Clone)]
pub enum ConfirmKind
{
  DeleteSelected(Vec<std::path::PathBuf>),
}

#[derive(Debug, Clone)]
pub struct ConfirmState
{
  pub title:       String,
  pub question:    String,
  pub default_yes: bool,
  pub kind:        ConfirmKind,
}

/// Mutable application state driving the three-pane UI.
pub struct App
{
  pub(crate) cwd:               PathBuf,
  pub(crate) current_entries:   Vec<DirEntryInfo>,
  pub(crate) parent_entries:    Vec<DirEntryInfo>,
  pub(crate) list_state:        ListState,
  pub(crate) preview:           PreviewState,
  // Messages
  pub(crate) recent_messages:   Vec<String>,
  // Overlay state (mutually exclusive)
  pub(crate) overlay:           Overlay,
  pub(crate) config:            crate::config::Config,
  pub(crate) keys:              KeyState,
  pub(crate) force_full_redraw: bool,
  pub(crate) lua:               Option<LuaRuntime>,
  pub(crate) selected:          std::collections::HashSet<std::path::PathBuf>,
  pub(crate) clipboard:         Option<Clipboard>,
  // In-memory runtime settings
  pub(crate) sort_key:          SortKey,
  pub(crate) sort_reverse:      bool,
  pub(crate) info_mode:         InfoMode,
  pub(crate) display_mode:      DisplayMode,
  // Signal to exit after handling a key/action
  pub(crate) should_quit:       bool,
  // Key sequence handling
  // moved into `keys`
  // (which-key prefix moves under overlay)
}

impl App
{
  /// Construct a fresh [`App`] using the current working directory as the
  /// starting point.
  pub fn new() -> io::Result<Self>
  {
    let cwd = env::current_dir()?;
    // Temporary initial read with default sort (Name asc)
    let current_entries = {
      // Build a temporary App-like context for sorting
      let mut tmp = Vec::new();
      for de in (fs::read_dir(&cwd)?).flatten()
      {
        let path = de.path();
        let name = de.file_name().to_string_lossy().to_string();
        if let Ok(ft) = de.file_type()
        {
          let meta = fs::metadata(&path).ok();
          let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
          let mtime = meta.as_ref().and_then(|m| m.modified().ok());
          let ctime = meta.as_ref().and_then(|m| m.created().ok());
          tmp.push(DirEntryInfo {
            name,
            path,
            is_dir: ft.is_dir(),
            size,
            mtime,
            ctime,
          });
        }
      }
      tmp.sort_by(|a, b| match (a.is_dir, b.is_dir)
      {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
      });
      tmp
    };
    let parent_entries = if let Some(p) = cwd.parent()
    {
      // Same initial read for parent
      let mut tmp = Vec::new();
      for de in (fs::read_dir(p)?).flatten()
      {
        let path = de.path();
        let name = de.file_name().to_string_lossy().to_string();
        if let Ok(ft) = de.file_type()
        {
          let meta = fs::metadata(&path).ok();
          let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
          let mtime = meta.as_ref().and_then(|m| m.modified().ok());
          let ctime = meta.as_ref().and_then(|m| m.created().ok());
          tmp.push(DirEntryInfo {
            name,
            path,
            is_dir: ft.is_dir(),
            size,
            mtime,
            ctime,
          });
        }
      }
      tmp.sort_by(|a, b| match (a.is_dir, b.is_dir)
      {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
      });
      tmp
    }
    else
    {
      Vec::new()
    };

    let mut list_state = ListState::default();
    if !current_entries.is_empty()
    {
      list_state.select(Some(0));
    }
    let mut app = Self {
      cwd,
      current_entries,
      parent_entries,
      list_state,
      preview: PreviewState::default(),
      recent_messages: Vec::new(),
      overlay: Overlay::None,
      config: crate::config::Config::default(),
      keys: KeyState::default(),
      force_full_redraw: false,
      lua: None,
      selected: std::collections::HashSet::new(),
      clipboard: None,
      sort_key: SortKey::Name,
      sort_reverse: false,
      info_mode: InfoMode::None,
      display_mode: DisplayMode::Absolute,
      should_quit: false,
    };
    // Discover configuration paths (entry not executed yet)
    if let Ok(paths) = crate::config::discover_config_paths()
    {
      match crate::config::load_config(&paths)
      {
        Ok((cfg, maps, engine_opt)) =>
        {
          app.config = cfg;
          app.keys.maps = maps;
          app.rebuild_keymap_lookup();
          if let Some((eng, key, action_keys)) = engine_opt
          {
            app.lua = Some(LuaRuntime {
              engine:    eng,
              previewer: Some(key),
              actions:   action_keys,
            });
          }
          else
          {
            app.lua = None;
          }
          // Re-apply lists to honor config (e.g., show_hidden)
          // Also apply optional initial sort/show from config.ui
          if let Some(ref srt) = app.config.ui.sort
            && let Some(k) = crate::enums::sort_key_from_str(srt)
          {
            app.sort_key = k;
          }
          if let Some(b) = app.config.ui.sort_reverse
          {
            app.sort_reverse = b;
          }
          if let Some(ref sh) = app.config.ui.show
          {
            if sh.eq_ignore_ascii_case("none")
            {
              app.info_mode = crate::app::InfoMode::None;
            }
            else if let Some(m) = crate::enums::info_mode_from_str(sh)
            {
              app.info_mode = m;
            }
          }
          app.refresh_lists();
          // Apply display_mode from config if present
          if let Some(dm) = app.config.ui.display_mode.as_deref()
            && let Some(mode) = crate::enums::display_mode_from_str(dm)
          {
            app.display_mode = mode;
          }
        }
        Err(e) =>
        {
          eprintln!("lsv: config load error: {}", e);
        }
      }
    }
    app.refresh_preview();
    Ok(app)
  }

  /// Test helper: inject a prepared Lua engine and registered action keys.
  ///
  /// This lets integration tests execute Lua callbacks without loading files
  /// from disk.
  pub fn inject_lua_engine_for_tests(
    &mut self,
    engine: crate::config::LuaEngine,
    action_keys: Vec<mlua::RegistryKey>,
  )
  {
    self.lua =
      Some(LuaRuntime { engine, previewer: None, actions: action_keys });
  }

  pub(crate) fn selected_entry(&self) -> Option<&DirEntryInfo>
  {
    self.list_state.selected().and_then(|i| self.current_entries.get(i))
  }

  pub fn get_current_entry_name(
    &self,
    idx: usize,
  ) -> Option<String>
  {
    self.current_entries.get(idx).map(|e| e.name.clone())
  }

  pub fn select_index(
    &mut self,
    idx: usize,
  )
  {
    self.list_state.select(Some(idx));
    self.refresh_preview();
  }

  pub(crate) fn refresh_lists(&mut self)
  {
    self.current_entries = self.read_dir_sorted(&self.cwd).unwrap_or_default();
    if self.current_entries.len() > self.config.ui.max_list_items
    {
      self.current_entries.truncate(self.config.ui.max_list_items);
    }
    self.parent_entries = if let Some(p) = self.cwd.parent()
    {
      self.read_dir_sorted(p).unwrap_or_default()
    }
    else
    {
      Vec::new()
    };
    if self.parent_entries.len() > self.config.ui.max_list_items
    {
      self.parent_entries.truncate(self.config.ui.max_list_items);
    }
    // Clamp selection
    let max_idx = self.current_entries.len().saturating_sub(1);
    if let Some(sel) = self.list_state.selected()
    {
      self.list_state.select(
        if self.current_entries.is_empty()
        {
          None
        }
        else
        {
          Some(min(sel, max_idx))
        },
      );
    }
    else if !self.current_entries.is_empty()
    {
      self.list_state.select(Some(0));
    }
    // Invalidate dynamic preview cache on list refresh
    self.preview.cache_key = None;
    self.preview.cache_lines = None;
  }

  pub(crate) fn refresh_preview(&mut self)
  {
    // Avoid borrowing self while mutating by cloning the needed fields first
    let (is_dir, path) = match self.selected_entry()
    {
      Some(e) => (e.is_dir, e.path.clone()),
      None =>
      {
        self.preview.static_lines.clear();
        // Invalidate dynamic preview cache when nothing selected
        self.preview.cache_key = None;
        self.preview.cache_lines = None;
        return;
      }
    };

    const PREVIEW_LINES_LIMIT: usize = 200;
    let preview_limit = PREVIEW_LINES_LIMIT;
    if is_dir
    {
      match self.read_dir_sorted(&path)
      {
        Ok(list) =>
        {
          let mut lines = Vec::new();
          for e in list.into_iter().take(preview_limit)
          {
            let marker = if e.is_dir { "/" } else { "" };
            let formatted = format!("{}{}", e.name, marker);
            lines.push(crate::util::sanitize_line(&formatted));
          }
          self.preview.static_lines = lines;
        }
        Err(err) =>
        {
          self.preview.static_lines =
            vec![format!("<error reading directory: {}>", err)];
        }
      }
    }
    else
    {
      self.preview.static_lines =
        crate::util::read_file_head(&path, preview_limit)
          .map(|v| {
            v.into_iter().map(|s| crate::util::sanitize_line(&s)).collect()
          })
          .unwrap_or_else(|e| vec![format!("<error reading file: {}>", e)]);
      // Invalidate dynamic preview cache when selection changes
      self.preview.cache_key = None;
      self.preview.cache_lines = None;
    }
  }

  pub(crate) fn read_dir_sorted(
    &self,
    path: &Path,
  ) -> io::Result<Vec<DirEntryInfo>>
  {
    crate::core::listing::read_dir_sorted(
      path,
      self.config.ui.show_hidden,
      self.sort_key,
      self.sort_reverse,
    )
  }

  pub(crate) fn rebuild_keymap_lookup(&mut self)
  {
    self.keys.lookup.clear();
    self.keys.prefixes.clear();
    for m in &self.keys.maps
    {
      self.keys.lookup.insert(m.sequence.clone(), m.action.clone());
      // collect token-based prefixes for sequence matching
      let tokens = tokenize_sequence(&m.sequence);
      let mut acc = String::new();
      for (idx, t) in tokens.iter().enumerate()
      {
        acc.push_str(t);
        if idx + 1 < tokens.len()
        {
          self.keys.prefixes.insert(acc.clone());
        }
      }
    }
  }

  pub fn set_keymaps(
    &mut self,
    maps: Vec<crate::config::KeyMapping>,
  )
  {
    self.keys.maps = maps;
    self.rebuild_keymap_lookup();
  }

  pub fn get_keymap_action(
    &self,
    seq: &str,
  ) -> Option<String>
  {
    self.keys.lookup.get(seq).cloned()
  }

  pub fn has_prefix(
    &self,
    seq: &str,
  ) -> bool
  {
    self.keys.prefixes.contains(seq)
  }

  pub fn show_hidden(&self) -> bool
  {
    self.config.ui.show_hidden
  }
  pub fn get_date_format(&self) -> Option<String>
  {
    self.config.ui.date_format.clone()
  }
  // preview_lines removed: internal cap used instead
  pub fn set_force_full_redraw(
    &mut self,
    v: bool,
  )
  {
    self.force_full_redraw = v;
  }
  pub fn get_force_full_redraw(&self) -> bool
  {
    self.force_full_redraw
  }
  pub fn get_show_messages(&self) -> bool
  {
    matches!(self.overlay, Overlay::Messages)
  }
  pub fn get_show_output(&self) -> bool
  {
    matches!(self.overlay, Overlay::Output { .. })
  }
  pub fn get_show_whichkey(&self) -> bool
  {
    matches!(self.overlay, Overlay::WhichKey { .. })
  }
  pub fn get_output_title(&self) -> &str
  {
    if let Overlay::Output { ref title, .. } = self.overlay
    {
      title.as_str()
    }
    else
    {
      ""
    }
  }
  pub fn get_output_text(&self) -> String
  {
    if let Overlay::Output { ref lines, .. } = self.overlay
    {
      lines.join("\n")
    }
    else
    {
      String::new()
    }
  }
  pub fn current_has_entries(&self) -> bool
  {
    !self.current_entries.is_empty()
  }
  pub fn get_list_selected_index(&self) -> Option<usize>
  {
    self.list_state.selected()
  }
  pub fn get_quit(&self) -> bool
  {
    self.should_quit
  }
  pub fn get_sort_reverse(&self) -> bool
  {
    self.sort_reverse
  }
  pub fn set_sort_reverse(
    &mut self,
    v: bool,
  )
  {
    self.sort_reverse = v;
  }
  pub fn get_display_mode(&self) -> DisplayMode
  {
    self.display_mode
  }
  pub fn get_info_mode(&self) -> InfoMode
  {
    self.info_mode
  }

  pub fn get_entry(
    &self,
    idx: usize,
  ) -> Option<DirEntryInfo>
  {
    self.current_entries.get(idx).cloned()
  }

  pub fn set_cwd(
    &mut self,
    path: &std::path::Path,
  )
  {
    self.cwd = path.to_path_buf();
    self.refresh_lists();
    if !self.current_entries.is_empty()
    {
      self.list_state.select(Some(0));
      self.refresh_preview();
    }
  }

  pub fn get_whichkey_prefix(&self) -> String
  {
    if let Overlay::WhichKey { ref prefix } = self.overlay
    {
      return prefix.clone();
    }
    String::new()
  }
  pub fn get_sort_key(&self) -> crate::actions::SortKey
  {
    self.sort_key
  }
  pub fn set_config(
    &mut self,
    cfg: crate::config::Config,
  )
  {
    self.config = cfg;
  }
  pub fn get_config(&mut self) -> crate::config::Config
  {
    self.config.clone()
  }
  pub fn get_cwd_path(&self) -> std::path::PathBuf
  {
    self.cwd.clone()
  }

  pub fn preview_line_count(&self) -> usize
  {
    self.preview.static_lines.len()
  }

  pub(crate) fn toggle_select_current(&mut self)
  {
    if let Some(e) = self.selected_entry().cloned()
    {
      if self.selected.contains(&e.path)
      {
        self.selected.remove(&e.path);
      }
      else
      {
        self.selected.insert(e.path);
      }
    }
  }

  pub(crate) fn clear_all_selected(&mut self)
  {
    if !self.selected.is_empty()
    {
      self.selected.clear();
    }
  }

  pub(crate) fn copy_selection(&mut self)
  {
    let items: Vec<std::path::PathBuf> =
      self.selected.iter().cloned().collect();
    if items.is_empty()
    {
      self.add_message("Copy: no items selected");
      return;
    }
    self.clipboard = Some(Clipboard { op: ClipboardOp::Copy, items });
    self.add_message("Copied selection to clipboard");
    self.force_full_redraw = true;
  }

  pub(crate) fn move_selection(&mut self)
  {
    let items: Vec<std::path::PathBuf> =
      self.selected.iter().cloned().collect();
    if items.is_empty()
    {
      self.add_message("Move: no items selected");
      return;
    }
    self.clipboard = Some(Clipboard { op: ClipboardOp::Move, items });
    self.add_message("Move selection armed");
    self.force_full_redraw = true;
  }

  pub(crate) fn clear_clipboard(&mut self)
  {
    self.clipboard = None;
    self.add_message("Clipboard cleared");
    self.force_full_redraw = true;
  }

  pub(crate) fn paste_clipboard(&mut self)
  {
    let Some(cb) = self.clipboard.clone()
    else
    {
      self.add_message("Paste: clipboard empty");
      return;
    };
    let dest_dir = self.cwd.clone();
    let mut ok = 0usize;
    let mut skipped = 0usize;
    let mut errs = 0usize;
    for src in cb.items.iter()
    {
      if matches!(cb.op, ClipboardOp::Move) && dest_dir.starts_with(src)
      {
        self
          .add_message(&format!("Skip (move into subdir): {}", src.display()));
        skipped += 1;
        continue;
      }
      let Some(name) = src.file_name()
      else
      {
        skipped += 1;
        continue;
      };
      let dest_path = dest_dir.join(name);
      if dest_path.exists()
      {
        self.add_message(&format!("Skip (exists): {}", dest_path.display()));
        skipped += 1;
        continue;
      }
      let res = match cb.op
      {
        ClipboardOp::Copy => fs_ops::copy_path_recursive(src, &dest_path),
        ClipboardOp::Move => fs_ops::move_path_with_fallback(src, &dest_path),
      };
      match res
      {
        Ok(()) => ok += 1,
        Err(e) =>
        {
          errs += 1;
          self.add_message(&format!(
            "Error: {} -> {}: {}",
            src.display(),
            dest_path.display(),
            e
          ));
        }
      }
    }
    if matches!(cb.op, ClipboardOp::Move)
    {
      for p in cb.items.iter()
      {
        self.selected.remove(p);
      }
    }
    self.clipboard = None;
    self.refresh_lists();
    self.refresh_preview();
    self.add_message(&format!(
      "Paste: ok={} skipped={} errors={}",
      ok, skipped, errs
    ));
  }

  pub fn recent_messages_len(&self) -> usize
  {
    self.recent_messages.len()
  }

  pub fn add_message(
    &mut self,
    msg: &str,
  )
  {
    let m = msg.trim().to_string();
    if m.is_empty()
    {
      return;
    }
    self.recent_messages.push(m);
    if self.recent_messages.len() > 100
    {
      let _ = self.recent_messages.drain(0..self.recent_messages.len() - 100);
    }
    self.force_full_redraw = true;
  }

  pub(crate) fn theme_root_dir(&self) -> Option<PathBuf>
  {
    crate::config::discover_config_paths().ok().map(|p| p.root)
  }

  pub(crate) fn open_theme_picker(&mut self)
  {
    let root = match self.theme_root_dir()
    {
      Some(p) => p,
      None =>
      {
        self.add_message("Theme picker: unable to determine config directory");
        return;
      }
    };
    let themes_dir = root.join("themes");
    let read_dir = match fs::read_dir(&themes_dir)
    {
      Ok(rd) => rd,
      Err(_) =>
      {
        self.add_message(&format!(
          "Theme picker: no themes directory at {}",
          themes_dir.display()
        ));
        return;
      }
    };

    let mut entries: Vec<ThemePickerEntry> = Vec::new();
    for entry in read_dir
    {
      match entry
      {
        Ok(dir_entry) =>
        {
          let path = dir_entry.path();
          if !path.is_file()
          {
            continue;
          };

          if let Some(ext) = path.extension().and_then(|s| s.to_str())
          {
            if !ext.eq_ignore_ascii_case("lua")
            {
              continue;
            }
          }
          else
          {
            continue;
          }

          match crate::config::load_theme_from_file(&path)
          {
            Ok(theme) =>
            {
              let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| path.display().to_string());
              entries.push(ThemePickerEntry { name, path, theme });
            }
            Err(e) =>
            {
              self.add_message(&format!(
                "Theme picker: failed to load {} ({})",
                path.display(),
                e
              ));
            }
          }
        }
        Err(e) =>
        {
          self.add_message(&format!(
            "Theme picker: error reading themes directory ({})",
            e
          ));
        }
      }
    }

    if entries.is_empty()
    {
      self.add_message(&format!(
        "Theme picker: no .lua themes found in {}",
        themes_dir.display()
      ));
      return;
    }

    entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    let current_path = self.config.ui.theme_path.clone();
    let mut selected = 0usize;
    if let Some(cur) = current_path.as_ref()
      && let Some(idx) = entries.iter().position(|e| e.path == *cur)
    {
      selected = idx;
    }

    let state = ThemePickerState {
      entries,
      selected,
      original_theme: self.config.ui.theme.clone(),
      original_theme_path: current_path,
    };

    self.keys.pending.clear();
    self.keys.last_at = None;
    self.overlay = Overlay::ThemePicker(Box::new(state));
    self.force_full_redraw = true;
  }

  pub(crate) fn open_add_entry_prompt(&mut self)
  {
    crate::core::overlays::open_add_entry_prompt(self)
  }

  pub(crate) fn open_rename_entry_prompt(&mut self)
  {
    crate::core::overlays::open_rename_entry_prompt(self)
  }

  pub(crate) fn request_delete_selected(&mut self)
  {
    crate::core::overlays::request_delete_selected(self)
  }

  pub(crate) fn perform_delete_path(
    &mut self,
    path: &std::path::Path,
  )
  {
    crate::trace::log(format!("[delete] perform path='{}'", path.display()));
    let res = fs_ops::remove_path_all(path);
    match res
    {
      Ok(_) =>
      {
        crate::trace::log("[delete] success");
        self.add_message("Deleted");
        // Remove from selection if present
        self.selected.remove(path);
      }
      Err(e) =>
      {
        crate::trace::log(format!("[delete] error: {}", e));
        self.add_message(&format!("Delete error: {}", e));
      }
    }
    self.refresh_lists();
    self.refresh_preview();
  }

  pub(crate) fn theme_picker_move(
    &mut self,
    delta: isize,
  )
  {
    crate::core::overlays::theme_picker_move(self, delta)
  }

  pub(crate) fn confirm_theme_picker(&mut self)
  {
    crate::core::overlays::confirm_theme_picker(self)
  }

  pub(crate) fn cancel_theme_picker(&mut self)
  {
    if let Overlay::ThemePicker(state) =
      std::mem::replace(&mut self.overlay, Overlay::None)
    {
      let st = *state;
      self.config.ui.theme = st.original_theme;
      self.config.ui.theme_path = st.original_theme_path;
      self.force_full_redraw = true;
    }
  }

  pub(crate) fn is_theme_picker_active(&self) -> bool
  {
    matches!(self.overlay, Overlay::ThemePicker(_))
  }

  pub fn display_output(
    &mut self,
    title: &str,
    text: &str,
  )
  {
    let lines: Vec<String> =
      text.replace('\r', "").lines().map(|s| s.to_string()).collect();
    self.overlay = Overlay::Output { title: title.to_string(), lines };
    self.force_full_redraw = true;
  }
}

pub(crate) fn common_affixes(names: &[String]) -> (String, String)
{
  if names.is_empty()
  {
    return (String::new(), String::new());
  }

  fn common_prefix(
    a: &str,
    b: &str,
  ) -> String
  {
    let mut out = String::new();
    for (ca, cb) in a.chars().zip(b.chars())
    {
      if ca == cb
      {
        out.push(ca);
      }
      else
      {
        break;
      }
    }
    out
  }
  fn common_suffix(
    a: &str,
    b: &str,
  ) -> String
  {
    let mut rev: Vec<char> = Vec::new();
    for (ca, cb) in a.chars().rev().zip(b.chars().rev())
    {
      if ca == cb
      {
        rev.push(ca);
      }
      else
      {
        break;
      }
    }
    rev.into_iter().rev().collect()
  }

  let mut pre = names[0].clone();
  for n in names.iter().skip(1)
  {
    pre = common_prefix(&pre, n);
    if pre.is_empty()
    {
      break;
    }
  }
  let mut suf = names[0].clone();
  for n in names.iter().skip(1)
  {
    suf = common_suffix(&suf, n);
    if suf.is_empty()
    { /* keep going to ensure empty is final */ }
  }
  (pre, suf)
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfoMode
{
  None,
  Size,
  Created,
  Modified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode
{
  Absolute,
  Friendly,
}
