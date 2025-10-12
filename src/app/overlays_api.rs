//! Overlay and overlays-related helpers for App.

use std::fs;

use crate::app::{
  App,
  Overlay,
  ThemePickerEntry,
  ThemePickerState,
};

impl App
{
  pub(crate) fn open_theme_picker(&mut self)
  {
    let root = match self.theme_root_dir()
    {
      Some(p) => p,
      None =>
      {
        self.add_message("Theme picker: unable to determine config directory");
        return;
      }
    };
    let themes_dir = {
      let module_dir = root.join("lua").join("themes");
      if std::fs::metadata(&module_dir).map(|m| m.is_dir()).unwrap_or(false)
      {
        module_dir
      }
      else
      {
        root.join("themes")
      }
    };
    let read_dir = match fs::read_dir(&themes_dir)
    {
      Ok(rd) => rd,
      Err(_) =>
      {
        self.add_message(&format!(
          "Theme picker: no themes directory at {}",
          themes_dir.display()
        ));
        return;
      }
    };

    let mut entries: Vec<ThemePickerEntry> = Vec::new();
    for entry in read_dir
    {
      match entry
      {
        Ok(dir_entry) =>
        {
          let path = dir_entry.path();
          if !path.is_file()
          {
            continue;
          };
          if let Some(ext) = path.extension().and_then(|s| s.to_str())
          {
            if !ext.eq_ignore_ascii_case("lua")
            {
              continue;
            }
          }
          else
          {
            continue;
          }
          match crate::config::load_theme_from_file(&path)
          {
            Ok(theme) =>
            {
              let name = path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| path.display().to_string());
              entries.push(ThemePickerEntry { name, path, theme });
            }
            Err(e) =>
            {
              self.add_message(&format!(
                "Theme picker: failed to load {} ({})",
                path.display(),
                e
              ));
            }
          }
        }
        Err(e) =>
        {
          self.add_message(&format!(
            "Theme picker: error reading themes directory ({})",
            e
          ));
        }
      }
    }

    if entries.is_empty()
    {
      self.add_message(&format!(
        "Theme picker: no .lua themes found in {}",
        themes_dir.display()
      ));
      return;
    }

    entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    let current_path = self.config.ui.theme_path.clone();
    let mut selected = 0usize;
    if let Some(cur) = current_path.as_ref()
      && let Some(idx) = entries.iter().position(|e| e.path == *cur)
    {
      selected = idx;
    }
    let state = ThemePickerState {
      entries,
      selected,
      original_theme: self.config.ui.theme.clone(),
      original_theme_path: current_path,
    };
    self.keys.pending.clear();
    self.keys.last_at = None;
    self.overlay = Overlay::ThemePicker(Box::new(state));
    self.force_full_redraw = true;
  }

  pub(crate) fn open_add_entry_prompt(&mut self)
  {
    crate::core::overlays::open_add_entry_prompt(self)
  }

  pub(crate) fn open_rename_entry_prompt(&mut self)
  {
    crate::core::overlays::open_rename_entry_prompt(self)
  }

  pub(crate) fn request_delete_selected(&mut self)
  {
    crate::core::overlays::request_delete_selected(self)
  }

  pub(crate) fn perform_delete_path(
    &mut self,
    path: &std::path::Path,
  )
  {
    crate::trace::log(format!("[delete] perform path='{}'", path.display()));
    let res = crate::core::fs_ops::remove_path_all(path);
    match res
    {
      Ok(_) =>
      {
        crate::trace::log("[delete] success");
        self.add_message("Deleted");
        self.selected.remove(path);
      }
      Err(e) =>
      {
        crate::trace::log(format!("[delete] error: {}", e));
        self.add_message(&format!("Delete error: {}", e));
      }
    }
    self.refresh_lists();
    self.refresh_preview();
  }
}
