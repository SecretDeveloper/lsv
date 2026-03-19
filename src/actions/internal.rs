// Internal actions and sorting controls
// This module is a child of the crate root and can access crate-private items.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey
{
    Name,
    Size,
    MTime,
    CTime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InternalAction
{
    Quit,
    Sort(SortKey),
    ToggleSortReverse,
    SetInfo(crate::app::InfoMode),
    SetDisplayMode(crate::app::DisplayMode),
    GoTop,
    GoBottom,
    NavUp,
    NavDown,
    NavParent,
    NavEnter,
    MarksAddWait,
    MarksGotoWait,
    RunCommand(String),
    ClipboardCopy,
    ClipboardMove,
    ClipboardPaste,
    ClipboardClear,
    CloseOverlays,
}

pub(crate) fn parse_internal_action(s: &str) -> Option<InternalAction>
{
    let low = s.trim().to_ascii_lowercase();
    if low == "quit" || low == "q"
    {
        return Some(InternalAction::Quit);
    }
    if low == "sort:reverse:toggle" || low == "sort:rev:toggle"
    {
        return Some(InternalAction::ToggleSortReverse);
    }
    if low.starts_with("sort:")
    {
        let parts: Vec<&str> = low.split(':').collect();
        if parts.len() >= 2
        {
            return crate::enums::sort_key_from_str(parts[1])
                .map(InternalAction::Sort);
        }
    }
    // Primary: show:* controls info display
    if low.starts_with("show:")
    {
        let parts: Vec<&str> = low.split(':').collect();
        if parts.len() >= 2
        {
            if parts[1] == "friendly"
            {
                return Some(InternalAction::SetDisplayMode(
                    crate::app::DisplayMode::Friendly,
                ));
            }
            return crate::enums::info_mode_from_str(parts[1])
                .map(InternalAction::SetInfo);
        }
    }
    if low.starts_with("display:")
    {
        let parts: Vec<&str> = low.split(':').collect();
        if parts.len() >= 2
        {
            return crate::enums::display_mode_from_str(parts[1])
                .map(InternalAction::SetDisplayMode);
        }
    }
    if low == "nav:top" || low == "top" || low == "gg"
    {
        return Some(InternalAction::GoTop);
    }
    if low == "nav:bottom" || low == "bottom" || low == "g$"
    {
        return Some(InternalAction::GoBottom);
    }
    if low == "nav:up"
    {
        return Some(InternalAction::NavUp);
    }
    if low == "nav:down"
    {
        return Some(InternalAction::NavDown);
    }
    if low == "nav:parent" || low == "nav:left"
    {
        return Some(InternalAction::NavParent);
    }
    if low == "nav:enter" || low == "nav:right"
    {
        return Some(InternalAction::NavEnter);
    }
    if low == "marks:add_wait" || low == "marks:add"
    {
        return Some(InternalAction::MarksAddWait);
    }
    if low == "marks:goto_wait" || low == "marks:goto"
    {
        return Some(InternalAction::MarksGotoWait);
    }
    if let Some(cmd) = low.strip_prefix("cmd:")
    {
        return Some(InternalAction::RunCommand(cmd.to_string()));
    }
    if low == "clipboard:copy"
    {
        return Some(InternalAction::ClipboardCopy);
    }
    if low == "clipboard:move"
    {
        return Some(InternalAction::ClipboardMove);
    }
    if low == "clipboard:paste"
    {
        return Some(InternalAction::ClipboardPaste);
    }
    if low == "clipboard:clear"
    {
        return Some(InternalAction::ClipboardClear);
    }
    if low == "overlay:close"
    {
        return Some(InternalAction::CloseOverlays);
    }
    None
}

pub(crate) fn execute_internal_action(
    app: &mut crate::app::App,
    action: InternalAction,
)
{
    let needs_meta = |info: crate::app::InfoMode, sort: SortKey| {
        !matches!(info, crate::app::InfoMode::None)
            || !matches!(sort, SortKey::Name)
    };

    match action
    {
        InternalAction::Quit =>
        {
            app.should_quit = true;
        }
        InternalAction::Sort(key) =>
        {
            // Reselect current item by name after resort
            let current_name = app.selected_entry().map(|e| e.name.clone());
            app.sort_key = key;
            app.refresh_lists();
            if let Some(name) = current_name
            {
                crate::core::selection::reselect_by_name(app, &name);
            }
            app.refresh_preview();
        }
        InternalAction::ToggleSortReverse =>
        {
            let current_name = app.selected_entry().map(|e| e.name.clone());
            app.sort_reverse = !app.sort_reverse;
            app.refresh_lists();
            if let Some(name) = current_name
            {
                crate::core::selection::reselect_by_name(app, &name);
            }
            app.refresh_preview();
        }
        InternalAction::SetInfo(mode) =>
        {
            let had_meta = needs_meta(app.info_mode, app.sort_key);
            app.info_mode = mode;
            let need_meta_now = needs_meta(app.info_mode, app.sort_key);
            if !had_meta && need_meta_now
            {
                let current_name = app.selected_entry().map(|e| e.name.clone());
                app.refresh_lists();
                if let Some(name) = current_name
                {
                    crate::core::selection::reselect_by_name(app, &name);
                }
                app.refresh_preview();
            }
            app.force_full_redraw = true;
        }
        InternalAction::SetDisplayMode(style) =>
        {
            let had_meta = needs_meta(app.info_mode, app.sort_key);
            app.display_mode = style;
            // If no info is selected yet, default to Modified so date becomes
            // visible
            if matches!(app.info_mode, crate::app::InfoMode::None)
            {
                app.info_mode = crate::app::InfoMode::Modified;
            }
            let need_meta_now = needs_meta(app.info_mode, app.sort_key);
            if !had_meta && need_meta_now
            {
                let current_name = app.selected_entry().map(|e| e.name.clone());
                app.refresh_lists();
                if let Some(name) = current_name
                {
                    crate::core::selection::reselect_by_name(app, &name);
                }
                app.refresh_preview();
            }
            app.force_full_redraw = true;
        }
        InternalAction::GoTop =>
        {
            if !app.current_entries.is_empty()
            {
                app.list_state.select(Some(0));
                app.refresh_preview();
            }
        }
        InternalAction::GoBottom =>
        {
            if !app.current_entries.is_empty()
            {
                let last = app.current_entries.len().saturating_sub(1);
                app.list_state.select(Some(last));
                app.refresh_preview();
            }
        }
        InternalAction::NavUp =>
        {
            if let Some(sel) = app.list_state.selected()
                && sel > 0
            {
                app.list_state.select(Some(sel - 1));
                app.refresh_preview();
            }
        }
        InternalAction::NavDown =>
        {
            if let Some(sel) = app.list_state.selected()
            {
                if sel + 1 < app.current_entries.len()
                {
                    app.list_state.select(Some(sel + 1));
                    app.refresh_preview();
                }
            }
            else if !app.current_entries.is_empty()
            {
                app.list_state.select(Some(0));
                app.refresh_preview();
            }
        }
        InternalAction::NavEnter =>
        {
            if let Some(entry) = app.selected_entry()
                && entry.is_dir
            {
                app.cwd = entry.path.clone();
                app.refresh_lists();
                if app.current_entries.is_empty()
                {
                    app.list_state.select(None);
                }
                else
                {
                    app.list_state.select(Some(0));
                }
                app.refresh_preview();
            }
        }
        InternalAction::NavParent =>
        {
            if let Some(parent) = app.cwd.parent()
            {
                let just_left = app
                    .cwd
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string());
                app.cwd = parent.to_path_buf();
                app.refresh_lists();
                if let Some(name) = just_left
                    && let Some(idx) =
                        app.current_entries.iter().position(|e| e.name == name)
                {
                    app.list_state.select(Some(idx));
                }
                app.refresh_preview();
            }
        }
        InternalAction::MarksAddWait =>
        {
            // Prefer the newer prompt overlay when available.
            crate::core::overlays::open_mark_add_prompt(app);
        }
        InternalAction::MarksGotoWait =>
        {
            app.pending_goto = true;
            app.add_message("Goto: type a letter to jump to its mark");
        }
        InternalAction::RunCommand(cmd) =>
        {
            app.execute_command_line(&cmd);
        }
        InternalAction::ClipboardCopy =>
        {
            app.copy_selection();
        }
        InternalAction::ClipboardMove =>
        {
            app.move_selection();
        }
        InternalAction::ClipboardPaste =>
        {
            app.paste_clipboard();
        }
        InternalAction::ClipboardClear =>
        {
            app.clear_all_selected();
        }
        InternalAction::CloseOverlays =>
        {
            app.overlay = crate::app::Overlay::None;
            app.force_full_redraw = true;
        }
    }
}

