//! Loading and translating configuration between Lua and Rust.
//!
//! The real application consumes Lua configuration files. This module exposes
//! helpers to load them, merge user values with defaults, and convert between
//! Lua tables and strongly typed Rust structures. Integration tests reuse these
//! APIs to fabricate configurations dynamically.

use mlua::{
  Error as LuaError,
  Function,
  Lua,
  LuaOptions,
  RegistryKey,
  Result as LuaResult,
  StdLib,
  Table,
  Value,
};
use std::{
  cell::RefCell,
  env,
  fs,
  path::{
    Path,
    PathBuf,
  },
  rc::Rc,
};

const BUILTIN_DEFAULTS_LUA: &str = include_str!("lua/defaults.lua");

#[derive(Debug, Clone, Default)]
/// Icon configuration flags. Icons are optional and require a compatible font.
pub struct IconsConfig
{
  pub enabled:      bool,
  pub preset:       Option<String>,
  pub font:         Option<String>,
  // Optional defaults + per-extension map (lowercased keys)
  pub default_file: Option<String>,
  pub default_dir:  Option<String>,
  pub extensions:   std::collections::HashMap<String, String>,
  // Optional per-folder-name icon overrides (lowercased keys)
  pub folders:      std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default)]
/// Key-handling configuration (currently only sequence timeout).
pub struct KeysConfig
{
  pub sequence_timeout_ms: u64,
}

#[derive(Debug, Clone, Default)]
/// Top-level configuration composed from Lua input.
pub struct Config
{
  pub config_version: u32,
  pub icons:          IconsConfig,
  pub keys:           KeysConfig,
  pub ui:             UiConfig,
  // no built-in commands in action-first config; users bind via map_action
}

#[derive(Debug, Clone)]
/// A single key mapping supplied by `lsv.map_action` or legacy bindings.
pub struct KeyMapping
{
  pub sequence:    String,
  pub action:      String,
  pub description: Option<String>,
}

#[derive(Debug, Clone, Default)]
/// Pane split percentages for the parent/current/preview columns.
pub struct UiPanes
{
  pub parent:  u16,
  pub current: u16,
  pub preview: u16,
}

#[derive(Debug, Clone)]
/// User interface configuration block replicated from Lua.
pub struct UiConfig
{
  pub panes:          Option<UiPanes>,
  pub show_hidden:    bool,
  pub max_list_items: usize,
  pub date_format:    Option<String>,
  pub header_left:    Option<String>,
  pub header_right:   Option<String>,
  pub header_bg:      Option<String>,
  pub header_fg:      Option<String>,
  pub row:            Option<UiRowFormat>,
  pub row_widths:     Option<UiRowWidths>,
  pub display_mode:   Option<String>,
  pub sort:           Option<String>,
  pub sort_reverse:   Option<bool>,
  pub show:           Option<String>,
  pub theme_path:     Option<PathBuf>,
  pub theme:          Option<UiTheme>,
  pub confirm_delete: bool,
  pub modals:         Option<UiModals>,
}

impl Default for UiConfig
{
  fn default() -> Self
  {
    Self {
      panes:          None,
      show_hidden:    false,
      max_list_items: 5000,
      date_format:    None,
      header_left:    None,
      header_right:   None,
      header_bg:      None,
      header_fg:      None,
      row:            None,
      row_widths:     None,
      display_mode:   None,
      sort:           None,
      sort_reverse:   None,
      show:           None,
      theme_path:     None,
      theme:          None,
      confirm_delete: true,
      modals:         None,
    }
  }
}

#[derive(Debug, Clone, Default)]
pub struct UiModalConfig
{
  pub width_pct:  u16, // 10..=100
  pub height_pct: u16, // 10..=100
}

#[derive(Debug, Clone, Default)]
pub struct UiModals
{
  pub prompt:  UiModalConfig,
  pub confirm: UiModalConfig,
  pub theme:   UiModalConfig,
}

#[derive(Debug, Clone)]
/// Template strings used to render each row in the directory panes.
pub struct UiRowFormat
{
  pub icon:   String,
  pub left:   String,
  pub middle: String,
  pub right:  String,
}

