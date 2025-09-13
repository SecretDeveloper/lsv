use std::io;

use crate::App;

pub(crate) fn dispatch_action(app: &mut App, action: &str) -> io::Result<bool> {
  // Support multiple commands separated by ';'
  let parts: Vec<&str> = action.split(';').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
  if parts.len() > 1 {
    let mut any = false;
    for p in parts {
      if run_single_action(app, p)? {
        any = true;
      }
      if app.should_quit { break; }
    }
    return Ok(any);
  }
  run_single_action(app, action)
}

fn run_single_action(app: &mut App, action: &str) -> io::Result<bool> {
  if let Some(rest) = action.strip_prefix("run_shell:") {
    if let Ok(idx) = rest.parse::<usize>() {
      if idx < app.config.shell_cmds.len() {
        let sc = app.config.shell_cmds[idx].clone();
        crate::cmd::run_shell_command(app, &sc);
        return Ok(true);
      }
    }
  }
  if let Some(rest) = action.strip_prefix("run_lua:") {
    if let Ok(idx) = rest.parse::<usize>() {
      return run_lua_action(app, idx);
    }
  }
  if let Some(int) = crate::actions::parse_internal_action(action) {
    crate::actions::execute_internal_action(app, int);
    return Ok(true);
  }
  Ok(false)
}

fn run_lua_action(app: &mut App, idx: usize) -> io::Result<bool> {
  let (engine, funcs) = match (app.lua_engine.as_ref(), app.lua_action_fns.as_ref()) {
    (Some(eng), Some(vec)) => (eng, vec),
    _ => return Ok(false),
  };
  if idx >= funcs.len() { return Ok(false); }
  let lua = engine.lua();
  let func = lua.registry_value::<mlua::Function>(&funcs[idx])
    .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("lua fn lookup: {e}")))?;
  // Build lsv helpers (placeholder; reserved for future helpers)
  let lsv_tbl = lua.create_table().map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
  // Build config snapshot table
  let cfg_tbl = crate::config_data::to_lua_config_table(lua, app)
    .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("build config tbl: {e}")))?;
  // Call function(lsv, config): may return a table; if not, use mutated arg
  let ret_val: mlua::Value = func
    .call((lsv_tbl, cfg_tbl.clone()))
    .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("lua fn: {e}")))?;
  let candidate_tbl = match ret_val {
    mlua::Value::Table(t) => t,
    _ => cfg_tbl,
  };
  // Validate and apply
  let data = crate::config_data::from_lua_config_table(candidate_tbl)
    .map_err(|msg| io::Error::new(io::ErrorKind::Other, format!("Config validation error: {}", msg)))?;
  apply_config_data(app, &data);
  Ok(true)
}

fn apply_config_data(app: &mut App, data: &crate::config_data::ConfigData) {
  // Collect diffs
  let mut relist = false;
  let mut redraw_only = false;
  let mut layout_change = false;
  let mut refresh_preview_only = false;

  // Capture current selection name for reselection on relist
  let selected_name = app.selected_entry().map(|e| e.name.clone());

  // Keys
  if app.config.keys.sequence_timeout_ms != data.keys_sequence_timeout_ms {
    app.config.keys.sequence_timeout_ms = data.keys_sequence_timeout_ms;
  }

  // UI panes
  let current_panes = app.config.ui.panes.clone().unwrap_or(crate::config::UiPanes { parent: 30, current: 40, preview: 30 });
  if current_panes.parent != data.ui.panes.parent
    || current_panes.current != data.ui.panes.current
    || current_panes.preview != data.ui.panes.preview
  {
    layout_change = true;
    app.config.ui.panes = Some(crate::config::UiPanes {
      parent: data.ui.panes.parent,
      current: data.ui.panes.current,
      preview: data.ui.panes.preview,
    });
  }

  // Hidden files affects listing
  if app.config.ui.show_hidden != data.ui.show_hidden {
    app.config.ui.show_hidden = data.ui.show_hidden;
    relist = true;
  }

  // Date format affects render only
  if app.config.ui.date_format != data.ui.date_format {
    app.config.ui.date_format = data.ui.date_format.clone();
    redraw_only = true;
  }

  // Display mode affects render only
  if app.display_mode != data.ui.display_mode {
    app.display_mode = data.ui.display_mode;
    redraw_only = true;
  }

  // Preview lines changes preview output trimming
  if app.config.ui.preview_lines != data.ui.preview_lines {
    app.config.ui.preview_lines = data.ui.preview_lines;
    refresh_preview_only = true;
  }

  // Max list items impacts listing/retrieval
  if app.config.ui.max_list_items != data.ui.max_list_items {
    app.config.ui.max_list_items = data.ui.max_list_items;
    relist = true;
  }

  // Row templates affect render only
  let current_row = app.config.ui.row.clone().unwrap_or_default();
  if current_row.icon != data.ui.row.icon
    || current_row.left != data.ui.row.left
    || current_row.middle != data.ui.row.middle
    || current_row.right != data.ui.row.right
  {
    app.config.ui.row = Some(crate::config::UiRowFormat {
      icon: data.ui.row.icon.clone(),
      left: data.ui.row.left.clone(),
      middle: data.ui.row.middle.clone(),
      right: data.ui.row.right.clone(),
    });
    redraw_only = true;
  }

  // Theme appearance affects render only
  let mut theme_changed = false;
  let cur_theme = app.config.ui.theme.clone().unwrap_or_default();
  let new_theme = if let Some(th) = data.ui.theme.as_ref() {
    let mut t = crate::config::UiTheme::default();
    t.pane_bg = th.pane_bg.clone();
    t.border_fg = th.border_fg.clone();
    t.item_fg = th.item_fg.clone();
    t.item_bg = th.item_bg.clone();
    t.selected_item_fg = th.selected_item_fg.clone();
    t.selected_item_bg = th.selected_item_bg.clone();
    t.title_fg = th.title_fg.clone();
    t.title_bg = th.title_bg.clone();
    t.info_fg = th.info_fg.clone();
    t.dir_fg = th.dir_fg.clone();
    t.dir_bg = th.dir_bg.clone();
    t.file_fg = th.file_fg.clone();
    t.file_bg = th.file_bg.clone();
    t.hidden_fg = th.hidden_fg.clone();
    t.hidden_bg = th.hidden_bg.clone();
    t.exec_fg = th.exec_fg.clone();
    t.exec_bg = th.exec_bg.clone();
    Some(t)
  } else { None };
  if new_theme.as_ref() != Some(&cur_theme) {
    app.config.ui.theme = new_theme;
    theme_changed = true;
  }
  if theme_changed { redraw_only = true; }

  // Sorting affects listing
  if app.sort_key != data.sort_key || app.sort_reverse != data.sort_reverse {
    app.sort_key = data.sort_key;
    app.sort_reverse = data.sort_reverse;
    relist = true;
  }

  // Info field affects render only
  if app.info_mode != data.show_field {
    app.info_mode = data.show_field;
    redraw_only = true;
  }

  // Apply effects
  if relist {
    app.refresh_lists();
    if let Some(name) = selected_name.as_ref() {
      if let Some(idx) = app.current_entries.iter().position(|e| &e.name == name) {
        app.list_state.select(Some(idx));
      }
    }
    app.refresh_preview();
    app.force_full_redraw = true;
    return;
  }

  if refresh_preview_only {
    app.refresh_preview();
  }

  if redraw_only || layout_change {
    app.force_full_redraw = true;
  }
}
