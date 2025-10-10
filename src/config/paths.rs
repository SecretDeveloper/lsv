use std::{
  env,
  fs,
  path::{
    Path,
    PathBuf,
  },
};

/// Resolved configuration locations for lsv.
#[derive(Debug, Clone)]
pub struct ConfigPaths
{
  pub root:   PathBuf,
  pub entry:  PathBuf,
  pub exists: bool,
}

/// Discover the effective configuration directory and entry point.
///
/// Checks `LSV_CONFIG_DIR`, then `XDG_CONFIG_HOME/lsv`.
///
/// Platform-specific fallbacks:
/// - Unix: `~/.config/lsv`
/// - Windows: `%LOCALAPPDATA%\\lsv`, then `%APPDATA%\\lsv`, then
///   `%USERPROFILE%\\.config\\lsv`
///
/// The returned struct includes the root directory, the path to `init.lua`, and
/// whether the file currently exists.
pub fn discover_config_paths() -> std::io::Result<ConfigPaths>
{
  fn root_from_env() -> Option<PathBuf>
  {
    if let Ok(dir) = env::var("LSV_CONFIG_DIR")
      && !dir.trim().is_empty()
    {
      return Some(PathBuf::from(dir));
    }
    None
  }

  let root = if let Some(over) = root_from_env()
  {
    over
  }
  else if let Ok(xdg) = env::var("XDG_CONFIG_HOME")
    && !xdg.trim().is_empty()
  {
    Path::new(&xdg).join("lsv")
  }
  else
  {
    #[cfg(windows)]
    {
      if let Ok(local) = env::var("LOCALAPPDATA")
        && !local.trim().is_empty()
      {
        Path::new(&local).join("lsv")
      }
      else if let Ok(app) = env::var("APPDATA")
        && !app.trim().is_empty()
      {
        Path::new(&app).join("lsv")
      }
      else if let Ok(up) = env::var("USERPROFILE")
        && !up.trim().is_empty()
      {
        Path::new(&up).join(".config").join("lsv")
      }
      else
      {
        Path::new(".config").join("lsv")
      }
    }
    #[cfg(not(windows))]
    {
      if let Ok(home) = env::var("HOME")
        && !home.trim().is_empty()
      {
        Path::new(&home).join(".config").join("lsv")
      }
      else
      {
        Path::new(".config").join("lsv")
      }
    }
  };

  let entry = root.join("init.lua");
  let exists = fs::metadata(&entry).map(|m| m.is_file()).unwrap_or(false);
  Ok(ConfigPaths { root, entry, exists })
}
