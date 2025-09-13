use std::cmp::min;
use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use mlua::RegistryKey;
use ratatui::widgets::ListState;

use crate::actions::SortKey;

#[derive(Debug, Clone)]
pub struct DirEntryInfo {
  pub(crate) name: String,
  pub(crate) path: PathBuf,
  pub(crate) is_dir: bool,
  pub(crate) size: u64,
  pub(crate) mtime: Option<SystemTime>,
  pub(crate) ctime: Option<SystemTime>,
}

pub struct App {
  pub(crate) cwd: PathBuf,
  pub(crate) parent: Option<PathBuf>,
  pub(crate) current_entries: Vec<DirEntryInfo>,
  pub(crate) parent_entries: Vec<DirEntryInfo>,
  pub(crate) list_state: ListState,
  pub(crate) preview_lines: Vec<String>,
  pub(crate) preview_title: String,
  pub(crate) config_paths: Option<crate::config::ConfigPaths>,
  pub(crate) config: crate::config::Config,
  pub(crate) keymaps: Vec<crate::config::KeyMapping>,
  pub(crate) keymap_lookup: std::collections::HashMap<String, String>,
  pub(crate) force_full_redraw: bool,
  pub(crate) status_error: Option<String>,
  pub(crate) lua_engine: Option<crate::config::LuaEngine>,
  pub(crate) previewer_fn: Option<RegistryKey>,
  pub(crate) lua_action_fns: Option<Vec<RegistryKey>>,
  // In-memory runtime settings
  pub(crate) sort_key: SortKey,
  pub(crate) sort_reverse: bool,
  pub(crate) info_mode: InfoMode,
  pub(crate) display_mode: DisplayMode,
  // Signal to exit after handling a key/action
  pub(crate) should_quit: bool,
  // Key sequence handling
  pub(crate) pending_seq: String,
  pub(crate) last_seq_time: Option<std::time::Instant>,
  pub(crate) prefix_set: std::collections::HashSet<String>,
  // Which-key panel state
  pub(crate) show_whichkey: bool,
  pub(crate) whichkey_prefix: String,
}

