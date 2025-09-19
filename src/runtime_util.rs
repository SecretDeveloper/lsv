use std::io;
use crossterm::event::Event;
use crate::app::App;

// Process a single crossterm event and return true if the app should exit.
pub fn process_event(app: &mut App, ev: Event) -> io::Result<bool> {
  match ev {
    Event::Key(key) => crate::input::handle_key(app, key),
    Event::Resize(_, _) => Ok(false),
    _ => Ok(false),
  }
}

