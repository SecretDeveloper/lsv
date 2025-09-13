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

#[derive(Debug, Clone, Default, PartialEq)]
pub struct UiThemeData {
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
  pub row_widths: Option<super::config::UiRowWidths>,
  pub theme: Option<UiThemeData>,
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
  if let Some(rw) = app.config.ui.row_widths.as_ref() {
    let rw_tbl = lua.create_table()?;
    rw_tbl.set("icon", rw.icon as u64)?;
    rw_tbl.set("left", rw.left as u64)?;
    rw_tbl.set("middle", rw.middle as u64)?;
    rw_tbl.set("right", rw.right as u64)?;
    ui.set("row_widths", rw_tbl)?;
  }

  // theme
  if let Some(theme) = app.config.ui.theme.as_ref() {
    let theme_tbl = lua.create_table()?;
    if let Some(v) = theme.pane_bg.as_ref() { theme_tbl.set("pane_bg", v.as_str())?; }
    if let Some(v) = theme.border_fg.as_ref() { theme_tbl.set("border_fg", v.as_str())?; }
    if let Some(v) = theme.item_fg.as_ref() { theme_tbl.set("item_fg", v.as_str())?; }
    if let Some(v) = theme.item_bg.as_ref() { theme_tbl.set("item_bg", v.as_str())?; }
    if let Some(v) = theme.selected_item_fg.as_ref() { theme_tbl.set("selected_item_fg", v.as_str())?; }
    if let Some(v) = theme.selected_item_bg.as_ref() { theme_tbl.set("selected_item_bg", v.as_str())?; }
    if let Some(v) = theme.title_fg.as_ref() { theme_tbl.set("title_fg", v.as_str())?; }
    if let Some(v) = theme.title_bg.as_ref() { theme_tbl.set("title_bg", v.as_str())?; }
    if let Some(v) = theme.info_fg.as_ref() { theme_tbl.set("info_fg", v.as_str())?; }
    if let Some(v) = theme.dir_fg.as_ref() { theme_tbl.set("dir_fg", v.as_str())?; }
    if let Some(v) = theme.dir_bg.as_ref() { theme_tbl.set("dir_bg", v.as_str())?; }
    if let Some(v) = theme.file_fg.as_ref() { theme_tbl.set("file_fg", v.as_str())?; }
    if let Some(v) = theme.file_bg.as_ref() { theme_tbl.set("file_bg", v.as_str())?; }
    if let Some(v) = theme.hidden_fg.as_ref() { theme_tbl.set("hidden_fg", v.as_str())?; }
    if let Some(v) = theme.hidden_bg.as_ref() { theme_tbl.set("hidden_bg", v.as_str())?; }
    if let Some(v) = theme.exec_fg.as_ref() { theme_tbl.set("exec_fg", v.as_str())?; }
    if let Some(v) = theme.exec_bg.as_ref() { theme_tbl.set("exec_bg", v.as_str())?; }
    ui.set("theme", theme_tbl)?;
  }

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
  let theme = match ui_tbl.get::<Value>("theme") {
    Ok(Value::Table(t)) => {
      let mut th = UiThemeData::default();
      th.pane_bg = get_opt_str(&t, "pane_bg")?;
      th.border_fg = get_opt_str(&t, "border_fg")?;
      th.item_fg = get_opt_str(&t, "item_fg")?;
      th.item_bg = get_opt_str(&t, "item_bg")?;
      th.selected_item_fg = get_opt_str(&t, "selected_item_fg")?;
      th.selected_item_bg = get_opt_str(&t, "selected_item_bg")?;
      th.title_fg = get_opt_str(&t, "title_fg")?;
      th.title_bg = get_opt_str(&t, "title_bg")?;
      th.info_fg = get_opt_str(&t, "info_fg")?;
      th.dir_fg = get_opt_str(&t, "dir_fg")?;
      th.dir_bg = get_opt_str(&t, "dir_bg")?;
      th.file_fg = get_opt_str(&t, "file_fg")?;
      th.file_bg = get_opt_str(&t, "file_bg")?;
      th.hidden_fg = get_opt_str(&t, "hidden_fg")?;
      th.hidden_bg = get_opt_str(&t, "hidden_bg")?;
      th.exec_fg = get_opt_str(&t, "exec_fg")?;
      th.exec_bg = get_opt_str(&t, "exec_bg")?;
      Some(th)
    }
    _ => None,
  };
  let ui = UiData {
    panes: UiPanesData { parent, current, preview },
    show_hidden,
    date_format,
    display_mode,
    preview_lines: preview_lines_u64 as usize,
    max_list_items: max_list_items_u64 as usize,
    row,
    row_widths: match ui_tbl.get::<Value>("row_widths") {
      Ok(Value::Table(t)) => {
        let mut rw = super::config::UiRowWidths::default();
        rw.icon = t.get::<u64>("icon").unwrap_or(0) as u16;
        rw.left = t.get::<u64>("left").unwrap_or(0) as u16;
        rw.middle = t.get::<u64>("middle").unwrap_or(0) as u16;
        rw.right = t.get::<u64>("right").unwrap_or(0) as u16;
        Some(rw)
      }
      _ => None,
    },
    theme,
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
