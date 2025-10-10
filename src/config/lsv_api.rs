use mlua::{
  Error as LuaError,
  Lua,
  Table,
  Value,
};
use std::{
  cell::RefCell,
  io,
  rc::Rc,
};

use super::{
  Config,
  UiPanes,
  load_theme_table_from_path,
  merge_theme_table,
  resolve_theme_path,
};

pub(crate) fn install_lsv_api(
  lua: &Lua,
  config_acc: Rc<RefCell<Config>>,
  maps: Rc<RefCell<Vec<super::KeyMapping>>>,
  previewer_key_out: Rc<RefCell<Option<mlua::RegistryKey>>>,
  lua_action_keys_out: Rc<RefCell<Vec<mlua::RegistryKey>>>,
  config_root: Option<std::path::PathBuf>,
) -> io::Result<()>
{
  let globals = lua.globals();
  let lsv: Table =
    lua.create_table().map_err(|e| io::Error::other(e.to_string()))?;

  // lsv.config(tbl): apply fields into Config accumulator
  let config_acc_clone = Rc::clone(&config_acc);
  let theme_root = config_root.clone();
  // Clone shared accumulators for use inside config_fn closure
  let maps_in_cfg = Rc::clone(&maps);
  let actions_in_cfg = Rc::clone(&lua_action_keys_out);
  let config_fn = lua
    .create_function(move |lua, tbl: Value| {
      if let Value::Table(t) = tbl
      {
        let mut cfg_mut = config_acc_clone
          .try_borrow_mut()
          .map_err(|e| LuaError::RuntimeError(e.to_string()))?;
        if let Ok(v) = t.get::<u32>("config_version")
        {
          cfg_mut.config_version = v;
        }
        // icons
        if let Ok(icons_tbl) = t.get::<Table>("icons")
        {
          let mut icons = cfg_mut.icons.clone();
          if let Ok(b) = icons_tbl.get::<bool>("enabled")
          {
            icons.enabled = b;
          }
          if let Ok(s) = icons_tbl.get::<String>("preset")
          {
            icons.preset = Some(s);
          }
          if let Ok(s) = icons_tbl.get::<String>("font")
          {
            icons.font = Some(s);
          }
          if let Ok(s) = icons_tbl.get::<String>("default_file")
          {
            icons.default_file = Some(s);
          }
          if let Ok(s) = icons_tbl.get::<String>("default_dir")
          {
            icons.default_dir = Some(s);
          }
          // Legacy: icons.by_ext (deprecated). Parse first so `extensions`
          // wins.
          if let Ok(ext_tbl) = icons_tbl.get::<Table>("by_ext")
          {
            for pair in ext_tbl.pairs::<Value, Value>()
            {
              let (k, v) =
                pair.map_err(|e| LuaError::RuntimeError(e.to_string()))?;
              match (k, v)
              {
                (Value::String(ks), Value::String(vs)) =>
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
                (Value::Table(t), _) =>
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
            for pair in ext_tbl.pairs::<Value, Value>()
            {
              let (k, v) =
                pair.map_err(|e| LuaError::RuntimeError(e.to_string()))?;
              match (k, v)
              {
                (Value::String(ks), Value::String(vs)) =>
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
                (Value::Table(t), _) =>
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
          if let Ok(map_tbl) = icons_tbl.get::<Table>("mappings")
          {
            if let Ok(ext_tbl) = map_tbl.get::<Table>("extensions")
            {
              for pair in ext_tbl.pairs::<Value, Value>()
              {
                let (k, v) =
                  pair.map_err(|e| LuaError::RuntimeError(e.to_string()))?;
                match (k, v)
                {
                  (Value::String(ks), Value::String(vs)) =>
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
                  (Value::Table(t), _) =>
                  {
                    if let Ok(icon) = t.get::<String>("icon")
                      && let Ok(list) = t.get::<Table>("names")
                    {
                      for n in list.sequence_values::<String>().flatten()
                      {
                        let n = n.trim().to_string();
                        if !n.is_empty()
                        {
                          icons
                            .extensions
                            .insert(n.to_lowercase(), icon.clone());
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
              for pair in f_tbl.pairs::<Value, Value>()
              {
                let (k, v) =
                  pair.map_err(|e| LuaError::RuntimeError(e.to_string()))?;
                match (k, v)
                {
                  (Value::String(ks), Value::String(vs)) =>
                  {
                    if let (Ok(k), Ok(icon)) = (ks.to_str(), vs.to_str())
                    {
                      for name in k.split([',', '|', ';', '/'])
                      {
                        let n = name.trim();
                        if !n.is_empty()
                        {
                          icons
                            .folders
                            .insert(n.to_lowercase(), icon.to_string());
                        }
                      }
                    }
                  }
                  (Value::Table(t), _) =>
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
          // Legacy alias: icons.by_name
          if let Ok(f_tbl) = icons_tbl.get::<Table>("by_name")
          {
            for pair in f_tbl.pairs::<Value, Value>()
            {
              let (k, v) =
                pair.map_err(|e| LuaError::RuntimeError(e.to_string()))?;
              if let (Value::String(ks), Value::String(vs)) = (k, v)
                && let (Ok(k), Ok(v)) = (ks.to_str(), vs.to_str())
              {
                icons.folders.insert(k.to_lowercase(), v.to_string());
              }
            }
          }
          // Legacy alias: icons.folders (simple map name->icon)
          if let Ok(f_tbl) = icons_tbl.get::<Table>("folders")
          {
            for pair in f_tbl.pairs::<Value, Value>()
            {
              let (k, v) =
                pair.map_err(|e| LuaError::RuntimeError(e.to_string()))?;
              if let (Value::String(ks), Value::String(vs)) = (k, v)
                && let (Ok(k), Ok(v)) = (ks.to_str(), vs.to_str())
              {
                icons.folders.insert(k.to_lowercase(), v.to_string());
              }
            }
          }
          cfg_mut.icons = icons;
        }
        if let Ok(keys_tbl) = t.get::<Table>("keys")
        {
          let mut keys = cfg_mut.keys.clone();
          if let Ok(ms) = keys_tbl.get::<u64>("sequence_timeout_ms")
          {
            keys.sequence_timeout_ms = ms;
          }
          cfg_mut.keys = keys;
        }
        if let Ok(ui_tbl) = t.get::<Table>("ui")
        {
          merge_ui_table(lua, theme_root.as_deref(), &ui_tbl, &mut cfg_mut)?;
        }

        // Top-level actions table (collect both Lua fn and string actions)
        if let Ok(actions_tbl) = t.get::<Table>("actions")
        {
          for pair in actions_tbl.sequence_values::<Value>()
          {
            if let Value::Table(t) =
              pair.map_err(|e| LuaError::RuntimeError(e.to_string()))?
            {
              // Lua function action: fn = function(lsv, config) ... end
              if let Ok(func) = t.get::<mlua::Function>("fn")
              {
                let keymap = t
                  .get::<String>("keymap")
                  .map_err(|e| LuaError::RuntimeError(e.to_string()))?;
                let desc = t.get::<String>("description").ok();
                let reg = lua
                  .create_registry_value(func)
                  .map_err(|e| LuaError::RuntimeError(e.to_string()))?;
                let idx = actions_in_cfg.borrow().len();
                actions_in_cfg.borrow_mut().push(reg);
                maps_in_cfg.borrow_mut().push(super::KeyMapping {
                  sequence:    keymap,
                  action:      format!("run_lua:{}", idx),
                  description: desc,
                });
                continue;
              }
              // String action
              if let (Ok(kseq), Ok(action_str)) =
                (t.get::<String>("keymap"), t.get::<String>("action"))
              {
                let desc = t.get::<String>("description").ok();
                maps_in_cfg.borrow_mut().push(super::KeyMapping {
                  sequence:    kseq,
                  action:      action_str,
                  description: desc,
                });
              }
            }
          }
        }
      }
      Ok(true)
    })
    .map_err(|e| io::Error::other(e.to_string()))?;

  // Accumulate keymaps
  let maps_for_config = Rc::clone(&maps);
  let mapkey_fn = lua
    .create_function(
      move |_, (seq, action, desc): (String, String, Option<String>)| {
        maps_for_config.borrow_mut().push(super::KeyMapping {
          sequence: seq,
          action,
          description: desc,
        });
        Ok(true)
      },
    )
    .map_err(|e| io::Error::other(e.to_string()))?;

  // set_previewer(function)
  let prev_out = Rc::clone(&previewer_key_out);
  let set_previewer_fn = lua
    .create_function(move |lua, func: mlua::Function| {
      let key = lua.create_registry_value(func)?;
      *prev_out.borrow_mut() = Some(key);
      Ok(true)
    })
    .map_err(|e| io::Error::other(e.to_string()))?;

  // lsv.map_action(keymap_or_list, description, fn)
  let actions_acc_outer = Rc::clone(&lua_action_keys_out);
  let maps_for_actions_outer = Rc::clone(&maps);
  let map_action_fn = lua
    .create_function(
      move |lua, (keymaps_val, desc, func): (Value, String, mlua::Function)| {
        let reg = lua.create_registry_value(func)?;
        let idx = actions_acc_outer.borrow().len();
        actions_acc_outer.borrow_mut().push(reg);
        let action_str = format!("run_lua:{}", idx);
        match keymaps_val
        {
          Value::String(s) =>
          {
            let seq = s.to_str().map(|v| v.to_string()).unwrap_or_default();
            maps_for_actions_outer.borrow_mut().push(super::KeyMapping {
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
                maps_for_actions_outer.borrow_mut().push(super::KeyMapping {
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
    )
    .map_err(|e| io::Error::other(e.to_string()))?;

  // Wire helpers
  lsv.set("config", config_fn).map_err(|e| io::Error::other(e.to_string()))?;
  lsv.set("mapkey", mapkey_fn).map_err(|e| io::Error::other(e.to_string()))?;
  lsv
    .set("set_previewer", set_previewer_fn)
    .map_err(|e| io::Error::other(e.to_string()))?;
  lsv
    .set("map_action", map_action_fn)
    .map_err(|e| io::Error::other(e.to_string()))?;

  // lsv.quote
  let quote_fn = lua
    .create_function(|_, s: String| {
      #[cfg(windows)]
      {
        Ok(format!("\"{}\"", s.replace('"', "\"\"")))
      }
      #[cfg(not(windows))]
      {
        Ok(format!("'{}'", s.replace('\'', "'\\''")))
      }
    })
    .map_err(|e| io::Error::other(e.to_string()))?;
  lsv.set("quote", quote_fn).map_err(|e| io::Error::other(e.to_string()))?;

  // get_os_name, getenv, trace
  let os_fn = lua
    .create_function(|_, ()| Ok(std::env::consts::OS.to_string()))
    .map_err(|e| io::Error::other(e.to_string()))?;
  lsv.set("get_os_name", os_fn).map_err(|e| io::Error::other(e.to_string()))?;
  let getenv_fn = lua
    .create_function(|_, (name, default): (String, Option<String>)| {
      Ok(std::env::var(&name).ok().or(default))
    })
    .map_err(|e| io::Error::other(e.to_string()))?;
  lsv.set("getenv", getenv_fn).map_err(|e| io::Error::other(e.to_string()))?;
  let trace_fn = lua
    .create_function(|_, text: String| {
      crate::trace::log(text);
      Ok(true)
    })
    .map_err(|e| io::Error::other(e.to_string()))?;
  lsv.set("trace", trace_fn).map_err(|e| io::Error::other(e.to_string()))?;

  // Unknown function guard at config time
  let mt = lua.create_table().map_err(|e| io::Error::other(e.to_string()))?;
  let idx = lua
    .create_function(move |lua, (_tbl, key): (Table, Value)| {
      let name = match key
      {
        Value::String(s) => s
          .to_str()
          .map(|v| v.to_string())
          .unwrap_or_else(|_| String::from("?")),
        other => format!("{:?}", other),
      };
      let func = lua.create_function(move |_, ()| -> mlua::Result<()> {
        Err(mlua::Error::RuntimeError(format!(
          "unknown lsv function: {}",
          name
        )))
      })?;
      Ok(func)
    })
    .map_err(|e| io::Error::other(e.to_string()))?;
  mt.set("__index", idx).map_err(|e| io::Error::other(e.to_string()))?;
  let _ = lsv.set_metatable(Some(mt));
  globals.set("lsv", lsv).map_err(|e| io::Error::other(e.to_string()))?;
  Ok(())
}

// Small helper to merge UI table (moved from config.rs; kept private here)
fn merge_ui_table(
  lua: &Lua,
  theme_root: Option<&std::path::Path>,
  ui_tbl: &Table,
  cfg_mut: &mut Config,
) -> Result<(), LuaError>
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
  // Theme loading from file path
  if let Ok(theme_path_str) = ui_tbl.get::<String>("theme_path")
  {
    if theme_path_str.trim().is_empty()
    {
      return Err(LuaError::RuntimeError(
        "ui.theme_path must be a non-empty string".to_string(),
      ));
    }
    let resolved_path = resolve_theme_path(&theme_path_str, theme_root);
    let theme_tbl = load_theme_table_from_path(lua, &resolved_path)?;
    let mut th = cfg_mut.ui.theme.clone().unwrap_or_default();
    merge_theme_table(&theme_tbl, &mut th);
    cfg_mut.ui.theme = Some(th);
    cfg_mut.ui.theme_path = Some(resolved_path);
  }
  // Inline or module theme overlay
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
        // Load module via Lua require() and merge returned table
        let globals = lua.globals();
        let require_fn: mlua::Function = globals.get("require")?;
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
        // Record resolved path if possible (root/lua/<mod>.lua)
        if let Some(root) = theme_root
        {
          let rel_path = mod_name.replace('.', "/");
          let path = root.join("lua").join(format!("{}.lua", rel_path));
          if let Ok(canon) = std::fs::canonicalize(&path)
          {
            cfg_mut.ui.theme_path = Some(canon);
          }
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
  if let Ok(s) = ui_tbl.get::<String>("sort")
  {
    cfg_mut.ui.sort = Some(s);
  }
  if let Ok(b) = ui_tbl.get::<bool>("sort_reverse")
  {
    cfg_mut.ui.sort_reverse = Some(b);
  }
  if let Ok(s) = ui_tbl.get::<String>("show")
  {
    cfg_mut.ui.show = Some(s);
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
  Ok(())
}
