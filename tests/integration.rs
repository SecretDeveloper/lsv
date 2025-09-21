mod config_tests
{
  #[test]
  fn config_overlay_and_keymaps()
  {
    let code = r#"
lsv.config({
  config_version = 1,
  keys = { sequence_timeout_ms = 600 },
  ui = {
    show_hidden = true,
    panes = { parent = 10, current = 20, preview = 70 },
    date_format = "%Y",
    preview_lines = 80,
    max_list_items = 1234,
    row = { icon = "X ", left = "{name}", middle = "", right = "{info}" },
    row_widths = { icon = 2, left = 40, middle = 0, right = 12 },
    theme = { item_fg = "white", dir_fg = "blue" },
    display_mode = "friendly",
    sort = "size",
    sort_reverse = true,
    show = "size",
  },
  actions = {
    { keymap = "ss", fn = function(lsv, config) config.ui.sort = "size" end, description = "Sort by size" },
    { keymap = "q", action = "quit", description = "Quit" },
  },
})

lsv.map_action("gs", "Git Status", function(lsv, config) end)
lsv.set_previewer(function(ctx) return nil end)
"#;

    let (cfg, maps, engine_opt) =
      lsv::config::load_config_from_code(code, Some(std::path::Path::new(".")))
        .expect("load config");

    assert_eq!(cfg.config_version, 1);
    assert_eq!(cfg.keys.sequence_timeout_ms, 600);
    assert!(cfg.ui.show_hidden);
    assert_eq!(cfg.ui.preview_lines, 80);
    assert_eq!(cfg.ui.max_list_items, 1234);
    assert_eq!(
      cfg.ui.panes.as_ref().map(|p| (p.parent, p.current, p.preview)),
      Some((10, 20, 70))
    );
    assert_eq!(cfg.ui.date_format.as_deref(), Some("%Y"));
    assert_eq!(cfg.ui.row.as_ref().map(|r| r.icon.as_str()), Some("X "));
    assert_eq!(
      cfg.ui.row_widths.as_ref().map(|w| (w.icon, w.left, w.middle, w.right)),
      Some((2, 40, 0, 12))
    );
    assert_eq!(
      cfg.ui.theme.as_ref().and_then(|t| t.item_fg.as_deref()),
      Some("white")
    );
    assert_eq!(
      cfg.ui.theme.as_ref().and_then(|t| t.dir_fg.as_deref()),
      Some("blue")
    );
    assert_eq!(cfg.ui.display_mode.as_deref(), Some("friendly"));
    assert_eq!(cfg.ui.sort.as_deref(), Some("size"));
    assert_eq!(cfg.ui.sort_reverse, Some(true));
    assert_eq!(cfg.ui.show.as_deref(), Some("size"));

    let mut by_seq: std::collections::HashMap<String, String> =
      std::collections::HashMap::new();
    for m in &maps
    {
      by_seq.insert(m.sequence.clone(), m.action.clone());
    }
    assert_eq!(by_seq.get("q").map(String::as_str), Some("quit"));
    assert!(
      by_seq.get("ss").map(|s| s.starts_with("run_lua:")).unwrap_or(false)
    );
    assert!(
      by_seq.get("gs").map(|s| s.starts_with("run_lua:")).unwrap_or(false)
    );

    let action_count =
      engine_opt.as_ref().map(|(_, _, keys)| keys.len()).unwrap_or(0);
    assert!(action_count >= 2, "expected at least our two action functions");
  }
}

mod require_tests
{
  #[test]
  fn restricted_require_allows_relative_modules_under_root_lua()
  {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_path_buf();
    let lua_dir = root.join("lua");
    std::fs::create_dir_all(&lua_dir).expect("mkdir lua");
    std::fs::write(lua_dir.join("mymod.lua"), b"return '%Y'\n")
      .expect("write module");
    let code = r#"
local fmt = require('mymod')
lsv.config({ ui = { date_format = fmt } })
"#;
    let (cfg, _maps, _eng) =
      lsv::config::load_config_from_code(code, Some(&root))
        .expect("load config");
    assert_eq!(cfg.ui.date_format.as_deref(), Some("%Y"));
  }

  #[test]
  fn restricted_require_blocks_parent_traversal()
  {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_path_buf();
    let bad_code = r#" local x = require('../outside') "#;
    let err = lsv::config::load_config_from_code(bad_code, Some(&root))
      .err()
      .expect("should error");
    let msg = format!("{}", err);
    assert!(
      msg.contains("invalid module name")
        || msg.contains("inline init.lua execution failed")
    );
  }
}

mod effects_tests
{
  #[test]
  fn lua_action_returns_effects_and_overlay()
  {
    let code = r#"
lsv.map_action('tt', 'Test Effects', function(lsv, config)
  config.messages = 'show'
  lsv.display_output('Hello World', 'Output')
  config.redraw = true
  config.quit = true
  -- overlay change (partial; not a full validated overlay)
  config.ui = config.ui or {}
  config.ui.show_hidden = true
  return config
end)
"#;
    let (_cfg, _maps, engine_opt) =
      lsv::config::load_config_from_code(code, None).expect("load with action");
    let (engine, _prev, keys) = engine_opt.expect("engine present");
    let mut app = lsv::app::App::new().expect("app new");
    app.inject_lua_engine_for_tests(engine, keys);
    let (fx, overlay) = lsv::actions::lua_glue::call_lua_action(&mut app, 0)
      .expect("call action");
    assert!(fx.quit);
    assert!(overlay.is_some(), "merged overlay should be present");
  }

  #[test]
  fn parse_effects_from_table()
  {
    let lua = mlua::Lua::new();
    let tbl = lua.create_table().unwrap();
    let ctx = lua.create_table().unwrap();
    ctx.set("selected_index", 3u64).unwrap();
    tbl.set("context", ctx).unwrap();
    tbl.set("messages", "toggle").unwrap();
    tbl.set("output", "show").unwrap();
    tbl.set("output_text", "hi").unwrap();
    tbl.set("output_title", "T").unwrap();
    tbl.set("redraw", true).unwrap();
    tbl.set("quit", true).unwrap();
    let fx = lsv::actions::effects::parse_effects_from_lua(&tbl);
    assert_eq!(fx.selection, Some(3));
    assert!(fx.redraw);
    assert!(fx.quit);
    assert!(matches!(
      fx.messages,
      lsv::actions::effects::OverlayToggle::Toggle
    ));
    assert!(matches!(
      fx.output_overlay,
      lsv::actions::effects::OverlayToggle::Show
    ));
    let (title, text) = fx.output.expect("output");
    assert_eq!(title, "T");
    assert_eq!(text, "hi");
  }