impl Default for UiRowFormat
{
  fn default() -> Self
  {
    Self {
      icon:   " ".to_string(),
      left:   "{name}".to_string(),
      middle: "".to_string(),
      right:  "{info}".to_string(),
    }
  }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
/// Optional fixed column widths for the row layout.
pub struct UiRowWidths
{
  pub icon:   u16,
  pub left:   u16,
  pub middle: u16,
  pub right:  u16,
}

#[derive(Debug, Clone, Default, PartialEq)]
/// Theme colours for the UI. Fields are optional and fall back to defaults.
pub struct UiTheme
{
  pub pane_bg:               Option<String>,
  pub border_fg:             Option<String>,
  pub item_fg:               Option<String>,
  pub item_bg:               Option<String>,
  pub selected_item_fg:      Option<String>,
  pub selected_item_bg:      Option<String>,
  pub title_fg:              Option<String>,
  pub title_bg:              Option<String>,
  pub info_fg:               Option<String>,
  pub dir_fg:                Option<String>,
  pub dir_bg:                Option<String>,
  pub file_fg:               Option<String>,
  pub file_bg:               Option<String>,
  pub hidden_fg:             Option<String>,
  pub hidden_bg:             Option<String>,
  pub exec_fg:               Option<String>,
  pub exec_bg:               Option<String>,
  // Selection indicator (bar) colours
  pub selection_bar_fg:      Option<String>,
  pub selection_bar_copy_fg: Option<String>,
  pub selection_bar_move_fg: Option<String>,
}

pub(crate) fn merge_theme_table(
  theme_tbl: &Table,
  theme: &mut UiTheme,
)
{
  if let Ok(s) = theme_tbl.get::<String>("pane_bg")
  {
    theme.pane_bg = Some(s);
  }
  if let Ok(s) = theme_tbl.get::<String>("border_fg")
  {
    theme.border_fg = Some(s);
  }
  if let Ok(s) = theme_tbl.get::<String>("item_fg")
  {
    theme.item_fg = Some(s);
  }
  if let Ok(s) = theme_tbl.get::<String>("item_bg")
  {
    theme.item_bg = Some(s);
  }
  if let Ok(s) = theme_tbl.get::<String>("selected_item_fg")
  {
    theme.selected_item_fg = Some(s);
  }
  if let Ok(s) = theme_tbl.get::<String>("selected_item_bg")
  {
    theme.selected_item_bg = Some(s);
  }
  if let Ok(s) = theme_tbl.get::<String>("title_fg")
  {
    theme.title_fg = Some(s);
  }
  if let Ok(s) = theme_tbl.get::<String>("title_bg")
  {
    theme.title_bg = Some(s);
  }
  if let Ok(s) = theme_tbl.get::<String>("info_fg")
  {
    theme.info_fg = Some(s);
  }
  if let Ok(s) = theme_tbl.get::<String>("dir_fg")
  {
    theme.dir_fg = Some(s);
  }
  if let Ok(s) = theme_tbl.get::<String>("dir_bg")
  {
    theme.dir_bg = Some(s);
  }
  if let Ok(s) = theme_tbl.get::<String>("file_fg")
  {
    theme.file_fg = Some(s);
  }
  if let Ok(s) = theme_tbl.get::<String>("file_bg")
  {
    theme.file_bg = Some(s);
  }
  if let Ok(s) = theme_tbl.get::<String>("hidden_fg")
  {
    theme.hidden_fg = Some(s);
  }
  if let Ok(s) = theme_tbl.get::<String>("hidden_bg")
  {
    theme.hidden_bg = Some(s);
  }
  if let Ok(s) = theme_tbl.get::<String>("exec_fg")
  {
    theme.exec_fg = Some(s);
  }
  if let Ok(s) = theme_tbl.get::<String>("exec_bg")
  {
    theme.exec_bg = Some(s);
  }
  if let Ok(s) = theme_tbl.get::<String>("selection_bar_fg")
  {
    theme.selection_bar_fg = Some(s);
  }
  if let Ok(s) = theme_tbl.get::<String>("selection_bar_copy_fg")
  {
    theme.selection_bar_copy_fg = Some(s);
  }
  if let Ok(s) = theme_tbl.get::<String>("selection_bar_move_fg")
  {
    theme.selection_bar_move_fg = Some(s);
  }
}

fn resolve_theme_path(
  theme_path: &str,
  root: Option<&Path>,
) -> PathBuf
{
  let candidate = Path::new(theme_path);
  if candidate.is_absolute()
  {
    candidate.to_path_buf()
  }
  else if let Some(base) = root
  {
    base.join(candidate)
  }
  else
  {
    candidate.to_path_buf()
  }
}

pub(crate) fn load_theme_table_from_path(
  lua: &Lua,
  path: &Path,
) -> mlua::Result<Table>
{
  crate::trace::log(format!("[lua] read theme: {}", path.display()));
  let code = fs::read_to_string(path).map_err(|e| {
    LuaError::RuntimeError(format!(
      "read theme '{}' failed: {}",
      path.display(),
      e
    ))
  })?;
  crate::trace::log(format!("[lua] eval theme: {}", path.display()));
  let chunk = lua.load(&code).set_name(path.to_string_lossy());
  let value = match chunk.eval::<Value>()
  {
    Ok(v) => v,
    Err(e) =>
    {
      crate::trace::log(format!(
        "[lua] theme eval error ({}): {}",
        path.display(),
        e
      ));
      return Err(e);
    }
  };
  match value
  {
    Value::Table(t) => Ok(t),
    other => Err(LuaError::RuntimeError(format!(
      "theme file '{}' must return a table (got {:?})",
      path.display(),
      other.type_name()
    ))),
  }
}

/// Load a standalone theme file from disk.
///
/// The theme is returned as a [`UiTheme`] without mutating any global state.
pub fn load_theme_from_file(path: &Path) -> std::io::Result<UiTheme>
{
  let lua = Lua::new_with(
    StdLib::STRING | StdLib::TABLE | StdLib::MATH,
    LuaOptions::default(),
  )
  .map_err(|e| io_err(format!("lua init failed: {e}")))?;
  let tbl = load_theme_table_from_path(&lua, path)
    .map_err(|e| io_err(format!("load theme '{}': {e}", path.display())))?;
  let mut theme = UiTheme::default();
  merge_theme_table(&tbl, &mut theme);
  Ok(theme)
}

// No ShellCmd in action-first config

/// LuaEngine creates a sandboxed Lua runtime for lsv configuration.
/// Safety model:
/// - Load only BASE | STRING | TABLE | MATH stdlibs (no io/os/debug/package).
/// - Provide an `lsv` table with stub functions (`config`, `mapkey`).
/// - A restricted `require()` will be added in a later step.
pub struct LuaEngine
{
  lua: Lua,
}

impl LuaEngine
{
  /// Initialize a new sandboxed Lua state.
  pub fn new() -> LuaResult<Self>
  {
    let lua = Lua::new_with(
      StdLib::STRING | StdLib::TABLE | StdLib::MATH,
      LuaOptions::default(),
    )?;

    // Inject `lsv` namespace with stub APIs that accept calls from user config.
    {
      let globals = lua.globals();
      let lsv: Table = lua.create_table()?;

      // lsv.config(tbl): accept and store later (currently a no-op returning
      // true)
      let config_fn = lua.create_function(|_, _tbl: mlua::Value| {
        // Parsing/validation will be implemented in later steps.
        Ok(true)
      })?;

      // lsv.mapkey(seq, action, description?): accept and store later (no-op
      // returning true)
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
  pub fn lua(&self) -> &Lua
  {
    &self.lua
  }
}

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
/// Checks `LSV_CONFIG_DIR`, then `XDG_CONFIG_HOME/lsv`, then `~/.config/lsv`.
/// The returned struct includes the root directory, the path to `init.lua`, and
/// whether the file currently exists.
pub fn discover_config_paths() -> std::io::Result<ConfigPaths>
{
  // Helper to decide a config root
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
  {
    Path::new(&xdg).join("lsv")
  }
  else if let Ok(home) = env::var("HOME")
  {
    Path::new(&home).join(".config").join("lsv")
  }
  else
  {
    // Fallback to current dir .config/lsv to avoid empty paths in exotic envs
    Path::new(".config").join("lsv")
  };

  let entry = root.join("init.lua");
  let exists = fs::metadata(&entry).map(|m| m.is_file()).unwrap_or(false);
  Ok(ConfigPaths { root, entry, exists })
}

type ConfigArtifacts =
  (Config, Vec<KeyMapping>, Option<(LuaEngine, RegistryKey, Vec<RegistryKey>)>);

/// Load and parse configuration using the restricted Lua runtime.
///
/// Returns the merged [`Config`], key mappings, and (optionally) the prepared
/// Lua engine alongside registry keys for preview and action callbacks.
pub fn load_config(paths: &ConfigPaths) -> std::io::Result<ConfigArtifacts>
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
  install_lsv_api(
    lua,
    Rc::clone(&config_acc),
    Rc::clone(&keymaps_acc),
    Rc::clone(&previewer_key_acc),
    Rc::clone(&lua_action_keys_acc),
    Some(paths.root.clone()),
  )
  .map_err(|e| io_err(format!("lsv api install failed: {e}")))?;
  install_require(lua, &paths.root.join("lua"))
    .map_err(|e| io_err(format!("require install failed: {e}")))?;

  // 1) Execute built-in defaults
  crate::trace::log("[lua] exec builtin/defaults.lua");
  {
    let chunk = lua.load(BUILTIN_DEFAULTS_LUA).set_name("builtin/defaults.lua");
    if let Err(e) = chunk.exec()
    {
      crate::trace::log(format!("[lua] defaults.lua error: {}", e));
      return Err(io_err(format!("defaults.lua execution failed: {e}")));
    }
  }

  // 2) Execute user config if present
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
    // Ensure we always have a previewer key (no-op) if actions exist
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

fn io_err(msg: String) -> std::io::Error
{
  std::io::Error::other(msg)
}

/// Load configuration from a Lua source string for tests or programmatic use.
///
/// Defaults are loaded first, followed by the provided snippet. The `root`
/// parameter controls the directory used for `require()` (modules are resolved
/// under `root/lua`). Returns the same tuple as [`load_config`].
#[allow(dead_code)]
pub fn load_config_from_code(
  code: &str,
  root: Option<&std::path::Path>,
) -> std::io::Result<ConfigArtifacts>
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
  install_lsv_api(
    lua,
    Rc::clone(&config_acc),
    Rc::clone(&keymaps_acc),
    Rc::clone(&previewer_key_acc),
    Rc::clone(&lua_action_keys_acc),
    config_root.clone(),
  )
  .map_err(|e| io_err(format!("lsv api install failed: {e}")))?;

  // install restricted require rooted at <root>/lua
  let base =
    match config_root.as_ref()
    {
      Some(p) => p.clone(),
      None => std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from(".")),
    };
  install_require(lua, &base.join("lua"))
    .map_err(|e| io_err(format!("require install failed: {e}")))?;

