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
  if let Some(int) = actions::parse_internal_action(action) {
    actions::execute_internal_action(app, int);
    return Ok(true);
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