  #[test]
  fn overlay_toggle_from_str_mapping()
  {
    use lsv::actions::effects::OverlayToggle;
    assert!(matches!(OverlayToggle::from("toggle"), OverlayToggle::Toggle));
    assert!(matches!(OverlayToggle::from("show"), OverlayToggle::Show));
    assert!(matches!(OverlayToggle::from("hide"), OverlayToggle::Hide));
    assert!(matches!(OverlayToggle::from(""), OverlayToggle::None));
    assert!(matches!(OverlayToggle::from("unknown"), OverlayToggle::None));
  }

  #[test]
  fn parse_effects_defaults_when_missing()
  {
    let lua = mlua::Lua::new();
    let tbl = lua.create_table().unwrap();
    let fx = lsv::actions::effects::parse_effects_from_lua(&tbl);
    assert_eq!(fx.selection, None);
    assert!(!fx.redraw);
    assert!(!fx.quit);
    assert!(fx.output.is_none());
    assert!(matches!(fx.messages, lsv::actions::effects::OverlayToggle::None));
    assert!(matches!(
      fx.output_overlay,
      lsv::actions::effects::OverlayToggle::None
    ));
  }

  #[test]
  fn parse_effects_title_defaults_to_output()
  {
    let lua = mlua::Lua::new();
    let tbl = lua.create_table().unwrap();
    tbl.set("output_text", "body").unwrap();
    let fx = lsv::actions::effects::parse_effects_from_lua(&tbl);
    let (title, text) = fx.output.expect("output");
    assert_eq!(title, "Output");
    assert_eq!(text, "body");
  }

  #[test]
  fn parse_effects_selection_from_context()
  {
    let lua = mlua::Lua::new();
    let tbl = lua.create_table().unwrap();
    let ctx = lua.create_table().unwrap();
    ctx.set("selected_index", 5u64).unwrap();
    tbl.set("context", ctx).unwrap();
    let fx = lsv::actions::effects::parse_effects_from_lua(&tbl);
    assert_eq!(fx.selection, Some(5));
  }
}

mod keymap_tests
{
  #[test]
  fn keymap_prefix_building_and_lookup()
  {
    let mut app = lsv::app::App::new().expect("app new");
    let maps = vec![
      lsv::config::KeyMapping {
        sequence:    "a".into(),
        action:      "internal:noop".into(),
        description: Some("A".into()),
      },
      lsv::config::KeyMapping {
        sequence:    "ab".into(),
        action:      "internal:noop2".into(),
        description: Some("AB".into()),
      },
    ];
    app.test_set_keymaps(maps);
    assert_eq!(app.test_resolve_action("a").as_deref(), Some("internal:noop"));
    assert_eq!(
      app.test_resolve_action("ab").as_deref(),
      Some("internal:noop2")
    );
    assert!(app.test_has_prefix("a"));
    assert!(!app.test_has_prefix("ab"));
  }
}

mod apply_tests
{
  #[test]
  fn apply_config_overlay_relist_on_show_hidden()
  {
    let mut app = lsv::app::App::new().expect("app new");
    let lua = mlua::Lua::new();
    let tbl =
      lsv::config_data::to_lua_config_table(&lua, &app).expect("to table");
    let ui: mlua::Table = tbl.get("ui").expect("ui table");
    let new_val = !app.test_config_show_hidden();
    ui.set("show_hidden", new_val).expect("set show_hidden");
    let data =
      lsv::config_data::from_lua_config_table(tbl).expect("from table");
    lsv::actions::apply::apply_config_overlay(&mut app, &data);
    assert_eq!(app.test_config_show_hidden(), new_val);
    assert!(app.test_force_full_redraw(), "relist should force full redraw");
  }

  #[test]
  fn apply_config_overlay_redraw_only_on_date_format()
  {
    let mut app = lsv::app::App::new().expect("app new");
    app.test_set_force_full_redraw(false);
    let lua = mlua::Lua::new();
    let tbl =
      lsv::config_data::to_lua_config_table(&lua, &app).expect("to table");
    let ui: mlua::Table = tbl.get("ui").expect("ui table");
    ui.set("date_format", "%Y").expect("set date_format");
    let data =
      lsv::config_data::from_lua_config_table(tbl).expect("from table");
    lsv::actions::apply::apply_config_overlay(&mut app, &data);
    assert_eq!(app.test_config_date_format().as_deref(), Some("%Y"));
    assert!(
      app.test_force_full_redraw(),
      "date format change should force redraw"
    );
  }

  #[test]
  fn apply_config_overlay_refresh_preview_only_on_preview_lines()
  {
    let mut app = lsv::app::App::new().expect("app new");
    app.test_set_force_full_redraw(false);
    let lua = mlua::Lua::new();
    let tbl =
      lsv::config_data::to_lua_config_table(&lua, &app).expect("to table");
    let ui: mlua::Table = tbl.get("ui").expect("ui table");
    let new_lines = app.test_config_preview_lines() + 1;
    ui.set("preview_lines", new_lines as u64).expect("set preview_lines");
    let data =
      lsv::config_data::from_lua_config_table(tbl).expect("from table");
    lsv::actions::apply::apply_config_overlay(&mut app, &data);
    assert_eq!(app.test_config_preview_lines(), new_lines);
    assert!(
      !app.test_force_full_redraw(),
      "preview-only changes should not force full redraw"
    );
  }

  #[test]
  fn apply_effects_selection_and_overlays()
  {
    let mut app = lsv::app::App::new().expect("app new");
    // start with overlays off
    // (no setters: validate via effects changes below)

    // messages: toggle should show messages and hide others
    let mut fx = lsv::actions::effects::ActionEffects::default();
    fx.messages = lsv::actions::effects::OverlayToggle::Toggle;
    lsv::actions::apply::apply_effects(&mut app, fx.clone());
    assert!(app.test_show_messages());
    assert!(!app.test_show_output() && !app.test_show_whichkey());

    // output overlay: show should hide others
    fx.messages = lsv::actions::effects::OverlayToggle::None;
    fx.output_overlay = lsv::actions::effects::OverlayToggle::Show;
    lsv::actions::apply::apply_effects(&mut app, fx.clone());
    assert!(app.test_show_output());
    assert!(!app.test_show_messages() && !app.test_show_whichkey());

    // output content should populate and turn on Output panel
    fx.output_overlay = lsv::actions::effects::OverlayToggle::None;
    fx.output = Some(("T".to_string(), "Body".to_string()));
    lsv::actions::apply::apply_effects(&mut app, fx.clone());
    assert!(app.test_show_output());
    assert_eq!(app.test_output_title(), "T");
    assert!(app.test_output_text().contains("Body"));

    // selection update within bounds
    if app.test_has_entries()
    {
      fx.output = None;
      fx.selection = Some(0);
      lsv::actions::apply::apply_effects(&mut app, fx.clone());
      assert_eq!(app.test_selected_index(), Some(0));
    }

    // quit + redraw flags
    fx.selection = None;
    fx.redraw = true;
    fx.quit = true;
    lsv::actions::apply::apply_effects(&mut app, fx);
    assert!(app.test_force_full_redraw() && app.test_should_quit());
  }
}

