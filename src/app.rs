//! Core application state, used both by the TUI and integration tests.
//!
//! The [`App`] struct models the in-memory view of the three-pane interface
//! (current directory listing, preview cache, overlays, etc.). The binary owns
//! an instance of `App`, but tests can create their own to simulate navigation
//! or exercise Lua actions.

use ratatui::widgets::ListState;
use std::{
    env,
    fs,
    io,
    path::PathBuf,
};

use crate::actions::SortKey;

pub(crate) mod state;
pub use state::{
    App,
    Clipboard,
    ClipboardOp,
    CommandPaneState,
    ConfirmKind,
    ConfirmState,
    DirEntryInfo,
    DisplayMode,
    InfoMode,
    KeyState,
    LuaRuntime,
    Overlay,
    PreviewState,
    PromptKind,
    PromptState,
    RunningPreview,
    ThemePickerEntry,
    ThemePickerState,
};

pub(crate) mod commands;
pub(crate) mod keys;
pub(crate) mod marks;
pub(crate) mod nav;
pub(crate) mod overlays_api;
pub(crate) mod preview_ctrl;
pub(crate) mod selection;

// Re-exported types live in state.rs

impl App
{
    /// Construct a fresh [`App`] using the current working directory as the
    /// starting point.
    pub fn new() -> io::Result<Self>
    {
        let cwd = env::current_dir()?;
        // Temporary initial read with default sort (Name asc)
        let current_entries = {
            // Build a temporary App-like context for sorting
            let mut tmp = Vec::new();
            for de in (fs::read_dir(&cwd)?).flatten()
            {
                let path = de.path();
                let name = de.file_name().to_string_lossy().to_string();
                if let Ok(ft) = de.file_type()
                {
                    let meta = fs::metadata(&path).ok();
                    let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                    let mtime = meta.as_ref().and_then(|m| m.modified().ok());
                    let ctime = meta.as_ref().and_then(|m| m.created().ok());
                    tmp.push(DirEntryInfo {
                        name,
                        path,
                        is_dir: ft.is_dir(),
                        size,
                        mtime,
                        ctime,
                    });
                }
            }
            tmp.sort_by(|a, b| match (a.is_dir, b.is_dir)
                {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                });
            tmp
        };
        let parent_entries = if let Some(p) = cwd.parent()
        {
            // Same initial read for parent
            let mut tmp = Vec::new();
            for de in (fs::read_dir(p)?).flatten()
            {
                let path = de.path();
                let name = de.file_name().to_string_lossy().to_string();
                if let Ok(ft) = de.file_type()
                {
                    let meta = fs::metadata(&path).ok();
                    let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                    let mtime = meta.as_ref().and_then(|m| m.modified().ok());
                    let ctime = meta.as_ref().and_then(|m| m.created().ok());
                    tmp.push(DirEntryInfo {
                        name,
                        path,
                        is_dir: ft.is_dir(),
                        size,
                        mtime,
                        ctime,
                    });
                }
            }
            tmp.sort_by(|a, b| match (a.is_dir, b.is_dir)
                {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
                });
            tmp
        }
        else
        {
            Vec::new()
        };

        let mut list_state = ListState::default();
        if !current_entries.is_empty()
        {
            list_state.select(Some(0));
        }
        let mut app = Self {
            cwd,
            current_entries,
            parent_entries,
            list_state,
            preview: PreviewState::default(),
            recent_messages: Vec::new(),
            overlay: Overlay::None,
            config: crate::config::Config::default(),
            keys: KeyState::default(),
            force_full_redraw: false,
            lua: None,
            selected: std::collections::HashSet::new(),
            clipboard: None,
            sort_key: SortKey::Name,
            sort_reverse: false,
            info_mode: InfoMode::None,
            display_mode: DisplayMode::Absolute,
            should_quit: false,
            search_query: None,
            _search_locked: false,
            marks: std::collections::HashMap::new(),
            pending_mark: false,
            pending_goto: false,
            running_preview: None,
        };
        // Load marks from config root
        if let Some(root) = app.theme_root_dir()
        {
            let path = root.join("marks");
            app.marks = crate::core::marks::load_marks(&path);
        }
        // Discover configuration paths (entry not executed yet)
        if let Ok(paths) = crate::config::discover_config_paths()
        {
            match crate::config::load_config(&paths)
            {
                Ok((cfg, maps, engine_opt)) =>
                {
                    app.config = cfg;
                    app.keys.maps = maps;
                    app.rebuild_keymap_lookup();
                    if let Some((eng, key, action_keys)) = engine_opt
                    {
                        app.lua = Some(LuaRuntime {
                            engine:    eng,
                            previewer: Some(key),
                            actions:   action_keys,
                        });
                    }
                    else
                    {
                        app.lua = None;
                    }
                    // Re-apply lists to honor config (e.g., show_hidden)
                    // Also apply optional initial sort/show from config.ui
                    if let Some(ref srt) = app.config.ui.sort
                    && let Some(k) = crate::enums::sort_key_from_str(srt)
                    {
                        app.sort_key = k;
                    }
                    if let Some(b) = app.config.ui.sort_reverse
                    {
                        app.sort_reverse = b;
                    }
                    if let Some(ref sh) = app.config.ui.show
                    {
                        if sh.eq_ignore_ascii_case("none")
                        {
                            app.info_mode = crate::app::InfoMode::None;
                        }
                        else if let Some(m) = crate::enums::info_mode_from_str(sh)
                        {
                            app.info_mode = m;
                        }
                    }
                    app.refresh_lists();
                    // Apply display_mode from config if present
                    if let Some(dm) = app.config.ui.display_mode.as_deref()
                    && let Some(mode) = crate::enums::display_mode_from_str(dm)
                    {
                        app.display_mode = mode;
                    }
                }
                Err(e) =>
                {
                    eprintln!("lsv: config load error: {}", e);
                }
            }
        }
        app.refresh_preview();
        Ok(app)
    }

