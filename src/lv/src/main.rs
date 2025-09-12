use std::io;
use std::time::Duration;

// ANSI rendering and pane helpers live in ui module
use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
  EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

mod config;
mod preview;
mod cmd;
mod ui;
mod trace;
mod actions;
mod input;
mod enums;
mod config_data;
mod app;

pub use app::App;

fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut app = App::new()?;
  run_app(&mut app)?;
  Ok(())
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
          Ok(Event::Key(key)) => match input::handle_key(app, key) {
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


fn dispatch_action(app: &mut App, action: &str) -> io::Result<bool> {
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
  if let Some(int) = actions::parse_internal_action(action) {
    actions::execute_internal_action(app, int);
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
  // Build lv helpers (placeholder; reserved for future helpers)
  let lv_tbl = lua.create_table().map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
  // Build config snapshot table
  let cfg_tbl = crate::config_data::to_lua_config_table(lua, app)
    .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("build config tbl: {e}")))?;

  // Call function(lv, config): may return a table; if not, use mutated arg
  let ret_val: mlua::Value = func
    .call((lv_tbl, cfg_tbl.clone()))
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

// rebuild_keymap_lookup now lives on App in app module

// panel_title moved to ui::panes

fn ui(
  f: &mut ratatui::Frame,
  app: &mut App,
) {
  ui::draw(f, app);
}

// sanitize_line moved to app module

// trace logging moved to crate::trace
