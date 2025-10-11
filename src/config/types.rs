use std::path::PathBuf;

#[derive(Debug, Clone, Default)]
/// Icon configuration flags. Icons are optional and require a compatible font.
pub struct IconsConfig
{
  pub enabled:      bool,
  pub preset:       Option<String>,
  pub font:         Option<String>,
  // Optional defaults + per-extension map (lowercased keys)
  pub default_file: Option<String>,
  pub default_dir:  Option<String>,
  pub extensions:   std::collections::HashMap<String, String>,
  // Optional per-folder-name icon overrides (lowercased keys)
  pub folders:      std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Default)]
/// Key-handling configuration (currently only sequence timeout).
pub struct KeysConfig
{
  pub sequence_timeout_ms: u64,
}

#[derive(Debug, Clone, Default)]
/// Top-level configuration composed from Lua input.
pub struct Config
{
  pub config_version: u32,
  pub icons:          IconsConfig,
  pub keys:           KeysConfig,
  pub ui:             UiConfig,
}

#[derive(Debug, Clone)]
/// A single key mapping supplied by `lsv.map_action` or legacy bindings.
pub struct KeyMapping
{
  pub sequence:    String,
  pub action:      String,
  pub description: Option<String>,
}

#[derive(Debug, Clone, Default)]
/// Pane split percentages for the parent/current/preview columns.
pub struct UiPanes
{
  pub parent:  u16,
  pub current: u16,
  pub preview: u16,
}

#[derive(Debug, Clone)]
/// User interface configuration block replicated from Lua.
pub struct UiConfig
{
  pub panes:          Option<UiPanes>,
  pub show_hidden:    bool,
  pub max_list_items: usize,
  pub date_format:    Option<String>,
  pub header_left:    Option<String>,
  pub header_right:   Option<String>,
  pub header_bg:      Option<String>,
  pub header_fg:      Option<String>,
  pub row:            Option<UiRowFormat>,
  pub row_widths:     Option<UiRowWidths>,
  pub display_mode:   Option<String>,
  pub sort:           Option<String>,
  pub sort_reverse:   Option<bool>,
  pub show:           Option<String>,
  pub theme_path:     Option<PathBuf>,
  pub theme:          Option<UiTheme>,
  pub confirm_delete: bool,
  pub modals:         Option<UiModals>,
}

impl Default for UiConfig
{
  fn default() -> Self
  {
    Self {
      panes:          None,
      show_hidden:    false,
      max_list_items: 5000,
      date_format:    None,
      header_left:    None,
      header_right:   None,
      header_bg:      None,
      header_fg:      None,
      row:            Some(UiRowFormat::default()),
      row_widths:     None,
      display_mode:   None,
      sort:           None,
      sort_reverse:   None,
      show:           None,
      theme_path:     None,
      theme:          None,
      confirm_delete: true,
      modals:         None,
    }
  }
}

#[derive(Debug, Clone, Default)]
pub struct UiModalConfig
{
  pub width_pct:  u16, // 10..=100
  pub height_pct: u16, // 10..=100
}

#[derive(Debug, Clone, Default)]
pub struct UiModals
{
  pub prompt:  UiModalConfig,
  pub confirm: UiModalConfig,
  pub theme:   UiModalConfig,
}

#[derive(Debug, Clone)]
/// Template strings used to render each row in the directory panes.
pub struct UiRowFormat
{
  pub icon:   String,
  pub left:   String,
  pub middle: String,
  pub right:  String,
}

impl Default for UiRowFormat
{
  fn default() -> Self
  {
    Self {
      icon:   " ".to_string(),
      left:   "{name}".to_string(),
      middle: "".to_string(),
      right:  "{info}".to_string(),
    }
  }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
/// Optional fixed column widths for the row layout.
pub struct UiRowWidths
{
  pub icon:   u16,
  pub left:   u16,
  pub middle: u16,
  pub right:  u16,
}

#[derive(Debug, Clone, Default, PartialEq)]
/// Theme colours for the UI. Fields are optional and fall back to defaults.
pub struct UiTheme
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
  // Selection indicator (bar) colours
  pub selection_bar_fg:      Option<String>,
  pub selection_bar_copy_fg: Option<String>,
  pub selection_bar_move_fg: Option<String>,
}
