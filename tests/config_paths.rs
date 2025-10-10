use std::env;

fn with_env<T>(
  k: &str,
  v: Option<&str>,
  f: impl FnOnce() -> T,
) -> T
{
  let old = env::var(k).ok();
  unsafe {
    match v
    {
      Some(val) => env::set_var(k, val),
      None => env::remove_var(k),
    }
  }
  let out = f();
  unsafe {
    match old
    {
      Some(s) => env::set_var(k, s),
      None => env::remove_var(k),
    }
  }
  out
}

#[test]
fn discover_config_paths_honors_lsv_config_dir()
{
  let tmp = tempfile::tempdir().unwrap();
  let dir = tmp.path().join("conf");
  std::fs::create_dir_all(&dir).unwrap();
  let res = with_env("LSV_CONFIG_DIR", Some(dir.to_str().unwrap()), || {
    lsv::config::discover_config_paths().unwrap()
  });
  assert_eq!(res.root, dir);
  assert_eq!(res.entry, dir.join("init.lua"));
  assert!(!res.exists);
}

#[test]
#[cfg(not(windows))]
fn discover_config_paths_uses_xdg_when_set()
{
  let tmp = tempfile::tempdir().unwrap();
  let xdg = tmp.path().join("xdg");
  std::fs::create_dir_all(&xdg).unwrap();
  let res = with_env("LSV_CONFIG_DIR", None, || {
    with_env("XDG_CONFIG_HOME", Some(xdg.to_str().unwrap()), || {
      lsv::config::discover_config_paths().unwrap()
    })
  });
  assert_eq!(res.root, xdg.join("lsv"));
  assert_eq!(res.entry, xdg.join("lsv").join("init.lua"));
}
