use mlua::{
  Error as LuaError, Function, Lua, LuaOptions, RegistryKey,
  Result as LuaResult, StdLib, Table, Value,
};
use std::cell::RefCell;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

#[derive(Debug, Clone, Default)]
pub struct IconsConfig {
  pub enabled: bool,
  pub preset: Option<String>,
  pub font: Option<String>,
}

#[derive(Debug, Clone)]
pub struct KeysConfig {
  pub sequence_timeout_ms: u64,
}

impl Default for KeysConfig {
  fn default() -> Self {
    Self { sequence_timeout_ms: 600 }
  }
}

#[derive(Debug, Clone, Default)]
pub struct CommandSpec {
  pub cmd: Vec<String>,
  pub args: Vec<String>,
  pub when: Option<String>,
  pub cwd: Option<String>,
  pub interactive: bool,
  pub env: Vec<(String, String)>,
  pub confirm: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub config_version: u32,
    pub icons: IconsConfig,
    pub keys: KeysConfig,
    pub ui: UiConfig,
    pub commands: Vec<(String, CommandSpec)>,
    pub shell_cmds: Vec<ShellCmd>,
}

#[derive(Debug, Clone)]
pub struct KeyMapping {
  pub sequence: String,
  pub action: String,
  pub description: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct UiPanes {
  pub parent: u16,
  pub current: u16,
  pub preview: u16,
}

#[derive(Debug, Clone)]
pub struct UiConfig {
  pub panes: Option<UiPanes>,
  pub show_hidden: bool,
  pub preview_lines: usize,
  pub max_list_items: usize,
  pub date_format: Option<String>,
  pub row: Option<UiRowFormat>,
  pub display_mode: Option<String>,
  pub sort: Option<String>,
  pub sort_reverse: Option<bool>,
  pub show: Option<String>,
}

impl Default for UiConfig {
  fn default() -> Self {
    Self {
      panes: None,
      show_hidden: false,
      preview_lines: 100,
      max_list_items: 5000,
      date_format: None,
      row: None,
      display_mode: None,
      sort: None,
      sort_reverse: None,
      show: None,
    }
  }
}

#[derive(Debug, Clone)]
pub struct UiRowFormat {
  pub icon: String,
  pub left: String,
  pub middle: String,
  pub right: String,
}

impl Default for UiRowFormat {
  fn default() -> Self {
    Self {
      icon: " ".to_string(),
      left: "{name}".to_string(),
      middle: "".to_string(),
      right: "{info}".to_string(),
    }
  }
}

#[derive(Debug, Clone, Default)]
pub struct ShellCmd {
  pub cmd: String,
  pub description: Option<String>,
}


/// LuaEngine creates a sandboxed Lua runtime for lsv configuration.
/// Safety model:
/// - Load only BASE | STRING | TABLE | MATH stdlibs (no io/os/debug/package).
/// - Provide an `lsv` table with stub functions (`config`, `mapkey`).
/// - A restricted `require()` will be added in a later step.
pub struct LuaEngine {
  lua: Lua,
}

impl LuaEngine {
  /// Initialize a new sandboxed Lua state.
  pub fn new() -> LuaResult<Self> {
    let lua = Lua::new_with(
      StdLib::STRING | StdLib::TABLE | StdLib::MATH,
      LuaOptions::default(),
    )?;

    // Inject `lsv` namespace with stub APIs that accept calls from user config.
    {
      let globals = lua.globals();
      let lsv: Table = lua.create_table()?;

      // lsv.config(tbl): accept and store later (currently a no-op returning true)
      let config_fn = lua.create_function(|_, _tbl: mlua::Value| {
        // Parsing/validation will be implemented in later steps.
        Ok(true)
      })?;

      // lsv.mapkey(seq, action, description?): accept and store later (no-op returning true)
      let mapkey_fn = lua.create_function(
        |_, (_seq, _action, _desc): (String, String, Option<String>)| Ok(true),
      )?;

      lsv.set("config", config_fn)?;
      lsv.set("mapkey", mapkey_fn)?;

      globals.set("lsv", lsv)?;
    }

    Ok(Self { lua })
  }

