use std::{
  fs,
  path::{
    Path,
    PathBuf,
  },
};

use mlua::{
  Error as LuaError,
  Lua,
  Table,
  Value,
};

use super::UiTheme;

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

pub(crate) fn resolve_theme_path(
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
      "theme '{}' returned {} (table expected)",
      path.display(),
      other.type_name()
    ))),
  }
}
