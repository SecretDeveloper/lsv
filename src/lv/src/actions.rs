// Internal actions and sorting controls
// This module is a child of the crate root and can access crate-private items.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SortKey {
  Name,
  Size,
  MTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InternalAction {
  Quit,
  Sort(SortKey),
  ToggleSortReverse,
}

pub(crate) fn parse_internal_action(s: &str) -> Option<InternalAction> {
  let low = s.trim().to_ascii_lowercase();
  if low == "quit" || low == "q" {
    return Some(InternalAction::Quit);
  }
  if low == "sort:reverse:toggle" || low == "sort:rev:toggle" {
    return Some(InternalAction::ToggleSortReverse);
  }
  if low.starts_with("sort:") {
    let parts: Vec<&str> = low.split(':').collect();
    if parts.len() >= 2 {
      return match parts[1] {
        "name" => Some(InternalAction::Sort(SortKey::Name)),
        "size" => Some(InternalAction::Sort(SortKey::Size)),
        "mtime" | "time" | "date" => Some(InternalAction::Sort(SortKey::MTime)),
        _ => None,
      };
    }
  }
  None
}

pub(crate) fn execute_internal_action(app: &mut crate::App, action: InternalAction) {
  match action {
    InternalAction::Quit => {
      app.should_quit = true;
    }
    InternalAction::Sort(key) => {
      // Reselect current item by name after resort
      let current_name = app.selected_entry().map(|e| e.name.clone());
      app.sort_key = key;
      app.refresh_lists();
      if let Some(name) = current_name {
        if let Some(idx) = app
          .current_entries
          .iter()
          .position(|e| e.name == name)
        {
          app.list_state.select(Some(idx));
        }
      }
      app.refresh_preview();
    }
    InternalAction::ToggleSortReverse => {
      let current_name = app.selected_entry().map(|e| e.name.clone());
      app.sort_reverse = !app.sort_reverse;
      app.refresh_lists();
      if let Some(name) = current_name {
        if let Some(idx) = app
          .current_entries
          .iter()
          .position(|e| e.name == name)
        {
          app.list_state.select(Some(idx));
        }
      }
      app.refresh_preview();
    }
  }
}

