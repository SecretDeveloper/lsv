use mlua::{Lua, LuaOptions, Result as LuaResult, StdLib, Table, Value, Error as LuaError};
use std::env;
use std::path::{Path, PathBuf};
use std::fs;
use std::rc::Rc;
use std::cell::RefCell;

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
    fn default() -> Self { Self { sequence_timeout_ms: 600 } }
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
}

#[derive(Debug, Clone)]
pub struct KeyMapping { pub sequence: String, pub action: String, pub description: Option<String> }

#[derive(Debug, Clone, Default)]
pub struct UiPanes { pub parent: u16, pub current: u16, pub preview: u16 }

#[derive(Debug, Clone)]
pub struct UiConfig {
    pub panes: Option<UiPanes>,
    pub show_hidden: bool,
    pub preview_lines: usize,
    pub max_list_items: usize,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self { panes: None, show_hidden: false, preview_lines: 100, max_list_items: 5000 }
    }
}

/// LuaEngine creates a sandboxed Lua runtime for lv configuration.
/// Safety model:
/// - Load only BASE | STRING | TABLE | MATH stdlibs (no io/os/debug/package).
/// - Provide an `lv` table with stub functions (`config`, `mapkey`).
/// - A restricted `require()` will be added in a later step.
pub struct LuaEngine {
    lua: Lua,
}

impl LuaEngine {
    /// Initialize a new sandboxed Lua state.
    pub fn new() -> LuaResult<Self> {
        let lua = Lua::new_with(StdLib::STRING | StdLib::TABLE | StdLib::MATH, LuaOptions::default())?;

        // Inject `lv` namespace with stub APIs that accept calls from user config.
        {
            let globals = lua.globals();
            let lv: Table = lua.create_table()?;

            // lv.config(tbl): accept and store later (currently a no-op returning true)
            let config_fn = lua.create_function(|_, _tbl: mlua::Value| {
                // Parsing/validation will be implemented in later steps.
                Ok(true)
            })?;

            // lv.mapkey(seq, action, description?): accept and store later (no-op returning true)
            let mapkey_fn = lua.create_function(
                |_, (_seq, _action, _desc): (String, String, Option<String>)| Ok(true),
            )?;

            lv.set("config", config_fn)?;
            lv.set("mapkey", mapkey_fn)?;

            globals.set("lv", lv)?;
        }

        Ok(Self { lua })
    }

    /// Access to the underlying Lua state (temporary, for future loader work).
    pub fn lua(&self) -> &Lua {
        &self.lua
    }
}

/// Discovered configuration locations for lv
#[derive(Debug, Clone)]
pub struct ConfigPaths {
    pub root: PathBuf,
    pub entry: PathBuf,
    pub exists: bool,
}

/// Discover the lv config directory and entrypoint.
/// Order:
/// 1) $LV_CONFIG_DIR (root) → expects `lua/init.lua` inside
/// 2) $XDG_CONFIG_HOME/lv
/// 3) $HOME/.config/lv
pub fn discover_config_paths() -> std::io::Result<ConfigPaths> {
    // Helper to decide a config root
    fn root_from_env() -> Option<PathBuf> {
        if let Ok(dir) = env::var("LV_CONFIG_DIR") {
            if !dir.trim().is_empty() {
                return Some(PathBuf::from(dir));
            }
        }
        None
    }

    let root = if let Some(over) = root_from_env() {
        over
    } else if let Ok(xdg) = env::var("XDG_CONFIG_HOME") {
        Path::new(&xdg).join("lv")
    } else if let Ok(home) = env::var("HOME") {
        Path::new(&home).join(".config").join("lv")
    } else {
        // Fallback to current dir .config/lv to avoid empty paths in exotic envs
        Path::new(".config").join("lv")
    };

    let entry = root.join("lua").join("init.lua");
    let exists = fs::metadata(&entry).map(|m| m.is_file()).unwrap_or(false);
    Ok(ConfigPaths { root, entry, exists })
}

/// Load and parse configuration using a restricted Lua runtime.
pub fn load_config(paths: &ConfigPaths) -> std::io::Result<(Config, Vec<KeyMapping>)> {
    if !paths.exists {
        return Ok((Config::default(), Vec::new()));
    }

    let engine = LuaEngine::new().map_err(|e| io_err(format!("lua init failed: {e}")))?;
    let lua = engine.lua();

    let config_acc = Rc::new(RefCell::new(Config::default()));
    let keymaps_acc: Rc<RefCell<Vec<KeyMapping>>> = Rc::new(RefCell::new(Vec::new()));

    install_lv_api(lua, Rc::clone(&config_acc), Rc::clone(&keymaps_acc))
        .map_err(|e| io_err(format!("lv api install failed: {e}")))?;
    install_require(lua, &paths.root.join("lua"))
        .map_err(|e| io_err(format!("require install failed: {e}")))?;

    let code = fs::read_to_string(&paths.entry)
        .map_err(|e| io_err(format!("read init.lua failed: {e}")))?;
    lua.load(&code)
        .set_name(paths.entry.to_string_lossy())
        .exec()
        .map_err(|e| io_err(format!("init.lua execution failed: {e}")))?;

    let cfg = config_acc.borrow().clone();
    let maps = keymaps_acc.borrow().clone();
    Ok((cfg, maps))
}

fn io_err(msg: String) -> std::io::Error { std::io::Error::new(std::io::ErrorKind::Other, msg) }