  /// Access to the underlying Lua state (temporary, for future loader work).
  pub fn lua(&self) -> &Lua {
    &self.lua
  }
}

/// Discovered configuration locations for lsv
#[derive(Debug, Clone)]
pub struct ConfigPaths {
  pub root: PathBuf,
  pub entry: PathBuf,
  pub exists: bool,
}

/// Discover the lsv config directory and entrypoint.
/// Order:
/// 1) $LSV_CONFIG_DIR (root) → expects `init.lua` inside
/// 2) $XDG_CONFIG_HOME/lsv
/// 3) $HOME/.config/lsv
pub fn discover_config_paths() -> std::io::Result<ConfigPaths> {
  // Helper to decide a config root
  fn root_from_env() -> Option<PathBuf> {
    if let Ok(dir) = env::var("LSV_CONFIG_DIR") {
      if !dir.trim().is_empty() {
        return Some(PathBuf::from(dir));
      }
    }
    None
  }

  let root = if let Some(over) = root_from_env() {
    over
  } else if let Ok(xdg) = env::var("XDG_CONFIG_HOME") {
    Path::new(&xdg).join("lsv")
  } else if let Ok(home) = env::var("HOME") {
    Path::new(&home).join(".config").join("lsv")
  } else {
    // Fallback to current dir .config/lsv to avoid empty paths in exotic envs
    Path::new(".config").join("lsv")
  };

  let entry = root.join("init.lua");
  let exists = fs::metadata(&entry).map(|m| m.is_file()).unwrap_or(false);
  Ok(ConfigPaths { root, entry, exists })
}

/// Load and parse configuration using a restricted Lua runtime.
pub fn load_config(
  paths: &ConfigPaths
) -> std::io::Result<(Config, Vec<KeyMapping>, Option<(LuaEngine, RegistryKey, Vec<RegistryKey>)>)>
{
  if !paths.exists {
    return Ok((Config::default(), Vec::new(), None));
  }

  let engine =
    LuaEngine::new().map_err(|e| io_err(format!("lua init failed: {e}")))?;
  let lua = engine.lua();

  let config_acc = Rc::new(RefCell::new(Config::default()));
  let keymaps_acc: Rc<RefCell<Vec<KeyMapping>>> =
    Rc::new(RefCell::new(Vec::new()));

  let previewer_key_acc: Rc<RefCell<Option<RegistryKey>>> = Rc::new(RefCell::new(None));
  let lua_action_keys_acc: Rc<RefCell<Vec<RegistryKey>>> = Rc::new(RefCell::new(Vec::new()));
  install_lsv_api(
    lua,
    Rc::clone(&config_acc),
    Rc::clone(&keymaps_acc),
    Rc::clone(&previewer_key_acc),
    Rc::clone(&lua_action_keys_acc),
  )
  .map_err(|e| io_err(format!("lsv api install failed: {e}")))?;
  install_require(lua, &paths.root.join("lua"))
    .map_err(|e| io_err(format!("require install failed: {e}")))?;

  let code = fs::read_to_string(&paths.entry)
    .map_err(|e| io_err(format!("read init.lua failed: {e}")))?;
  lua
    .load(&code)
    .set_name(paths.entry.to_string_lossy())
    .exec()
    .map_err(|e| io_err(format!("init.lua execution failed: {e}")))?;

  let cfg = config_acc.borrow().clone();
  let maps = keymaps_acc.borrow().clone();
  let key_opt = previewer_key_acc.borrow_mut().take();
  let action_keys = std::mem::take(&mut *lua_action_keys_acc.borrow_mut());
  let engine_opt = key_opt.map(|key| (engine, key, action_keys));
  Ok((cfg, maps, engine_opt))
}

fn io_err(msg: String) -> std::io::Error {
  std::io::Error::new(std::io::ErrorKind::Other, msg)
}

fn install_lsv_api(
  lua: &Lua,
  cfg: Rc<RefCell<Config>>,
  maps: Rc<RefCell<Vec<KeyMapping>>>,
  previewer_key_out: Rc<RefCell<Option<RegistryKey>>>,
  lua_action_keys_out: Rc<RefCell<Vec<RegistryKey>>>,
) -> mlua::Result<()> {
  let globals = lua.globals();
  let lsv: Table = match globals.get::<Value>("lsv") {
    Ok(Value::Table(t)) => t,
    _ => lua.create_table()?,
  };

  // lsv.config(table)
  let cfg_clone = Rc::clone(&cfg);
  let maps_for_config = Rc::clone(&maps);
  let actions_for_config = Rc::clone(&lua_action_keys_out);
  let config_fn = lua.create_function(move |lua, tbl: Table| {
    let mut cfg_mut = cfg_clone.borrow_mut();
    // Accumulate keymaps (both from commands and actions) and push at the end
    let mut seq_keymaps_acc: Vec<(String, String, Option<String>)> = Vec::new();
    if let Ok(v) = tbl.get::<u32>("config_version") {
      cfg_mut.config_version = v;
    }
    if let Ok(icons_tbl) = tbl.get::<Table>("icons") {
      let mut icons = IconsConfig::default();
      if let Ok(b) = icons_tbl.get::<bool>("enabled") {
        icons.enabled = b;
      }
      if let Ok(p) = icons_tbl.get::<String>("preset") {
        icons.preset = Some(p);
      }
      if let Ok(f) = icons_tbl.get::<String>("font") {
        icons.font = Some(f);
      }
      cfg_mut.icons = icons;
    }
    if let Ok(keys_tbl) = tbl.get::<Table>("keys") {
      let mut keys = KeysConfig::default();
      if let Ok(ms) = keys_tbl.get::<u64>("sequence_timeout_ms") {
        keys.sequence_timeout_ms = ms;
      }
      cfg_mut.keys = keys;
    }
    // ui
    if let Ok(ui_tbl) = tbl.get::<Table>("ui") {
      let mut ui = UiConfig::default();
      if let Ok(panes_tbl) = ui_tbl.get::<Table>("panes") {
        let mut panes = UiPanes { parent: 20, current: 30, preview: 50 };
        if let Ok(v) = panes_tbl.get::<u16>("parent") {
          panes.parent = v;
        }
        if let Ok(v) = panes_tbl.get::<u16>("current") {
          panes.current = v;
        }
        if let Ok(v) = panes_tbl.get::<u16>("preview") {
          panes.preview = v;
        }
        ui.panes = Some(panes);
      }
      if let Ok(b) = ui_tbl.get::<bool>("show_hidden") {
        ui.show_hidden = b;
      }
      if let Ok(n) = ui_tbl.get::<u64>("preview_lines") {
        ui.preview_lines = n as usize;
      }
      if let Ok(n) = ui_tbl.get::<u64>("max_list_items") {
        ui.max_list_items = n as usize;
      }
      if let Ok(s) = ui_tbl.get::<String>("date_format") {
        ui.date_format = Some(s);
      }
      if let Ok(row_tbl) = ui_tbl.get::<Table>("row") {
        let mut rf = UiRowFormat::default();
        if let Ok(s) = row_tbl.get::<String>("icon") { rf.icon = s; }
        if let Ok(s) = row_tbl.get::<String>("left") { rf.left = s; }
        if let Ok(s) = row_tbl.get::<String>("middle") { rf.middle = s; }
        if let Ok(s) = row_tbl.get::<String>("right") { rf.right = s; }
        ui.row = Some(rf);
      }
      if let Ok(s) = ui_tbl.get::<String>("display_mode") {
        ui.display_mode = Some(s);
      }
      if let Ok(sort_str) = ui_tbl.get::<String>("sort") {
        ui.sort = Some(sort_str);
      }
      if let Ok(b) = ui_tbl.get::<bool>("sort_reverse") {
        ui.sort_reverse = Some(b);
      }
      if let Ok(show_str) = ui_tbl.get::<String>("show") {
        ui.show = Some(show_str);
      }
      cfg_mut.ui = ui;
    }
    if let Ok(cmds_tbl) = tbl.get::<Table>("commands") {
      let mut cmds = Vec::new();
      for pair in cmds_tbl.pairs::<Value, Value>() {
        let (k, v) = pair?;
        if let Value::String(sname) = k {
          if let Value::Table(t) = v {
            let name = sname.to_str()?.to_string();
            let mut spec = CommandSpec::default();
            if let Ok(arr) = t.get::<Vec<String>>("cmd") {
              spec.cmd = arr;
            } else if let Ok(s) = t.get::<String>("cmd") {
              spec.cmd = vec![s];
            }
            if let Ok(arr) = t.get::<Vec<String>>("args") {
              spec.args = arr;
            }
            if let Ok(s) = t.get::<String>("when") {
              spec.when = Some(s);
            }
            if let Ok(s) = t.get::<String>("cwd") {
              spec.cwd = Some(s);
            }
            if let Ok(b) = t.get::<bool>("interactive") {
              spec.interactive = b;
            }
            if let Ok(env_tbl) = t.get::<Table>("env") {
              for kv in env_tbl.pairs::<String, String>() {
                let (ek, ev) = kv?;
                spec.env.push((ek, ev));
              }
            }
            if let Ok(s) = t.get::<String>("confirm") {
              spec.confirm = Some(s);
            }
            cmds.push((name, spec));
          }
        } else if let Value::Integer(_) = k {
          if let Value::Table(t) = v {
            // External shell command
            if let Ok(cmd_str) = t.get::<String>("cmd") {
              let desc = t.get::<String>("description").ok();
              let keymap = t.get::<String>("keymap").ok();
              let idx = cfg_mut.shell_cmds.len();
              cfg_mut.shell_cmds.push(ShellCmd {
                cmd: cmd_str.clone(),
                description: desc.clone(),
              });
              if let Some(kseq) = keymap {
                seq_keymaps_acc.push((kseq, format!("run_shell:{}", idx), desc));
              }
            }
            // Internal action
            if let Ok(action_str) = t.get::<String>("action") {
              let desc = t.get::<String>("description").ok();
              if let Ok(kseq) = t.get::<String>("keymap") {
                seq_keymaps_acc.push((kseq, action_str, desc));
              }
            }
          }
        }
      }
      cfg_mut.commands = cmds;
    }

    // Separate top-level actions table for internal actions
    if let Ok(actions_tbl) = tbl.get::<Table>("actions") {
      let mut acc = actions_for_config.borrow_mut();
      for pair in actions_tbl.sequence_values::<Value>() {
        if let Value::Table(t) = pair? {
          // Lua function action: fn = function(lsv, config) ... end
          if let Ok(func) = t.get::<Function>("fn") {
            let keymap = t.get::<String>("keymap")?;
            let desc = t.get::<String>("description").ok();
            let reg = lua.create_registry_value(func)?;
            let idx = acc.len();
            acc.push(reg);
            seq_keymaps_acc.push((keymap, format!("run_lua:{}", idx), desc));
            continue;
          }
          // String action fallback
          if let (Ok(kseq), Ok(action_str)) = (t.get::<String>("keymap"), t.get::<String>("action")) {
            let desc = t.get::<String>("description").ok();
            seq_keymaps_acc.push((kseq, action_str, desc));
          }
        }
      }
      drop(acc);
    }

    // Push accumulated keymaps (from commands and actions) once
    drop(cfg_mut);
    {
      let mut km = maps_for_config.borrow_mut();
      for (seq, action, desc) in seq_keymaps_acc.into_iter() {
        km.push(KeyMapping { sequence: seq, action, description: desc });
      }
    }
    Ok(true)
  })?;

  // lsv.mapkey(seq, action, desc?)
  let maps_clone = Rc::clone(&maps);
  let mapkey_fn = lua.create_function(
    move |_, (seq, action, desc): (String, String, Option<String>)| {
      maps_clone.borrow_mut().push(KeyMapping {
        sequence: seq,
        action,
        description: desc,
      });
      Ok(true)
    },
  )?;

  // lsv.set_previewer(function(ctx) -> string|nil)
  let prev_out = Rc::clone(&previewer_key_out);
  let set_previewer_fn = lua.create_function(move |lua, func: Function| {
    let key = lua.create_registry_value(func)?;
    *prev_out.borrow_mut() = Some(key);
    Ok(true)
  })?;

  // lsv.map_action(keymap, description, fn)
  let maps_for_actions_outer = Rc::clone(&maps);
  let actions_acc_outer = Rc::clone(&lua_action_keys_out);
  let map_action_fn = lua.create_function(move |lua, (keymap, desc, func): (String, String, Function)| {
    let reg = lua.create_registry_value(func)?;
    let idx = actions_acc_outer.borrow().len();
    actions_acc_outer.borrow_mut().push(reg);
    maps_for_actions_outer.borrow_mut().push(KeyMapping { sequence: keymap, action: format!("run_lua:{}", idx), description: Some(desc) });
    Ok(true)
  })?;

  // lsv.map_command(keymap, description, cmd_string)
  let cfg_for_cmds = Rc::clone(&cfg);
  let maps_for_cmds = Rc::clone(&maps);
  let map_command_fn = lua.create_function(move |_, (keymap, desc, cmd): (String, String, String)| {
    let mut cfg_mut = cfg_for_cmds.borrow_mut();
    let idx = cfg_mut.shell_cmds.len();
    cfg_mut.shell_cmds.push(ShellCmd { cmd: cmd.clone(), description: Some(desc.clone()) });
    drop(cfg_mut);
    maps_for_cmds.borrow_mut().push(KeyMapping { sequence: keymap, action: format!("run_shell:{}", idx), description: Some(desc) });
    Ok(true)
  })?;

  lsv.set("config", config_fn)?;
  lsv.set("mapkey", mapkey_fn)?;
  lsv.set("set_previewer", set_previewer_fn)?;
  lsv.set("map_action", map_action_fn)?;
  lsv.set("map_command", map_command_fn)?;
  globals.set("lsv", lsv)?;
  Ok(())
}

fn install_require(
  lua: &Lua,
  lua_root: &Path,
) -> mlua::Result<()> {
  let root = lua_root.to_path_buf();
  let require_fn = lua.create_function(move |lua, name: String| {
    if name.contains("..") || name.starts_with('/') {
      return Err(LuaError::external("invalid module name"));
    }
    let rel_path = name.replace('.', "/");
    let path = root.join(format!("{}.lua", rel_path));
    // Canonicalize and ensure under root
    let canon = std::fs::canonicalize(&path)
      .map_err(|e| LuaError::external(format!("{e}")))?;
    let canon_root = std::fs::canonicalize(&root)
      .map_err(|e| LuaError::external(format!("{e}")))?;
    if !canon.starts_with(&canon_root) {
      return Err(LuaError::external("module outside config root"));
    }
    let code = std::fs::read_to_string(&canon)
      .map_err(|e| LuaError::external(format!("{e}")))?;
    let chunk = lua.load(&code).set_name(name);
    chunk.eval::<Value>()
  })?;
  let globals = lua.globals();
  globals.set("require", require_fn)?;
  Ok(())
}
