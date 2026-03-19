//! Command pane verbs and routing for App.

use crate::app::{
    App,
    CommandPaneState,
    InfoMode,
    Overlay,
};

impl App
{
    pub(crate) fn open_search(&mut self)
    {
        self.overlay = Overlay::CommandPane(Box::new(CommandPaneState {
            prompt:           "/".to_string(),
            input:            String::new(),
            cursor:           0,
            show_suggestions: false,
        }));
        self.force_full_redraw = true;
    }

    pub(crate) fn open_command(&mut self)
    {
        self.overlay = Overlay::CommandPane(Box::new(CommandPaneState {
            prompt:           ":".to_string(),
            input:            String::new(),
            cursor:           0,
            show_suggestions: false,
        }));
        self.force_full_redraw = true;
    }

    pub(crate) fn execute_command_line(
        &mut self,
        line: &str,
    )
    {
        let cmd = line.trim();
        let low = cmd.to_ascii_lowercase();
        // Split into tokens
        let mut parts = low.split_whitespace();
        let name = parts.next().unwrap_or("");
        match name
        {
            "" =>
            {}
            "show_marks" | "marks" =>
            {
                let text = self.list_marks_text();
                self.display_output("Marks", &text);
            }
            "delete_marks" | "delmark" =>
            {
                let mut removed = 0usize;
                for tok in parts
                {
                    if let Some(ch) = tok.chars().next()
                        && self.marks.remove(&ch).is_some()
                    {
                        removed += 1;
                    }
                }
                if removed > 0
                {
                    self.save_marks();
                }
                self.add_message(&format!("Deleted {} mark(s)", removed));
            }
            "search_text" | "find" => self.open_search(),
            "search_next" | "next" => self.search_next(),
            "search_prev" | "prev" => self.search_prev(),
            "toggle_messages" | "messages" =>
            {
                self.overlay = match self.overlay
                {
                    Overlay::Messages => Overlay::None,
                    _ => Overlay::Messages,
                };
                self.force_full_redraw = true;
            }
            "toggle_output" | "output" =>
            {
                self.overlay = match self.overlay
                {
                    Overlay::Output { .. } => Overlay::None,
                    _ => Overlay::Output {
                        title: String::from("Output"),
                        lines: Vec::new(),
                    },
                };
                self.force_full_redraw = true;
            }
            "change_theme" | "theme" => self.open_theme_picker(),
            "add_item" | "add" => self.open_add_entry_prompt(),
            "rename_selected" | "rename" => self.open_rename_entry_prompt(),
            "delete_selected" | "delete" => self.request_delete_selected(),
            "toggle_current_selected" | "select_toggle" =>
            {
                self.toggle_select_current()
            }
            "clear_selected" | "select_clear" => self.clear_all_selected(),
            "toggle_hidden_files" | "show_hidden_toggle" =>
            {
                self.config.ui.show_hidden = !self.config.ui.show_hidden;
                self.refresh_lists();
                self.refresh_preview();
                self.force_full_redraw = true;
            }
            "sort_name" => self.execute_command_line("sort name"),
            "sort_size" => self.execute_command_line("sort size"),
            "sort_modified_date" => self.execute_command_line("sort mtime"),
            "sort_created_date" => self.execute_command_line("sort created"),
            "sort" =>
            {
                if let Some(arg) = parts.next()
                    && let Some(k) = crate::enums::sort_key_from_str(arg)
                {
                    let current_name =
                        self.selected_entry().map(|e| e.name.clone());
                    self.sort_key = k;
                    self.refresh_lists();
                    if let Some(name) = current_name
                    {
                        crate::core::selection::reselect_by_name(self, &name);
                    }
                    self.refresh_preview();
                }
            }
            "reverse_sort" | "sort_reverse_toggle" =>
            {
                let current_name =
                    self.selected_entry().map(|e| e.name.clone());
                self.sort_reverse = !self.sort_reverse;
                self.refresh_lists();
                if let Some(name) = current_name
                {
                    crate::core::selection::reselect_by_name(self, &name);
                }
                self.refresh_preview();
            }
            "view_friendly_units" =>
            {
                self.execute_command_line("display friendly");
            }
            "view_precise_units" =>
            {
                self.execute_command_line("display absolute");
            }
            "display" =>
            {
                if let Some(arg) = parts.next()
                    && let Some(mode) = crate::enums::display_mode_from_str(arg)
                {
                    let had_meta = !matches!(self.info_mode, InfoMode::None)
                        || !matches!(self.sort_key, crate::actions::SortKey::Name);
                    self.display_mode = mode;
                    if matches!(self.info_mode, InfoMode::None)
                    {
                        self.info_mode = InfoMode::Modified;
                    }
                    let need_meta_now = !matches!(self.info_mode, InfoMode::None)
                        || !matches!(self.sort_key, crate::actions::SortKey::Name);
                    if !had_meta && need_meta_now
                    {
                        let current_name =
                            self.selected_entry().map(|e| e.name.clone());
                        self.refresh_lists();
                        if let Some(name) = current_name
                        {
                            crate::core::selection::reselect_by_name(self, &name);
                        }
                        self.refresh_preview();
                    }
                    self.force_full_redraw = true;
                }
            }
            "cd" =>
            {
                let rest = cmd.chars().skip(2).collect::<String>();
                let path = rest.trim();
                if !path.is_empty()
                {
                    let p = std::path::Path::new(path);
                    if p.is_dir()
                    {
                        self.set_cwd(p);
                    }
                    else
                    {
                        self.add_message(&format!(
                            "cd: not a directory: {}",
                            path
                        ));
                    }
                }
            }
            "add_mark" | "mark" =>
            {
                if let Some(arg) = parts.next()
                    && let Some(ch) = arg.chars().next()
                {
                    self.add_mark(ch);
                }
            }
            "goto_mark" | "goto" =>
            {
                if let Some(arg) = parts.next()
                    && let Some(ch) = arg.chars().next()
                {
                    self.goto_mark(ch);
                }
            }
            other =>
            {
                self.add_message(&format!("Unknown command: :{}", other));
            }
        }
    }

    pub(crate) fn search_next(&mut self)
    {
        if let Some(ref q) = self.search_query
        {
            let start = self.list_state.selected().unwrap_or(0);
            let next = if self.current_entries.is_empty()
            {
                None
            }
            else
            {
                self.find_match_from(
                    (start + 1) % self.current_entries.len(),
                    q,
                    false,
                )
            };
            if let Some(i) = next
            {
                self.list_state.select(Some(i));
                self.refresh_preview();
            }
        }
    }

    pub(crate) fn search_prev(&mut self)
    {
        if let Some(ref q) = self.search_query
        {
            let start = self.list_state.selected().unwrap_or(0);
            let len = self.current_entries.len();
            let prev_start = if len == 0 { 0 } else { (start + len - 1) % len };
            let prev = self.find_match_from(prev_start, q, true);
            if let Some(i) = prev
            {
                self.list_state.select(Some(i));
                self.refresh_preview();
            }
        }
    }
}