mod dispatcher_tests
{
  #[test]
  fn dispatch_internal_quit()
  {
    let mut app = lsv::app::App::new().expect("app new");
    let ran =
      lsv::actions::dispatch_action(&mut app, "quit").expect("dispatch");
    assert!(ran);
    assert!(app.test_should_quit());
  }

  #[test]
  fn dispatch_lua_action_by_index()
  {
    // Map a single action at index 0 that sets quit
    let code = r#"
lsv.map_action('x', 'Quit', function(lsv, config)
  config.quit = true
  return config
end)
"#;
    let (_cfg, _maps, engine_opt) =
      lsv::config::load_config_from_code(code, None).expect("load with action");
    let (engine, _prev, keys) = engine_opt.expect("engine present");
    let mut app = lsv::app::App::new().expect("app new");
    app.inject_lua_engine_for_tests(engine, keys);
    let ran = lsv::actions::dispatch_action(&mut app, "run_lua:0")
      .expect("dispatch lua");
    assert!(ran);
    assert!(app.test_should_quit());
  }

  #[test]
  fn dispatch_sequence_stops_after_quit()
  {
    // First action sets quit; second is an internal toggle we can detect
    let code = r#"
lsv.map_action('x', 'Quit', function(lsv, config)
  config.quit = true
  return config
end)
"#;
    let (_cfg, _maps, engine_opt) =
      lsv::config::load_config_from_code(code, None).expect("load with action");
    let (engine, _prev, keys) = engine_opt.expect("engine present");
    let mut app = lsv::app::App::new().expect("app new");
    app.inject_lua_engine_for_tests(engine, keys);
    app.test_set_sort_reverse(false);
    let ran =
      lsv::actions::dispatch_action(&mut app, "run_lua:0;sort:reverse:toggle")
        .expect("dispatch seq");
    assert!(ran);
    assert!(app.test_should_quit());
    // Should not have toggled since quit short-circuits
    assert_eq!(app.test_sort_reverse(), false);
  }

  #[test]
  fn dispatch_unknown_action_returns_false()
  {
    let mut app = lsv::app::App::new().expect("app new");
    let ran = lsv::actions::dispatch_action(&mut app, "no_such_action")
      .expect("dispatch");
    assert!(!ran);
    assert!(!app.test_should_quit());
  }
}

mod defaults_actions_tests
{
  #[test]
  fn zf_sets_friendly_display_mode()
  {
    // Create app with defaults loaded
    let mut app = lsv::app::App::new().expect("app new");
    // Resolve action bound to "zf"
    let action =
      app.test_resolve_action("zf").expect("default zf mapping present");
    let ran =
      lsv::actions::dispatch_action(&mut app, &action).expect("dispatch zf");
    assert!(ran);
    // Verify effect: display mode is Friendly
    assert!(matches!(app.test_display_mode(), lsv::app::DisplayMode::Friendly));
  }

  #[test]
  fn zc_sets_info_created_and_does_not_crash()
  {
    let mut app = lsv::app::App::new().expect("app new");
    let action =
      app.test_resolve_action("zc").expect("default zc mapping present");
    let ran =
      lsv::actions::dispatch_action(&mut app, &action).expect("dispatch zc");
    assert!(ran);
    assert!(matches!(app.test_info_mode(), lsv::app::InfoMode::Created));
    // No render test here; just verifying setting InfoMode::Created does not
    // panic
  }
}

mod internal_tests
{
  use std::fs;
  #[test]
  fn sort_and_reselect_by_name()
  {
    let temp = tempfile::tempdir().expect("tempdir");
    let dir = temp.path();
    // Create files with different sizes
    fs::write(dir.join("a.txt"), b"aaaa").unwrap();
    fs::write(dir.join("b.txt"), b"b").unwrap();
    let mut app = lsv::app::App::new().expect("app new");
    app.test_set_cwd(dir);
    // Select b.txt if present
    let idx_b =
      (0..100).find(|&i| app.test_entry_name(i).as_deref() == Some("b.txt"));
    if let Some(i) = idx_b
    {
      app.test_select_index(i);
    }
    // Sort by size via internal action
    let ran =
      lsv::actions::dispatch_action(&mut app, "sort:size").expect("dispatch");
    assert!(ran);
    // After sort, selection should still be b.txt
    if let Some(sel) = app.test_selected_index()
    {
      assert_eq!(app.test_entry_name(sel).as_deref(), Some("b.txt"));
    }
  }

  #[test]
  fn toggle_sort_reverse()
  {
    let mut app = lsv::app::App::new().expect("app new");
    let ran = lsv::actions::dispatch_action(&mut app, "sort:reverse:toggle")
      .expect("dispatch");
    assert!(ran);
    assert!(app.test_sort_reverse());
  }

  #[test]
  fn set_info_and_display_modes()
  {
    let mut app = lsv::app::App::new().expect("app new");
    assert!(lsv::actions::dispatch_action(&mut app, "show:size").unwrap());
    assert!(matches!(app.test_info_mode(), lsv::app::InfoMode::Size));
    assert!(
      lsv::actions::dispatch_action(&mut app, "display:friendly").unwrap()
    );
    assert!(matches!(app.test_display_mode(), lsv::app::DisplayMode::Friendly));
  }

  #[test]
  fn navigation_top_bottom()
  {
    let temp = tempfile::tempdir().expect("tempdir");
    let dir = temp.path();
    fs::write(dir.join("a.txt"), b"a").unwrap();
    fs::write(dir.join("b.txt"), b"b").unwrap();
    fs::write(dir.join("c.txt"), b"c").unwrap();
    let mut app = lsv::app::App::new().expect("app new");
    app.test_set_cwd(dir);
    let ran =
      lsv::actions::dispatch_action(&mut app, "nav:bottom").expect("dispatch");
    assert!(ran);
    if app.test_has_entries()
    {
      let last = (0..)
        .take(100)
        .position(|i| app.test_get_entry(i).is_none())
        .unwrap_or(0)
        .saturating_sub(1);
      assert_eq!(app.test_selected_index(), Some(last));
      assert!(lsv::actions::dispatch_action(&mut app, "nav:top").unwrap());
      assert_eq!(app.test_selected_index(), Some(0));
    }
  }
}

mod config_rs_tests
{
  #[test]
  fn mapkey_legacy_adds_mapping()
  {
    let code = r#"
lsv.mapkey('x', 'quit', 'Legacy Quit')
"#;
    let (_cfg, maps, engine_opt) =
      lsv::config::load_config_from_code(code, None).expect("load config");
    let quit = maps.iter().find(|m| m.sequence == "x").expect("map x");
    assert_eq!(quit.action.as_str(), "quit");
    assert_eq!(quit.description.as_deref(), Some("Legacy Quit"));
    // No actions registered for legacy mapkey
    // Defaults may have actions loaded; we only assert our legacy map was added
    assert!(engine_opt.is_some());
  }

