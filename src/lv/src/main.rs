use std::cmp::min;
use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::Duration;

// ANSI rendering and pane helpers live in ui module
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
  EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use mlua::RegistryKey;
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::widgets::ListState;

mod config;
mod preview;
mod cmd;
mod ui;
mod trace;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut app = App::new()?;
  run_app(&mut app)?;
  Ok(())
}

pub struct DirEntryInfo {
  name: String,
  path: PathBuf,
  is_dir: bool,
}

pub struct App {
  pub(crate) cwd: PathBuf,
  pub(crate) parent: Option<PathBuf>,
  pub(crate) current_entries: Vec<DirEntryInfo>,
  pub(crate) parent_entries: Vec<DirEntryInfo>,
  pub(crate) list_state: ListState,
  pub(crate) preview_lines: Vec<String>,
  pub(crate) preview_title: String,
  pub(crate) config_paths: Option<config::ConfigPaths>,
  pub(crate) config: config::Config,
  pub(crate) keymaps: Vec<config::KeyMapping>,
  pub(crate) keymap_lookup: std::collections::HashMap<String, String>,
  pub(crate) force_full_redraw: bool,
  pub(crate) status_error: Option<String>,
  pub(crate) lua_engine: Option<config::LuaEngine>,
  pub(crate) previewer_fn: Option<RegistryKey>,
}

