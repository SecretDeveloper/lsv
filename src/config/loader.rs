use mlua::RegistryKey;
use std::{
  cell::RefCell,
  fs,
  io,
  path::Path,
  rc::Rc,
};

use super::{
  Config,
  ConfigPaths,
  KeyMapping,
  LuaEngine,
};

type ConfigArtifacts =
  (Config, Vec<KeyMapping>, Option<(LuaEngine, RegistryKey, Vec<RegistryKey>)>);

pub fn load_config(paths: &ConfigPaths) -> io::Result<ConfigArtifacts>
{
  let engine =
    LuaEngine::new().map_err(|e| io_err(format!("lua init failed: {e}")))?;
  let lua = engine.lua();

  let config_acc = Rc::new(RefCell::new(Config::default()));
  let keymaps_acc: Rc<RefCell<Vec<KeyMapping>>> =
    Rc::new(RefCell::new(Vec::new()));
  let previewer_key_acc: Rc<RefCell<Option<RegistryKey>>> =
    Rc::new(RefCell::new(None));
  let lua_action_keys_acc: Rc<RefCell<Vec<RegistryKey>>> =
    Rc::new(RefCell::new(Vec::new()));

  super::install_lsv_api(
    lua,
    Rc::clone(&config_acc),
    Rc::clone(&keymaps_acc),
    Rc::clone(&previewer_key_acc),
    Rc::clone(&lua_action_keys_acc),
    Some(paths.root.clone()),
  )
  .map_err(|e| io_err(format!("lsv api install failed: {e}")))?;
  super::install_require(lua, &paths.root.join("lua"))
    .map_err(|e| io_err(format!("require install failed: {e}")))?;

  crate::trace::log("[lua] exec builtin/defaults.lua");
  {
    let chunk =
      lua.load(super::BUILTIN_DEFAULTS_LUA).set_name("builtin/defaults.lua");
    if let Err(e) = chunk.exec()
    {
      crate::trace::log(format!("[lua] defaults.lua error: {}", e));
      return Err(io_err(format!("defaults.lua execution failed: {e}")));
    }
  }

  if paths.exists
  {
    let code = fs::read_to_string(&paths.entry)
      .map_err(|e| io_err(format!("read init.lua failed: {e}")))?;
    crate::trace::log(format!(
      "[lua] exec user config: {}",
      paths.entry.to_string_lossy()
    ));
    let chunk = lua.load(&code).set_name(paths.entry.to_string_lossy());
    if let Err(e) = chunk.exec()
    {
      crate::trace::log(format!(
        "[lua] user config error ({}): {}",
        paths.entry.to_string_lossy(),
        e
      ));
      return Err(io_err(format!("init.lua execution failed: {e}")));
    }
  }

  let cfg = config_acc.borrow().clone();
  let maps = keymaps_acc.borrow().clone();
  let key_opt = previewer_key_acc.borrow_mut().take();
  let action_keys = std::mem::take(&mut *lua_action_keys_acc.borrow_mut());
  let engine_opt = if key_opt.is_some() || !action_keys.is_empty()
  {
    let key = match key_opt
    {
      Some(k) => k,
      None =>
      {
        let f: mlua::Function = lua
          .create_function(|_, _ctx: mlua::Value| Ok(mlua::Value::Nil))
          .map_err(|e| io_err(format!("create noop previewer failed: {e}")))?;
        lua
          .create_registry_value(f)
          .map_err(|e| io_err(format!("registry noop previewer failed: {e}")))?
      }
    };
    Some((engine, key, action_keys))
  }
  else
  {
    None
  };
  Ok((cfg, maps, engine_opt))
}

#[allow(dead_code)]
pub fn load_config_from_code(
  code: &str,
  root: Option<&Path>,
) -> io::Result<ConfigArtifacts>
{
  let engine =
    LuaEngine::new().map_err(|e| io_err(format!("lua init failed: {e}")))?;
  let lua = engine.lua();

  let config_acc = Rc::new(RefCell::new(Config::default()));
  let keymaps_acc: Rc<RefCell<Vec<KeyMapping>>> =
    Rc::new(RefCell::new(Vec::new()));
  let previewer_key_acc: Rc<RefCell<Option<RegistryKey>>> =
    Rc::new(RefCell::new(None));
  let lua_action_keys_acc: Rc<RefCell<Vec<RegistryKey>>> =
    Rc::new(RefCell::new(Vec::new()));
  let config_root = root.map(|p| p.to_path_buf());

  super::install_lsv_api(
    lua,
    Rc::clone(&config_acc),
    Rc::clone(&keymaps_acc),
    Rc::clone(&previewer_key_acc),
    Rc::clone(&lua_action_keys_acc),
    config_root.clone(),
  )
  .map_err(|e| io_err(format!("lsv api install failed: {e}")))?;

  let base =
    match config_root.as_ref()
    {
      Some(p) => p.clone(),
      None => std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from(".")),
    };
  super::install_require(lua, &base.join("lua"))
    .map_err(|e| io_err(format!("require install failed: {e}")))?;

  crate::trace::log("[lua] exec builtin/defaults.lua (inline)");
  lua
    .load(super::BUILTIN_DEFAULTS_LUA)
    .set_name("builtin/defaults.lua")
    .exec()
    .map_err(|e| {
      crate::trace::log(format!("[lua] defaults.lua error: {}", e));
      io_err(format!("defaults.lua execution failed: {e}"))
    })?;

  crate::trace::log("[lua] exec inline init.lua");
  lua.load(code).set_name("inline init.lua").exec().map_err(|e| {
    crate::trace::log(format!("[lua] inline init.lua error: {}", e));
    io_err(format!("inline init.lua execution failed: {e}"))
  })?;

  let cfg = config_acc.borrow().clone();
  let maps = keymaps_acc.borrow().clone();
  let key_opt = previewer_key_acc.borrow_mut().take();
  let action_keys = std::mem::take(&mut *lua_action_keys_acc.borrow_mut());
  let engine_opt = if key_opt.is_some() || !action_keys.is_empty()
  {
    let key = match key_opt
    {
      Some(k) => k,
      None =>
      {
        let f: mlua::Function = lua
          .create_function(|_, _ctx: mlua::Value| Ok(mlua::Value::Nil))
          .map_err(|e| io_err(format!("create noop previewer failed: {e}")))?;
        lua
          .create_registry_value(f)
          .map_err(|e| io_err(format!("registry noop previewer failed: {e}")))?
      }
    };
    Some((engine, key, action_keys))
  }
  else
  {
    None
  };
  Ok((cfg, maps, engine_opt))
}

fn io_err(msg: String) -> io::Error
{
  io::Error::other(msg)
}