impl App {
  pub fn new() -> io::Result<Self> {
    let cwd = env::current_dir()?;
    let parent = cwd.parent().map(|p| p.to_path_buf());
    // Temporary initial read with default sort (Name asc)
    let current_entries = {
      // Build a temporary App-like context for sorting
      let mut tmp = Vec::new();
      for e in fs::read_dir(&cwd)? {
        if let Ok(de) = e {
          let path = de.path();
          let name = de.file_name().to_string_lossy().to_string();
          if let Ok(ft) = de.file_type() {
            let meta = fs::metadata(&path).ok();
            let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            let mtime = meta.as_ref().and_then(|m| m.modified().ok());
            let ctime = meta.as_ref().and_then(|m| m.created().ok());
            tmp.push(DirEntryInfo { name, path, is_dir: ft.is_dir(), size, mtime, ctime });
          }
        }
      }
      tmp.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
      });
      tmp
    };
    let parent_entries = if let Some(ref p) = parent {
      // Same initial read for parent
      let mut tmp = Vec::new();
      for e in fs::read_dir(p)? {
        if let Ok(de) = e {
          let path = de.path();
          let name = de.file_name().to_string_lossy().to_string();
          if let Ok(ft) = de.file_type() {
            let meta = fs::metadata(&path).ok();
            let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            let mtime = meta.as_ref().and_then(|m| m.modified().ok());
            let ctime = meta.as_ref().and_then(|m| m.created().ok());
            tmp.push(DirEntryInfo { name, path, is_dir: ft.is_dir(), size, mtime, ctime });
          }
        }
      }
      tmp.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
      });
      tmp
    } else { Vec::new() };

    let mut list_state = ListState::default();
    if !current_entries.is_empty() {
      list_state.select(Some(0));
    }
    let mut app = Self {
      cwd,
      parent,
      current_entries,
      parent_entries,
      list_state,
      preview_lines: Vec::new(),
      preview_title: String::new(),
      config_paths: None,
      config: crate::config::Config::default(),
      keymaps: Vec::new(),
      keymap_lookup: std::collections::HashMap::new(),
      force_full_redraw: false,
      status_error: None,
      lua_engine: None,
      previewer_fn: None,
      lua_action_fns: None,
      sort_key: SortKey::Name,
      sort_reverse: false,
      info_mode: InfoMode::None,
      display_mode: DisplayMode::Absolute,
      should_quit: false,
      pending_seq: String::new(),
      last_seq_time: None,
      prefix_set: std::collections::HashSet::new(),
      show_whichkey: false,
      whichkey_prefix: String::new(),
    };
    // Discover configuration paths (entry not executed yet)
    if let Ok(paths) = crate::config::discover_config_paths() {
      match crate::config::load_config(&paths) {
        Ok((cfg, maps, engine_opt)) => {
          app.config_paths = Some(paths);
          app.config = cfg;
          app.keymaps = maps;
          app.add_default_keymaps();
          app.rebuild_keymap_lookup();
          app.status_error = None;
          if let Some((eng, key, action_keys)) = engine_opt {
            app.lua_engine = Some(eng);
            app.previewer_fn = Some(key);
            app.lua_action_fns = Some(action_keys);
          } else {
            app.lua_engine = None;
            app.previewer_fn = None;
            app.lua_action_fns = None;
          }
          // Re-apply lists to honor config (e.g., show_hidden)
          // Also apply optional initial sort/show from config.ui
          if let Some(ref srt) = app.config.ui.sort {
            if let Some(k) = crate::enums::sort_key_from_str(srt) {
              app.sort_key = k;
            }
          }
          if let Some(b) = app.config.ui.sort_reverse {
            app.sort_reverse = b;
          }
          if let Some(ref sh) = app.config.ui.show {
            if sh.eq_ignore_ascii_case("none") {
              app.info_mode = crate::app::InfoMode::None;
            } else if let Some(m) = crate::enums::info_mode_from_str(sh) {
              app.info_mode = m;
            }
          }
          app.refresh_lists();
          // Apply display_mode from config if present
          if let Some(dm) = app.config.ui.display_mode.as_deref() {
            if let Some(mode) = crate::enums::display_mode_from_str(dm) {
              app.display_mode = mode;
            }
          }
        }
        Err(e) => {
          eprintln!("lsv: config load error: {}", e);
          app.config_paths = Some(paths);
          app.status_error = Some(format!("Config error: {}", e));
        }
      }
    }
    app.refresh_preview();
    Ok(app)
  }

  pub(crate) fn selected_entry(&self) -> Option<&DirEntryInfo> {
    self.list_state.selected().and_then(|i| self.current_entries.get(i))
  }

  pub(crate) fn refresh_lists(&mut self) {
    self.parent = self.cwd.parent().map(|p| p.to_path_buf());
    self.current_entries = self
      .read_dir_sorted(&self.cwd)
      .unwrap_or_default();
    if self.current_entries.len() > self.config.ui.max_list_items {
      self.current_entries.truncate(self.config.ui.max_list_items);
    }
    self.parent_entries = if let Some(ref p) = self.parent {
      self.read_dir_sorted(p).unwrap_or_default()
    } else {
      Vec::new()
    };
    if self.parent_entries.len() > self.config.ui.max_list_items {
      self.parent_entries.truncate(self.config.ui.max_list_items);
    }
    // Clamp selection
    let max_idx = self.current_entries.len().saturating_sub(1);
    if let Some(sel) = self.list_state.selected() {
      self.list_state.select(if self.current_entries.is_empty() {
        None
      } else {
        Some(min(sel, max_idx))
      });
    } else if !self.current_entries.is_empty() {
      self.list_state.select(Some(0));
    }
  }

  pub(crate) fn refresh_preview(&mut self) {
    // Avoid borrowing self while mutating by cloning the needed fields first
    let (is_dir, path) = match self.selected_entry() {
      Some(e) => (e.is_dir, e.path.clone()),
      None => {
        self.preview_title.clear();
        self.preview_lines.clear();
        return;
      }
    };

    let preview_limit = self.config.ui.preview_lines;
    if is_dir {
      self.preview_title = format!("dir: {}", path.display());
      match self.read_dir_sorted(&path) {
        Ok(list) => {
          let mut lines = Vec::new();
          for e in list.into_iter().take(preview_limit) {
            let marker = if e.is_dir { "/" } else { "" };
            let formatted = format!("{}{}", e.name, marker);
            lines.push(crate::util::sanitize_line(&formatted));
          }
          self.preview_lines = lines;
        }
        Err(err) => {
          self.preview_lines =
            vec![format!("<error reading directory: {}>", err)];
        }
      }
    } else {
      self.preview_title = format!("file: {}", path.display());
      self.preview_lines = crate::util::read_file_head(&path, preview_limit)
        .map(|v| v.into_iter().map(|s| crate::util::sanitize_line(&s)).collect())
        .unwrap_or_else(|e| vec![format!("<error reading file: {}>", e)]);
    }
  }

  pub(crate) fn read_dir_sorted(&self, path: &Path) -> io::Result<Vec<DirEntryInfo>> {
    let mut entries: Vec<DirEntryInfo> = fs::read_dir(path)?
      .filter_map(|res| res.ok())
      .filter_map(|e| {
        let path = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if !self.config.ui.show_hidden && name.starts_with('.') {
          return None;
        }
        match e.file_type() {
          Ok(ft) => {
            let meta = fs::metadata(&path).ok();
            let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            let mtime = meta
              .as_ref()
              .and_then(|m| m.modified().ok());
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
      match (a.is_dir, b.is_dir) {
        (true, false) => return std::cmp::Ordering::Less,
        (false, true) => return std::cmp::Ordering::Greater,
        _ => {}
      }

      let ord = match sort_key {
        SortKey::Name => a
          .name
          .to_lowercase()
          .cmp(&b.name.to_lowercase()),
        SortKey::Size => a.size.cmp(&b.size),
        SortKey::MTime => {
          let at = a
            .mtime
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
          let bt = b
            .mtime
            .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
          at.cmp(&bt)
        }
      };
      if reverse { ord.reverse() } else { ord }
    });

    Ok(entries)
  }

  pub(crate) fn rebuild_keymap_lookup(&mut self) {
    self.keymap_lookup.clear();
    self.prefix_set.clear();
    for m in &self.keymaps {
      self.keymap_lookup.insert(m.sequence.clone(), m.action.clone());
      // collect prefixes for sequence matching
      let s = &m.sequence;
      let mut chars = s.chars();
      let mut prefix = String::new();
      while let Some(c) = chars.next() {
        prefix.push(c);
        // do not include the full sequence as prefix-only
        if prefix.len() < s.len() {
          self.prefix_set.insert(prefix.clone());
        }
      }
    }
  }

  pub(crate) fn add_default_keymaps(&mut self) {
    // Defaults are provided by builtin Lua (loaded before user config).
    // No Rust-managed defaults here.
  }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfoMode {
  None,
  Size,
  Created,
  Modified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode {
  Absolute,
  Friendly,
}
