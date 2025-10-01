use std::{
  io,
  time::Duration,
};

use crossterm::{
  event,
  event::Event,
  execute,
  terminal::{
    EnterAlternateScreen,
    LeaveAlternateScreen,
    disable_raw_mode,
    enable_raw_mode,
  },
};
use ratatui::{
  Terminal,
  backend::CrosstermBackend,
};

use crate::app::App;

pub fn run_app(app: &mut App) -> Result<(), Box<dyn std::error::Error>>
{
  enable_raw_mode()?;
  let mut stdout = io::stdout();
  execute!(stdout, EnterAlternateScreen)?;
  let backend = CrosstermBackend::new(stdout);
  let mut terminal = Terminal::new(backend)?;
  terminal.clear()?;

  // Ensure we always restore the terminal even if an error occurs during event
  // handling
  let res: Result<(), Box<dyn std::error::Error>> = {
    let mut result: Result<(), Box<dyn std::error::Error>> = Ok(());
    loop
    {
      // Drain any running preview process output into the preview buffer
      if let Some(ref rp) = app.running_preview
      {
        while let Ok(opt) = rp.rx.try_recv()
        {
          match opt
          {
            Some(line) =>
            {
              app.preview.static_lines.push(crate::util::sanitize_line(&line));
              // Trim to a reasonable bound to avoid unbounded growth
              if app.preview.static_lines.len() > 2000
              {
                let _ = app
                  .preview
                  .static_lines
                  .drain(0..app.preview.static_lines.len() - 2000);
              }
            }
            None =>
            {
              app.running_preview = None;
              // Recompute normal preview for current selection
              app.refresh_preview();
              break;
            }
          }
        }
      }
      if app.force_full_redraw
      {
        let _ = terminal.clear();
        app.force_full_redraw = false;
      }
      if let Err(e) = terminal.draw(|f| crate::ui::draw(f, app))
      {
        // Log draw errors with a backtrace for diagnostics
        let bt = std::backtrace::Backtrace::force_capture();
        crate::trace::log(format!("[runtime] draw error: {}", e));
        crate::trace::log(format!("[runtime] backtrace:\n{}", bt));
        result = Err(e.into());
        break;
      }
      match crossterm::event::poll(Duration::from_millis(200))
      {
        Ok(true) => match event::read()
        {
          Ok(Event::Key(key)) => match crate::input::handle_key(app, key)
          {
            Ok(true) => break, // graceful exit
            Ok(false) =>
            {}
            Err(e) =>
            {
              let bt = std::backtrace::Backtrace::force_capture();
              crate::trace::log(format!("[runtime] input error: {}", e));
              crate::trace::log(format!("[runtime] backtrace:\n{}", bt));
              result = Err(e.into());
              break;
            }
          },
          Ok(Event::Resize(_, _)) =>
          {}
          Ok(_) =>
          {}
          Err(e) =>
          {
            let bt = std::backtrace::Backtrace::force_capture();
            crate::trace::log(format!("[runtime] event read error: {}", e));
            crate::trace::log(format!("[runtime] backtrace:\n{}", bt));
            result = Err(e.into());
            break;
          }
        },
        Ok(false) =>
        {}
        Err(e) =>
        {
          let bt = std::backtrace::Backtrace::force_capture();
          crate::trace::log(format!("[runtime] poll error: {}", e));
          crate::trace::log(format!("[runtime] backtrace:\n{}", bt));
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
  // Clear caches tied to this session
  crate::ui::clear_owner_cache();
  res
}