  #[test]
  fn set_previewer_registers_function()
  {
    let code = r#"
lsv.set_previewer(function(ctx) return nil end)
"#;
    let (_cfg, _maps, engine_opt) =
      lsv::config::load_config_from_code(code, None).expect("load config");
    assert!(engine_opt.is_some(), "engine and previewer key expected");
  }

  #[test]
  fn actions_table_collects_both_fn_and_string()
  {
    let code = r#"
lsv.config({
  actions = {
    { keymap = 'k1', fn = function(lsv, config) return {} end, description = 'Lua Fn' },
    { keymap = 'k2', action = 'quit', description = 'String Quit' },
  }
})
"#;
    let (_cfg, maps, engine_opt) =
      lsv::config::load_config_from_code(code, None).expect("load config");
    // k1 should map to run_lua:<idx>
    let m1 = maps.iter().find(|m| m.sequence == "k1").expect("k1");
    assert!(m1.action.starts_with("run_lua:"));
    assert_eq!(m1.description.as_deref(), Some("Lua Fn"));
    // k2 should be direct string action
    let m2 = maps.iter().find(|m| m.sequence == "k2").expect("k2");
    assert_eq!(m2.action.as_str(), "quit");
    assert_eq!(m2.description.as_deref(), Some("String Quit"));
    // Engine should have at least one action function
    let count = engine_opt.as_ref().map(|(_, _, keys)| keys.len()).unwrap_or(0);
    assert!(count >= 1);
  }

  #[test]
  fn parse_icons_theme_rowwidths_and_keys()
  {
    let code = r#"
lsv.config({
  icons = { enabled = true, preset = 'devicons', font = 'Nerd' },
  keys = { sequence_timeout_ms = 700 },
  ui = {
    row_widths = { icon = 2, left = 40, middle = 0, right = 12 },
    theme = { dir_fg = 'blue', item_fg = 'white' },
  },
})
"#;
    let (cfg, _maps, _eng) =
      lsv::config::load_config_from_code(code, None).expect("load config");
    assert!(cfg.icons.enabled);
    assert_eq!(cfg.icons.preset.as_deref(), Some("devicons"));
    assert_eq!(cfg.icons.font.as_deref(), Some("Nerd"));
    assert_eq!(cfg.keys.sequence_timeout_ms, 700);
    assert_eq!(
      cfg.ui.row_widths.as_ref().map(|w| (w.icon, w.left, w.middle, w.right)),
      Some((2, 40, 0, 12))
    );
    assert_eq!(
      cfg.ui.theme.as_ref().and_then(|t| t.dir_fg.as_deref()),
      Some("blue")
    );
    assert_eq!(
      cfg.ui.theme.as_ref().and_then(|t| t.item_fg.as_deref()),
      Some("white")
    );
  }

  #[test]
  fn defaults_are_loaded_when_no_user_code()
  {
    let (cfg, _maps, _eng) =
      lsv::config::load_config_from_code("", None).expect("load defaults");
    // Spot-check a few defaults set in defaults.lua
    assert_eq!(cfg.keys.sequence_timeout_ms, 0);
    assert_eq!(cfg.ui.show_hidden, false);
    assert_eq!(cfg.ui.preview_lines, 100);
    assert_eq!(cfg.ui.max_list_items, 5000);
    assert_eq!(cfg.ui.row.as_ref().map(|r| r.left.as_str()), Some("{name}"));
  }

  #[test]
  fn config_calls_merge_across_invocations()
  {
    let code = r#"
lsv.config({ ui = { display_mode = 'friendly' } })
lsv.config({ ui = { preview_lines = 77 } })
"#;
    let (cfg, _maps, _eng) =
      lsv::config::load_config_from_code(code, None).expect("load config");
    assert_eq!(cfg.ui.preview_lines, 77);
    assert_eq!(cfg.ui.display_mode.as_deref(), Some("friendly"));
  }

  #[test]
  fn require_nested_module_subdir_allowed()
  {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path().to_path_buf();
    // Create root/lua/sub/mod.lua returning number 33
    let lua_dir = root.join("lua/sub");
    std::fs::create_dir_all(&lua_dir).expect("mkdir");
    std::fs::write(lua_dir.join("mod.lua"), b"return 33\n")
      .expect("write module");
    let code = r#"
local v = require('sub.mod')
lsv.config({ ui = { preview_lines = v } })
"#;
    let (cfg, _maps, _eng) =
      lsv::config::load_config_from_code(code, Some(&root))
        .expect("load config");
    assert_eq!(cfg.ui.preview_lines, 33);
  }

  #[test]
  fn require_absolute_path_is_blocked()
  {
    let root = tempfile::tempdir().expect("tempdir").path().to_path_buf();
    let code = r#" local v = require('/etc/passwd') "#;
    let err = lsv::config::load_config_from_code(code, Some(&root))
      .err()
      .expect("should error");
    let msg = format!("{}", err);
    assert!(
      msg.contains("invalid module name")
        || msg.contains("inline init.lua execution failed")
    );
  }

  #[test]
  fn map_action_indices_are_increasing_in_code_order()
  {
    let code = r#"
lsv.map_action('a1', 'A1', function(lsv, config) end)
lsv.map_action('a2', 'A2', function(lsv, config) end)
lsv.map_action('a3', 'A3', function(lsv, config) end)
"#;
    let (_cfg, maps, _eng) =
      lsv::config::load_config_from_code(code, None).expect("load config");
    // Extract indices for our sequences
    let idx = |seq: &str| -> usize {
      maps
        .iter()
        .find(|m| m.sequence == seq)
        .and_then(|m| m.action.strip_prefix("run_lua:")?.parse().ok())
        .unwrap()
    };
    let i1 = idx("a1");
    let i2 = idx("a2");
    let i3 = idx("a3");
    assert!(i1 < i2 && i2 < i3);
  }

  #[test]
  fn mapkey_and_map_action_coexist()
  {
    let code = r#"
lsv.mapkey('x', 'quit', 'Legacy Quit')
lsv.map_action('y', 'Lua Quit', function(lsv, config) lsv.quit() end)
"#;
    let (_cfg, maps, _eng) =
      lsv::config::load_config_from_code(code, None).expect("load config");
    assert!(maps.iter().any(|m| m.sequence == "x" && m.action == "quit"));
    assert!(
      maps
        .iter()
        .any(|m| m.sequence == "y" && m.action.starts_with("run_lua:"))
    );
  }

