use super::{
  Config,
  KeyMapping,
  UiModalConfig,
  UiModals,
  UiPanes,
  UiRowFormat,
  UiRowWidths,
  UiTheme,
};

/// Built-in default keymaps defined in Rust.
/// These mirror the previous Lua defaults and are applied before user config.
pub fn rust_default_keymaps() -> Vec<KeyMapping>
{
  vec![
    KeyMapping {
      sequence:    "q".into(),
      action:      "quit".into(),
      description: Some("Quit lsv".into()),
    },
    // Sorting
    KeyMapping {
      sequence:    "sn".into(),
      action:      "sort:name".into(),
      description: Some("Sort by name".into()),
    },
    KeyMapping {
      sequence:    "ss".into(),
      action:      "sort:size".into(),
      description: Some("Sort by size".into()),
    },
    KeyMapping {
      sequence:    "sr".into(),
      action:      "sort:reverse:toggle".into(),
      description: Some("Toggle reverse sort".into()),
    },
    KeyMapping {
      sequence:    "sm".into(),
      action:      "sort:mtime".into(),
      description: Some("Sort by modified time".into()),
    },
    KeyMapping {
      sequence:    "sc".into(),
      action:      "sort:created".into(),
      description: Some("Sort by created time".into()),
    },
    // Navigation
    KeyMapping {
      sequence:    "gg".into(),
      action:      "nav:top".into(),
      description: Some("Go to top".into()),
    },
    KeyMapping {
      sequence:    "G".into(),
      action:      "nav:bottom".into(),
      description: Some("Go to bottom".into()),
    },
    // Info/Display
    KeyMapping {
      sequence:    "zn".into(),
      action:      "show:none".into(),
      description: Some("Info: none".into()),
    },
    KeyMapping {
      sequence:    "zs".into(),
      action:      "show:size".into(),
      description: Some("Info: size".into()),
    },
    KeyMapping {
      sequence:    "zc".into(),
      action:      "show:created".into(),
      description: Some("Info: created date".into()),
    },
    KeyMapping {
      sequence:    "zf".into(),
      action:      "display:friendly".into(),
      description: Some("Display: friendly".into()),
    },
    KeyMapping {
      sequence:    "za".into(),
      action:      "display:absolute".into(),
      description: Some("Display: absolute".into()),
    },
    // Show hidden toggle and overlays
    KeyMapping {
      sequence:    "zh".into(),
      action:      "cmd:show_hidden_toggle".into(),
      description: Some("Toggle Show Hidden".into()),
    },
    KeyMapping {
      sequence:    "zm".into(),
      action:      "cmd:messages".into(),
      description: Some("Show Messages".into()),
    },
    KeyMapping {
      sequence:    "zo".into(),
      action:      "cmd:output".into(),
      description: Some("Show Output".into()),
    },
    // Find
    KeyMapping {
      sequence:    "/".into(),
      action:      "cmd:find".into(),
      description: Some("Find in current".into()),
    },
    KeyMapping {
      sequence:    "n".into(),
      action:      "cmd:next".into(),
      description: Some("Find next".into()),
    },
    KeyMapping {
      sequence:    "b".into(),
      action:      "cmd:prev".into(),
      description: Some("Find previous".into()),
    },
    // Theme picker
    KeyMapping {
      sequence:    "ut".into(),
      action:      "cmd:theme".into(),
      description: Some("UI Theme picker".into()),
    },
    KeyMapping {
      sequence:    "Ut".into(),
      action:      "cmd:theme".into(),
      description: Some("UI Theme picker".into()),
    },
    // File ops
    KeyMapping {
      sequence:    "a".into(),
      action:      "cmd:add".into(),
      description: Some("Add file/folder".into()),
    },
    KeyMapping {
      sequence:    "r".into(),
      action:      "cmd:rename".into(),
      description: Some("Rename selected".into()),
    },
    KeyMapping {
      sequence:    "D".into(),
      action:      "cmd:delete".into(),
      description: Some("Delete selected".into()),
    },
    // Selection
    KeyMapping {
      sequence:    " ".into(),
      action:      "cmd:select_toggle".into(),
      description: Some("Toggle selected".into()),
    },
    KeyMapping {
      sequence:    "u".into(),
      action:      "cmd:select_clear".into(),
      description: Some("Clear selected".into()),
    },
    // Clipboard
    KeyMapping {
      sequence:    "c".into(),
      action:      "clipboard:copy".into(),
      description: Some("Copy selected".into()),
    },
    KeyMapping {
      sequence:    "x".into(),
      action:      "clipboard:move".into(),
      description: Some("Move selected".into()),
    },
    KeyMapping {
      sequence:    "v".into(),
      action:      "clipboard:paste".into(),
      description: Some("Paste clipboard".into()),
    },
    // Overlays
    KeyMapping {
      sequence:    "<Esc>".into(),
      action:      "overlay:close".into(),
      description: Some("Close overlays".into()),
    },
  ]
}

