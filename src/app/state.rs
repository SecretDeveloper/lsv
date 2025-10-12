use std::{
  path::PathBuf,
  time::SystemTime,
};

use mlua::RegistryKey;
use ratatui::widgets::ListState;

#[derive(Debug, Clone)]
/// Runtime state for lsv, including directory listings, preview cache, overlay
/// flags, and configuration.
pub struct DirEntryInfo
{
  pub(crate) name:   String,
  pub(crate) path:   PathBuf,
  pub(crate) is_dir: bool,
  pub(crate) size:   u64,
  pub(crate) mtime:  Option<SystemTime>,
  pub(crate) ctime:  Option<SystemTime>,
}

#[derive(Debug, Clone)]
pub struct ThemePickerEntry
{
  pub name:  String,
  pub path:  PathBuf,
  pub theme: crate::config::UiTheme,
}

#[derive(Debug, Clone)]
pub struct ThemePickerState
{
  pub entries:             Vec<ThemePickerEntry>,
  pub selected:            usize,
  pub original_theme:      Option<crate::config::UiTheme>,
  pub original_theme_path: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub enum Overlay
{
  None,
  WhichKey
  {
    prefix: String,
  },
  Messages,
  Output
  {
    title: String,
    lines: Vec<String>,
  },
  ThemePicker(Box<ThemePickerState>),
  Prompt(Box<PromptState>),
  Confirm(Box<ConfirmState>),
  CommandPane(Box<CommandPaneState>),
}

#[derive(Debug, Clone, Default)]
pub struct PreviewState
{
  pub static_lines: Vec<String>,
  pub cache_key:    Option<(std::path::PathBuf, u16, u16)>,
  pub cache_lines:  Option<Vec<String>>,
}

#[derive(Debug, Clone, Default)]
pub struct KeyState
{
  pub maps:     Vec<crate::config::KeyMapping>,
  pub lookup:   std::collections::HashMap<String, String>,
  pub prefixes: std::collections::HashSet<String>,
  pub pending:  String,
  pub last_at:  Option<std::time::Instant>,
}

pub struct LuaRuntime
{
  pub engine:    crate::config::LuaEngine,
  pub previewer: Option<RegistryKey>,
  pub actions:   Vec<RegistryKey>,
}

#[derive(Debug, Clone)]
pub enum PromptKind
{
  AddEntry,
  RenameEntry
  {
    from: std::path::PathBuf,
  },
  RenameMany
  {
    items: Vec<std::path::PathBuf>,
    pre:   String,
    suf:   String,
  },
}

#[derive(Debug, Clone)]
pub struct PromptState
{
  pub title:  String,
  pub input:  String,
  pub cursor: usize,
  pub kind:   PromptKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardOp
{
  Copy,
  Move,
}

#[derive(Debug, Clone)]
pub struct Clipboard
{
  pub op:    ClipboardOp,
  pub items: Vec<std::path::PathBuf>,
}

#[derive(Debug, Clone)]
pub enum ConfirmKind
{
  DeleteSelected(Vec<std::path::PathBuf>),
}

#[derive(Debug, Clone)]
pub struct ConfirmState
{
  pub title:       String,
  pub question:    String,
  pub default_yes: bool,
  pub kind:        ConfirmKind,
}

#[derive(Debug, Clone)]
pub struct CommandPaneState
{
  pub prompt:           String,
  pub input:            String,
  pub cursor:           usize,
  pub show_suggestions: bool,
}

/// Mutable application state driving the three-pane UI.
pub struct App
{
  pub(crate) cwd:               PathBuf,
  pub(crate) current_entries:   Vec<DirEntryInfo>,
  pub(crate) parent_entries:    Vec<DirEntryInfo>,
  pub(crate) list_state:        ListState,
  pub(crate) preview:           PreviewState,
  pub(crate) recent_messages:   Vec<String>,
  pub(crate) overlay:           Overlay,
  pub(crate) config:            crate::config::Config,
  pub(crate) keys:              KeyState,
  pub(crate) force_full_redraw: bool,
  pub(crate) lua:               Option<LuaRuntime>,
  pub(crate) selected:          std::collections::HashSet<std::path::PathBuf>,
  pub(crate) clipboard:         Option<Clipboard>,
  pub(crate) sort_key:          crate::actions::SortKey,
  pub(crate) sort_reverse:      bool,
  pub(crate) info_mode:         InfoMode,
  pub(crate) display_mode:      DisplayMode,
  pub(crate) should_quit:       bool,
  pub(crate) search_query:      Option<String>,
  pub(crate) _search_locked:    bool,
  pub(crate) marks: std::collections::HashMap<char, std::path::PathBuf>,
  pub(crate) pending_mark:      bool,
  pub(crate) pending_goto:      bool,
  pub(crate) running_preview:   Option<RunningPreview>,
}

pub struct RunningPreview
{
  pub rx: std::sync::mpsc::Receiver<Option<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InfoMode
{
  None,
  Size,
  Created,
  Modified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode
{
  Absolute,
  Friendly,
}