  #[test]
  fn invalid_types_are_ignored_not_applied()
  {
    let code = r#"
lsv.config({
  keys = { sequence_timeout_ms = 'abc' },  -- wrong type
  ui = {
    preview_lines = 'foo',      -- wrong type
    row_widths = { icon = 'x', left = 'y', middle = 'z', right = 'w' },
    theme = { item_fg = 123 },  -- wrong type
  },
})
"#;
    let (cfg, _maps, _eng) =
      lsv::config::load_config_from_code(code, None).expect("load config");
    // Defaults remain because bad types are ignored
    assert_eq!(cfg.keys.sequence_timeout_ms, 0);
    assert_eq!(cfg.ui.preview_lines, 100);
    assert_eq!(
      cfg.ui.row_widths.as_ref().map(|w| (w.icon, w.left, w.middle, w.right)),
      Some((0, 0, 0, 0))
    );
    assert_eq!(
      cfg.ui.theme.as_ref().and_then(|t| t.item_fg.as_deref()),
      Some("123")
    );
  }

  #[test]
  fn set_previewer_wrong_type_errors()
  {
    let code = r#" lsv.set_previewer(123) "#;
    let err = lsv::config::load_config_from_code(code, None)
      .err()
      .expect("should error");
    let msg = format!("{}", err);
    assert!(
      msg.contains("lsv api install failed")
        || msg.contains("execution failed")
        || msg.contains("error")
    );
  }

  #[test]
  fn actions_table_with_wrong_types_is_ignored()
  {
    let code = r#"
lsv.config({
  actions = {
    { keymap = 'bad', fn = 123 },          -- wrong type for fn
    { keymap = 4, action = 5 },            -- wrong types
  }
})
"#;
    let (_cfg, maps, _eng) =
      lsv::config::load_config_from_code(code, None).expect("load config");
    // No mapping for 'bad' should exist
    assert!(maps.iter().find(|m| m.sequence == "bad").is_none());
  }
}

mod config_data_tests
{
  #[test]
  fn roundtrip_to_from_lua_table()
  {
    let mut app = lsv::app::App::new().expect("app new");
    // Adjust some runtime-facing state before snapshot for diversity
    app.test_set_force_full_redraw(false);
    let lua = mlua::Lua::new();
    let tbl =
      lsv::config_data::to_lua_config_table(&lua, &app).expect("to table");

    // Mutate table to desired values
    let keys: mlua::Table = tbl.get("keys").unwrap();
    keys.set("sequence_timeout_ms", 123u64).unwrap();

    let ui: mlua::Table = tbl.get("ui").unwrap();
    let panes: mlua::Table = ui.get("panes").unwrap();
    panes.set("parent", 10u16).unwrap();
    panes.set("current", 20u16).unwrap();
    panes.set("preview", 70u16).unwrap();
    ui.set("show_hidden", true).unwrap();
    ui.set("date_format", "%Y").unwrap();
    ui.set("display_mode", "friendly").unwrap();
    ui.set("preview_lines", 80u64).unwrap();
    ui.set("max_list_items", 2345u64).unwrap();
    let row: mlua::Table = ui.get("row").unwrap();
    row.set("icon", "X ").unwrap();
    row.set("left", "{name}").unwrap();
    row.set("middle", "").unwrap();
    row.set("right", "{info}").unwrap();
    // row_widths present
    let rw = lua.create_table().unwrap();
    rw.set("icon", 2u64).unwrap();
    rw.set("left", 40u64).unwrap();
    rw.set("middle", 0u64).unwrap();
    rw.set("right", 12u64).unwrap();
    ui.set("row_widths", rw).unwrap();
    // theme present (partial)
    let theme = lua.create_table().unwrap();
    theme.set("dir_fg", "cyan").unwrap();
    theme.set("item_fg", "white").unwrap();
    ui.set("theme", theme).unwrap();
    // sort/show
    ui.set("sort", "size").unwrap();
    ui.set("sort_reverse", true).unwrap();
    ui.set("show", "modified").unwrap();

    let cfgd =
      lsv::config_data::from_lua_config_table(tbl).expect("from table");
    assert_eq!(cfgd.keys_sequence_timeout_ms, 123);
    assert_eq!(cfgd.ui.panes.parent, 10);
    assert_eq!(cfgd.ui.panes.current, 20);
    assert_eq!(cfgd.ui.panes.preview, 70);
    assert!(cfgd.ui.show_hidden);
    assert_eq!(cfgd.ui.date_format.as_deref(), Some("%Y"));
    assert_eq!(
      matches!(cfgd.ui.display_mode, lsv::app::DisplayMode::Friendly),
      true
    );
    assert_eq!(cfgd.ui.preview_lines, 80);
    assert_eq!(cfgd.ui.max_list_items, 2345);
    assert_eq!(cfgd.ui.row.icon.as_str(), "X ");
    assert_eq!(cfgd.ui.row.left.as_str(), "{name}");
    assert_eq!(cfgd.ui.row.right.as_str(), "{info}");
    assert_eq!(
      cfgd.ui.row_widths.as_ref().map(|w| (w.icon, w.left, w.middle, w.right)),
      Some((2, 40, 0, 12))
    );
    assert_eq!(
      cfgd.ui.theme.as_ref().and_then(|t| t.dir_fg.as_deref()),
      Some("cyan")
    );
    assert_eq!(
      cfgd.ui.theme.as_ref().and_then(|t| t.item_fg.as_deref()),
      Some("white")
    );
    assert!(matches!(cfgd.sort_key, lsv::actions::internal::SortKey::Size));
    assert!(cfgd.sort_reverse);
    assert!(matches!(cfgd.show_field, lsv::app::InfoMode::Modified));
  }

  #[test]
  fn invalid_display_mode_errors()
  {
    let app = lsv::app::App::new().expect("app new");
    let lua = mlua::Lua::new();
    let tbl =
      lsv::config_data::to_lua_config_table(&lua, &app).expect("to table");
    let ui: mlua::Table = tbl.get("ui").unwrap();
    ui.set("display_mode", "bogus").unwrap();
    let err = lsv::config_data::from_lua_config_table(tbl).unwrap_err();
    assert!(err.contains("ui.display_mode"));
  }

  #[test]
  fn invalid_sort_key_errors()
  {
    let app = lsv::app::App::new().expect("app new");
    let lua = mlua::Lua::new();
    let tbl =
      lsv::config_data::to_lua_config_table(&lua, &app).expect("to table");
    let ui: mlua::Table = tbl.get("ui").unwrap();
    ui.set("sort", "bogus").unwrap();
    let err = lsv::config_data::from_lua_config_table(tbl).unwrap_err();
    assert!(err.contains("sort.key must be one of"));
  }

  #[test]
  fn missing_keys_table_errors()
  {
    let app = lsv::app::App::new().expect("app new");
    let lua = mlua::Lua::new();
    let tbl =
      lsv::config_data::to_lua_config_table(&lua, &app).expect("to table");
    tbl.set("keys", mlua::Value::Nil).unwrap();
    let err = lsv::config_data::from_lua_config_table(tbl).unwrap_err();
    assert!(err.contains("missing or invalid table: keys"));
  }
}

