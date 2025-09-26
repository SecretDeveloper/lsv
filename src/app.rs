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

use crate::actions::SortKey;

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
  WhichKey { prefix: String },
  Messages,
  Output { title: String, lines: Vec<String> },
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

pub struct LuaRuntime
{
  pub engine:   crate::config::LuaEngine,
  pub previewer: Option<RegistryKey>,
  pub actions:  Vec<RegistryKey>,
}

#[derive(Debug, Clone)]
pub enum PromptKind
{
  AddEntry,
  RenameEntry { from: std::path::PathBuf },
}

#[derive(Debug, Clone)]
pub struct PromptState
{
  pub title:  String,
  pub input:  String,
  pub cursor: usize,
  pub kind:   PromptKind,
}

#[derive(Debug, Clone)]
pub enum ConfirmKind
{
  DeleteEntry(std::path::PathBuf),
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
            app.lua = Some(LuaRuntime { engine: eng, previewer: Some(key), actions: action_keys });
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
    self.lua = Some(LuaRuntime { engine, previewer: None, actions: action_keys });
  }

  pub(crate) fn selected_entry(&self) -> Option<&DirEntryInfo>
  {
    self.list_state.selected().and_then(|i| self.current_entries.get(i))
  }

  #[doc(hidden)]
  pub fn get_current_entry_name(
    &self,
    idx: usize,
  ) -> Option<String>
  {
    self.current_entries.get(idx).map(|e| e.name.clone())
  }
  #[doc(hidden)]
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

    let preview_limit = self.config.ui.preview_lines;
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
      self.preview.static_lines = crate::util::read_file_head(&path, preview_limit)
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
    let mut entries: Vec<DirEntryInfo> = fs::read_dir(path)?
      .filter_map(|res| res.ok())
      .filter_map(|e| {
        let path = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if !self.config.ui.show_hidden && name.starts_with('.')
        {
          return None;
        }
        match e.file_type()
        {
          Ok(ft) =>
          {
            let meta = fs::metadata(&path).ok();
            let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            let mtime = meta.as_ref().and_then(|m| m.modified().ok());
            let ctime = meta.as_ref().and_then(|m| m.created().ok());
            Some(DirEntryInfo {
              name,
              path,
              is_dir: ft.is_dir(),
              size,
              mtime,
              ctime,
            })
          }
          Err(_) => None,
        }
      })
      .collect();

    let sort_key = self.sort_key;
    let reverse = self.sort_reverse;

    entries.sort_by(|a, b| {
      // Always keep directories before files
      match (a.is_dir, b.is_dir)
      {
        (true, false) => return std::cmp::Ordering::Less,
        (false, true) => return std::cmp::Ordering::Greater,
        _ =>
        {}
      }

      let ord = match sort_key
      {
        SortKey::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        SortKey::Size => a.size.cmp(&b.size),
        SortKey::MTime =>
        {
          let at = a.mtime.unwrap_or(std::time::SystemTime::UNIX_EPOCH);
          let bt = b.mtime.unwrap_or(std::time::SystemTime::UNIX_EPOCH);
          at.cmp(&bt)
        }
        SortKey::CTime =>
        {
          let at = a.ctime.unwrap_or(std::time::SystemTime::UNIX_EPOCH);
          let bt = b.ctime.unwrap_or(std::time::SystemTime::UNIX_EPOCH);
          at.cmp(&bt)
        }
      };
      if reverse { ord.reverse() } else { ord }
    });

    Ok(entries)
  }

  pub(crate) fn rebuild_keymap_lookup(&mut self)
  {
    self.keys.lookup.clear();
    self.keys.prefixes.clear();
    for m in &self.keys.maps
    {
      self.keys.lookup.insert(m.sequence.clone(), m.action.clone());
      // collect prefixes for sequence matching
      let s = &m.sequence;
      let chars = s.chars();
      let mut prefix = String::new();
      for c in chars
      {
        prefix.push(c);
        // do not include the full sequence as prefix-only
        if prefix.len() < s.len()
        {
          self.keys.prefixes.insert(prefix.clone());
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
  pub fn preview_line_limit(&self) -> usize
  {
    self.config.ui.preview_lines
  }

  // Backwards compatibility for tests and callers expecting the old name
  pub fn get_preview_lines(&self) -> usize
  {
    self.preview_line_limit()
  }
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
  pub fn get_config( &mut self) -> crate::config::Config
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

  fn theme_root_dir(&self) -> Option<PathBuf>
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
          if !path.is_file() { continue };

          if let Some(ext) = path.extension().and_then(|s| s.to_str())
          {
            if !ext.eq_ignore_ascii_case("lua") { continue; }
          }
          else { continue; }

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
              self.add_message(&format!( "Theme picker: failed to load {} ({})",
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

  fn apply_theme_entry(
    &mut self,
    entry: ThemePickerEntry,
  )
  {
    self.config.ui.theme = Some(entry.theme);
    self.config.ui.theme_path = Some(entry.path);
    self.force_full_redraw = true;
  }

  pub(crate) fn open_add_entry_prompt(&mut self)
  {
    self.overlay = Overlay::Prompt(Box::new(PromptState {
      title:  "Name (end with '/' for folder):".to_string(),
      input:  String::new(),
      cursor: 0,
      kind:   PromptKind::AddEntry,
    }));
    self.force_full_redraw = true;
  }

  pub(crate) fn open_rename_entry_prompt(&mut self)
  {
    let (from_path, name) = match self.selected_entry()
    {
      Some(e) => (e.path.clone(), e.name.clone()),
      None =>
      {
        self.add_message("Rename: no selection");
        return;
      }
    };
    self.overlay = Overlay::Prompt(Box::new(PromptState {
      title:  format!("Rename '{}' to:", name),
      input:  name.clone(),
      cursor: name.len(),
      kind:   PromptKind::RenameEntry { from: from_path },
    }));
    self.force_full_redraw = true;
  }

  pub(crate) fn request_delete_selected(&mut self)
  {
    crate::trace::log("[delete] request_delete_selected()".to_string());
    let path = match self.selected_entry()
    {
      Some(e) => {
        crate::trace::log(format!("[delete] selected='{}'", e.path.display()));
        e.path.clone()
      }
      None =>
      {
        self.add_message("Delete: no selection");
        crate::trace::log("[delete] no selection".to_string());
        return;
      }
    };
    crate::trace::log(format!(
      "[delete] confirm_delete flag={}",
      self.config.ui.confirm_delete
    ));
    if self.config.ui.confirm_delete
    {
      let name = path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| path.to_string_lossy().to_string());
      crate::trace::log(format!("[delete] opening confirm for '{}'", name));
      self.overlay = Overlay::Confirm(Box::new(ConfirmState {
        title:       "Confirm Delete".to_string(),
        question:    format!("Delete '{}' ? (y/n)", name),
        default_yes: false,
        kind:        ConfirmKind::DeleteEntry(path),
      }));
      self.force_full_redraw = true;
    }
    else
    {
      crate::trace::log("[delete] confirm disabled -> deleting immediately".to_string());
      self.perform_delete_path(&path);
    }
  }

  pub(crate) fn perform_delete_path(&mut self, path: &std::path::Path)
  {
    crate::trace::log(format!("[delete] perform path='{}'", path.display()));
    let res = if path.is_dir()
    {
      std::fs::remove_dir_all(path)
    }
    else
    {
      std::fs::remove_file(path)
    };
    match res
    {
      Ok(_) => {
        crate::trace::log("[delete] success".to_string());
        self.add_message("Deleted");
      }
      Err(e) => {
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
    let entry = {
      let state = match self.overlay
      {
        Overlay::ThemePicker(ref mut s) => s.as_mut(),
        _ => return,
      };
      if state.entries.is_empty()
      {
        return;
      }
      let len = state.entries.len() as isize;
      let mut new_idx = state.selected as isize + delta;
      new_idx = new_idx.clamp(0, len.saturating_sub(1));
      if new_idx as usize == state.selected
      {
        None
      }
      else
      {
        state.selected = new_idx as usize;
        Some(state.entries[state.selected].clone())
      }
    };
    if let Some(entry) = entry
    {
      self.apply_theme_entry(entry);
    }
  }

  pub(crate) fn confirm_theme_picker(&mut self)
  {
    self.overlay = Overlay::None;
    self.force_full_redraw = true;
  }

  pub(crate) fn cancel_theme_picker(&mut self)
  {
    if let Overlay::ThemePicker(state) = std::mem::replace(&mut self.overlay, Overlay::None)
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
