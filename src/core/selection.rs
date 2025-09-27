/// Reselect the item with the given name in the current entries, if present.
pub fn reselect_by_name(
  app: &mut crate::app::App,
  name: &str,
)
{
  if let Some(idx) = app.current_entries.iter().position(|e| e.name == name)
  {
    app.list_state.select(Some(idx));
  }
}