impl App {
  fn new() -> io::Result<Self> {
    let cwd = env::current_dir()?;
    let parent = cwd.parent().map(|p| p.to_path_buf());
    let current_entries = read_dir_sorted(&cwd)?;
    let parent_entries =
      if let Some(ref p) = parent { read_dir_sorted(p)? } else { Vec::new() };

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
      config: config::Config::default(),
      keymaps: Vec::new(),
      keymap_lookup: std::collections::HashMap::new(),
      force_full_redraw: false,
      status_error: None,
      lua_engine: None,
      previewer_fn: None,
    };
    // Discover configuration paths (entry not executed yet)
    if let Ok(paths) = crate::config::discover_config_paths() {
      match crate::config::load_config(&paths) {
        Ok((cfg, maps, engine_opt)) => {
          app.config_paths = Some(paths);
          app.config = cfg;
          app.keymaps = maps;
          app.rebuild_keymap_lookup();
          app.status_error = None;
          if let Some((eng, key)) = engine_opt {
            app.lua_engine = Some(eng);
            app.previewer_fn = Some(key);
          } else {
            app.lua_engine = None;
            app.previewer_fn = None;
          }
        }
        Err(e) => {
          eprintln!("lv: config load error: {}", e);
          app.config_paths = Some(paths);
          app.status_error = Some(format!("Config error: {}", e));
        }
      }
    }
    app.refresh_preview();
    Ok(app)
  }

  fn selected_entry(&self) -> Option<&DirEntryInfo> {
    self.list_state.selected().and_then(|i| self.current_entries.get(i))
  }

  fn refresh_lists(&mut self) {
    self.parent = self.cwd.parent().map(|p| p.to_path_buf());
    self.current_entries = read_dir_sorted(&self.cwd).unwrap_or_default();
    if self.current_entries.len() > self.config.ui.max_list_items {
      self.current_entries.truncate(self.config.ui.max_list_items);
    }
    self.parent_entries = if let Some(ref p) = self.parent {
      read_dir_sorted(p).unwrap_or_default()
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

  fn refresh_preview(&mut self) {
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
      match read_dir_sorted(&path) {
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
}

fn read_dir_sorted(path: &Path) -> io::Result<Vec<DirEntryInfo>> {
  let mut entries: Vec<DirEntryInfo> = fs::read_dir(path)?
    .filter_map(|res| res.ok())
    .filter_map(|e| {
      let path = e.path();
      let name = e.file_name().to_string_lossy().to_string();
      match e.file_type() {
        Ok(ft) => Some(DirEntryInfo { name, path, is_dir: ft.is_dir() }),
        Err(_) => None,
      }
    })
    .collect();
  entries.sort_by(|a, b| match (a.is_dir, b.is_dir) {
    (true, false) => std::cmp::Ordering::Less,
    (false, true) => std::cmp::Ordering::Greater,
    _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
  });
  Ok(entries)
}

fn read_file_head(
  path: &Path,
  n: usize,
) -> io::Result<Vec<String>> {
  let file = File::open(path)?;
  let reader = BufReader::new(file);
  let mut lines = Vec::new();
  for (i, line) in reader.lines().enumerate() {
    if i >= n {
      break;
    }
    lines.push(line.unwrap_or_default());
  }
  Ok(lines)
}

fn run_app(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
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
          Ok(Event::Key(key)) => match handle_key(app, key) {
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

fn handle_key(
  app: &mut App,
  key: KeyEvent,
) -> io::Result<bool> {
  // First, try dynamic key mappings (single key only for now)
  if let KeyCode::Char(ch) = key.code {
    // Allow plain or SHIFT-modified letters; ignore Ctrl/Alt/Super
    let disallowed = key.modifiers.contains(KeyModifiers::CONTROL)
      || key.modifiers.contains(KeyModifiers::ALT)
      || key.modifiers.contains(KeyModifiers::SUPER);
    if !disallowed {
      let mut tried = std::collections::HashSet::new();
      for k in [
        ch.to_string(),
        ch.to_ascii_lowercase().to_string(),
        ch.to_ascii_uppercase().to_string(),
      ] {
        if !tried.insert(k.clone()) {
          continue;
        }
        if let Some(action) = app.keymap_lookup.get(&k).cloned() {
          if dispatch_action(app, &action).unwrap_or(false) {
            return Ok(false);
          }
        }
      }
    }
  }
  match (key.code, key.modifiers) {
    (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => return Ok(true),
    (KeyCode::Up, _) | (KeyCode::Char('k'), _) => {
      if let Some(sel) = app.list_state.selected() {
        if sel > 0 {
          app.list_state.select(Some(sel - 1));
          app.refresh_preview();
        }
      }
    }
    (KeyCode::Down, _) | (KeyCode::Char('j'), _) => {
      if let Some(sel) = app.list_state.selected() {
        if sel + 1 < app.current_entries.len() {
          app.list_state.select(Some(sel + 1));
          app.refresh_preview();
        }
      } else if !app.current_entries.is_empty() {
        app.list_state.select(Some(0));
        app.refresh_preview();
      }
    }
    (KeyCode::Enter, _) | (KeyCode::Right, _) => {
      if let Some(entry) = app.selected_entry() {
        if entry.is_dir {
          app.cwd = entry.path.clone();
          app.refresh_lists();
          app.refresh_preview();
        }
      }
    }
    (KeyCode::Backspace, _)
    | (KeyCode::Left, _)
    | (KeyCode::Char('h'), KeyModifiers::NONE) => {
      if let Some(parent) = app.cwd.parent() {
        // Remember the directory name we are leaving so we can reselect it
        let just_left =
          app.cwd.file_name().map(|s| s.to_string_lossy().to_string());
        app.cwd = parent.to_path_buf();
        app.refresh_lists();
        if let Some(name) = just_left {
          if let Some(idx) =
            app.current_entries.iter().position(|e| e.name == name)
          {
            app.list_state.select(Some(idx));
          }
        }
        app.refresh_preview();
      }
    }
    _ => {}
  }
  Ok(false)
}

fn dispatch_action(
  app: &mut App,
  action: &str,
) -> io::Result<bool> {
  if let Some(rest) = action.strip_prefix("run_shell:") {
    if let Ok(idx) = rest.parse::<usize>() {
      if idx < app.config.shell_cmds.len() {
                let sc = app.config.shell_cmds[idx].clone();
                crate::cmd::run_shell_command(app, &sc);
                return Ok(true);
      }
    }
  }
  Ok(false)
}

fn shell_escape(s: &str) -> String {
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

impl App {
  fn rebuild_keymap_lookup(&mut self) {
    self.keymap_lookup.clear();
    for m in &self.keymaps {
      // Only support single-key for now
      if m.sequence.chars().count() == 1 {
        self.keymap_lookup.insert(m.sequence.clone(), m.action.clone());
      }
    }
  }
}

// panel_title moved to ui::panes

fn ui(
  f: &mut ratatui::Frame,
  app: &mut App,
) {
  ui::draw(f, app);
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

// trace logging moved to crate::trace
