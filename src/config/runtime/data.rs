//! Helper types that convert between Lua configuration tables and strongly
//! typed Rust structures.
//!
//! These conversions are used when calling Lua actions: we build a snapshot of
//! the current configuration/context, pass it to Lua, and then merge any
//! returned changes back into Rust structs.

use crate::{
  app::App,
  enums::{
    display_mode_from_str,
    display_mode_to_str,
    info_mode_from_str,
    info_mode_to_str,
    sort_key_from_str,
    sort_key_to_str,
  },
};
use mlua::{
  Lua,
  Table,
};

/// Values-only snapshot of row formatting data used for Lua round-tripping.
#[derive(Debug, Clone)]
pub struct UiRowData
{
  pub icon:   String,
  pub left:   String,
  pub middle: String,
  pub right:  String,
}

#[derive(Debug, Clone, Default, PartialEq)]
/// Theme data mirrored into Lua.
pub struct UiThemeData
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
  pub selection_bar_fg:      Option<String>,
  pub selection_bar_copy_fg: Option<String>,
  pub selection_bar_move_fg: Option<String>,
}

#[derive(Debug, Clone)]
/// Pane split proportions mirrored into Lua.
pub struct UiPanesData
{
  pub parent:  u16,
  pub current: u16,
  pub preview: u16,
}

#[derive(Debug, Clone)]
/// User-interface block mirrored into Lua.
pub struct UiData
{
  pub panes:          UiPanesData,
  pub show_hidden:    bool,
  pub date_format:    Option<String>,
  pub display_mode:   crate::app::DisplayMode,
  pub max_list_items: usize,
  pub confirm_delete: bool,
  pub row:            UiRowData,
  pub row_widths:     Option<crate::config::UiRowWidths>,
  pub theme_path:     Option<String>,
  pub theme:          Option<UiThemeData>,
}

#[derive(Debug, Clone)]
/// Complete values-only configuration snapshot used during Lua calls.
pub struct ConfigData
{
  pub keys_sequence_timeout_ms: u64,
  pub ui: UiData,
  pub sort_key: crate::actions::SortKey,
  pub sort_reverse: bool,
  pub show_field: crate::app::InfoMode,
}