/// Produce lightweight effects for simple internal actions (quit/navigation)
/// without mutating the app directly. Returns None for actions that require
/// configuration or list mutations (sorting, display toggles).
pub(crate) fn internal_effects(
    app: &crate::app::App,
    action: &InternalAction,
) -> Option<super::effects::ActionEffects>
{
    use super::effects::ActionEffects;
    match action
    {
        InternalAction::Quit =>
        {
            let fx = ActionEffects { quit: true, ..Default::default() };
            Some(fx)
        }
        InternalAction::NavUp =>
        {
            if let Some(sel) = app.list_state.selected()
                && sel > 0
            {
                let fx = ActionEffects {
                    selection: Some(sel - 1),
                    ..Default::default()
                };
                Some(fx)
            }
            else
            {
                Some(ActionEffects::default())
            }
        }
        InternalAction::NavDown =>
        {
            let len = app.current_entries.len();
            if len == 0
            {
                return Some(ActionEffects::default());
            }
            let next = match app.list_state.selected()
            {
                Some(sel) if sel + 1 < len => Some(sel + 1),
                None => Some(0),
                _ => None,
            };
            if let Some(i) = next
            {
                let fx =
                    ActionEffects { selection: Some(i), ..Default::default() };
                Some(fx)
            }
            else
            {
                Some(ActionEffects::default())
            }
        }
        InternalAction::GoTop =>
        {
            if !app.current_entries.is_empty()
            {
                let fx =
                    ActionEffects { selection: Some(0), ..Default::default() };
                Some(fx)
            }
            else
            {
                Some(ActionEffects::default())
            }
        }
        InternalAction::GoBottom =>
        {
            if !app.current_entries.is_empty()
            {
                let last = app.current_entries.len().saturating_sub(1);
                let fx = ActionEffects {
                    selection: Some(last),
                    ..Default::default()
                };
                Some(fx)
            }
            else
            {
                Some(ActionEffects::default())
            }
        }
        InternalAction::MarksAddWait =>
        {
            let fx = ActionEffects {
                marks: super::effects::MarksCommand::AddWait,
                ..Default::default()
            };
            Some(fx)
        }
        InternalAction::MarksGotoWait =>
        {
            let fx = ActionEffects {
                marks: super::effects::MarksCommand::GotoWait,
                ..Default::default()
            };
            Some(fx)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests
{
    use std::fs;

    use super::{
        execute_internal_action,
        InternalAction,
    };

    #[test]
    fn set_info_size_refreshes_metadata_when_initially_missing()
    {
        let temp = tempfile::tempdir().expect("tempdir");
        let dir = temp.path();
        let file = dir.join("a.txt");
        fs::write(&file, b"abcdef").expect("write");

        let mut app = crate::app::App::new().expect("app");
        app.set_cwd(dir);

        let before = app
            .current_entries
            .iter()
            .find(|e| e.name == "a.txt")
            .expect("entry present")
            .size;
        assert_eq!(before, 0);

        execute_internal_action(
            &mut app,
            InternalAction::SetInfo(crate::app::InfoMode::Size),
        );

        let after = app
            .current_entries
            .iter()
            .find(|e| e.name == "a.txt")
            .expect("entry present")
            .size;
        assert_eq!(after, 6);
    }
}
