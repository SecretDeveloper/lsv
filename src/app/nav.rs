//! Navigation and list refresh for App.

use std::{
  cmp::min,
  io,
  path::Path,
};

use crate::{
  actions::SortKey,
  app::{
    App,
    DirEntryInfo,
    InfoMode,
    Overlay,
  },
};

impl App
{
  pub(crate) fn selected_entry(&self) -> Option<&DirEntryInfo>
  {
    self.list_state.selected().and_then(|i| self.current_entries.get(i))
  }

  pub fn get_current_entry_name(
    &self,
    idx: usize,
  ) -> Option<String>
  {
    self.current_entries.get(idx).map(|e| e.name.clone())
  }

  pub fn select_index(
    &mut self,
    idx: usize,
  )
  {
    self.list_state.select(Some(idx));
    self.refresh_preview();
  }

  pub(crate) fn refresh_lists(&mut self)
  {
    self.current_entries = self.read_dir_sorted(&self.cwd).unwrap_or_default();
    if self.current_entries.len() > self.config.ui.max_list_items
    {
      self.current_entries.truncate(self.config.ui.max_list_items);
    }
    self.parent_entries = if let Some(p) = self.cwd.parent()
    {
      self.read_dir_sorted(p).unwrap_or_default()
    }
    else
    {
      Vec::new()
    };
    if self.parent_entries.len() > self.config.ui.max_list_items
    {
      self.parent_entries.truncate(self.config.ui.max_list_items);
    }
    // Clamp selection
    let max_idx = self.current_entries.len().saturating_sub(1);
    if let Some(sel) = self.list_state.selected()
    {
      self.list_state.select(
        if self.current_entries.is_empty()
        {
          None
        }
        else
        {
          Some(min(sel, max_idx))
        },
      );
    }
    else if !self.current_entries.is_empty()
    {
      self.list_state.select(Some(0));
    }
    // Invalidate dynamic preview cache on list refresh
    self.preview.cache_key = None;
    self.preview.cache_lines = None;
  }

  pub(crate) fn read_dir_sorted(
    &self,
    path: &Path,
  ) -> io::Result<Vec<DirEntryInfo>>
  {
    let need_meta = !matches!(self.info_mode, InfoMode::None)
      || !matches!(self.sort_key, SortKey::Name);
    crate::core::listing::read_dir_sorted(
      path,
      self.config.ui.show_hidden,
      self.sort_key,
      self.sort_reverse,
      need_meta,
      self.config.ui.max_list_items,
    )
  }

  pub fn set_cwd(
    &mut self,
    path: &Path,
  )
  {
    self.cwd = path.to_path_buf();
    self.refresh_lists();
    if !self.current_entries.is_empty()
    {
      self.list_state.select(Some(0));
      self.refresh_preview();
    }
  }

  pub fn current_has_entries(&self) -> bool
  {
    !self.current_entries.is_empty()
  }

  pub fn get_whichkey_prefix(&self) -> String
  {
    if let Overlay::WhichKey { ref prefix } = self.overlay
    {
      return prefix.clone();
    }
    String::new()
  }
}