mod input_tests
{
  use crossterm::event::{
    KeyCode,
    KeyEvent,
    KeyModifiers,
  };
  use std::{
    fs,
    thread::sleep,
    time::Duration,
  };

  fn key(ch: char) -> KeyEvent
  {
    KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE)
  }

  #[test]
  fn which_key_toggle_with_question_mark()
  {
    let mut app = lsv::app::App::new().expect("app new");
    assert!(!app.test_show_whichkey());
    let _ = lsv::input::handle_key(&mut app, key('?')).unwrap();
    assert!(app.test_show_whichkey());
    // Toggle off
    let _ = lsv::input::handle_key(&mut app, key('?')).unwrap();
    assert!(!app.test_show_whichkey());
  }

  #[test]
  fn sequence_prefix_and_exact_match()
  {
    let mut app = lsv::app::App::new().expect("app new");
    // map "ss" -> sort:size
    app.test_set_keymaps(vec![lsv::config::KeyMapping {
      sequence:    "ss".into(),
      action:      "sort:size".into(),
      description: Some("sort size".into()),
    }]);
    // First 's' should open which-key with prefix
    let _ = lsv::input::handle_key(&mut app, key('s')).unwrap();
    assert!(app.test_show_whichkey());
    assert_eq!(app.test_whichkey_prefix().as_str(), "s");
    // Second 's' should dispatch and close overlay
    let _ = lsv::input::handle_key(&mut app, key('s')).unwrap();
    assert!(!app.test_show_whichkey());
    // sort applied
    assert!(matches!(
      app.test_sort_key(),
      lsv::actions::internal::SortKey::Size
    ));
  }

  #[test]
  fn sequence_timeout_clears_pending()
  {
    let mut app = lsv::app::App::new().expect("app new");
    app.test_set_keymaps(vec![lsv::config::KeyMapping {
      sequence:    "xy".into(),
      action:      "quit".into(),
      description: None,
    }]);
    // short timeout
    let code = r#"lsv.config({ keys = { sequence_timeout_ms = 10 } })"#;
    let (cfg, _maps, _eng) =
      lsv::config::load_config_from_code(code, None).unwrap();
    app.test_set_config(cfg);
    let _ = lsv::input::handle_key(&mut app, key('x')).unwrap();
    // sleep beyond timeout
    sleep(Duration::from_millis(20));
    // now 'y' should not complete sequence and should not quit
    let _ = lsv::input::handle_key(&mut app, key('y')).unwrap();
    assert!(!app.test_should_quit());
  }

  #[test]
  fn esc_clears_overlays_and_pending_seq()
  {
    let mut app = lsv::app::App::new().expect("app new");
    // Turn on overlays via effects
    let mut fx = lsv::actions::effects::ActionEffects::default();
    fx.messages = lsv::actions::effects::OverlayToggle::Show;
    lsv::actions::apply::apply_effects(&mut app, fx);
    // Now send ESC
    let _ = lsv::input::handle_key(
      &mut app,
      KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE),
    )
    .unwrap();
    assert!(!app.test_show_messages());
    assert!(!app.test_show_output());
    assert_eq!(app.test_whichkey_prefix().as_str(), "");
  }

  #[test]
  fn navigation_and_parent_current_dir_changes()
  {
    let temp = tempfile::tempdir().expect("tempdir");
    let dir = temp.path();
    fs::create_dir(dir.join("sub")).unwrap();
    fs::write(dir.join("a.txt"), b"a").unwrap();
    let mut app = lsv::app::App::new().expect("app new");
    app.test_set_cwd(dir);
    // Ensure selection exists
    if app.test_has_entries()
    {
      // Down should move selection if possible
      let _ = lsv::input::handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Down, KeyModifiers::NONE),
      )
      .unwrap();
      // Entering a directory if selected
      // Find index of 'sub'
      if let Some(idx) =
        (0..100).find(|&i| app.test_entry_name(i).as_deref() == Some("sub"))
      {
        app.test_select_index(idx);
        let prev = app.test_cwd_path();
        let _ = lsv::input::handle_key(
          &mut app,
          KeyEvent::new(KeyCode::Right, KeyModifiers::NONE),
        )
        .unwrap();
        assert_ne!(app.test_cwd_path(), prev);
        // Go back up
        let _ = lsv::input::handle_key(
          &mut app,
          KeyEvent::new(KeyCode::Left, KeyModifiers::NONE),
        )
        .unwrap();
        assert_eq!(app.test_cwd_path(), dir);
      }
    }
  }

  #[test]
  fn case_normalization_single_key_fallback()
  {
    let mut app = lsv::app::App::new().expect("app new");
    app.test_set_keymaps(vec![lsv::config::KeyMapping {
      sequence:    "q".into(),
      action:      "quit".into(),
      description: None,
    }]);
    // Press uppercase Q: handler tries 'Q','q','Q' variants
    let quit = lsv::input::handle_key(&mut app, key('Q')).unwrap();
    assert!(quit);
  }
}

mod main_rs_tests
{
  use std::{
    fs,
    panic,
  };

  #[test]
  fn panic_hook_logs_message()
  {
    // Setup trace log path
    let temp = tempfile::NamedTempFile::new().expect("trace file");
    let path = temp.path().to_path_buf();
    unsafe {
      std::env::set_var("LSV_TRACE", "1");
      std::env::set_var("LSV_TRACE_FILE", &path);
    }

    lsv::trace::install_panic_hook();
    let _ = panic::catch_unwind(|| {
      panic!("boom from test");
    });

    let data = fs::read_to_string(&path).expect("read trace");
    assert!(data.contains("[panic]"));
    assert!(data.contains("boom from test"));
  }
}

mod app_rs_tests
{
  use std::fs;

  #[test]
  fn initial_selection_after_set_cwd()
  {
    let temp = tempfile::tempdir().expect("tempdir");
    let dir = temp.path();
    fs::write(dir.join("a"), b"a").unwrap();
    fs::write(dir.join("b"), b"b").unwrap();
    let mut app = lsv::app::App::new().expect("app new");
    app.test_set_cwd(dir);
    // If entries exist, initial selection is 0
    if app.test_has_entries()
    {
      assert_eq!(app.test_selected_index(), Some(0));
      assert!(app.test_get_entry(0).is_some());
    }
  }

  #[test]
  fn display_output_overlays_and_content()
  {
    let mut app = lsv::app::App::new().expect("app new");
    app.test_display_output("Title", "Hello\r\nWorld");
    assert!(app.test_show_output());
    assert!(!app.test_show_whichkey());
    assert!(!app.test_show_messages());
    assert_eq!(app.test_output_title(), "Title");
    let text = app.test_output_text();
    assert!(text.contains("Hello"));
    assert!(text.contains("World"));
  }

