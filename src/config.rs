use mlua::{
  Error as LuaError, Function, Lua, LuaOptions, RegistryKey,
  Result as LuaResult, StdLib, Table, Value,
};
use std::cell::RefCell;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

const BUILTIN_DEFAULTS_LUA: &str = include_str!("lua/defaults.lua");

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
    // 0 means: no timeout for key sequences
    Self { sequence_timeout_ms: 0 }
  }
}

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub config_version: u32,
    pub icons: IconsConfig,
    pub keys: KeysConfig,
    pub ui: UiConfig,
    // no built-in commands in action-first config; users bind via map_action
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
  pub row_widths: Option<UiRowWidths>,
  pub display_mode: Option<String>,
  pub sort: Option<String>,
  pub sort_reverse: Option<bool>,
  pub show: Option<String>,
  pub theme: Option<UiTheme>,
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
      row_widths: None,
      display_mode: None,
      sort: None,
      sort_reverse: None,
      show: None,
      theme: None,
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UiRowWidths {
  pub icon: u16,
  pub left: u16,
  pub middle: u16,
  pub right: u16,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct UiTheme {
  pub pane_bg: Option<String>,
  pub border_fg: Option<String>,
  pub item_fg: Option<String>,
  pub item_bg: Option<String>,
  pub selected_item_fg: Option<String>,
  pub selected_item_bg: Option<String>,
  pub title_fg: Option<String>,
  pub title_bg: Option<String>,
  pub info_fg: Option<String>,
  pub dir_fg: Option<String>,
  pub dir_bg: Option<String>,
  pub file_fg: Option<String>,
  pub file_bg: Option<String>,
  pub hidden_fg: Option<String>,
  pub hidden_bg: Option<String>,
  pub exec_fg: Option<String>,
  pub exec_bg: Option<String>,
}

// No ShellCmd in action-first config


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

  // 1) Execute built-in defaults
  lua
    .load(BUILTIN_DEFAULTS_LUA)
    .set_name("builtin/defaults.lua")
    .exec()
    .map_err(|e| io_err(format!("defaults.lua execution failed: {e}")))?;

  // 2) Execute user config if present
  if paths.exists {
    let code = fs::read_to_string(&paths.entry)
      .map_err(|e| io_err(format!("read init.lua failed: {e}")))?;
    lua
      .load(&code)
      .set_name(paths.entry.to_string_lossy())
      .exec()
      .map_err(|e| io_err(format!("init.lua execution failed: {e}")))?;
  }

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
    // ui (merge overlay: only provided fields overwrite existing)
    if let Ok(ui_tbl) = tbl.get::<Table>("ui") {
      if let Ok(panes_tbl) = ui_tbl.get::<Table>("panes") {
        let mut panes = cfg_mut.ui.panes.clone().unwrap_or(UiPanes { parent: 30, current: 40, preview: 30 });
        if let Ok(v) = panes_tbl.get::<u16>("parent") { panes.parent = v; }
        if let Ok(v) = panes_tbl.get::<u16>("current") { panes.current = v; }
        if let Ok(v) = panes_tbl.get::<u16>("preview") { panes.preview = v; }
        cfg_mut.ui.panes = Some(panes);
      }
      if let Ok(b) = ui_tbl.get::<bool>("show_hidden") { cfg_mut.ui.show_hidden = b; }
      if let Ok(n) = ui_tbl.get::<u64>("preview_lines") { cfg_mut.ui.preview_lines = n as usize; }
      if let Ok(n) = ui_tbl.get::<u64>("max_list_items") { cfg_mut.ui.max_list_items = n as usize; }
      if let Ok(s) = ui_tbl.get::<String>("date_format") { cfg_mut.ui.date_format = Some(s); }
      if let Ok(row_tbl) = ui_tbl.get::<Table>("row") {
        let mut rf = cfg_mut.ui.row.clone().unwrap_or_default();
        if let Ok(s) = row_tbl.get::<String>("icon") { rf.icon = s; }
        if let Ok(s) = row_tbl.get::<String>("left") { rf.left = s; }
        if let Ok(s) = row_tbl.get::<String>("middle") { rf.middle = s; }
        if let Ok(s) = row_tbl.get::<String>("right") { rf.right = s; }
        cfg_mut.ui.row = Some(rf);
      }
      if let Ok(widths_tbl) = ui_tbl.get::<Table>("row_widths") {
        let mut rw = cfg_mut.ui.row_widths.clone().unwrap_or_default();
        if let Ok(v) = widths_tbl.get::<u64>("icon") { rw.icon = v as u16; }
        if let Ok(v) = widths_tbl.get::<u64>("left") { rw.left = v as u16; }
        if let Ok(v) = widths_tbl.get::<u64>("middle") { rw.middle = v as u16; }
        if let Ok(v) = widths_tbl.get::<u64>("right") { rw.right = v as u16; }
        cfg_mut.ui.row_widths = Some(rw);
      }
      if let Ok(theme_tbl) = ui_tbl.get::<Table>("theme") {
        let mut th = cfg_mut.ui.theme.clone().unwrap_or_default();
        if let Ok(s) = theme_tbl.get::<String>("pane_bg") { th.pane_bg = Some(s); }
        if let Ok(s) = theme_tbl.get::<String>("border_fg") { th.border_fg = Some(s); }
        if let Ok(s) = theme_tbl.get::<String>("item_fg") { th.item_fg = Some(s); }
        if let Ok(s) = theme_tbl.get::<String>("item_bg") { th.item_bg = Some(s); }
        if let Ok(s) = theme_tbl.get::<String>("selected_item_fg") { th.selected_item_fg = Some(s); }
        if let Ok(s) = theme_tbl.get::<String>("selected_item_bg") { th.selected_item_bg = Some(s); }
        if let Ok(s) = theme_tbl.get::<String>("title_fg") { th.title_fg = Some(s); }
        if let Ok(s) = theme_tbl.get::<String>("title_bg") { th.title_bg = Some(s); }
        if let Ok(s) = theme_tbl.get::<String>("info_fg") { th.info_fg = Some(s); }
        if let Ok(s) = theme_tbl.get::<String>("dir_fg") { th.dir_fg = Some(s); }
        if let Ok(s) = theme_tbl.get::<String>("dir_bg") { th.dir_bg = Some(s); }
        if let Ok(s) = theme_tbl.get::<String>("file_fg") { th.file_fg = Some(s); }
        if let Ok(s) = theme_tbl.get::<String>("file_bg") { th.file_bg = Some(s); }
        if let Ok(s) = theme_tbl.get::<String>("hidden_fg") { th.hidden_fg = Some(s); }
        if let Ok(s) = theme_tbl.get::<String>("hidden_bg") { th.hidden_bg = Some(s); }
        if let Ok(s) = theme_tbl.get::<String>("exec_fg") { th.exec_fg = Some(s); }
        if let Ok(s) = theme_tbl.get::<String>("exec_bg") { th.exec_bg = Some(s); }
        cfg_mut.ui.theme = Some(th);
      }
      if let Ok(s) = ui_tbl.get::<String>("display_mode") { cfg_mut.ui.display_mode = Some(s); }
      if let Ok(sort_str) = ui_tbl.get::<String>("sort") { cfg_mut.ui.sort = Some(sort_str); }
      if let Ok(b) = ui_tbl.get::<bool>("sort_reverse") { cfg_mut.ui.sort_reverse = Some(b); }
      if let Ok(show_str) = ui_tbl.get::<String>("show") { cfg_mut.ui.show = Some(show_str); }
    }
    // Note: legacy 'commands' table removed in favor of map_action + lsv.os_run

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


  lsv.set("config", config_fn)?;
  lsv.set("mapkey", mapkey_fn)?;
  lsv.set("set_previewer", set_previewer_fn)?;
  lsv.set("map_action", map_action_fn)?;
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
