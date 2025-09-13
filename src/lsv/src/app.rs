use std::cmp::min;
use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use crossterm::{event, execute};
use crossterm::event::Event;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};

use mlua::RegistryKey;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
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
            lines.push(sanitize_line(&formatted));
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
      self.preview_lines = read_file_head(&path, preview_limit)
        .map(|v| v.into_iter().map(|s| sanitize_line(&s)).collect())
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
    fn ensure(
      maps: &mut Vec<crate::config::KeyMapping>,
      seq: &str,
      action: &str,
      desc: &str,
    ) {
      if !maps.iter().any(|m| m.sequence == seq) {
        maps.push(crate::config::KeyMapping {
          sequence: seq.to_string(),
          action: action.to_string(),
          description: Some(desc.to_string()),
        });
      }
    }
    ensure(&mut self.keymaps, "q", "quit", "Quit lsv");
    ensure(&mut self.keymaps, "ss", "sort:size", "Sort by size");
    ensure(&mut self.keymaps, "sn", "sort:name", "Sort by name");
    ensure(
      &mut self.keymaps,
      "sr",
      "sort:reverse:toggle",
      "Toggle reverse sort",
    );
    // Info panel toggles under 'z'
    ensure(&mut self.keymaps, "zn", "info:none", "Info: none");
    ensure(&mut self.keymaps, "zs", "info:size", "Info: size");
    ensure(&mut self.keymaps, "zc", "info:created", "Info: created date");
    ensure(&mut self.keymaps, "zm", "info:modified", "Info: modified date");
    // Use 'za' for friendly date style
    ensure(&mut self.keymaps, "za", "display:friendly", "Display: friendly");
    ensure(&mut self.keymaps, "zf", "display:absolute", "Display: absolute");
  }
}

fn read_file_head(
  path: &Path,
  n: usize,
) -> io::Result<Vec<String>> {
  let file = File::open(path)?;
  let reader = BufReader::new(file);
  let mut lines = Vec::new();
  for (i, line) in reader.lines().enumerate() {
    if i >= n { break; }
    lines.push(line.unwrap_or_default());
  }
  Ok(lines)
}

fn sanitize_line(s: &str) -> String {
  let mut out = String::with_capacity(s.len());
  for ch in s.chars() {
    match ch {
      '\t' => out.push_str("    "),
      '\r' => {}
      c if c.is_control() => out.push(' '),
      c => out.push(c),
    }
  }
  out
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

pub fn run_app(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;
    // Ensure we always restore the terminal even if an error occurs during event handling
    let res: Result<(), Box<dyn std::error::Error>> = {
        let mut result: Result<(), Box<dyn std::error::Error>> = Ok(());
        loop {
            if app.force_full_redraw {
                let _ = terminal.clear();
                app.force_full_redraw = false;
            }
            if let Err(e) = terminal.draw(|f| ui(f, app)) {
                result = Err(e.into());
                break;
            }
            match crossterm::event::poll(Duration::from_millis(200)) {
                Ok(true) => match event::read() {
                    Ok(Event::Key(key)) => match crate::input::handle_key(app, key) {
                        Ok(true) => break, // graceful exit
                        Ok(false) => {}
                        Err(e) => {
                            result = Err(e.into());
                            break;
                        }
                    },
                    Ok(Event::Resize(_, _)) => {}
                    Ok(_) => {}
                    Err(e) => {
                        result = Err(e.into());
                        break;
                    }
                },
                Ok(false) => {}
                Err(e) => {
                    result = Err(e.into());
                    break;
                }
            }
        }
        result
    };
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    res
}

pub(crate) fn dispatch_action(app: &mut App, action: &str) -> io::Result<bool> {
    // Support multiple commands separated by ';'
    let parts: Vec<&str> = action.split(';').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
    if parts.len() > 1 {
        let mut any = false;
        for p in parts {
            if run_single_action(app, p)? {
                any = true;
            }
            if app.should_quit { break; }
        }
        return Ok(any);
    }
    run_single_action(app, action)
}

fn run_single_action(app: &mut App, action: &str) -> io::Result<bool> {
    if let Some(rest) = action.strip_prefix("run_shell:") {
        if let Ok(idx) = rest.parse::<usize>() {
            if idx < app.config.shell_cmds.len() {
                let sc = app.config.shell_cmds[idx].clone();
                crate::cmd::run_shell_command(app, &sc);
                return Ok(true);
            }
        }
    }
    if let Some(rest) = action.strip_prefix("run_lua:") {
        if let Ok(idx) = rest.parse::<usize>() {
            return run_lua_action(app, idx);
        }
    }
    if let Some(int) = crate::actions::parse_internal_action(action) {
        crate::actions::execute_internal_action(app, int);
        return Ok(true);
    }
    Ok(false)
}

fn run_lua_action(app: &mut App, idx: usize) -> io::Result<bool> {
    let (engine, funcs) = match (app.lua_engine.as_ref(), app.lua_action_fns.as_ref()) {
        (Some(eng), Some(vec)) => (eng, vec),
        _ => return Ok(false),
    };
    if idx >= funcs.len() { return Ok(false); }
    let lua = engine.lua();
    let func = lua.registry_value::<mlua::Function>(&funcs[idx])
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("lua fn lookup: {e}")))?;
    // Build lsv helpers (placeholder; reserved for future helpers)
    let lsv_tbl = lua.create_table().map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    // Build config snapshot table
    let cfg_tbl = crate::config_data::to_lua_config_table(lua, app)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("build config tbl: {e}")))?;
    // Call function(lsv, config): may return a table; if not, use mutated arg
    let ret_val: mlua::Value = func
        .call((lsv_tbl, cfg_tbl.clone()))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("lua fn: {e}")))?;
    let candidate_tbl = match ret_val {
        mlua::Value::Table(t) => t,
        _ => cfg_tbl,
    };
    // Validate and apply
    let data = crate::config_data::from_lua_config_table(candidate_tbl)
        .map_err(|msg| io::Error::new(io::ErrorKind::Other, format!("Config validation error: {}", msg)))?;
    apply_config_data(app, &data);
    Ok(true)
}

