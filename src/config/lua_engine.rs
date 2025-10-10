use mlua::{
  Lua,
  LuaOptions,
  Result as LuaResult,
  StdLib,
  Table,
};

/// LuaEngine creates a sandboxed Lua runtime for lsv configuration.
/// Safety model:
/// - Load only BASE | STRING | TABLE | MATH stdlibs (no io/os/debug/package).
/// - Provide an `lsv` table with stub functions (`config`, `mapkey`).
/// - A restricted `require()` is installed by the loader.
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
      let config_fn = lua.create_function(|_, _tbl: mlua::Value| Ok(true))?;

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