  // 1) Execute built-in defaults
  crate::trace::log("[lua] exec builtin/defaults.lua (inline)");
  lua
    .load(BUILTIN_DEFAULTS_LUA)
    .set_name("builtin/defaults.lua")
    .exec()
    .map_err(|e| {
      crate::trace::log(format!("[lua] defaults.lua error: {}", e));
      io_err(format!("defaults.lua execution failed: {e}"))
    })?;

  // 2) Execute provided code
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

fn install_lsv_api(
  lua: &Lua,
  cfg: Rc<RefCell<Config>>,
  maps: Rc<RefCell<Vec<KeyMapping>>>,
  previewer_key_out: Rc<RefCell<Option<RegistryKey>>>,
  lua_action_keys_out: Rc<RefCell<Vec<RegistryKey>>>,
  config_root: Option<PathBuf>,
) -> mlua::Result<()>
{
  let globals = lua.globals();
  let lsv: Table = match globals.get::<Value>("lsv")
  {
    Ok(Value::Table(t)) => t,
    _ => lua.create_table()?,
  };
  let theme_root = config_root;

  // lsv.config(table)
  let cfg_clone = Rc::clone(&cfg);
  let maps_for_config = Rc::clone(&maps);
  let actions_for_config = Rc::clone(&lua_action_keys_out);
  let config_fn = lua.create_function(move |lua, tbl: Table| {
    let mut cfg_mut = cfg_clone.borrow_mut();
    // Accumulate keymaps (both from commands and actions) and push at the end
    let mut seq_keymaps_acc: Vec<(String, String, Option<String>)> = Vec::new();
    if let Ok(v) = tbl.get::<u32>("config_version")
    {
      cfg_mut.config_version = v;
    }
    if let Ok(icons_tbl) = tbl.get::<Table>("icons")
    {
      let mut icons = IconsConfig::default();
      if let Ok(b) = icons_tbl.get::<bool>("enabled")
      {
        icons.enabled = b;
      }
      if let Ok(p) = icons_tbl.get::<String>("preset")
      {
        icons.preset = Some(p);
      }
      if let Ok(f) = icons_tbl.get::<String>("font")
      {
        icons.font = Some(f);
      }
      if let Ok(s) = icons_tbl.get::<String>("default_file")
      {
        icons.default_file = Some(s);
      }
      if let Ok(s) = icons_tbl.get::<String>("default_dir")
      {
        icons.default_dir = Some(s);
      }
      // Legacy: icons.by_ext (deprecated). Parse first so `extensions` wins.
      if let Ok(ext_tbl) = icons_tbl.get::<Table>("by_ext")
      {
        for pair in ext_tbl.pairs::<mlua::Value, mlua::Value>()
        {
          let (k, v) =
            pair.map_err(|e| LuaError::RuntimeError(e.to_string()))?;
          match (k, v)
          {
            (mlua::Value::String(ks), mlua::Value::String(vs)) =>
            {
              if let (Ok(k), Ok(icon)) = (ks.to_str(), vs.to_str())
              {
                for name in k.split([',', '|', ';', '/'])
                {
                  let n = name.trim();
                  if !n.is_empty()
                  {
                    icons.extensions.insert(n.to_lowercase(), icon.to_string());
                  }
                }
              }
            }
            (mlua::Value::Table(t), _) =>
            {
              if let Ok(icon) = t.get::<String>("icon")
                && let Ok(list) = t.get::<Table>("names")
              {
                for n in list.sequence_values::<String>().flatten()
                {
                  let n = n.trim().to_string();
                  if !n.is_empty()
                  {
                    icons.extensions.insert(n.to_lowercase(), icon.clone());
                  }
                }
              }
            }
            _ =>
            {}
          }
        }
      }
      // Preferred: icons.extensions
      if let Ok(ext_tbl) = icons_tbl.get::<Table>("extensions")
      {
        for pair in ext_tbl.pairs::<mlua::Value, mlua::Value>()
        {
          let (k, v) =
            pair.map_err(|e| LuaError::RuntimeError(e.to_string()))?;
          match (k, v)
          {
            (mlua::Value::String(ks), mlua::Value::String(vs)) =>
            {
              if let (Ok(k), Ok(icon)) = (ks.to_str(), vs.to_str())
              {
                for name in k.split([',', '|', ';', '/'])
                {
                  let n = name.trim();
                  if !n.is_empty()
                  {
                    icons.extensions.insert(n.to_lowercase(), icon.to_string());
                  }
                }
              }
            }
            (mlua::Value::Table(t), _) =>
            {
              if let Ok(icon) = t.get::<String>("icon")
                && let Ok(list) = t.get::<Table>("names")
              {
                for n in list.sequence_values::<String>().flatten()
                {
                  let n = n.trim().to_string();
                  if !n.is_empty()
                  {
                    icons.extensions.insert(n.to_lowercase(), icon.clone());
                  }
                }
              }
            }
            _ =>
            {}
          }
        }
      }
      // Combined mappings table: { extensions = {..}, folders = {..} }
      if let Ok(map_tbl) = icons_tbl.get::<Table>("mappings")
      {
        if let Ok(ext_tbl) = map_tbl.get::<Table>("extensions")
        {
          for pair in ext_tbl.pairs::<mlua::Value, mlua::Value>()
          {
            let (k, v) =
              pair.map_err(|e| LuaError::RuntimeError(e.to_string()))?;
            match (k, v)
            {
              (mlua::Value::String(ks), mlua::Value::String(vs)) =>
              {
                if let (Ok(k), Ok(icon)) = (ks.to_str(), vs.to_str())
                {
                  for name in k.split([',', '|', ';', '/'])
                  {
                    let n = name.trim();
                    if !n.is_empty()
                    {
                      icons
                        .extensions
                        .insert(n.to_lowercase(), icon.to_string());
                    }
                  }
                }
              }
              (mlua::Value::Table(t), _) =>
              {
                if let Ok(icon) = t.get::<String>("icon")
                  && let Ok(list) = t.get::<Table>("names")
                {
                  for n in list.sequence_values::<String>().flatten()
                  {
                    let n = n.trim().to_string();
                    if !n.is_empty()
                    {
                      icons.extensions.insert(n.to_lowercase(), icon.clone());
                    }
                  }
                }
              }
              _ =>
              {}
            }
          }
        }
        if let Ok(f_tbl) = map_tbl.get::<Table>("folders")
        {
          for pair in f_tbl.pairs::<mlua::Value, mlua::Value>()
          {
            let (k, v) =
              pair.map_err(|e| LuaError::RuntimeError(e.to_string()))?;
            match (k, v)
            {
              (mlua::Value::String(ks), mlua::Value::String(vs)) =>
              {
                if let (Ok(k), Ok(icon)) = (ks.to_str(), vs.to_str())
                {
                  for name in k.split([',', '|', ';', '/'])
                  {
                    let n = name.trim();
                    if !n.is_empty()
                    {
                      icons.folders.insert(n.to_lowercase(), icon.to_string());
                    }
                  }
                }
              }
              (mlua::Value::Table(t), _) =>
              {
                if let Ok(icon) = t.get::<String>("icon")
                  && let Ok(list) = t.get::<Table>("names")
                {
                  for n in list.sequence_values::<String>().flatten()
                  {
                    let n = n.trim().to_string();
                    if !n.is_empty()
                    {
                      icons.folders.insert(n.to_lowercase(), icon.clone());
                    }
                  }
                }
              }
              _ =>
              {}
            }
          }
        }
      }
      // icons.folders: folder name -> icon
      if let Ok(f_tbl) = icons_tbl.get::<Table>("folders")
      {
        for pair in f_tbl.pairs::<mlua::Value, mlua::Value>()
        {
          let (k, v) =
            pair.map_err(|e| LuaError::RuntimeError(e.to_string()))?;
          match (k, v)
          {
            (mlua::Value::String(ks), mlua::Value::String(vs)) =>
            {
              if let (Ok(k), Ok(icon)) = (ks.to_str(), vs.to_str())
              {
                for name in k.split([',', '|', ';', '/'])
                {
                  let n = name.trim();
                  if !n.is_empty()
                  {
                    icons.folders.insert(n.to_lowercase(), icon.to_string());
                  }
                }
              }
            }
            (mlua::Value::Table(t), _) =>
            {
              if let Ok(icon) = t.get::<String>("icon")
                && let Ok(list) = t.get::<Table>("names")
              {
                for n in list.sequence_values::<String>().flatten()
                {
                  let n = n.trim().to_string();
                  if !n.is_empty()
                  {
                    icons.folders.insert(n.to_lowercase(), icon.clone());
                  }
                }
              }
            }
            _ =>
            {}
          }
        }
      }
      // Legacy alias: icons.by_name
      if let Ok(f_tbl) = icons_tbl.get::<Table>("by_name")
      {
        for pair in f_tbl.pairs::<mlua::Value, mlua::Value>()
        {
          let (k, v) =
            pair.map_err(|e| LuaError::RuntimeError(e.to_string()))?;
          if let (mlua::Value::String(ks), mlua::Value::String(vs)) = (k, v)
            && let (Ok(k), Ok(v)) = (ks.to_str(), vs.to_str())
          {
            icons.folders.insert(k.to_lowercase(), v.to_string());
          }
        }
      }
      cfg_mut.icons = icons;
    }
    if let Ok(keys_tbl) = tbl.get::<Table>("keys")
    {
      let mut keys = KeysConfig::default();
      if let Ok(ms) = keys_tbl.get::<u64>("sequence_timeout_ms")
      {
        keys.sequence_timeout_ms = ms;
      }
      cfg_mut.keys = keys;
    }
    // ui (merge overlay: only provided fields overwrite existing)
    if let Ok(ui_tbl) = tbl.get::<Table>("ui")
    {
      if let Ok(panes_tbl) = ui_tbl.get::<Table>("panes")
      {
        let mut panes = cfg_mut.ui.panes.clone().unwrap_or(UiPanes {
          parent:  30,
          current: 40,
          preview: 30,
        });
        if let Ok(v) = panes_tbl.get::<u16>("parent")
        {
          panes.parent = v;
        }
        if let Ok(v) = panes_tbl.get::<u16>("current")
        {
          panes.current = v;
        }
        if let Ok(v) = panes_tbl.get::<u16>("preview")
        {
          panes.preview = v;
        }
        cfg_mut.ui.panes = Some(panes);
      }
      if let Ok(b) = ui_tbl.get::<bool>("show_hidden")
      {
        cfg_mut.ui.show_hidden = b;
      }
      if let Ok(n) = ui_tbl.get::<u64>("max_list_items")
      {
        cfg_mut.ui.max_list_items = n as usize;
      }
      if let Ok(s) = ui_tbl.get::<String>("date_format")
      {
        cfg_mut.ui.date_format = Some(s);
      }
      if let Ok(h_tbl) = ui_tbl.get::<Table>("header")
      {
        if let Ok(s) = h_tbl.get::<String>("left")
        {
          cfg_mut.ui.header_left = Some(s);
        }
        if let Ok(s) = h_tbl.get::<String>("right")
        {
          cfg_mut.ui.header_right = Some(s);
        }
        if let Ok(bg) = h_tbl.get::<String>("bg")
        {
          cfg_mut.ui.header_bg = Some(bg);
        }
        if let Ok(fg) = h_tbl.get::<String>("fg")
        {
          cfg_mut.ui.header_fg = Some(fg);
        }
      }
      if let Ok(s) = ui_tbl.get::<String>("header_bg")
      {
        cfg_mut.ui.header_bg = Some(s);
      }
      if let Ok(s) = ui_tbl.get::<String>("header_fg")
      {
        cfg_mut.ui.header_fg = Some(s);
      }
      if let Ok(row_tbl) = ui_tbl.get::<Table>("row")
      {
        let mut rf = cfg_mut.ui.row.clone().unwrap_or_default();
        if let Ok(s) = row_tbl.get::<String>("icon")
        {
          rf.icon = s;
        }
        if let Ok(s) = row_tbl.get::<String>("left")
        {
          rf.left = s;
        }
        if let Ok(s) = row_tbl.get::<String>("middle")
        {
          rf.middle = s;
        }
        if let Ok(s) = row_tbl.get::<String>("right")
        {
          rf.right = s;
        }
        cfg_mut.ui.row = Some(rf);
      }
      if let Ok(widths_tbl) = ui_tbl.get::<Table>("row_widths")
      {
        let mut rw = cfg_mut.ui.row_widths.clone().unwrap_or_default();
        if let Ok(v) = widths_tbl.get::<u64>("icon")
        {
          rw.icon = v as u16;
        }
        if let Ok(v) = widths_tbl.get::<u64>("left")
        {
          rw.left = v as u16;
        }
        if let Ok(v) = widths_tbl.get::<u64>("middle")
        {
          rw.middle = v as u16;
        }
        if let Ok(v) = widths_tbl.get::<u64>("right")
        {
          rw.right = v as u16;
        }
        cfg_mut.ui.row_widths = Some(rw);
      }
      if let Ok(theme_path_str) = ui_tbl.get::<String>("theme_path")
      {
        if theme_path_str.trim().is_empty()
        {
          return Err(LuaError::RuntimeError(
            "ui.theme_path must be a non-empty string".to_string(),
          ));
        }
        let resolved_path =
          resolve_theme_path(&theme_path_str, theme_root.as_deref());
        let theme_tbl = load_theme_table_from_path(lua, &resolved_path)?;
        let mut th = cfg_mut.ui.theme.clone().unwrap_or_default();
        merge_theme_table(&theme_tbl, &mut th);
        cfg_mut.ui.theme = Some(th);
        cfg_mut.ui.theme_path = Some(resolved_path);
      }
      // ui.theme may be either a table (inline) or a string (module name)
      if let Ok(val) = ui_tbl.get::<Value>("theme")
      {
        match val
        {
          Value::Table(theme_tbl) =>
          {
            let mut th = cfg_mut.ui.theme.clone().unwrap_or_default();
            merge_theme_table(&theme_tbl, &mut th);
            cfg_mut.ui.theme = Some(th);
          }
          Value::String(s) =>
          {
            let mod_name =
              s.to_str().map_err(|e| LuaError::RuntimeError(e.to_string()))?;
            // Resolve module file path under <root>/lua
            let base = match theme_root.as_ref()
            {
              Some(p) => p.clone(),
              None => std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from(".")),
            };
            let rel_path = mod_name.replace('.', "/");
            let path = base.join("lua").join(format!("{}.lua", rel_path));
            // Load via Lua require() to allow module code to run
            let globals = lua.globals();
            let require_fn: Function = globals.get("require")?;
            let loaded: Value = require_fn.call(&mod_name)?;
            let theme_tbl = match loaded
            {
              Value::Table(t) => t,
              other =>
              {
                return Err(LuaError::RuntimeError(format!(
                  "theme module '{}' must return a table (got {})",
                  mod_name,
                  other.type_name()
                )));
              }
            };
            let mut th = cfg_mut.ui.theme.clone().unwrap_or_default();
            merge_theme_table(&theme_tbl, &mut th);
            cfg_mut.ui.theme = Some(th);
            // Record the resolved path for theme picker integration if possible
            if let Ok(canon) = std::fs::canonicalize(&path)
            {
              cfg_mut.ui.theme_path = Some(canon);
            }
          }
          _ =>
          {}
        }
      }
      if let Ok(s) = ui_tbl.get::<String>("display_mode")
      {
        cfg_mut.ui.display_mode = Some(s);
      }
      if let Ok(sort_str) = ui_tbl.get::<String>("sort")
      {
        cfg_mut.ui.sort = Some(sort_str);
      }
      if let Ok(b) = ui_tbl.get::<bool>("sort_reverse")
      {
        cfg_mut.ui.sort_reverse = Some(b);
      }
      if let Ok(show_str) = ui_tbl.get::<String>("show")
      {
        cfg_mut.ui.show = Some(show_str);
      }
      if let Ok(b) = ui_tbl.get::<bool>("confirm_delete")
      {
        cfg_mut.ui.confirm_delete = b;
      }
      if let Ok(modals_tbl) = ui_tbl.get::<Table>("modals")
      {
        let mut modals = cfg_mut.ui.modals.clone().unwrap_or_default();
        if let Ok(p_tbl) = modals_tbl.get::<Table>("prompt")
        {
          let mut p = modals.prompt;
          if let Ok(v) = p_tbl.get::<u64>("width_pct")
          {
            p.width_pct = v as u16;
          }
          if let Ok(v) = p_tbl.get::<u64>("height_pct")
          {
            p.height_pct = v as u16;
          }
          modals.prompt = p;
        }
        if let Ok(c_tbl) = modals_tbl.get::<Table>("confirm")
        {
          let mut c = modals.confirm;
          if let Ok(v) = c_tbl.get::<u64>("width_pct")
          {
            c.width_pct = v as u16;
          }
          if let Ok(v) = c_tbl.get::<u64>("height_pct")
          {
            c.height_pct = v as u16;
          }
          modals.confirm = c;
        }
        if let Ok(t_tbl) = modals_tbl.get::<Table>("theme")
        {
          let mut t = modals.theme;
          if let Ok(v) = t_tbl.get::<u64>("width_pct")
          {
            t.width_pct = v as u16;
          }
          if let Ok(v) = t_tbl.get::<u64>("height_pct")
          {
            t.height_pct = v as u16;
          }
          modals.theme = t;
        }
        cfg_mut.ui.modals = Some(modals);
      }
    }
    // Note: legacy 'commands' table removed in favor of map_action + lsv.os_run

    // Separate top-level actions table for internal actions
    if let Ok(actions_tbl) = tbl.get::<Table>("actions")
    {
      let mut acc = actions_for_config.borrow_mut();
      for pair in actions_tbl.sequence_values::<Value>()
      {
        if let Value::Table(t) = pair?
        {
          // Lua function action: fn = function(lsv, config) ... end
          if let Ok(func) = t.get::<Function>("fn")
          {
            let keymap = t.get::<String>("keymap")?;
            let desc = t.get::<String>("description").ok();
            let reg = lua.create_registry_value(func)?;
            let idx = acc.len();
            acc.push(reg);
            seq_keymaps_acc.push((keymap, format!("run_lua:{}", idx), desc));
            continue;
          }
          // String action fallback
          if let (Ok(kseq), Ok(action_str)) =
            (t.get::<String>("keymap"), t.get::<String>("action"))
          {
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
      for (seq, action, desc) in seq_keymaps_acc.into_iter()
      {
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
  // lsv.map_action(keymap_or_list, description, fn)
  let map_action_fn = lua.create_function(
    move |lua, (keymaps_val, desc, func): (Value, String, Function)| {
      let reg = lua.create_registry_value(func)?;
      let idx = actions_acc_outer.borrow().len();
      actions_acc_outer.borrow_mut().push(reg);
      let action_str = format!("run_lua:{}", idx);
      match keymaps_val
      {
        Value::String(s) =>
        {
          let seq = s.to_str().map(|v| v.to_string()).unwrap_or_default();
          maps_for_actions_outer.borrow_mut().push(KeyMapping {
            sequence:    seq,
            action:      action_str.clone(),
            description: Some(desc.clone()),
          });
        }
        Value::Table(t) =>
        {
          for pair in t.sequence_values::<Value>()
          {
            if let Value::String(s) = pair?
            {
              let seq = s.to_str().map(|v| v.to_string()).unwrap_or_default();
              maps_for_actions_outer.borrow_mut().push(KeyMapping {
                sequence:    seq,
                action:      action_str.clone(),
                description: Some(desc.clone()),
              });
            }
          }
        }
        _ =>
        {}
      }
      Ok(true)
    },
  )?;

  lsv.set("config", config_fn)?;
  lsv.set("mapkey", mapkey_fn)?;
  lsv.set("set_previewer", set_previewer_fn)?;
  lsv.set("map_action", map_action_fn)?;
  // lsv.getenv(name, default?) -> string|nil: safe env access for config,
  // actions, previewers
  let getenv_fn =
    lua.create_function(|_, (name, default): (String, Option<String>)| {
      Ok(std::env::var(&name).ok().or(default))
    })?;
  lsv.set("getenv", getenv_fn)?;
  // Add metatable to error on unknown lsv.* at config time
  let mt = lua.create_table()?;
  let idx = lua.create_function(move |lua, (_tbl, key): (Table, Value)| {
    let name = match key
    {
      Value::String(s) => match s.to_str()
      {
        Ok(v) => v.to_string(),
        Err(_) => "?".to_string(),
      },
      other => format!("{:?}", other),
    };
    let func = lua.create_function(move |_, ()| -> mlua::Result<()> {
      Err(mlua::Error::RuntimeError(format!("unknown lsv function: {}", name)))
    })?;
    Ok(func)
  })?;
  mt.set("__index", idx)?;
  let _ = lsv.set_metatable(Some(mt));
  globals.set("lsv", lsv)?;
  Ok(())
}

fn install_require(
  lua: &Lua,
  lua_root: &Path,
) -> mlua::Result<()>
{
  let root = lua_root.to_path_buf();
  let require_fn = lua.create_function(move |lua, name: String| {
    if name.contains("..") || name.starts_with('/')
    {
      return Err(LuaError::external("invalid module name"));
    }
    let rel_path = name.replace('.', "/");
    let path = root.join(format!("{}.lua", rel_path));
    // Canonicalize and ensure under root
    let canon = std::fs::canonicalize(&path)
      .map_err(|e| LuaError::external(format!("{e}")))?;
    let canon_root = std::fs::canonicalize(&root)
      .map_err(|e| LuaError::external(format!("{e}")))?;
    if !canon.starts_with(&canon_root)
    {
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