  #[test]
  fn add_message_push_and_cap()
  {
    let mut app = lsv::app::App::new().expect("app new");
    for i in 0..105
    {
      app.test_add_message(&format!("msg-{i}"));
    }
    // recent_messages should be capped at 100
    assert!(app.test_recent_messages_len() <= 100);
    assert!(app.test_force_full_redraw());
  }

  #[test]
  fn refresh_preview_trims_to_preview_lines()
  {
    let temp = tempfile::tempdir().expect("tempdir");
    let dir = temp.path();
    let file = dir.join("long.txt");
    let content = (0..10).map(|i| format!("line-{i}\n")).collect::<String>();
    fs::write(&file, content).unwrap();
    let mut app = lsv::app::App::new().expect("app new");
    // Set preview_lines to 3 via config overlay injection
    let code = r#"lsv.config({ ui = { preview_lines = 3 } })"#;
    let (cfg, _maps, _eng) =
      lsv::config::load_config_from_code(code, None).unwrap();
    app.test_set_config(cfg);
    app.test_set_cwd(dir);
    // Select the long file
    if let Some(pos) =
      (0..100).find(|&i| app.test_entry_name(i).as_deref() == Some("long.txt"))
    {
      app.test_select_index(pos);
      // Refresh preview (selection change already triggers it), check length
      assert_eq!(app.test_preview_line_count(), 3);
    }
  }
}

mod runtime_rs_tests
{
  use crossterm::event::{
    Event,
    KeyCode,
    KeyEvent,
    KeyModifiers,
  };

  #[test]
  fn process_event_quit_returns_true()
  {
    let mut app = lsv::app::App::new().expect("app new");
    let quit = lsv::runtime_util::process_event(
      &mut app,
      Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)),
    )
    .unwrap();
    assert!(quit);
  }

  #[test]
  fn process_event_resize_returns_false()
  {
    let mut app = lsv::app::App::new().expect("app new");
    let cont =
      lsv::runtime_util::process_event(&mut app, Event::Resize(80, 24))
        .unwrap();
    assert!(!cont);
  }
}

mod util_rs_tests
{
  use std::{
    fs,
    path::Path,
  };

  #[test]
  fn sanitize_line_expands_tabs_and_strips_cr_and_controls()
  {
    let input = "a\tb\rc\x07d"; // tab, CR, bell
    let out = lsv::util::sanitize_line(input);
    // tab -> 4 spaces, CR removed, control -> space
    assert_eq!(out, "a    bc d");
  }

  #[test]
  fn shell_escape_quotes_and_handles_empty()
  {
    assert_eq!(lsv::util::shell_escape(""), "''");
    assert_eq!(lsv::util::shell_escape("abc"), "'abc'");
    assert_eq!(lsv::util::shell_escape("a'b"), "'a'\\''b'");
  }

  #[test]
  fn read_file_head_reads_at_most_n_lines()
  {
    let temp = tempfile::tempdir().expect("tempdir");
    let file = temp.path().join("sample.txt");
    fs::write(&file, "l1\nl2\nl3\n").unwrap();
    let v = lsv::util::read_file_head(Path::new(&file), 2).expect("read");
    assert_eq!(v, vec!["l1", "l2"]);
    let v2 = lsv::util::read_file_head(Path::new(&file), 10).expect("read");
    assert_eq!(v2, vec!["l1", "l2", "l3"]);
  }
}
mod partial_return_tests
{
  #[test]
  fn partial_overlay_return_is_merged_safely()
  {
    // Action returns only a tiny table; validator should still see a full
    // config
    let code = r#"
lsv.map_action('x', 'Partial', function(lsv, config)
  return { ui = { display_mode = 'friendly' } }
end)
"#;
    let (_cfg, _maps, engine_opt) =
      lsv::config::load_config_from_code(code, None).expect("load with action");
    let (engine, _prev, keys) = engine_opt.expect("engine present");
    let mut app = lsv::app::App::new().expect("app new");
    app.inject_lua_engine_for_tests(engine, keys);
    // Call the action by index 0 via dispatcher (covers merge path too)
    let ran = lsv::actions::dispatch_action(&mut app, "run_lua:0")
      .expect("dispatch lua");
    assert!(ran);
    // Display mode should be Friendly after overlay is applied
    assert!(matches!(app.test_display_mode(), lsv::app::DisplayMode::Friendly));
  }
}

mod lua_glue_tests
{
  use std::fs;

  fn make_app_with_actions(
    lua_src: &str,
    seq: &str,
  ) -> (lsv::app::App, usize)
  {
    let (_cfg, maps, engine_opt) =
      lsv::config::load_config_from_code(lua_src, None).expect("load lua");
    let (engine, _prev, keys) = engine_opt.expect("engine");
    let mut app = lsv::app::App::new().expect("app new");
    app.inject_lua_engine_for_tests(engine, keys);
    // Find index by looking up the mapping action string
    let idx = maps
      .iter()
      .find(|m| m.sequence == seq)
      .and_then(|m| m.action.strip_prefix("run_lua:")?.parse::<usize>().ok())
      .expect("mapped index");
    (app, idx)
  }

  #[test]
  fn lsv_select_item_sets_selection_effect()
  {
    // Two files to exercise selection reliably
    let temp = tempfile::tempdir().expect("tempdir");
    let dir = temp.path();
    fs::write(dir.join("x.txt"), b"x").unwrap();
    fs::write(dir.join("y.txt"), b"y").unwrap();
    let code = r#"
lsv.map_action('sel', 'Select first', function(lsv, config)
  lsv.select_item(0)
end)
"#;
    let (mut app, idx) = make_app_with_actions(code, "sel");
    app.test_set_cwd(dir);
    let (fx, _ov) =
      lsv::actions::lua_glue::call_lua_action(&mut app, idx).expect("call");
    lsv::actions::apply::apply_effects(&mut app, fx);
    assert_eq!(app.test_selected_index(), Some(0));
  }

  #[test]
  fn lsv_quit_sets_quit_effect()
  {
    let code = r#"
lsv.map_action('q', 'Quit', function(lsv, config)
  lsv.quit()
end)
"#;
    let (mut app, idx) = make_app_with_actions(code, "q");
    let (fx, _ov) =
      lsv::actions::lua_glue::call_lua_action(&mut app, idx).expect("call");
    lsv::actions::apply::apply_effects(&mut app, fx);
    assert!(app.test_should_quit());
  }

  #[test]
  fn lsv_display_output_sets_output_overlay()
  {
    let code = r#"
lsv.map_action('o', 'Output', function(lsv, config)
  lsv.display_output('Body', 'Title')
end)
"#;
    let (mut app, idx) = make_app_with_actions(code, "o");
    let (fx, _ov) =
      lsv::actions::lua_glue::call_lua_action(&mut app, idx).expect("call");
    let (title, text) = fx.output.clone().expect("fx.output present");
    assert_eq!(title, "Title");
    assert!(text.contains("Body"));
    lsv::actions::apply::apply_effects(&mut app, fx);
    assert!(app.test_show_output());
    assert_eq!(app.test_output_title(), "Title");
    assert!(app.test_output_text().contains("Body"));
  }

