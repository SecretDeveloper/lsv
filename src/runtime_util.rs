//! Utilities for driving the runtime event loop outside the main binary.

use crate::app::App;
use crossterm::event::Event;
use std::io;

/// Process a single crossterm event and return `Ok(true)` if the app should
/// exit.
pub fn process_event(
  app: &mut App,
  ev: Event,
) -> io::Result<bool>
{
  match ev
  {
    Event::Key(key) => crate::input::handle_key(app, key),
    Event::Resize(_, _) => Ok(false),
    _ => Ok(false),
  }
}
