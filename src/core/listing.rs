use std::{
  io,
  path::Path,
};

use crate::actions::internal::SortKey;

/// Read a directory and return entries sorted per key and direction.
/// Hidden files (dotfiles) are filtered when `show_hidden` is false.
pub fn read_dir_sorted(
  path: &Path,
  show_hidden: bool,
  sort_key: SortKey,
  sort_reverse: bool,
  need_meta: bool,
  max_items: usize,
) -> io::Result<Vec<crate::app::DirEntryInfo>>
{
  use std::fs;
  let mut entries: Vec<crate::app::DirEntryInfo> = fs::read_dir(path)?
    .filter_map(|res| res.ok())
    .filter_map(|e| {
      let path = e.path();
      let name = e.file_name().to_string_lossy().to_string();
      if !show_hidden && name.starts_with('.')
      {
        return None;
      }
      match e.file_type()
      {
        Ok(ft) =>
        {
          if need_meta && !matches!(sort_key, SortKey::Name)
          {
            // Sorting by size/mtime/ctime requires metadata for accuracy
            let meta = fs::metadata(&path).ok();
            let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            let mtime = meta.as_ref().and_then(|m| m.modified().ok());
            let ctime = meta.as_ref().and_then(|m| m.created().ok());
            Some(crate::app::DirEntryInfo {
              name,
              path,
              is_dir: ft.is_dir(),
              size,
              mtime,
              ctime,
            })
          }
          else if need_meta
          {
            // Name sort but meta requested for UI info; fetch once
            let meta = fs::metadata(&path).ok();
            let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            let mtime = meta.as_ref().and_then(|m| m.modified().ok());
            let ctime = meta.as_ref().and_then(|m| m.created().ok());
            Some(crate::app::DirEntryInfo {
              name,
              path,
              is_dir: ft.is_dir(),
              size,
              mtime,
              ctime,
            })
          }
          else
          {
            // Fast path: avoid metadata when not needed
            Some(crate::app::DirEntryInfo {
              name,
              path,
              is_dir: ft.is_dir(),
              size: 0,
              mtime: None,
              ctime: None,
            })
          }
        }
        Err(_) => None,
      }
    })
    .take(max_items)
    .collect();

  entries.sort_by(|a, b| {
    // Always keep directories before files
    match (a.is_dir, b.is_dir)
    {
      (true, false) => return std::cmp::Ordering::Less,
      (false, true) => return std::cmp::Ordering::Greater,
      _ =>
      {}
    }
    let ord = match sort_key
    {
      SortKey::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
      SortKey::Size =>
      {
        // When sorting by size, keep directories ordered by name instead of
        // their (often meaningless) filesystem size.
        if a.is_dir && b.is_dir
        {
          a.name.to_lowercase().cmp(&b.name.to_lowercase())
        }
        else
        {
          a.size.cmp(&b.size)
        }
      },
      SortKey::MTime =>
      {
        let at = a.mtime.unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let bt = b.mtime.unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        at.cmp(&bt)
      }
      SortKey::CTime =>
      {
        let at = a.ctime.unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        let bt = b.ctime.unwrap_or(std::time::SystemTime::UNIX_EPOCH);
        at.cmp(&bt)
      }
    };
    if sort_reverse
    {
      // For size sort, keep directories ordered by name even when reversed.
      if matches!(sort_key, SortKey::Size) && a.is_dir && b.is_dir
      {
        ord
      }
      else
      {
        ord.reverse()
      }
    }
    else
    {
      ord
    }
  });
  Ok(entries)
}
