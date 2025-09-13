use mlua::{Lua, Table, Value};

// A values-only snapshot of configuration used for Lua round-tripping.
// This excludes function fields (previewer/action fns) and keymaps.
#[derive(Debug, Clone)]
pub struct UiRowData {
  pub icon: String,
  pub left: String,
  pub middle: String,
  pub right: String,
}

#[derive(Debug, Clone)]
pub struct UiPanesData {
  pub parent: u16,
  pub current: u16,
  pub preview: u16,
}

#[derive(Debug, Clone)]
pub struct UiData {
  pub panes: UiPanesData,
  pub show_hidden: bool,
  pub date_format: Option<String>,
  pub display_mode: crate::app::DisplayMode,
  pub preview_lines: usize,
  pub max_list_items: usize,
  pub row: UiRowData,
}

#[derive(Debug, Clone)]
pub struct ConfigData {
  pub keys_sequence_timeout_ms: u64,
  pub ui: UiData,
  pub sort_key: crate::actions::SortKey,
  pub sort_reverse: bool,
  pub show_field: crate::app::InfoMode,
}

pub fn to_lua_config_table(lua: &Lua, app: &crate::App) -> mlua::Result<Table> {
  let tbl = lua.create_table()?;

  // keys
  let keys = lua.create_table()?;
  keys.set("sequence_timeout_ms", app.config.keys.sequence_timeout_ms)?;
  tbl.set("keys", keys)?;

  // ui
  let ui = lua.create_table()?;
  // panes
  let panes = lua.create_table()?;
  let (p, c, r) = if let Some(ref panes_cfg) = app.config.ui.panes {
    (panes_cfg.parent, panes_cfg.current, panes_cfg.preview)
  } else {
    (30u16, 40u16, 30u16)
  };
  panes.set("parent", p)?;
  panes.set("current", c)?;
  panes.set("preview", r)?;
  ui.set("panes", panes)?;
  ui.set("show_hidden", app.config.ui.show_hidden)?;
  if let Some(fmt) = app.config.ui.date_format.as_ref() {
    ui.set("date_format", fmt.as_str())?;
  }
  ui.set("display_mode", crate::enums::display_mode_to_str(app.display_mode))?;
  ui.set("preview_lines", app.config.ui.preview_lines as u64)?;
  ui.set("max_list_items", app.config.ui.max_list_items as u64)?;

  // row
  let row = lua.create_table()?;
  let row_cfg = app.config.ui.row.clone().unwrap_or_default();
  row.set("icon", row_cfg.icon)?;
  row.set("left", row_cfg.left)?;
  row.set("middle", row_cfg.middle)?;
  row.set("right", row_cfg.right)?;
  ui.set("row", row)?;

  tbl.set("ui", ui.clone())?;

  // sort and show as simple values under ui
  ui.set("sort", crate::enums::sort_key_to_str(app.sort_key))?;
  ui.set("sort_reverse", app.sort_reverse)?;

  // show: simple string label
  if let Some(lbl) = crate::enums::info_mode_to_str(app.info_mode) {
    ui.set("show", lbl)?;
  } else {
    ui.set("show", "none")?;
  }

  Ok(tbl)
}

pub fn from_lua_config_table(tbl: Table) -> Result<ConfigData, String> {
  // keys
  let keys_tbl: Table = get_req_tbl(&tbl, "keys")?;
  let keys_sequence_timeout_ms: u64 = get_u64(&keys_tbl, "sequence_timeout_ms")?;

  // ui
  let ui_tbl: Table = get_req_tbl(&tbl, "ui")?;
  let panes_tbl: Table = get_req_tbl(&ui_tbl, "panes")?;
  let parent = get_u16(&panes_tbl, "parent")?;
  let current = get_u16(&panes_tbl, "current")?;
  let preview = get_u16(&panes_tbl, "preview")?;
  let show_hidden = get_bool(&ui_tbl, "show_hidden")?;
  let date_format = get_opt_str(&ui_tbl, "date_format")?;
  let display_mode_str = get_str_or_default(&ui_tbl, "display_mode", "absolute")?;
  let display_mode = crate::enums::display_mode_from_str(&display_mode_str)
    .ok_or_else(|| format!("ui.display_mode must be 'absolute' or 'friendly'"))?;
  let preview_lines_u64 = get_u64(&ui_tbl, "preview_lines")?;
  let max_list_items_u64 = get_u64(&ui_tbl, "max_list_items")?;
  let row_tbl: Table = get_req_tbl(&ui_tbl, "row")?;
  let row = UiRowData {
    icon: get_string(&row_tbl, "icon")?,
    left: get_string(&row_tbl, "left")?,
    middle: get_string(&row_tbl, "middle")?,
    right: get_string(&row_tbl, "right")?,
  };
  let ui = UiData {
    panes: UiPanesData { parent, current, preview },
    show_hidden,
    date_format,
    display_mode,
    preview_lines: preview_lines_u64 as usize,
    max_list_items: max_list_items_u64 as usize,
    row,
  };

  // sort (under ui)
  let sort_key_str = get_string(&ui_tbl, "sort")?;
  let sort_key = crate::enums::sort_key_from_str(&sort_key_str)
    .ok_or_else(|| format!("sort.key must be one of name|size|mtime"))?;
  let sort_reverse = get_bool(&ui_tbl, "sort_reverse")?;

  // show (under ui)
  let field_str = get_string(&ui_tbl, "show")?;
  let show_field = if field_str.eq_ignore_ascii_case("none") {
    crate::app::InfoMode::None
  } else {
    crate::enums::info_mode_from_str(&field_str)
      .unwrap_or(crate::app::InfoMode::None)
  };

  Ok(ConfigData {
    keys_sequence_timeout_ms,
    ui,
    sort_key,
    sort_reverse,
    show_field,
  })
}

// ---------- small helpers ----------

fn get_req_tbl(t: &Table, key: &str) -> Result<Table, String> {
  t.get::<Table>(key).map_err(|_| format!("missing or invalid table: {}", key))
}

fn get_string(t: &Table, key: &str) -> Result<String, String> {
  t.get::<String>(key).map_err(|_| format!("{} must be string", key))
}

fn get_opt_str(t: &Table, key: &str) -> Result<Option<String>, String> {
  match t.get::<Value>(key) {
    Ok(Value::String(s)) => {
      let st = match s.to_str() { Ok(v) => v.to_string(), Err(_) => String::new() };
      Ok(Some(st))
    }
    Ok(Value::Nil) | Err(_) => Ok(None),
    _ => Err(format!("{} must be string or nil", key)),
  }
}

fn get_bool(t: &Table, key: &str) -> Result<bool, String> {
  t.get::<bool>(key).map_err(|_| format!("{} must be boolean", key))
}

fn get_u64(t: &Table, key: &str) -> Result<u64, String> {
  t.get::<u64>(key).map_err(|_| format!("{} must be integer", key))
}

fn get_u16(t: &Table, key: &str) -> Result<u16, String> {
  t.get::<u16>(key).map_err(|_| format!("{} must be integer (0..=65535)", key))
}

fn get_str_or_default(t: &Table, key: &str, default: &str) -> Result<String, String> {
  match t.get::<Value>(key) {
    Ok(Value::String(s)) => Ok(s.to_str().map(|st| st.to_string()).unwrap_or(default.to_string())),
    Ok(Value::Nil) | Err(_) => Ok(default.to_string()),
    _ => Err(format!("{} must be string", key)),
  }
}
