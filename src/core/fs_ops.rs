use std::{
  io,
  path::Path,
};

/// Recursively copy a file or directory tree from `src` to `dst`.
pub fn copy_path_recursive(
  src: &Path,
  dst: &Path,
) -> io::Result<()>
{
  let meta = std::fs::metadata(src)?;
  if meta.is_dir()
  {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)?
    {
      let de = entry?;
      let p = de.path();
      let name = de.file_name();
      let target = dst.join(name);
      copy_path_recursive(&p, &target)?;
    }
    Ok(())
  }
  else
  {
    std::fs::copy(src, dst).map(|_| ())
  }
}

/// Move a path via rename, falling back to copy+remove on cross-device moves.
pub fn move_path_with_fallback(
  src: &Path,
  dst: &Path,
) -> io::Result<()>
{
  match std::fs::rename(src, dst)
  {
    Ok(()) => Ok(()),
    Err(_e) =>
    {
      copy_path_recursive(src, dst)?;
      let meta = std::fs::metadata(src)?;
      if meta.is_dir()
      {
        std::fs::remove_dir_all(src)
      }
      else
      {
        std::fs::remove_file(src)
      }
    }
  }
}

/// Remove a path (file or directory recursively).
pub fn remove_path_all(path: &Path) -> io::Result<()>
{
  if path.is_dir()
  {
    std::fs::remove_dir_all(path)
  }
  else
  {
    std::fs::remove_file(path)
  }
}
