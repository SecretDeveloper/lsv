use mlua::{
  Error as LuaError,
  Lua,
  Value,
};
use std::path::Path;

pub(crate) fn install_require(
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