fn install_lv_api(lua: &Lua, cfg: Rc<RefCell<Config>>, maps: Rc<RefCell<Vec<KeyMapping>>>) -> mlua::Result<()> {
    let globals = lua.globals();
    let lv: Table = match globals.get::<Value>("lv") {
        Ok(Value::Table(t)) => t,
        _ => lua.create_table()?,
    };

    // lv.config(table)
    let cfg_clone = Rc::clone(&cfg);
    let config_fn = lua.create_function(move |_, tbl: Table| {
        let mut cfg_mut = cfg_clone.borrow_mut();
        if let Ok(v) = tbl.get::<u32>("config_version") { cfg_mut.config_version = v; }
        if let Ok(icons_tbl) = tbl.get::<Table>("icons") {
            let mut icons = IconsConfig::default();
            if let Ok(b) = icons_tbl.get::<bool>("enabled") { icons.enabled = b; }
            if let Ok(p) = icons_tbl.get::<String>("preset") { icons.preset = Some(p); }
            if let Ok(f) = icons_tbl.get::<String>("font") { icons.font = Some(f); }
            cfg_mut.icons = icons;
        }
        if let Ok(keys_tbl) = tbl.get::<Table>("keys") {
            let mut keys = KeysConfig::default();
            if let Ok(ms) = keys_tbl.get::<u64>("sequence_timeout_ms") { keys.sequence_timeout_ms = ms; }
            cfg_mut.keys = keys;
        }
        // ui
        if let Ok(ui_tbl) = tbl.get::<Table>("ui") {
            let mut ui = UiConfig::default();
            if let Ok(panes_tbl) = ui_tbl.get::<Table>("panes") {
                let mut panes = UiPanes { parent: 30, current: 40, preview: 30 };
                if let Ok(v) = panes_tbl.get::<u16>("parent") { panes.parent = v; }
                if let Ok(v) = panes_tbl.get::<u16>("current") { panes.current = v; }
                if let Ok(v) = panes_tbl.get::<u16>("preview") { panes.preview = v; }
                ui.panes = Some(panes);
            }
            if let Ok(b) = ui_tbl.get::<bool>("show_hidden") { ui.show_hidden = b; }
            if let Ok(n) = ui_tbl.get::<u64>("preview_lines") { ui.preview_lines = n as usize; }
            if let Ok(n) = ui_tbl.get::<u64>("max_list_items") { ui.max_list_items = n as usize; }
            cfg_mut.ui = ui;
        }
        if let Ok(cmds_tbl) = tbl.get::<Table>("commands") {
            let mut cmds = Vec::new();
            for pair in cmds_tbl.pairs::<String, Value>() {
                let (name, v) = pair?;
                if let Value::Table(t) = v {
                    let mut spec = CommandSpec::default();
                    if let Ok(arr) = t.get::<Vec<String>>("cmd") { spec.cmd = arr; }
                    else if let Ok(s) = t.get::<String>("cmd") { spec.cmd = vec![s]; }
                    if let Ok(arr) = t.get::<Vec<String>>("args") { spec.args = arr; }
                    if let Ok(s) = t.get::<String>("when") { spec.when = Some(s); }
                    if let Ok(s) = t.get::<String>("cwd") { spec.cwd = Some(s); }
                    if let Ok(b) = t.get::<bool>("interactive") { spec.interactive = b; }
                    if let Ok(env_tbl) = t.get::<Table>("env") {
                        for kv in env_tbl.pairs::<String, String>() { let (k, v) = kv?; spec.env.push((k, v)); }
                    }
                    if let Ok(s) = t.get::<String>("confirm") { spec.confirm = Some(s); }
                    cmds.push((name, spec));
                }
            }
            cfg_mut.commands = cmds;
        }
        Ok(true)
    })?;

    // lv.mapkey(seq, action, desc?)
    let maps_clone = Rc::clone(&maps);
    let mapkey_fn = lua.create_function(move |_, (seq, action, desc): (String, String, Option<String>)| {
        maps_clone.borrow_mut().push(KeyMapping { sequence: seq, action, description: desc });
        Ok(true)
    })?;

    lv.set("config", config_fn)?;
    lv.set("mapkey", mapkey_fn)?;
    globals.set("lv", lv)?;
    Ok(())
}

fn install_require(lua: &Lua, lua_root: &Path) -> mlua::Result<()> {
    let root = lua_root.to_path_buf();
    let require_fn = lua.create_function(move |lua, name: String| {
        if name.contains("..") || name.starts_with('/') { return Err(LuaError::external("invalid module name")); }
        let rel_path = name.replace('.', "/");
        let path = root.join(format!("{}.lua", rel_path));
        // Canonicalize and ensure under root
        let canon = std::fs::canonicalize(&path).map_err(|e| LuaError::external(format!("{e}")))?;
        let canon_root = std::fs::canonicalize(&root).map_err(|e| LuaError::external(format!("{e}")))?;
        if !canon.starts_with(&canon_root) { return Err(LuaError::external("module outside config root")); }
        let code = std::fs::read_to_string(&canon).map_err(|e| LuaError::external(format!("{e}")))?;
        let chunk = lua.load(&code).set_name(name);
        chunk.eval::<Value>()
    })?;
    let globals = lua.globals();
    globals.set("require", require_fn)?;
    Ok(())
}
