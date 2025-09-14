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
  // Build config snapshot table (mutable by Lua)
  let cfg_tbl = crate::config_data::to_lua_config_table(lua, app)
    .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("build config tbl: {e}")))?;
  // Build lsv helpers that can update config.context
  let lsv_tbl = {
    let tbl = lua.create_table().map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    // lsv.select_item(index)
    let cfg_ref = cfg_tbl.clone();
    let select_item_fn = lua.create_function(move |lua, idx: i64| {
      let ctx: mlua::Table = match cfg_ref.get("context") { Ok(t) => t, Err(_) => {
        let t = lua.create_table()?; cfg_ref.set("context", t.clone())?; t }
      };
      let i = if idx < 0 { 0 } else { idx as u64 };
      ctx.set("selected_index", i)?;
      Ok(true)
    }).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    tbl.set("select_item", select_item_fn).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    // lsv.select_last_item()
    let cfg_ref2 = cfg_tbl.clone();
    let select_last_fn = lua.create_function(move |_, ()| {
      if let Ok(ctx) = cfg_ref2.get::<mlua::Table>("context") {
        let len = ctx.get::<u64>("current_len").unwrap_or(0);
        if len > 0 { ctx.set("selected_index", len - 1)?; }
      }
      Ok(true)
    }).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    tbl.set("select_last_item", select_last_fn).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    // lsv.quit()
    let cfg_ref3 = cfg_tbl.clone();
    let quit_fn = lua.create_function(move |_, ()| { cfg_ref3.set("quit", true)?; Ok(true) })
      .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    tbl.set("quit", quit_fn).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    // lsv.display_output(text, title?)
    let cfg_ref4 = cfg_tbl.clone();
    let display_output_fn = lua.create_function(move |_, (text, title): (String, Option<String>)| {
      cfg_ref4.set("output_text", text)?;
      if let Some(t) = title { cfg_ref4.set("output_title", t)?; }
      Ok(true)
    }).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    tbl.set("display_output", display_output_fn).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    // lsv.os_run(cmd): run external command and put output in config
    let cfg_ref5 = cfg_tbl.clone();
    let cwd_str = app.cwd.to_string_lossy().to_string();
    let sel_path = app
      .selected_entry()
      .map(|e| e.path.clone())
      .unwrap_or_else(|| app.cwd.clone());
    let sel_dir = sel_path.parent().unwrap_or(&app.cwd).to_path_buf();
    let path_str = sel_path.to_string_lossy().to_string();
    let dir_str = sel_dir.to_string_lossy().to_string();
    let name_str = sel_path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let os_run_fn = lua.create_function(move |_, cmd: String| {
      let out = std::process::Command::new("sh")
        .arg("-lc").arg(&cmd)
        .current_dir(&cwd_str)
        .env("LSV_PATH", &path_str)
        .env("LSV_DIR", &dir_str)
        .env("LSV_NAME", &name_str)
        .output();
      match out {
        Ok(output) => {
          let mut buf = Vec::new();
          buf.extend_from_slice(&output.stdout);
          if !output.stderr.is_empty() { buf.push(b'\n'); buf.extend_from_slice(&output.stderr); }
          let text = String::from_utf8_lossy(&buf).to_string();
          cfg_ref5.set("output_text", text)?;
          cfg_ref5.set("output_title", format!("$ {}", cmd))?;
          Ok(true)
        }
        Err(e) => {
          cfg_ref5.set("output_text", format!("<error: {}>", e))?;
          cfg_ref5.set("output_title", format!("$ {}", cmd))?;
          Ok(true)
        }
      }
    }).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    tbl.set("os_run", os_run_fn).map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    tbl
  };
  // Call function(lsv, config): may return a table; if not, use mutated arg
  let ret_val: mlua::Value = func
    .call((lsv_tbl, cfg_tbl.clone()))
    .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("lua fn: {e}")))?;
  let candidate_tbl = match ret_val {
    mlua::Value::Table(t) => t,
    _ => cfg_tbl,
  };

  // Handle context-driven navigation
  if let Ok(ctx) = candidate_tbl.get::<mlua::Table>("context") {
    if let Ok(sel_idx) = ctx.get::<u64>("selected_index") {
      let len = app.current_entries.len();
      if len > 0 {
        let idx = (sel_idx as usize).min(len.saturating_sub(1));
        app.list_state.select(Some(idx));
        app.refresh_preview();
      }
    }
  }
  // Handle messages directive
  if let Ok(m) = candidate_tbl.get::<String>("messages") {
    match m.as_str() {
      "toggle" => { app.show_messages = !app.show_messages; app.force_full_redraw = true; }
      "show" => { app.show_messages = true; app.force_full_redraw = true; }
      "hide" => { app.show_messages = false; app.force_full_redraw = true; }
      _ => {}
    }
  }
  // Handle special action directives embedded in the returned table (legacy)
  if let Ok(nav) = candidate_tbl.get::<String>("nav") {
    match nav.as_str() {
      "top" => {
        if !app.current_entries.is_empty() {
          app.list_state.select(Some(0));
          app.refresh_preview();
        }
      }
      "bottom" => {
        if !app.current_entries.is_empty() {
          let last = app.current_entries.len().saturating_sub(1);
          app.list_state.select(Some(last));
          app.refresh_preview();
        }
      }
      _ => {}
    }
  }
  // Handle output request from Lua
  if let Ok(text) = candidate_tbl.get::<String>("output_text") {
    let title = candidate_tbl.get::<String>("output_title").unwrap_or_else(|_| String::from("Output"));
    app.display_output(&title, &text);
  }
  if let Ok(o) = candidate_tbl.get::<String>("output") {
    match o.as_str() {
      "toggle" => { app.show_output = !app.show_output; app.show_messages = false; app.show_whichkey = false; app.force_full_redraw = true; }
      "show" => { app.show_output = true; app.show_messages = false; app.show_whichkey = false; app.force_full_redraw = true; }
      "hide" => { app.show_output = false; app.force_full_redraw = true; }
      _ => {}
    }
  }
  if let Ok(q) = candidate_tbl.get::<bool>("quit") {
    if q { app.should_quit = true; }
  }
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

  // Row widths affect layout only
  let cur_widths = app.config.ui.row_widths.clone().unwrap_or_default();
  let new_widths = match data.ui.row_widths.as_ref() {
    Some(rw) => crate::config::UiRowWidths { icon: rw.icon, left: rw.left, middle: rw.middle, right: rw.right },
    None => crate::config::UiRowWidths::default(),
  };
  if cur_widths != new_widths {
    app.config.ui.row_widths = Some(new_widths);
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