/// Convert the current [`App`] state into a Lua table expected by actions.
pub fn to_lua_config_table(
  lua: &Lua,
  app: &App,
) -> mlua::Result<Table>
{
  let tbl = lua.create_table()?;

  // keys
  let keys = lua.create_table()?;
  keys.set("sequence_timeout_ms", app.config.keys.sequence_timeout_ms)?;
  tbl.set("keys", keys)?;

  // ui
  let ui = lua.create_table()?;
  // panes
  let panes = lua.create_table()?;
  let (p, c, r) = if let Some(ref panes_cfg) = app.config.ui.panes
  {
    (panes_cfg.parent, panes_cfg.current, panes_cfg.preview)
  }
  else
  {
    (30u16, 40u16, 30u16)
  };
  panes.set("parent", p)?;
  panes.set("current", c)?;
  panes.set("preview", r)?;
  ui.set("panes", panes)?;
  ui.set("show_hidden", app.config.ui.show_hidden)?;
  if let Some(fmt) = app.config.ui.date_format.as_ref()
  {
    ui.set("date_format", fmt.as_str())?;
  }
  ui.set("display_mode", display_mode_to_str(app.display_mode))?;
  ui.set("max_list_items", app.config.ui.max_list_items as u64)?;
  ui.set("confirm_delete", app.config.ui.confirm_delete)?;

  // context snapshot for actions
  let ctx = lua.create_table()?;
  ctx.set("cwd", app.cwd.to_string_lossy().to_string())?;
  let sel_idx = app.list_state.selected().map(|i| i as u64).unwrap_or(u64::MAX);
  ctx.set("selected_index", sel_idx)?;
  ctx.set("current_len", app.current_entries.len() as u64)?;
  // Include commonly used path fields for convenience in actions
  use chrono::{
    DateTime,
    Local,
  };
  if let Some(sel) = app.selected_entry()
  {
    let path_s = sel.path.to_string_lossy().to_string();
    let dir_s =
      sel.path.parent().unwrap_or(&app.cwd).to_string_lossy().to_string();
    let name_s = sel
      .path
      .file_name()
      .map(|s| s.to_string_lossy().to_string())
      .unwrap_or_default();
    let ext_s =
      sel.path.extension().and_then(|s| s.to_str()).unwrap_or("").to_string();
    ctx.set("current_file", path_s)?;
    ctx.set("current_file_dir", dir_s)?;
    ctx.set("current_file_name", name_s)?;
    ctx.set("current_file_extension", ext_s)?;
    if let Some(ct) = sel.ctime
    {
      let fmt =
        app.config.ui.date_format.as_deref().unwrap_or("%Y-%m-%d %H:%M");
      let dt: DateTime<Local> = DateTime::from(ct);
      ctx.set("current_file_ctime", dt.format(fmt).to_string())?;
    }
    if let Some(mt) = sel.mtime
    {
      let fmt =
        app.config.ui.date_format.as_deref().unwrap_or("%Y-%m-%d %H:%M");
      let dt: DateTime<Local> = DateTime::from(mt);
      ctx.set("current_file_mtime", dt.format(fmt).to_string())?;
    }
  }
  else
  {
    let path_s = app.cwd.to_string_lossy().to_string();
    let dir_s = app.cwd.to_string_lossy().to_string();
    let name_s = app
      .cwd
      .file_name()
      .map(|s| s.to_string_lossy().to_string())
      .unwrap_or_default();
    ctx.set("current_file", path_s)?;
    ctx.set("current_file_dir", dir_s)?;
    ctx.set("current_file_name", name_s)?;
    ctx.set("current_file_extension", "")?;
  }
  ui.set(
    "context_note",
    "actions should use top-level context; kept for compatibility",
  )?;
  tbl.set("context", ctx.clone())?;

  // row
  let row = lua.create_table()?;
  let row_cfg = app.config.ui.row.clone().unwrap_or_default();
  row.set("icon", row_cfg.icon)?;
  row.set("left", row_cfg.left)?;
  row.set("middle", row_cfg.middle)?;
  row.set("right", row_cfg.right)?;
  ui.set("row", row)?;
  if let Some(rw) = app.config.ui.row_widths.as_ref()
  {
    let rw_tbl = lua.create_table()?;
    rw_tbl.set("icon", rw.icon as u64)?;
    rw_tbl.set("left", rw.left as u64)?;
    rw_tbl.set("middle", rw.middle as u64)?;
    rw_tbl.set("right", rw.right as u64)?;
    ui.set("row_widths", rw_tbl)?;
  }

  // theme
  if let Some(theme) = app.config.ui.theme.as_ref()
  {
    let theme_tbl = lua.create_table()?;
    if let Some(v) = theme.pane_bg.as_ref()
    {
      theme_tbl.set("pane_bg", v.as_str())?;
    }
    if let Some(v) = theme.border_fg.as_ref()
    {
      theme_tbl.set("border_fg", v.as_str())?;
    }
    if let Some(v) = theme.item_fg.as_ref()
    {
      theme_tbl.set("item_fg", v.as_str())?;
    }
    if let Some(v) = theme.item_bg.as_ref()
    {
      theme_tbl.set("item_bg", v.as_str())?;
    }
    if let Some(v) = theme.selected_item_fg.as_ref()
    {
      theme_tbl.set("selected_item_fg", v.as_str())?;
    }
    if let Some(v) = theme.selected_item_bg.as_ref()
    {
      theme_tbl.set("selected_item_bg", v.as_str())?;
    }
    if let Some(v) = theme.title_fg.as_ref()
    {
      theme_tbl.set("title_fg", v.as_str())?;
    }
    if let Some(v) = theme.title_bg.as_ref()
    {
      theme_tbl.set("title_bg", v.as_str())?;
    }
    if let Some(v) = theme.info_fg.as_ref()
    {
      theme_tbl.set("info_fg", v.as_str())?;
    }
    if let Some(v) = theme.dir_fg.as_ref()
    {
      theme_tbl.set("dir_fg", v.as_str())?;
    }
    if let Some(v) = theme.dir_bg.as_ref()
    {
      theme_tbl.set("dir_bg", v.as_str())?;
    }
    if let Some(v) = theme.file_fg.as_ref()
    {
      theme_tbl.set("file_fg", v.as_str())?;
    }
    if let Some(v) = theme.file_bg.as_ref()
    {
      theme_tbl.set("file_bg", v.as_str())?;
    }
    if let Some(v) = theme.hidden_fg.as_ref()
    {
      theme_tbl.set("hidden_fg", v.as_str())?;
    }
    if let Some(v) = theme.hidden_bg.as_ref()
    {
      theme_tbl.set("hidden_bg", v.as_str())?;
    }
    if let Some(v) = theme.exec_fg.as_ref()
    {
      theme_tbl.set("exec_fg", v.as_str())?;
    }
    if let Some(v) = theme.exec_bg.as_ref()
    {
      theme_tbl.set("exec_bg", v.as_str())?;
    }
    if let Some(v) = theme.selection_bar_fg.as_ref()
    {
      theme_tbl.set("selection_bar_fg", v.as_str())?;
    }
    if let Some(v) = theme.selection_bar_copy_fg.as_ref()
    {
      theme_tbl.set("selection_bar_copy_fg", v.as_str())?;
    }
    if let Some(v) = theme.selection_bar_move_fg.as_ref()
    {
      theme_tbl.set("selection_bar_move_fg", v.as_str())?;
    }
    ui.set("theme", theme_tbl)?;
  }
  if let Some(tp) = app.config.ui.theme_path.as_ref()
  {
    ui.set("theme_path", tp.to_string_lossy().to_string())?;
  }

  tbl.set("ui", &ui)?;

  // sort/show (under ui)
  ui.set("sort", sort_key_to_str(app.sort_key))?;
  ui.set("sort_reverse", app.sort_reverse)?;
  let show = info_mode_to_str(app.info_mode).unwrap_or("none");
  ui.set("show", show)?;

  Ok(tbl)
}