/// Default header templates used when the user doesn't set `ui.header`.
pub const DEFAULT_HEADER_LEFT: &str = "{username}@{hostname}:{current_file}";
pub const DEFAULT_HEADER_RIGHT: &str = "{current_file_size}  {owner}  \
                                        {current_file_permissions}  \
                                        {current_file_ctime}";

/// Default modal sizes (percentages of terminal) mirrored by overlay fallbacks.
pub fn default_modals() -> UiModals
{
  UiModals {
    prompt:  UiModalConfig { width_pct: 50, height_pct: 10 },
    confirm: UiModalConfig { width_pct: 50, height_pct: 10 },
    theme:   UiModalConfig { width_pct: 60, height_pct: 60 },
  }
}

pub const DEFAULT_DATE_FORMAT: &str = "%Y-%m-%d %H:%M";

pub fn default_panes() -> UiPanes
{
  UiPanes { parent: 10, current: 20, preview: 70 }
}

pub fn default_row_widths() -> UiRowWidths
{
  UiRowWidths { icon: 0, left: 0, middle: 0, right: 0 }
}

pub fn default_theme() -> UiTheme
{
  UiTheme {
    pane_bg:               Some("#101114".into()),
    border_fg:             Some("gray".into()),
    item_fg:               Some("white".into()),
    item_bg:               Some("#101114".into()),
    selected_item_fg:      Some("black".into()),
    selected_item_bg:      Some("cyan".into()),
    title_fg:              Some("gray".into()),
    title_bg:              Some("#101114".into()),
    info_fg:               Some("gray".into()),
    dir_fg:                Some("cyan".into()),
    dir_bg:                Some("#101114".into()),
    file_fg:               Some("white".into()),
    file_bg:               Some("#101114".into()),
    hidden_fg:             Some("darkgray".into()),
    hidden_bg:             Some("#101114".into()),
    exec_fg:               Some("green".into()),
    exec_bg:               Some("#101114".into()),
    selection_bar_fg:      Some("cyan".into()),
    selection_bar_copy_fg: Some("green".into()),
    selection_bar_move_fg: Some("yellow".into()),
  }
}

/// Apply built-in defaults to any unset UI fields.
pub fn apply_config_defaults(cfg: &mut Config)
{
  // Icons default already fine
  // Keys default already fine
  let ui = &mut cfg.ui;
  if ui.panes.is_none()
  {
    ui.panes = Some(default_panes());
  }
  if ui.date_format.is_none()
  {
    ui.date_format = Some(DEFAULT_DATE_FORMAT.into());
  }
  if ui.row.is_none()
  {
    ui.row = Some(UiRowFormat::default());
  }
  if ui.row_widths.is_none()
  {
    ui.row_widths = Some(default_row_widths());
  }
  if ui.modals.is_none()
  {
    ui.modals = Some(default_modals());
  }
  if ui.theme.is_none()
  {
    ui.theme = Some(default_theme());
  }
  if ui.header_left.is_none()
  {
    ui.header_left = Some(DEFAULT_HEADER_LEFT.into());
  }
  if ui.header_right.is_none()
  {
    ui.header_right = Some(DEFAULT_HEADER_RIGHT.into());
  }
  if ui.display_mode.is_none()
  {
    ui.display_mode = Some("absolute".into());
  }
  if ui.sort.is_none()
  {
    ui.sort = Some("name".into());
  }
  if ui.sort_reverse.is_none()
  {
    ui.sort_reverse = Some(false);
  }
  if ui.show.is_none()
  {
    ui.show = Some("none".into());
  }
}
