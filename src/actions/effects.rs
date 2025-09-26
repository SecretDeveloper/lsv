//! Light-weight side effects returned from Lua actions.

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlayToggle
{
  #[default]
  None,
  Toggle,
  Show,
  Hide,
}

#[derive(Default, Debug, Clone)]
pub struct ActionEffects
{
  pub selection:      Option<usize>,
  pub quit:           bool,
  pub redraw:         bool,
  pub messages:       OverlayToggle,
  pub output_overlay: OverlayToggle,
  pub output:         Option<(String, String)>, // (title, text)
  pub theme_picker:   ThemePickerCommand,
  pub prompt:         PromptCommand,
  pub confirm:        ConfirmCommand,
}
use mlua::Table;

impl From<&str> for OverlayToggle
{
  fn from(s: &str) -> Self
  {
    match s
    {
      "toggle" => OverlayToggle::Toggle,
      "show" => OverlayToggle::Show,
      "hide" => OverlayToggle::Hide,
      _ => OverlayToggle::None,
    }
  }
}

pub fn parse_effects_from_lua(tbl: &Table) -> ActionEffects
{
  let mut fx = ActionEffects::default();
  // selection via context.selected_index
  if let Ok(ctx) = tbl.get::<Table>("context")
    && let Ok(sel_idx) = ctx.get::<u64>("selected_index")
  {
    fx.selection = Some(sel_idx as usize);
  }
  // overlays and output
  if let Ok(s) = tbl.get::<String>("messages")
  {
    fx.messages = s.as_str().into();
  }
  if let Ok(s) = tbl.get::<String>("output")
  {
    fx.output_overlay = s.as_str().into();
  }
  if let Ok(text) = tbl.get::<String>("output_text")
  {
    let title = tbl
      .get::<String>("output_title")
      .unwrap_or_else(|_| String::from("Output"));
    fx.output = Some((title, text));
  }
  // redraw/quit
  fx.redraw = tbl.get::<bool>("redraw").unwrap_or(false);
  fx.quit = tbl.get::<bool>("quit").unwrap_or(false);
  if let Ok(tp) = tbl.get::<String>("theme_picker")
    && tp == "open"
  {
    fx.theme_picker = ThemePickerCommand::Open;
  }
  if let Ok(p) = tbl.get::<String>("prompt")
  {
    if p == "add_entry" || p == "add" || p == "new"
    {
      fx.prompt = PromptCommand::OpenAddEntry;
    }
    else if p == "rename_entry" || p == "rename"
    {
      fx.prompt = PromptCommand::OpenRenameEntry;
    }
  }
  if let Ok(c) = tbl.get::<String>("confirm")
  {
    if c == "delete" || c == "remove" || c == "rm"
    {
      crate::trace::log("[effects] confirm request 'delete'".to_string());
      fx.confirm = ConfirmCommand::Delete;
    }
  }

  fx
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThemePickerCommand
{
  #[default]
  None,
  Open,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptCommand
{
  #[default]
  None,
  OpenAddEntry,
  OpenRenameEntry,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmCommand
{
  #[default]
  None,
  Delete,
}