    fn find_match_from(
        &self,
        start: usize,
        pat: &str,
        backwards: bool,
    ) -> Option<usize>
    {
        if self.current_entries.is_empty() || pat.is_empty()
        {
            return None;
        }
        let pat_l = pat.to_lowercase();
        let len = self.current_entries.len();
        if backwards
        {
            let mut idx = start;
            for _ in 0..len
            {
                if let Some(e) = self.current_entries.get(idx)
                && e.name.to_lowercase().contains(&pat_l)
                {
                    return Some(idx);
                }
                if idx == 0
                {
                    idx = len - 1;
                }
                else
                {
                    idx -= 1;
                }
            }
        }
        else
        {
            let mut idx = start;
            for _ in 0..len
            {
                if let Some(e) = self.current_entries.get(idx)
                && e.name.to_lowercase().contains(&pat_l)
                {
                    return Some(idx);
                }
                idx = (idx + 1) % len;
            }
        }
        None
    }


    #[allow(dead_code)]
    pub(crate) fn update_search_live(
        &mut self,
        q: &str,
    )
    {
        if q.is_empty()
        {
            return;
        }
        let start = self.list_state.selected().unwrap_or(0);
        let len = self.current_entries.len();
        if len == 0
        {
            return;
        }
        // Try from current to include current when first typing
        if let Some(i) = self.find_match_from(start, q, false)
        {
            self.list_state.select(Some(i));
            self.refresh_preview();
            // regular draw is enough
        }
    }

    /// Test helper: inject a prepared Lua engine and registered action keys.
    ///
    /// This lets integration tests execute Lua callbacks without loading files
    /// from disk.
    pub fn inject_lua_engine_for_tests(
        &mut self,
        engine: crate::config::LuaEngine,
        action_keys: Vec<mlua::RegistryKey>,
    )
    {
        self.lua =
            Some(LuaRuntime { engine, previewer: None, actions: action_keys });
    }

    pub fn show_hidden(&self) -> bool
    {
        self.config.ui.show_hidden
    }
    pub fn get_date_format(&self) -> Option<String>
    {
        self.config.ui.date_format.clone()
    }

    pub fn set_force_full_redraw(
        &mut self,
        v: bool,
    )
    {
        self.force_full_redraw = v;
    }
    pub fn get_force_full_redraw(&self) -> bool
    {
        self.force_full_redraw
    }
    pub fn get_show_messages(&self) -> bool
    {
        matches!(self.overlay, Overlay::Messages)
    }
    pub fn get_show_output(&self) -> bool
    {
        matches!(self.overlay, Overlay::Output { .. })
    }
    pub fn get_show_whichkey(&self) -> bool
    {
        matches!(self.overlay, Overlay::WhichKey { .. })
    }
    pub fn get_output_title(&self) -> &str
    {
        if let Overlay::Output { ref title, .. } = self.overlay
        {
            title.as_str()
        }
        else
        {
            ""
        }
    }
    pub fn get_output_text(&self) -> String
    {
        if let Overlay::Output { ref lines, .. } = self.overlay
        {
            lines.join("\n")
        }
        else
        {
            String::new()
        }
    }

    pub fn get_list_selected_index(&self) -> Option<usize>
    {
        self.list_state.selected()
    }
    pub fn get_quit(&self) -> bool
    {
        self.should_quit
    }
    pub fn get_sort_reverse(&self) -> bool
    {
        self.sort_reverse
    }
    pub fn set_sort_reverse(
        &mut self,
        v: bool,
    )
    {
        self.sort_reverse = v;
    }
    pub fn get_display_mode(&self) -> DisplayMode
    {
        self.display_mode
    }
    pub fn get_info_mode(&self) -> InfoMode
    {
        self.info_mode
    }

    pub fn get_entry(
        &self,
        idx: usize,
    ) -> Option<DirEntryInfo>
    {
        self.current_entries.get(idx).cloned()
    }

    pub fn get_sort_key(&self) -> crate::actions::SortKey
    {
        self.sort_key
    }
    pub fn set_config(
        &mut self,
        cfg: crate::config::Config,
    )
    {
        self.config = cfg;
    }
    pub fn get_config(&mut self) -> crate::config::Config
    {
        self.config.clone()
    }
    pub fn get_cwd_path(&self) -> std::path::PathBuf
    {
        self.cwd.clone()
    }

