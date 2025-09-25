// Internal actions and sorting controls
// This module is a child of the crate root and can access crate-private items.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey
{
  Name,
  Size,
  MTime,
  CTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InternalAction
{
  Quit,
  Sort(SortKey),
  ToggleSortReverse,
  SetInfo(crate::app::InfoMode),
  SetDisplayMode(crate::app::DisplayMode),
  GoTop,
  GoBottom,
}

pub(crate) fn parse_internal_action(s: &str) -> Option<InternalAction>
{
  let low = s.trim().to_ascii_lowercase();
  if low == "quit" || low == "q"
  {
    return Some(InternalAction::Quit);
  }
  if low == "sort:reverse:toggle" || low == "sort:rev:toggle"
  {
    return Some(InternalAction::ToggleSortReverse);
  }
  if low.starts_with("sort:")
  {
    let parts: Vec<&str> = low.split(':').collect();
    if parts.len() >= 2
    {
      return crate::enums::sort_key_from_str(parts[1])
        .map(InternalAction::Sort);
    }
  }
  // Primary: show:* controls info display
  if low.starts_with("show:")
  {
    let parts: Vec<&str> = low.split(':').collect();
    if parts.len() >= 2
    {
      if parts[1] == "friendly"
      {
        return Some(InternalAction::SetDisplayMode(
          crate::app::DisplayMode::Friendly,
        ));
      }
      return crate::enums::info_mode_from_str(parts[1])
        .map(InternalAction::SetInfo);
    }
  }
  if low.starts_with("display:")
  {
    let parts: Vec<&str> = low.split(':').collect();
    if parts.len() >= 2
    {
      return crate::enums::display_mode_from_str(parts[1])
        .map(InternalAction::SetDisplayMode);
    }
  }
  if low == "nav:top" || low == "top" || low == "gg"
  {
    return Some(InternalAction::GoTop);
  }
  if low == "nav:bottom" || low == "bottom" || low == "g$"
  {
    return Some(InternalAction::GoBottom);
  }
  None
}

pub(crate) fn execute_internal_action(
  app: &mut crate::app::App,
  action: InternalAction,
)
{
  match action
  {
    InternalAction::Quit =>
    {
      app.should_quit = true;
    }
    InternalAction::Sort(key) =>
    {
      // Reselect current item by name after resort
      let current_name = app.selected_entry().map(|e| e.name.clone());
      app.sort_key = key;
      app.refresh_lists();
      if let Some(name) = current_name
        && let Some(idx) =
          app.current_entries.iter().position(|e| e.name == name)
      {
        app.list_state.select(Some(idx));
      }
      app.refresh_preview();
    }
    InternalAction::ToggleSortReverse =>
    {
      let current_name = app.selected_entry().map(|e| e.name.clone());
      app.sort_reverse = !app.sort_reverse;
      app.refresh_lists();
      if let Some(name) = current_name
        && let Some(idx) =
          app.current_entries.iter().position(|e| e.name == name)
      {
        app.list_state.select(Some(idx));
      }
      app.refresh_preview();
    }
    InternalAction::SetInfo(mode) =>
    {
      app.info_mode = mode;
      app.force_full_redraw = true;
    }
    InternalAction::SetDisplayMode(style) =>
    {
      app.display_mode = style;
      // If no info is selected yet, default to Modified so date becomes visible
      if matches!(app.info_mode, crate::app::InfoMode::None)
      {
        app.info_mode = crate::app::InfoMode::Modified;
      }
      app.force_full_redraw = true;
    }
    InternalAction::GoTop =>
    {
      if !app.current_entries.is_empty()
      {
        app.list_state.select(Some(0));
        app.refresh_preview();
      }
    }
    InternalAction::GoBottom =>
    {
      if !app.current_entries.is_empty()
      {
        let last = app.current_entries.len().saturating_sub(1);
        app.list_state.select(Some(last));
        app.refresh_preview();
      }
    }
  }
}