fn apply_config_data(app: &mut App, data: &crate::config_data::ConfigData) {
    // Collect diffs
    let mut relist = false;
    let mut redraw_only = false;
    let mut layout_change = false;
    let mut refresh_preview_only = false;

    // Capture current selection name for reselection on relist
    let selected_name = app.selected_entry().map(|e| e.name.clone());

    // Keys
    if app.config.keys.sequence_timeout_ms != data.keys_sequence_timeout_ms {
        app.config.keys.sequence_timeout_ms = data.keys_sequence_timeout_ms;
    }

    // UI panes
    let current_panes = app.config.ui.panes.clone().unwrap_or(crate::config::UiPanes { parent: 30, current: 40, preview: 30 });
    if current_panes.parent != data.ui.panes.parent
    || current_panes.current != data.ui.panes.current
    || current_panes.preview != data.ui.panes.preview
    {
        layout_change = true;
        app.config.ui.panes = Some(crate::config::UiPanes {
            parent: data.ui.panes.parent,
            current: data.ui.panes.current,
            preview: data.ui.panes.preview,
        });
    }

    // Hidden files affects listing
    if app.config.ui.show_hidden != data.ui.show_hidden {
        app.config.ui.show_hidden = data.ui.show_hidden;
        relist = true;
    }

    // Date format affects render only
    if app.config.ui.date_format != data.ui.date_format {
        app.config.ui.date_format = data.ui.date_format.clone();
        redraw_only = true;
    }

    // Display mode affects render only
    if app.display_mode != data.ui.display_mode {
        app.display_mode = data.ui.display_mode;
        redraw_only = true;
    }

    // Preview lines changes preview output trimming
    if app.config.ui.preview_lines != data.ui.preview_lines {
        app.config.ui.preview_lines = data.ui.preview_lines;
        refresh_preview_only = true;
    }

    // Max list items impacts listing/retrieval
    if app.config.ui.max_list_items != data.ui.max_list_items {
        app.config.ui.max_list_items = data.ui.max_list_items;
        relist = true;
    }

    // Row templates affect render only
    let current_row = app.config.ui.row.clone().unwrap_or_default();
    if current_row.icon != data.ui.row.icon
    || current_row.left != data.ui.row.left
    || current_row.middle != data.ui.row.middle
    || current_row.right != data.ui.row.right
    {
        app.config.ui.row = Some(crate::config::UiRowFormat {
            icon: data.ui.row.icon.clone(),
            left: data.ui.row.left.clone(),
            middle: data.ui.row.middle.clone(),
            right: data.ui.row.right.clone(),
        });
        redraw_only = true;
    }

    // Sorting affects listing
    if app.sort_key != data.sort_key || app.sort_reverse != data.sort_reverse {
        app.sort_key = data.sort_key;
        app.sort_reverse = data.sort_reverse;
        relist = true;
    }

    // Info field affects render only
    if app.info_mode != data.show_field {
        app.info_mode = data.show_field;
        redraw_only = true;
    }

    // Apply effects
    if relist {
        app.refresh_lists();
        if let Some(name) = selected_name.as_ref() {
            if let Some(idx) = app.current_entries.iter().position(|e| &e.name == name) {
                app.list_state.select(Some(idx));
            }
        }
        app.refresh_preview();
        app.force_full_redraw = true;
        return;
    }

    if refresh_preview_only {
        app.refresh_preview();
    }

    if redraw_only || layout_change {
        app.force_full_redraw = true;
    }
}

pub(crate) fn shell_escape(s: &str) -> String {
    if s.is_empty() {
        "''".to_string()
    } else {
        let mut out = String::from("'");
        for ch in s.chars() {
            if ch == '\'' {
                out.push_str("'\\''");
            } else {
                out.push(ch);
            }
        }
        out.push('\'');
        out
    }
}

fn ui(
    f: &mut ratatui::Frame,
    app: &mut App,
) {
    crate::ui::draw(f, app);
}