/// Parse a table returned from Lua into a typed overlay.
pub fn from_lua_config_table(tbl: Table) -> Result<ConfigData, String>
{
  let mut data = ConfigData {
    keys_sequence_timeout_ms: 0,
    ui: UiData {
      panes:          UiPanesData { parent: 30, current: 40, preview: 30 },
      show_hidden:    false,
      date_format:    None,
      display_mode:   crate::app::DisplayMode::Friendly,
      max_list_items: 5000,
      confirm_delete: true,
      row:            UiRowData {
        icon:   " ".into(),
        left:   "{name}".into(),
        middle: "".into(),
        right:  "{info}".into(),
      },
      row_widths:     None,
      theme_path:     None,
      theme:          None,
    },
    sort_key: crate::actions::SortKey::Name,
    sort_reverse: false,
    show_field: crate::app::InfoMode::None,
  };

  let keys = tbl
    .get::<Table>("keys")
    .map_err(|_| "missing or invalid table: keys".to_string())?;
  if let Ok(ms) = keys.get::<u64>("sequence_timeout_ms")
  {
    data.keys_sequence_timeout_ms = ms;
  }

  if let Ok(ui) = tbl.get::<Table>("ui")
  {
    if let Ok(panes) = ui.get::<Table>("panes")
    {
      if let Ok(v) = panes.get::<u64>("parent")
      {
        data.ui.panes.parent = v as u16;
      }
      if let Ok(v) = panes.get::<u64>("current")
      {
        data.ui.panes.current = v as u16;
      }
      if let Ok(v) = panes.get::<u64>("preview")
      {
        data.ui.panes.preview = v as u16;
      }
    }
    if let Ok(b) = ui.get::<bool>("show_hidden")
    {
      data.ui.show_hidden = b;
    }
    if let Ok(s) = ui.get::<String>("date_format")
    {
      data.ui.date_format = Some(s);
    }
    if let Ok(s) = ui.get::<String>("display_mode")
    {
      let Some(mode) = display_mode_from_str(&s)
      else
      {
        return Err(
          "ui.display_mode must be one of: absolute|friendly".to_string(),
        );
      };
      data.ui.display_mode = mode;
    }
    if let Ok(n) = ui.get::<u64>("max_list_items")
    {
      data.ui.max_list_items = n as usize;
    }
    if let Ok(b) = ui.get::<bool>("confirm_delete")
    {
      data.ui.confirm_delete = b;
    }

    if let Ok(row) = ui.get::<Table>("row")
    {
      let mut rd = data.ui.row.clone();
      if let Ok(s) = row.get::<String>("icon")
      {
        rd.icon = s;
      }
      if let Ok(s) = row.get::<String>("left")
      {
        rd.left = s;
      }
      if let Ok(s) = row.get::<String>("middle")
      {
        rd.middle = s;
      }
      if let Ok(s) = row.get::<String>("right")
      {
        rd.right = s;
      }
      data.ui.row = rd;
    }
    if let Ok(rw) = ui.get::<Table>("row_widths")
    {
      let mut widths = crate::config::UiRowWidths::default();
      if let Ok(v) = rw.get::<u64>("icon")
      {
        widths.icon = v as u16;
      }
      if let Ok(v) = rw.get::<u64>("left")
      {
        widths.left = v as u16;
      }
      if let Ok(v) = rw.get::<u64>("middle")
      {
        widths.middle = v as u16;
      }
      if let Ok(v) = rw.get::<u64>("right")
      {
        widths.right = v as u16;
      }
      data.ui.row_widths = Some(widths);
    }
    if let Ok(s) = ui.get::<String>("theme_path")
    {
      data.ui.theme_path = Some(s);
    }
    if let Ok(theme_tbl) = ui.get::<Table>("theme")
    {
      let mut th = UiThemeData::default();
      if let Ok(v) = theme_tbl.get::<String>("pane_bg")
      {
        th.pane_bg = Some(v);
      }
      if let Ok(v) = theme_tbl.get::<String>("border_fg")
      {
        th.border_fg = Some(v);
      }
      if let Ok(v) = theme_tbl.get::<String>("item_fg")
      {
        th.item_fg = Some(v);
      }
      if let Ok(v) = theme_tbl.get::<String>("item_bg")
      {
        th.item_bg = Some(v);
      }
      if let Ok(v) = theme_tbl.get::<String>("selected_item_fg")
      {
        th.selected_item_fg = Some(v);
      }
      if let Ok(v) = theme_tbl.get::<String>("selected_item_bg")
      {
        th.selected_item_bg = Some(v);
      }
      if let Ok(v) = theme_tbl.get::<String>("title_fg")
      {
        th.title_fg = Some(v);
      }
      if let Ok(v) = theme_tbl.get::<String>("title_bg")
      {
        th.title_bg = Some(v);
      }
      if let Ok(v) = theme_tbl.get::<String>("info_fg")
      {
        th.info_fg = Some(v);
      }
      if let Ok(v) = theme_tbl.get::<String>("dir_fg")
      {
        th.dir_fg = Some(v);
      }
      if let Ok(v) = theme_tbl.get::<String>("dir_bg")
      {
        th.dir_bg = Some(v);
      }
      if let Ok(v) = theme_tbl.get::<String>("file_fg")
      {
        th.file_fg = Some(v);
      }
      if let Ok(v) = theme_tbl.get::<String>("file_bg")
      {
        th.file_bg = Some(v);
      }
      if let Ok(v) = theme_tbl.get::<String>("hidden_fg")
      {
        th.hidden_fg = Some(v);
      }
      if let Ok(v) = theme_tbl.get::<String>("hidden_bg")
      {
        th.hidden_bg = Some(v);
      }
      if let Ok(v) = theme_tbl.get::<String>("exec_fg")
      {
        th.exec_fg = Some(v);
      }
      if let Ok(v) = theme_tbl.get::<String>("exec_bg")
      {
        th.exec_bg = Some(v);
      }
      if let Ok(v) = theme_tbl.get::<String>("selection_bar_fg")
      {
        th.selection_bar_fg = Some(v);
      }
      if let Ok(v) = theme_tbl.get::<String>("selection_bar_copy_fg")
      {
        th.selection_bar_copy_fg = Some(v);
      }
      if let Ok(v) = theme_tbl.get::<String>("selection_bar_move_fg")
      {
        th.selection_bar_move_fg = Some(v);
      }
      data.ui.theme = Some(th);
    }
  }

  if let Ok(ui) = tbl.get::<Table>("ui")
  {
    if let Ok(v) = ui.get::<String>("sort")
    {
      let Some(k) = sort_key_from_str(&v)
      else
      {
        return Err(
          "sort.key must be one of: name|size|mtime|created".to_string(),
        );
      };
      data.sort_key = k;
    }
    if let Ok(b) = ui.get::<bool>("sort_reverse")
    {
      data.sort_reverse = b;
    }
    if let Ok(v) = ui.get::<String>("show")
      && let Some(m) = info_mode_from_str(&v)
    {
      data.show_field = m;
    }
  }

  Ok(data)
}
