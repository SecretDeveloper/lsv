//! Selection and clipboard operations for App.

use crate::app::{
  App,
  Clipboard,
  ClipboardOp,
};

impl App
{
  pub(crate) fn toggle_select_current(&mut self)
  {
    if let Some(e) = self.selected_entry().cloned()
    {
      if self.selected.contains(&e.path)
      {
        self.selected.remove(&e.path);
      }
      else
      {
        self.selected.insert(e.path);
      }
    }
  }

  pub(crate) fn clear_all_selected(&mut self)
  {
    if !self.selected.is_empty()
    {
      self.selected.clear();
    }
  }

  pub(crate) fn copy_selection(&mut self)
  {
    let items: Vec<std::path::PathBuf> =
      self.selected.iter().cloned().collect();
    if items.is_empty()
    {
      self.add_message("Copy: no items selected");
      return;
    }
    self.clipboard = Some(Clipboard { op: ClipboardOp::Copy, items });
    self.add_message("Copied selection to clipboard");
    self.force_full_redraw = true;
  }

  pub(crate) fn move_selection(&mut self)
  {
    let items: Vec<std::path::PathBuf> =
      self.selected.iter().cloned().collect();
    if items.is_empty()
    {
      self.add_message("Move: no items selected");
      return;
    }
    self.clipboard = Some(Clipboard { op: ClipboardOp::Move, items });
    self.add_message("Move selection armed");
    self.force_full_redraw = true;
  }

  pub(crate) fn clear_clipboard(&mut self)
  {
    self.clipboard = None;
    self.add_message("Clipboard cleared");
    self.force_full_redraw = true;
  }

  pub(crate) fn paste_clipboard(&mut self)
  {
    let Some(cb) = self.clipboard.clone()
    else
    {
      self.add_message("Paste: clipboard empty");
      return;
    };
    let dest_dir = self.cwd.clone();
    let mut ok = 0usize;
    let mut skipped = 0usize;
    let mut errs = 0usize;
    for src in cb.items.iter()
    {
      if matches!(cb.op, ClipboardOp::Move) && dest_dir.starts_with(src)
      {
        self
          .add_message(&format!("Skip (move into subdir): {}", src.display()));
        skipped += 1;
        continue;
      }
      let Some(name) = src.file_name()
      else
      {
        skipped += 1;
        continue;
      };
      let dest_path = dest_dir.join(name);
      if dest_path.exists()
      {
        self.add_message(&format!("Skip (exists): {}", dest_path.display()));
        skipped += 1;
        continue;
      }
      let res = match cb.op
      {
        ClipboardOp::Copy =>
        {
          crate::core::fs_ops::copy_path_recursive(src, &dest_path)
        }
        ClipboardOp::Move =>
        {
          crate::core::fs_ops::move_path_with_fallback(src, &dest_path)
        }
      };
      match res
      {
        Ok(()) => ok += 1,
        Err(e) =>
        {
          errs += 1;
          self.add_message(&format!(
            "Error: {} -> {}: {}",
            src.display(),
            dest_path.display(),
            e
          ));
        }
      }
    }
    if matches!(cb.op, ClipboardOp::Move)
    {
      for p in cb.items.iter()
      {
        self.selected.remove(p);
      }
    }
    self.clipboard = None;
    self.refresh_lists();
    self.refresh_preview();
    self.add_message(&format!(
      "Paste: ok={} skipped={} errors={}",
      ok, skipped, errs
    ));
    self.force_full_redraw = true;
  }
}
