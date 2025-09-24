// Apply ActionEffects and config overlays produced by Lua actions.
//
// - `apply_effects` handles transient UI state (selection, overlays, output,
//   redraw, quit).
// - `apply_config_overlay` applies validated, structural changes coming from
//   Lua (panes, theme, sort, etc.), computing minimal relist/redraw.
use super::effects::{
  ActionEffects,
  OverlayToggle,
  ThemePickerCommand,
};
use crate::app::App;

pub fn apply_effects(
  app: &mut App,
  fx: ActionEffects,
)
{
  if let Some(idx) = fx.selection
  {
    let len = app.current_entries.len();
    if len > 0
    {
      let i = idx.min(len.saturating_sub(1));
      app.list_state.select(Some(i));
      app.refresh_preview();
    }
  }

  match fx.messages
  {
    OverlayToggle::Toggle =>
    {
      app.show_messages = !app.show_messages;
      app.show_output = false;
      app.show_whichkey = false;
    }
    OverlayToggle::Show =>
    {
      app.show_messages = true;
      app.show_output = false;
      app.show_whichkey = false;
    }
    OverlayToggle::Hide | OverlayToggle::None =>
    {}
  }

  match fx.output_overlay
  {
    OverlayToggle::Toggle =>
    {
      app.show_output = !app.show_output;
      app.show_messages = false;
      app.show_whichkey = false;
    }
    OverlayToggle::Show =>
    {
      app.show_output = true;
      app.show_messages = false;
      app.show_whichkey = false;
    }
    OverlayToggle::Hide | OverlayToggle::None =>
    {}
  }

  if let Some((title, text)) = fx.output
  {
    app.display_output(&title, &text);
  }

  match fx.theme_picker
  {
    ThemePickerCommand::Open =>
    {
      app.open_theme_picker();
    }
    ThemePickerCommand::None =>
    {}
  }

  if fx.redraw
  {
    app.force_full_redraw = true;
  }
  if fx.quit
  {
    app.should_quit = true;
  }
}

