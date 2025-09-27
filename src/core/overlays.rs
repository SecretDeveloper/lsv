use crate::app::{
  App,
  ConfirmKind,
  ConfirmState,
  Overlay,
  PromptKind,
  PromptState,
  ThemePickerEntry,
};
use std::path::PathBuf;

pub fn apply_theme_entry(
  app: &mut App,
  entry: ThemePickerEntry,
)
{
  app.config.ui.theme = Some(entry.theme);
  app.config.ui.theme_path = Some(entry.path);
  app.force_full_redraw = true;
}

pub fn theme_picker_move(
  app: &mut App,
  delta: isize,
)
{
  let entry = {
    let state = match app.overlay
    {
      Overlay::ThemePicker(ref mut s) => s.as_mut(),
      _ => return,
    };
    if state.entries.is_empty()
    {
      return;
    }
    let len = state.entries.len() as isize;
    let mut new_idx = state.selected as isize + delta;
    new_idx = new_idx.clamp(0, len.saturating_sub(1));
    if new_idx as usize == state.selected
    {
      None
    }
    else
    {
      state.selected = new_idx as usize;
      Some(state.entries[state.selected].clone())
    }
  };
  if let Some(entry) = entry
  {
    apply_theme_entry(app, entry);
  }
}

pub fn confirm_theme_picker(app: &mut App)
{
  app.overlay = Overlay::None;
  app.force_full_redraw = true;
}

pub fn open_add_entry_prompt(app: &mut App)
{
  app.overlay = Overlay::Prompt(Box::new(PromptState {
    title:  "Name (end with '/' for folder):".to_string(),
    input:  String::new(),
    cursor: 0,
    kind:   PromptKind::AddEntry,
  }));
  app.force_full_redraw = true;
}

pub fn open_rename_entry_prompt(app: &mut App)
{
  if !app.selected.is_empty()
  {
    let items: Vec<PathBuf> = app.selected.iter().cloned().collect();
    let names: Vec<String> = items
      .iter()
      .filter_map(|p| {
        p.file_name().and_then(|s| s.to_str()).map(|s| s.to_string())
      })
      .collect();
    if names.is_empty()
    {
      app.add_message("Rename: no valid file names selected");
      return;
    }
    let (pre, suf) = crate::app::common_affixes(&names);
    let template = format!("{}{}{}", pre, "{}", suf);
    let title = if names.len() == 1
    {
      format!("Rename '{}' to:", names[0])
    }
    else
    {
      format!("Rename {} items (use {{}} for variable part):", names.len())
    };
    app.overlay = Overlay::Prompt(Box::new(PromptState {
      title,
      input: template.clone(),
      cursor: template.len(),
      kind: PromptKind::RenameMany { items, pre, suf },
    }));
    app.force_full_redraw = true;
    return;
  }
  let (from_path, name) = match app.selected_entry()
  {
    Some(e) => (e.path.clone(), e.name.clone()),
    None =>
    {
      app.add_message("Rename: no selection");
      return;
    }
  };
  app.overlay = Overlay::Prompt(Box::new(PromptState {
    title:  format!("Rename '{}' to:", name),
    input:  name.clone(),
    cursor: name.len(),
    kind:   PromptKind::RenameEntry { from: from_path },
  }));
  app.force_full_redraw = true;
}

pub fn request_delete_selected(app: &mut App)
{
  crate::trace::log("[delete] request_delete_selected()");
  if app.selected.is_empty()
  {
    app.add_message("Delete: no items selected");
    return;
  }
  let items: Vec<PathBuf> = app.selected.iter().cloned().collect();
  if app.config.ui.confirm_delete
  {
    let question = if items.len() == 1
    {
      let name = items[0]
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| items[0].to_string_lossy().to_string());
      format!("Delete '{}' ? (y/n)", name)
    }
    else
    {
      format!("Delete {} selected items? (y/n)", items.len())
    };
    app.overlay = Overlay::Confirm(Box::new(ConfirmState {
      title: "Confirm Delete".to_string(),
      question,
      default_yes: false,
      kind: ConfirmKind::DeleteSelected(items),
    }));
    app.force_full_redraw = true;
  }
  else
  {
    for p in app.selected.clone().into_iter()
    {
      app.perform_delete_path(&p);
    }
  }
}
