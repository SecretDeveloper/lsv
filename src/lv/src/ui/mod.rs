pub mod ansi;
pub mod panes;

use ratatui::layout::{Direction, Layout};

pub fn draw(
  f: &mut ratatui::Frame,
  app: &mut crate::App,
) {
  let constraints = panes::pane_constraints(app);
  let chunks = Layout::default()
    .direction(Direction::Horizontal)
    .constraints(constraints)
    .split(f.area());

  panes::draw_parent_panel(f, chunks[0], app);
  panes::draw_current_panel(f, chunks[1], app);
  crate::preview::draw_preview_panel(f, chunks[2], app);

  if let Some(msg) = &app.status_error {
    panes::draw_error_bar(f, f.area(), msg);
  }

  // which-key overlay (draw last so it appears on top)
  if app.show_whichkey {
    panes::draw_whichkey_panel(f, f.area(), app);
  }
}