// Apply a validated config overlay to the App, computing the minimal
// necessary updates (relist, redraw, preview refresh) to keep UI responsive.
pub fn apply_config_overlay(
  app: &mut App,
  data: &crate::config_data::ConfigData,
)
{
  let mut relist = false;
  let mut redraw_only = false;
  let mut layout_change = false;
  let mut refresh_preview_only = false;

  // Preserve selection by name on relist
  let selected_name = app.selected_entry().map(|e| e.name.clone());

  // Keys: sequence timeout
  if app.config.keys.sequence_timeout_ms != data.keys_sequence_timeout_ms
  {
    app.config.keys.sequence_timeout_ms = data.keys_sequence_timeout_ms;
  }

  // UI panes: affects layout
  let current_panes =
    app.config.ui.panes.clone().unwrap_or(crate::config::UiPanes {
      parent:  30,
      current: 40,
      preview: 30,
    });
  if current_panes.parent != data.ui.panes.parent
    || current_panes.current != data.ui.panes.current
    || current_panes.preview != data.ui.panes.preview
  {
    layout_change = true;
    app.config.ui.panes = Some(crate::config::UiPanes {
      parent:  data.ui.panes.parent,
      current: data.ui.panes.current,
      preview: data.ui.panes.preview,
    });
  }

  // Hidden files: change listing
  if app.config.ui.show_hidden != data.ui.show_hidden
  {
    app.config.ui.show_hidden = data.ui.show_hidden;
    relist = true;
  }

  // Date format: render only
  if app.config.ui.date_format != data.ui.date_format
  {
    app.config.ui.date_format = data.ui.date_format.clone();
    redraw_only = true;
  }

  // Display mode: render only
  if app.display_mode != data.ui.display_mode
  {
    app.display_mode = data.ui.display_mode;
    redraw_only = true;
  }

  // Preview lines: affects preview trimming
  if app.config.ui.preview_lines != data.ui.preview_lines
  {
    app.config.ui.preview_lines = data.ui.preview_lines;
    refresh_preview_only = true;
  }

  // Max list items: impacts listing
  if app.config.ui.max_list_items != data.ui.max_list_items
  {
    app.config.ui.max_list_items = data.ui.max_list_items;
    relist = true;
  }

  // Row templates: render only
  let current_row = app.config.ui.row.clone().unwrap_or_default();
  if current_row.icon != data.ui.row.icon
    || current_row.left != data.ui.row.left
    || current_row.middle != data.ui.row.middle
    || current_row.right != data.ui.row.right
  {
    app.config.ui.row = Some(crate::config::UiRowFormat {
      icon:   data.ui.row.icon.clone(),
      left:   data.ui.row.left.clone(),
      middle: data.ui.row.middle.clone(),
      right:  data.ui.row.right.clone(),
    });
    redraw_only = true;
  }

  // Row widths: render only
  let cur_widths = app.config.ui.row_widths.clone().unwrap_or_default();
  let new_widths = match data.ui.row_widths.as_ref()
  {
    Some(rw) => crate::config::UiRowWidths {
      icon:   rw.icon,
      left:   rw.left,
      middle: rw.middle,
      right:  rw.right,
    },
    None => crate::config::UiRowWidths::default(),
  };
  if cur_widths != new_widths
  {
    app.config.ui.row_widths = Some(new_widths);
    redraw_only = true;
  }

  // Theme: render only
  let mut theme_changed = false;
  let cur_theme = app.config.ui.theme.clone().unwrap_or_default();
  let new_theme = if let Some(th) = data.ui.theme.as_ref()
  {
    let t = crate::config::UiTheme {
      pane_bg:          th.pane_bg.clone(),
      border_fg:        th.border_fg.clone(),
      item_fg:          th.item_fg.clone(),
      item_bg:          th.item_bg.clone(),
      selected_item_fg: th.selected_item_fg.clone(),
      selected_item_bg: th.selected_item_bg.clone(),
      title_fg:         th.title_fg.clone(),
      title_bg:         th.title_bg.clone(),
      info_fg:          th.info_fg.clone(),
      dir_fg:           th.dir_fg.clone(),
      dir_bg:           th.dir_bg.clone(),
      file_fg:          th.file_fg.clone(),
      file_bg:          th.file_bg.clone(),
      hidden_fg:        th.hidden_fg.clone(),
      hidden_bg:        th.hidden_bg.clone(),
      exec_fg:          th.exec_fg.clone(),
      exec_bg:          th.exec_bg.clone(),
    };
    Some(t)
  }
  else
  {
    None
  };
  if new_theme.as_ref() != Some(&cur_theme)
  {
    app.config.ui.theme = new_theme;
    theme_changed = true;
  }
  let new_theme_path =
    data.ui.theme_path.as_ref().map(std::path::PathBuf::from);
  if app.config.ui.theme_path.as_ref().map(|p| p.as_path())
    != new_theme_path.as_deref()
  {
    app.config.ui.theme_path = new_theme_path;
  }
  if theme_changed
  {
    redraw_only = true;
  }

  // Sorting: change listing
  if app.sort_key != data.sort_key || app.sort_reverse != data.sort_reverse
  {
    app.sort_key = data.sort_key;
    app.sort_reverse = data.sort_reverse;
    relist = true;
  }

  // Info field: render only
  if app.info_mode != data.show_field
  {
    app.info_mode = data.show_field;
    redraw_only = true;
  }

  // Apply effects
  if relist
  {
    app.refresh_lists();
    if let Some(name) = selected_name.as_ref()
    {
      if let Some(idx) =
        app.current_entries.iter().position(|e| &e.name == name)
      {
        app.list_state.select(Some(idx));
      }
    }
    app.refresh_preview();
    app.force_full_redraw = true;
    return;
  }

  if refresh_preview_only
  {
    app.refresh_preview();
  }

  if redraw_only || layout_change
  {
    app.force_full_redraw = true;
  }
}