    pub fn preview_line_count(&self) -> usize
    {
        self.preview.static_lines.len()
    }

    pub fn recent_messages_len(&self) -> usize
    {
        self.recent_messages.len()
    }

    pub fn add_message(
        &mut self,
        msg: &str,
    )
    {
        let m = msg.trim().to_string();
        if m.is_empty()
        {
            return;
        }
        self.recent_messages.push(m);
        if self.recent_messages.len() > 100
        {
            let _ = self.recent_messages.drain(0..self.recent_messages.len() - 100);
        }
        self.force_full_redraw = true;
    }

    pub fn clear_recent_messages(&mut self)
    {
        if !self.recent_messages.is_empty()
        {
            self.recent_messages.clear();
            self.force_full_redraw = true;
        }
    }

    pub fn set_theme_by_name(
        &mut self,
        name: &str,
    ) -> bool
    {
        let root = match self.theme_root_dir()
        {
            Some(p) => p,
            None =>
            {
                self.add_message("Theme: unable to determine config directory");
                return false;
            }
        };
        // Prefer <root>/lua/themes then <root>/themes
        let themes_dir = {
            let module_dir = root.join("lua").join("themes");
            if std::fs::metadata(&module_dir).map(|m| m.is_dir()).unwrap_or(false)
            {
                module_dir
            }
            else
            {
                root.join("themes")
            }
        };
        let rd = match std::fs::read_dir(&themes_dir)
        {
            Ok(v) => v,
            Err(_) => return false,
        };
        let target_lower = name.to_lowercase();
        for ent in rd.flatten()
        {
            let path = ent.path();
            if !path.is_file()
            {
                continue;
            }
            if let Some(ext) = path.extension().and_then(|s| s.to_str())
            {
                if !ext.eq_ignore_ascii_case("lua")
                {
                    continue;
                }
            }
            else
            {
                continue;
            }
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if stem.to_lowercase() == target_lower
            {
                match crate::config::load_theme_from_file(&path)
                {
                    Ok(theme) =>
                    {
                        self.config.ui.theme = Some(theme);
                        self.config.ui.theme_path = Some(path.clone());
                        self.force_full_redraw = true;
                        return true;
                    }
                    Err(e) =>
                    {
                        self.add_message(&format!(
                            "Theme: failed to load {} ({})",
                            path.display(),
                            e
                        ));
                        return false;
                    }
                }
            }
        }
        false
    }

    pub(crate) fn theme_root_dir(&self) -> Option<PathBuf>
    {
        crate::config::discover_config_paths().ok().map(|p| p.root)
    }

    pub(crate) fn theme_picker_move(
        &mut self,
        delta: isize,
    )
    {
        crate::core::overlays::theme_picker_move(self, delta)
    }

    pub(crate) fn confirm_theme_picker(&mut self)
    {
        crate::core::overlays::confirm_theme_picker(self)
    }

    pub(crate) fn cancel_theme_picker(&mut self)
    {
        if let Overlay::ThemePicker(state) =
        std::mem::replace(&mut self.overlay, Overlay::None)
        {
            let st = *state;
            self.config.ui.theme = st.original_theme;
            self.config.ui.theme_path = st.original_theme_path;
            self.force_full_redraw = true;
        }
    }

    pub(crate) fn is_theme_picker_active(&self) -> bool
    {
        matches!(self.overlay, Overlay::ThemePicker(_))
    }

    pub fn display_output(
        &mut self,
        title: &str,
        text: &str,
    )
    {
        let lines: Vec<String> =
        text.replace('\r', "").lines().map(|s| s.to_string()).collect();
        self.overlay = Overlay::Output { title: title.to_string(), lines };
        self.force_full_redraw = true;
    }
}

pub(crate) fn common_affixes(names: &[String]) -> (String, String)
{
    if names.is_empty()
    {
        return (String::new(), String::new());
    }

    fn common_prefix(
        a: &str,
        b: &str,
    ) -> String
    {
        let mut out = String::new();
        for (ca, cb) in a.chars().zip(b.chars())
        {
            if ca == cb
            {
                out.push(ca);
            }
            else
            {
                break;
            }
        }
        out
    }
    fn common_suffix(
        a: &str,
        b: &str,
    ) -> String
    {
        let mut rev: Vec<char> = Vec::new();
        for (ca, cb) in a.chars().rev().zip(b.chars().rev())
        {
            if ca == cb
            {
                rev.push(ca);
            }
            else
            {
                break;
            }
        }
        rev.into_iter().rev().collect()
    }

    let mut pre = names[0].clone();
    for n in names.iter().skip(1)
    {
        pre = common_prefix(&pre, n);
        if pre.is_empty()
        {
            break;
        }
    }
    let mut suf = names[0].clone();
    for n in names.iter().skip(1)
    {
        suf = common_suffix(&suf, n);
        if suf.is_empty()
        { /* keep going to ensure empty is final */ }
    }
    (pre, suf)
}