  #[test]
  fn lsv_os_run_captures_env_name()
  {
    // Prepare a known selection name
    let temp = tempfile::tempdir().expect("tempdir");
    let dir = temp.path();
    fs::write(dir.join("hello.txt"), b"hi").unwrap();
    let code = r#"
lsv.map_action('r', 'Run', function(lsv, config)
  lsv.os_run('printf "$LSV_NAME"')
end)
"#;
    let (mut app, idx) = make_app_with_actions(code, "r");
    app.test_set_cwd(dir);
    // Ensure selection is the file we just wrote
    let pos = (0..100)
      .find(|&i| app.test_entry_name(i).as_deref() == Some("hello.txt"))
      .expect("find hello");
    app.test_select_index(pos);
    let (fx, _ov) =
      lsv::actions::lua_glue::call_lua_action(&mut app, idx).expect("call");
    // We expect an output effect with the name
    let (_title, text) = fx.output.clone().expect("fx.output present");
    assert!(text.contains("hello.txt"));
    // Apply and verify overlay behavior
    lsv::actions::apply::apply_effects(&mut app, fx);
    assert!(app.test_output_text().contains("hello.txt"));
  }

  #[test]
  fn lsv_select_last_item_goes_to_end()
  {
    let temp = tempfile::tempdir().expect("tempdir");
    let dir = temp.path();
    fs::write(dir.join("a"), b"a").unwrap();
    fs::write(dir.join("b"), b"b").unwrap();
    let code = r#"
lsv.map_action('last', 'Last', function(lsv, config)
  lsv.select_last_item()
end)
"#;
    let (mut app, idx) = make_app_with_actions(code, "last");
    app.test_set_cwd(dir);
    let (fx, _ov) =
      lsv::actions::lua_glue::call_lua_action(&mut app, idx).expect("call");
    lsv::actions::apply::apply_effects(&mut app, fx);
    if app.test_has_entries()
    {
      // last index should be selected
      let last = (0..100)
        .position(|i| app.test_get_entry(i).is_none())
        .unwrap_or(0)
        .saturating_sub(1);
      assert_eq!(app.test_selected_index(), Some(last));
    }
  }

  #[test]
  fn mutate_config_and_return_nil_still_applies_overlay()
  {
    let code = r#"
lsv.map_action('friendly', 'Friendly', function(lsv, config)
  config.ui = config.ui or {}
  config.ui.display_mode = 'friendly'
  return nil  -- rely on mutation
end)
"#;
    let (mut app, idx) = make_app_with_actions(code, "friendly");
    let (fx, ov) =
      lsv::actions::lua_glue::call_lua_action(&mut app, idx).expect("call");
    // Effects may be empty; overlay should exist because we mutated config
    if let Some(data) = ov
    {
      lsv::actions::apply::apply_config_overlay(&mut app, &data);
    }
    assert!(matches!(app.test_display_mode(), lsv::app::DisplayMode::Friendly));
    // Also apply any effects (no-ops)
    lsv::actions::apply::apply_effects(&mut app, fx);
  }

  #[test]
  fn return_effects_only_toggle_messages_and_output()
  {
    let code = r#"
lsv.map_action('ov', 'Overlays', function(lsv, config)
  return { messages = 'toggle', output = 'show' }
end)
"#;
    let (mut app, idx) = make_app_with_actions(code, "ov");
    let (fx, _ov) =
      lsv::actions::lua_glue::call_lua_action(&mut app, idx).expect("call");
    // Verify parsed effects
    use lsv::actions::effects::OverlayToggle;
    assert!(matches!(fx.messages, OverlayToggle::Toggle));
    assert!(matches!(fx.output_overlay, OverlayToggle::Show));
    lsv::actions::apply::apply_effects(&mut app, fx);
    assert!(app.test_show_output());
    assert!(!app.test_show_messages());
  }

  #[test]
  fn recursive_merge_theme_overlay()
  {
    let code = r#"
lsv.map_action('theme', 'Theme', function(lsv, config)
  return { ui = { theme = { dir_fg = 'magenta' } } }
end)
"#;
    let (mut app, idx) = make_app_with_actions(code, "theme");
    let (_fx, ov) =
      lsv::actions::lua_glue::call_lua_action(&mut app, idx).expect("call");
    let data = ov.expect("overlay present");
    lsv::actions::apply::apply_config_overlay(&mut app, &data);
    assert_eq!(app.test_theme_dir_fg().as_deref(), Some("magenta"));
  }

  #[test]
  fn display_output_default_title_when_missing()
  {
    let code = r#"
lsv.map_action('od', 'Output default', function(lsv, config)
  lsv.display_output('Body')
end)
"#;
    let (mut app, idx) = make_app_with_actions(code, "od");
    let (fx, _ov) =
      lsv::actions::lua_glue::call_lua_action(&mut app, idx).expect("call");
    let (title, text) = fx.output.clone().expect("fx.output present");
    assert_eq!(title, "Output");
    assert!(text.contains("Body"));
    lsv::actions::apply::apply_effects(&mut app, fx);
    assert_eq!(app.test_output_title(), "Output");
  }

  #[test]
  fn e_mapping_passes_path_to_command()
  {
    use std::fs;
    let temp = tempfile::tempdir().expect("tempdir");
    let dir = temp.path();
    // Use a path with space to verify quoting
    let fname = "hello world.txt";
    let fpath = dir.join(fname);
    fs::write(&fpath, b"content").unwrap();
    let code = r#"
local function shquote(s)
  return "'" .. tostring(s):gsub("'", "'\\''") .. "'"
end
lsv.map_action('e', 'Edit', function(lsv, config)
  local path = (config.context and config.context.path) or "."
  -- Simulate editor by printing the argument we pass
  lsv.os_run("printf 'EDIT:%s' " .. shquote(path))
end)
"#;
    let (mut app, idx) = make_app_with_actions(code, "e");
    app.test_set_cwd(dir);
    // Select our file
    let pos = (0..100)
      .find(|&i| app.test_entry_name(i).as_deref() == Some(fname))
      .expect("find file");
    app.test_select_index(pos);
    let (fx, _ov) =
      lsv::actions::lua_glue::call_lua_action(&mut app, idx).expect("call");
    // Verify captured output contains the expanded absolute path
    let (_title, text) = fx.output.clone().expect("fx.output present");
    let abs = fpath.to_string_lossy();
    assert!(text.contains("EDIT:"));
    assert!(
      text.contains(&*abs),
      "expected output to contain path: {} in {}",
      abs,
      text
    );
    lsv::actions::apply::apply_effects(&mut app, fx);
    assert!(app.test_show_output());
    assert!(app.test_output_text().contains(&*abs));
  }
}
