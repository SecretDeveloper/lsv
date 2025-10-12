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
      "marks" =>
      {
        let text = self.list_marks_text();
        self.display_output("Marks", &text);
      }
      "delmark" =>
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
      "find" => self.open_search(),
      "next" => self.search_next(),
      "prev" => self.search_prev(),
      "messages" =>
      {
        self.overlay = match self.overlay
        {
          Overlay::Messages => Overlay::None,
          _ => Overlay::Messages,
        };
        self.force_full_redraw = true;
      }
      "output" =>
      {
        self.overlay = match self.overlay
        {
          Overlay::Output { .. } => Overlay::None,
          _ =>
          {
            Overlay::Output { title: String::from("Output"), lines: Vec::new() }
          }
        };
        self.force_full_redraw = true;
      }
      "theme" => self.open_theme_picker(),
      "add" => self.open_add_entry_prompt(),
      "rename" => self.open_rename_entry_prompt(),
      "delete" => self.request_delete_selected(),
      "select_toggle" => self.toggle_select_current(),
      "select_clear" => self.clear_all_selected(),
      "show_hidden_toggle" =>
      {
        self.config.ui.show_hidden = !self.config.ui.show_hidden;
        self.refresh_lists();
        self.refresh_preview();
        self.force_full_redraw = true;
      }
      "sort" =>
      {
        if let Some(arg) = parts.next()
          && let Some(k) = crate::enums::sort_key_from_str(arg)
        {
          let current_name = self.selected_entry().map(|e| e.name.clone());
          self.sort_key = k;
          self.refresh_lists();
          if let Some(name) = current_name
          {
            crate::core::selection::reselect_by_name(self, &name);
          }
          self.refresh_preview();
        }
      }
      "sort_reverse_toggle" =>
      {
        let current_name = self.selected_entry().map(|e| e.name.clone());
        self.sort_reverse = !self.sort_reverse;
        self.refresh_lists();
        if let Some(name) = current_name
        {
          crate::core::selection::reselect_by_name(self, &name);
        }
        self.refresh_preview();
      }
      "display" =>
      {
        if let Some(arg) = parts.next()
          && let Some(mode) = crate::enums::display_mode_from_str(arg)
        {
          self.display_mode = mode;
          if matches!(self.info_mode, InfoMode::None)
          {
            self.info_mode = InfoMode::Modified;
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
            self.add_message(&format!("cd: not a directory: {}", path));
          }
        }
      }
      "mark" =>
      {
        if let Some(arg) = parts.next()
          && let Some(ch) = arg.chars().next()
        {
          self.add_mark(ch);
        }
      }
      "goto" =>
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
        self.find_match_from((start + 1) % self.current_entries.len(), q, false)
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
