use std::cmp::min;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Terminal;
use mlua::{Function as LuaFunction, RegistryKey};
use std::process::Command;

mod config;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new()?;
    run_app(&mut app)?;
    Ok(())
}

struct DirEntryInfo {
    name: String,
    path: PathBuf,
    is_dir: bool,
}

struct App {
    cwd: PathBuf,
    parent: Option<PathBuf>,
    current_entries: Vec<DirEntryInfo>,
    parent_entries: Vec<DirEntryInfo>,
    list_state: ListState,
    preview_lines: Vec<String>,
    preview_title: String,
    config_paths: Option<config::ConfigPaths>,
    config: config::Config,
    keymaps: Vec<config::KeyMapping>,
    keymap_lookup: std::collections::HashMap<String, String>,
    force_full_redraw: bool,
    status_error: Option<String>,
    lua_engine: Option<config::LuaEngine>,
    previewer_fn: Option<RegistryKey>,
}

impl App {
    fn new() -> io::Result<Self> {
        let cwd = env::current_dir()?;
        let parent = cwd.parent().map(|p| p.to_path_buf());
        let current_entries = read_dir_sorted(&cwd)?;
        let parent_entries = if let Some(ref p) = parent { read_dir_sorted(p)? } else { Vec::new() };

        let mut list_state = ListState::default();
        if !current_entries.is_empty() {
            list_state.select(Some(0));
        }
        let mut app = Self {
            cwd,
            parent,
            current_entries,
            parent_entries,
            list_state,
            preview_lines: Vec::new(),
            preview_title: String::new(),
            config_paths: None,
            config: config::Config::default(),
            keymaps: Vec::new(),
            keymap_lookup: std::collections::HashMap::new(),
            force_full_redraw: false,
            status_error: None,
            lua_engine: None,
            previewer_fn: None,
        };
        // Discover configuration paths (entry not executed yet)
        if let Ok(paths) = crate::config::discover_config_paths() {
            match crate::config::load_config(&paths) {
                Ok((cfg, maps, engine_opt)) => {
                    app.config_paths = Some(paths);
                    app.config = cfg;
                    app.keymaps = maps;
                    app.rebuild_keymap_lookup();
                    app.status_error = None;
                    if let Some((eng, key)) = engine_opt {
                        app.lua_engine = Some(eng);
                        app.previewer_fn = Some(key);
                    } else {
                        app.lua_engine = None;
                        app.previewer_fn = None;
                    }
                }
                Err(e) => {
                    eprintln!("lv: config load error: {}", e);
                    app.config_paths = Some(paths);
                    app.status_error = Some(format!("Config error: {}", e));
                }
            }
        }
        app.refresh_preview();
        Ok(app)
    }

    fn selected_entry(&self) -> Option<&DirEntryInfo> {
        self.list_state.selected().and_then(|i| self.current_entries.get(i))
    }

    fn refresh_lists(&mut self) {
        self.parent = self.cwd.parent().map(|p| p.to_path_buf());
        self.current_entries = read_dir_sorted(&self.cwd).unwrap_or_default();
        if self.current_entries.len() > self.config.ui.max_list_items {
            self.current_entries.truncate(self.config.ui.max_list_items);
        }
        self.parent_entries = if let Some(ref p) = self.parent { read_dir_sorted(p).unwrap_or_default() } else { Vec::new() };
        if self.parent_entries.len() > self.config.ui.max_list_items {
            self.parent_entries.truncate(self.config.ui.max_list_items);
        }
        // Clamp selection
        let max_idx = self.current_entries.len().saturating_sub(1);
        if let Some(sel) = self.list_state.selected() {
            self.list_state.select(if self.current_entries.is_empty() { None } else { Some(min(sel, max_idx)) });
        } else if !self.current_entries.is_empty() {
            self.list_state.select(Some(0));
        }
    }

    fn refresh_preview(&mut self) {
        // Avoid borrowing self while mutating by cloning the needed fields first
        let (is_dir, path) = match self.selected_entry() {
            Some(e) => (e.is_dir, e.path.clone()),
            None => {
                self.preview_title.clear();
                self.preview_lines.clear();
                return;
            }
        };

        let preview_limit = self.config.ui.preview_lines;
        if is_dir {
            self.preview_title = format!("dir: {}", path.display());
            match read_dir_sorted(&path) {
                Ok(list) => {
                    let mut lines = Vec::new();
                    for e in list.into_iter().take(preview_limit) {
                        let marker = if e.is_dir { "/" } else { "" };
                        let formatted = format!("{}{}", e.name, marker);
                        lines.push(sanitize_line(&formatted));
                    }
                    self.preview_lines = lines;
                }
                Err(err) => {
                    self.preview_lines = vec![format!("<error reading directory: {}>", err)];
                }
            }
        } else {
            self.preview_title = format!("file: {}", path.display());
            self.preview_lines = read_file_head(&path, preview_limit)
                .map(|v| v.into_iter().map(|s| sanitize_line(&s)).collect())
                .unwrap_or_else(|e| vec![format!("<error reading file: {}>", e)]);
        }
    }
}

fn read_dir_sorted(path: &Path) -> io::Result<Vec<DirEntryInfo>> {
    let mut entries: Vec<DirEntryInfo> = fs::read_dir(path)?
        .filter_map(|res| res.ok())
        .filter_map(|e| {
            let path = e.path();
            let name = e.file_name().to_string_lossy().to_string();
            match e.file_type() {
                Ok(ft) => Some(DirEntryInfo { name, path, is_dir: ft.is_dir() }),
                Err(_) => None,
            }
        })
        .collect();
    entries.sort_by(|a, b| {
        match (a.is_dir, b.is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        }
    });
    Ok(entries)
}

fn read_file_head(path: &Path, n: usize) -> io::Result<Vec<String>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut lines = Vec::new();
    for (i, line) in reader.lines().enumerate() {
        if i >= n { break; }
        lines.push(line.unwrap_or_default());
    }
    Ok(lines)
}

fn run_app(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // Ensure we always restore the terminal even if an error occurs during event handling
    let res: Result<(), Box<dyn std::error::Error>> = {
        let mut result: Result<(), Box<dyn std::error::Error>> = Ok(());
        loop {
            if app.force_full_redraw {
                let _ = terminal.clear();
                app.force_full_redraw = false;
            }
            if let Err(e) = terminal.draw(|f| ui(f, app)) {
                result = Err(e.into());
                break;
            }

            match crossterm::event::poll(Duration::from_millis(200)) {
                Ok(true) => match event::read() {
                    Ok(Event::Key(key)) => match handle_key(app, key) {
                        Ok(true) => break, // graceful exit
                        Ok(false) => {}
                        Err(e) => { result = Err(e.into()); break; }
                    },
                    Ok(Event::Resize(_, _)) => {}
                    Ok(_) => {}
                    Err(e) => { result = Err(e.into()); break; }
                },
                Ok(false) => {}
                Err(e) => { result = Err(e.into()); break; }
            }
        }
        result
    };

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

fn handle_key(app: &mut App, key: KeyEvent) -> io::Result<bool> {
    // First, try dynamic key mappings (single key only for now)
    if let KeyCode::Char(ch) = key.code {
        // Allow plain or SHIFT-modified letters; ignore Ctrl/Alt/Super
        let disallowed = key.modifiers.contains(KeyModifiers::CONTROL)
            || key.modifiers.contains(KeyModifiers::ALT)
            || key.modifiers.contains(KeyModifiers::SUPER);
        if !disallowed {
            let mut tried = std::collections::HashSet::new();
            for k in [
                ch.to_string(),
                ch.to_ascii_lowercase().to_string(),
                ch.to_ascii_uppercase().to_string(),
            ] {
                if !tried.insert(k.clone()) { continue; }
                if let Some(action) = app.keymap_lookup.get(&k).cloned() {
                    if dispatch_action(app, &action).unwrap_or(false) {
                        return Ok(false);
                    }
                }
            }
        }
    }
    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => return Ok(true),
        (KeyCode::Up, _) | (KeyCode::Char('k'), _) => {
            if let Some(sel) = app.list_state.selected() {
                if sel > 0 { app.list_state.select(Some(sel - 1)); app.refresh_preview(); }
            }
        }
        (KeyCode::Down, _) | (KeyCode::Char('j'), _) => {
            if let Some(sel) = app.list_state.selected() {
                if sel + 1 < app.current_entries.len() { app.list_state.select(Some(sel + 1)); app.refresh_preview(); }
            } else if !app.current_entries.is_empty() {
                app.list_state.select(Some(0)); app.refresh_preview();
            }
        }
        (KeyCode::Enter, _) | (KeyCode::Right, _) => {
            if let Some(entry) = app.selected_entry() {
                if entry.is_dir {
                    app.cwd = entry.path.clone();
                    app.refresh_lists();
                    app.refresh_preview();
                }
            }
        }
        (KeyCode::Backspace, _) | (KeyCode::Left, _) | (KeyCode::Char('h'), KeyModifiers::NONE) => {
            if let Some(parent) = app.cwd.parent() {
                // Remember the directory name we are leaving so we can reselect it
                let just_left = app
                    .cwd
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string());
                app.cwd = parent.to_path_buf();
                app.refresh_lists();
                if let Some(name) = just_left {
                    if let Some(idx) = app
                        .current_entries
                        .iter()
                        .position(|e| e.name == name)
                    {
                        app.list_state.select(Some(idx));
                    }
                }
                app.refresh_preview();
            }
        }
        _ => {}
    }
    Ok(false)
}

fn dispatch_action(app: &mut App, action: &str) -> io::Result<bool> {
    if let Some(rest) = action.strip_prefix("run_shell:") {
        if let Ok(idx) = rest.parse::<usize>() {
            if idx < app.config.shell_cmds.len() {
                let sc = app.config.shell_cmds[idx].clone();
                run_shell_command(app, &sc);
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn run_shell_command(app: &mut App, sc: &config::ShellCmd) {
    let selection_path = app.selected_entry().map(|e| e.path.clone()).unwrap_or_else(|| app.cwd.clone());
    let cwd = app.cwd.clone();

    let mut cmd_str = sc.cmd.clone();
    // Template replacements
    let path_str = selection_path.to_string_lossy().to_string();
    let dir_str = selection_path.parent().unwrap_or(&cwd).to_string_lossy().to_string();
    let name_str = selection_path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let ext_str = selection_path.extension().and_then(|s| s.to_str()).unwrap_or("").to_string();
    // Consistent placeholders
    cmd_str = cmd_str.replace("{path}", &shell_escape(&path_str));
    cmd_str = cmd_str.replace("{directory}", &shell_escape(&dir_str));
    cmd_str = cmd_str.replace("{dir}", &shell_escape(&dir_str));
    cmd_str = cmd_str.replace("{name}", &shell_escape(&name_str));
    cmd_str = cmd_str.replace("{extension}", &shell_escape(&ext_str));
    // Also support $f shorthand from sample: replace $f with shell-escaped path
    cmd_str = cmd_str.replace("$f", &shell_escape(&path_str));
    // Trim leading '&' (treated as hint)
    let cmd_trimmed = cmd_str.trim_start_matches('&').to_string();

    let is_interactive = looks_interactive(&sc.cmd);
    // in_preview removed: commands are handled via suspend or background spawn

    // Trace command intent
    trace_log(format!("[cmd] built cmd='{}' cwd='{}' file='{}'", cmd_trimmed, cwd.display(), path_str));
    // If command appears interactive, suspend TUI and run attached to terminal
    if is_interactive {
        // Suspend TUI and run interactive command attached to the terminal
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(stdout, LeaveAlternateScreen);
        let status = Command::new("sh")
            .arg("-lc")
            .arg(&cmd_trimmed)
            .current_dir(&cwd)
            .env("LV_PATH", &path_str)
            .env("LV_DIR", &dir_str)
            .env("LV_NAME", &name_str)
            .status();
        let _ = enable_raw_mode();
        let mut stdout2 = io::stdout();
        let _ = execute!(stdout2, EnterAlternateScreen);
        app.refresh_lists();
        app.refresh_preview();
        app.force_full_redraw = true;
        trace_log(format!("[cmd] interactive exit={:?}", status.as_ref().ok().and_then(|s| s.code())));
        let _ = status;
        return;
    }

    // Default: spawn asynchronously (background)
    match Command::new("sh")
        .arg("-lc")
        .arg(&cmd_trimmed)
        .current_dir(&cwd)
        .env("LV_PATH", &path_str)
        .env("LV_DIR", &dir_str)
        .env("LV_NAME", &name_str)
        .spawn() {
        Ok(child) => trace_log(format!("[cmd] spawned pid={}", child.id())),
        Err(e) => trace_log(format!("[cmd] spawn error: {}", e)),
    }
    app.refresh_lists();
    app.refresh_preview();
}

fn looks_interactive(cmd: &str) -> bool {
    // Heuristic detection for full-screen / interactive tools that require a TTY
    // Heuristic for interactive commands; no explicit hide_preview flag anymore.
    let lower = cmd.to_ascii_lowercase();
    let needles = [
        "nvim", "vim", "vi ", "nano", "emacs", "tmux", "less", "more", "ssh ", "top", "htop",
    ];
    needles.iter().any(|n| lower.contains(n))
}

fn shell_escape(s: &str) -> String {
    if s.is_empty() { "''".to_string() } else {
        let mut out = String::from("'");
        for ch in s.chars() {
            if ch == '\'' { out.push_str("'\\''"); } else { out.push(ch); }
        }
        out.push('\'');
        out
    }
}

impl App {
    fn rebuild_keymap_lookup(&mut self) {
        self.keymap_lookup.clear();
        for m in &self.keymaps {
            // Only support single-key for now
            if m.sequence.chars().count() == 1 {
                self.keymap_lookup.insert(m.sequence.clone(), m.action.clone());
            }
        }
    }
}

fn panel_title<'a>(label: &'a str, path: Option<&Path>) -> Line<'a> {
    let path_str = path.map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| String::from("<root>"));
    Line::from(vec![
        Span::styled(label, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(path_str, Style::default().fg(Color::Gray)),
    ])
}

fn ui(f: &mut ratatui::Frame, app: &mut App) {
    let constraints = pane_constraints(app);
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints)
        .split(f.area());

    draw_parent_panel(f, chunks[0], app);
    draw_current_panel(f, chunks[1], app);
    draw_preview_panel(f, chunks[2], app);

    if let Some(msg) = &app.status_error {
        draw_error_bar(f, f.area(), msg);
    }
}

fn pane_constraints(app: &App) -> [Constraint; 3] {
    // Defaults
    let (mut p, mut c, mut r) = (30u16, 40u16, 30u16);
    if let Some(panes) = app.config.ui.panes.as_ref() {
        p = panes.parent;
        c = panes.current;
        r = panes.preview;
    }
    let total = p.saturating_add(c).saturating_add(r);
    if total == 0 {
        return [Constraint::Percentage(30), Constraint::Percentage(40), Constraint::Percentage(30)];
    }
    let p_norm = (p as u32 * 100 / total as u32) as u16;
    let c_norm = (c as u32 * 100 / total as u32) as u16;
    let r_norm = 100u16.saturating_sub(p_norm).saturating_sub(c_norm);
    return [Constraint::Percentage(p_norm), Constraint::Percentage(c_norm), Constraint::Percentage(r_norm)];
}

fn draw_parent_panel(f: &mut ratatui::Frame, area: Rect, app: &App) {
    // Clear area to prevent artifacts when content shrinks
    f.render_widget(Clear, area);
    let title = panel_title("Parent", app.parent.as_deref());
    let block = Block::default().borders(Borders::ALL).title(title);
    let items: Vec<ListItem> = app
        .parent_entries
        .iter()
        .map(|e| {
            let marker = if e.is_dir { "/" } else { "" };
            ListItem::new(Line::from(Span::raw(format!("{}{}", e.name, marker))))
        })
        .collect();
    let list = List::new(items).block(block);
    f.render_widget(list, area);
}

fn draw_current_panel(f: &mut ratatui::Frame, area: Rect, app: &mut App) {
    // Clear area to prevent artifacts when content shrinks
    f.render_widget(Clear, area);
    let title = panel_title("Current", Some(&app.cwd));
    let block = Block::default().borders(Borders::ALL).title(title);
    let items: Vec<ListItem> = app
        .current_entries
        .iter()
        .map(|e| {
            let marker = if e.is_dir { "/" } else { "" };
            ListItem::new(Line::from(Span::raw(format!("{}{}", e.name, marker))))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .highlight_symbol("▶ ");

    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn draw_preview_panel(f: &mut ratatui::Frame, area: Rect, app: &App) {
    // Clear area to prevent artifacts when content shrinks or lines are shorter
    f.render_widget(Clear, area);
    // Try dynamic preview via Lua previewer or rule-based previewers
    let mut dynamic_lines: Option<Vec<String>> = None;
    if let Some(sel) = app.selected_entry() {
        if !sel.is_dir {
            dynamic_lines = run_previewer(app, &sel.path, area, app.config.ui.preview_lines);
        }
    }
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::from(vec![
            Span::styled("Preview", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled(&app.preview_title, Style::default().fg(Color::Gray)),
        ]));

    let text: Vec<Line> = if let Some(lines) = dynamic_lines.as_ref() {
        if lines.is_empty() {
            vec![Line::from(Span::styled("<no selection>", Style::default().fg(Color::DarkGray)))]
        } else {
            lines.iter().map(|l| Line::from(ansi_spans(l))).collect()
        }
    } else if app.preview_lines.is_empty() {
        vec![Line::from(Span::styled("<no selection>", Style::default().fg(Color::DarkGray)))]
    } else {
        app.preview_lines.iter().map(|l| Line::from(ansi_spans(l))).collect()
    };

    let para = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    f.render_widget(para, area);
}

fn draw_error_bar(f: &mut ratatui::Frame, area: Rect, msg: &str) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);
    let bar = layout[1];
    let text = Line::from(Span::styled(
        msg.to_string(),
        Style::default().fg(Color::Black).bg(Color::Red).add_modifier(Modifier::BOLD),
    ));
    let para = Paragraph::new(text);
    f.render_widget(Clear, bar);
    f.render_widget(para, bar);
}

fn sanitize_line(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\t' => out.push_str("    "),
            '\r' => {},
            c if c.is_control() => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

fn run_previewer(app: &App, path: &Path, area: Rect, limit: usize) -> Option<Vec<String>> {
    // 1) Lua previewer function (if configured)
    if let (Some(engine), Some(key)) = (app.lua_engine.as_ref(), app.previewer_fn.as_ref()) {
        let lua = engine.lua();
        if let Ok(func) = lua.registry_value::<LuaFunction>(key) {
            let path_str = path.to_string_lossy().to_string();
            let dir_str = path.parent().unwrap_or_else(|| Path::new(".")).to_string_lossy().to_string();
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_string();
            let is_binary = file_is_binary(path);
            if let Ok(ctx) = lua.create_table() {
                let _ = ctx.set("path", path_str.clone());
                let _ = ctx.set("directory", dir_str.clone());
                let _ = ctx.set("extension", ext);
                let _ = ctx.set("is_binary", is_binary);
                let _ = ctx.set("height", area.height as i64 -5);
                let _ = ctx.set("width", area.width as i64 -5);
                let _ = ctx.set("preview_x", area.x as i64);
                let _ = ctx.set("preview_y", area.y as i64);
                if let Ok(ret) = func.call::<Option<String>>(ctx) {
                    if let Some(mut cmd) = ret {
                        let name_str = path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                        let ext_str = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_string();
                        cmd = cmd.replace("{path}", &shell_escape(&path_str));
                        cmd = cmd.replace("{directory}", &shell_escape(&dir_str));
                        cmd = cmd.replace("{dir}", &shell_escape(&dir_str));
                        cmd = cmd.replace("{name}", &shell_escape(&name_str));
                        cmd = cmd.replace("{extension}", &shell_escape(&ext_str));
                        cmd = cmd.replace("{width}", &area.width.to_string());
                        cmd = cmd.replace("{height}", &area.height.to_string());
                        return run_previewer_command(&cmd, &dir_str, &path_str, &name_str, limit);
                    }
                }
            }
        }
    }

    // 2) Legacy rule-based previewers
    use globset::{Glob, GlobMatcher};
    let filename = path.file_name()?.to_string_lossy();
    let mut selected_cmd: Option<String> = None;
    for p in &app.config.previewers {
        if let Some(pat) = &p.pattern {
            if let Ok(glob) = Glob::new(pat) {
                let matcher: GlobMatcher = glob.compile_matcher();
                if matcher.is_match(&*filename) || matcher.is_match(path) {
                    selected_cmd = Some(p.cmd.clone());
                    break;
                }
            }
        } else if p.mime.is_some() {
            continue; // future: mime detection
        }
    }
    let cmd_str = selected_cmd?;
    let mut cmd = cmd_str.clone();
    let path_str = path.to_string_lossy().to_string();
    let dir_str = path.parent().unwrap_or_else(|| Path::new(".")).to_string_lossy().to_string();
    let name_str = path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let ext_str = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_string();
    cmd = cmd.replace("{path}", &shell_escape(&path_str));
    cmd = cmd.replace("{directory}", &shell_escape(&dir_str));
    cmd = cmd.replace("{dir}", &shell_escape(&dir_str));
    cmd = cmd.replace("{name}", &shell_escape(&name_str));
    cmd = cmd.replace("{extension}", &shell_escape(&ext_str));
    cmd = cmd.replace("{width}", &area.width.to_string());
    cmd = cmd.replace("{height}", &area.height.to_string());
    cmd = cmd.replace("{preview_x}", &area.x.to_string());
    cmd = cmd.replace("{preview_y}", &area.y.to_string());

    run_previewer_command(&cmd, &dir_str, &path_str, &name_str, limit)
}

fn run_previewer_command(cmd: &str, dir_str: &str, path_str: &str, name_str: &str, limit: usize) -> Option<Vec<String>> {
    trace_log(format!("[preview] cmd='{}' cwd='{}' file='{}'", cmd, dir_str, path_str));
    match Command::new("sh")
        .arg("-lc")
        .arg(cmd)
        .current_dir(dir_str)
        .env("LV_PATH", path_str)
        .env("LV_DIR", dir_str)
        .env("LV_NAME", name_str)
        .env("FORCE_COLOR", "1")
        .env("CLICOLOR_FORCE", "1")
        .output() {
        Ok(out) => {
            let mut buf = Vec::new();
            buf.extend_from_slice(&out.stdout);
            if !out.stderr.is_empty() { buf.push(b'\n'); buf.extend_from_slice(&out.stderr); }
            let text = String::from_utf8_lossy(&buf).replace('\r', "");
            trace_log(format!("[preview] exit_code={:?} bytes_out={}", out.status.code(), text.len()));
            trace_log_snippet("[preview] output", &text, 8192);
            let mut lines: Vec<String> = Vec::new();
            for l in text.lines() {
                lines.push(l.to_string());
                if lines.len() >= limit { break; }
            }
            Some(lines)
        }
        Err(e) => { trace_log(format!("[preview] error spawning: {}", e)); None }
    }
}

fn file_is_binary(path: &Path) -> bool {
    if let Ok(mut f) = File::open(path) {
        let mut buf = [0u8; 4096];
        if let Ok(n) = f.read(&mut buf) {
            let slice = &buf[..n];
            if slice.contains(&0) { return true; }
            if std::str::from_utf8(slice).is_err() { return true; }
        }
    }
    false
}

fn ansi_spans(s: &str) -> Vec<Span<'_>> {
    let bytes = s.as_bytes();
    let mut spans: Vec<Span> = Vec::new();
    let mut style = Style::default();
    let mut i: usize = 0;
    let mut seg_start: usize = 0; // byte index of current plain segment
    while i < bytes.len() {
        if bytes[i] == 0x1B && i + 1 < bytes.len() {
            // flush plain UTF-8 substring before escape
            if seg_start < i {
                if let Some(seg) = s.get(seg_start..i) {
                    spans.push(Span::styled(seg.to_string(), style));
                }
            }
            // parse escape sequence
            match bytes[i + 1] {
                b'[' => {
                    // CSI: ESC [ ... final (0x40-0x7E)
                    i += 2;
                    let start = i;
                    while i < bytes.len() && !(bytes[i] >= 0x40 && bytes[i] <= 0x7E) { i += 1; }
                    if i >= bytes.len() { break; }
                    let finalb = bytes[i];
                    let params = &s[start..i];
                    if finalb == b'm' { apply_sgr_seq(params, &mut style); }
                    i += 1; // consume final
                    seg_start = i;
                }
                b']' => {
                    // OSC: ESC ] ... BEL or ESC \
                    i += 2;
                    loop {
                        if i >= bytes.len() { break; }
                        if bytes[i] == 0x07 { i += 1; break; }
                        if bytes[i] == 0x1B && i + 1 < bytes.len() && bytes[i + 1] == b'\\' { i += 2; break; }
                        i += 1;
                    }
                    seg_start = i;
                }
                b'(' | b')' | b'*' | b'+' => {
                    // Charset selection (3-byte sequence)
                    i += 3;
                    seg_start = i;
                }
                _ => {
                    // Unknown escape; skip ESC and next byte
                    i += 2;
                    seg_start = i;
                }
            }
        } else if bytes[i] == b'\r' {
            // Carriage return: treat as line start; drop preceding segment
            i += 1;
            seg_start = i;
        } else {
            i += 1;
        }
    }
    // flush trailing segment
    if seg_start < bytes.len() {
        if let Some(seg) = s.get(seg_start..bytes.len()) {
            spans.push(Span::styled(seg.to_string(), style));
        }
    }
    spans
}

fn apply_sgr_seq(seq: &str, style: &mut Style) {
    let nums: Vec<i32> = seq.split(';').filter_map(|t| t.parse::<i32>().ok()).collect();
    if nums.is_empty() { *style = Style::default(); return; }
    let mut i = 0;
    while i < nums.len() {
        match nums[i] {
            0 => { *style = Style::default(); },
            1 => { *style = style.add_modifier(Modifier::BOLD); },
            3 => { *style = style.add_modifier(Modifier::ITALIC); },
            4 => { *style = style.add_modifier(Modifier::UNDERLINED); },
            22 => { *style = style.remove_modifier(Modifier::BOLD); },
            23 => { *style = style.remove_modifier(Modifier::ITALIC); },
            24 => { *style = style.remove_modifier(Modifier::UNDERLINED); },
            30..=37 => { style.fg = Some(basic_color((nums[i]-30) as u8, false)); },
            90..=97 => { style.fg = Some(basic_color((nums[i]-90) as u8, true)); },
            40..=47 => { style.bg = Some(basic_color((nums[i]-40) as u8, false)); },
            100..=107 => { style.bg = Some(basic_color((nums[i]-100) as u8, true)); },
            38 => {
                if i+1 < nums.len() {
                    match nums[i+1] {
                        5 => { if i+2 < nums.len() { style.fg = Some(Color::Indexed(nums[i+2] as u8)); i += 2; } },
                        2 => { if i+4 < nums.len() { style.fg = Some(Color::Rgb(nums[i+2] as u8, nums[i+3] as u8, nums[i+4] as u8)); i += 4; } },
                        _ => {}
                    }
                }
            }
            48 => {
                if i+1 < nums.len() {
                    match nums[i+1] {
                        5 => { if i+2 < nums.len() { style.bg = Some(Color::Indexed(nums[i+2] as u8)); i += 2; } },
                        2 => { if i+4 < nums.len() { style.bg = Some(Color::Rgb(nums[i+2] as u8, nums[i+3] as u8, nums[i+4] as u8)); i += 4; } },
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }
}

fn basic_color(code: u8, bright: bool) -> Color {
    match (code, bright) {
        (0, false) => Color::Black,
        (1, false) => Color::Red,
        (2, false) => Color::Green,
        (3, false) => Color::Yellow,
        (4, false) => Color::Blue,
        (5, false) => Color::Magenta,
        (6, false) => Color::Cyan,
        (7, false) => Color::Gray,
        (0, true) => Color::DarkGray,
        (1, true) => Color::LightRed,
        (2, true) => Color::LightGreen,
        (3, true) => Color::LightYellow,
        (4, true) => Color::LightBlue,
        (5, true) => Color::LightMagenta,
        (6, true) => Color::LightCyan,
        (7, true) => Color::White,
        _ => Color::White,
    }
}

fn trace_enabled() -> bool {
    std::env::var("LV_TRACE").map(|v| !v.is_empty() && v != "0").unwrap_or(false)
}

fn trace_log<S: AsRef<str>>(s: S) {
    if !trace_enabled() { return; }
    let line = format!("{} {}\n", now_millis(), s.as_ref());
    if let Some(path) = trace_file_path() {
        let _ = OpenOptions::new().create(true).append(true).open(path).and_then(|mut f| {
            use std::io::Write;
            f.write_all(line.as_bytes())
        });
    }
}

fn trace_log_snippet(tag: &str, text: &str, max: usize) {
    if !trace_enabled() { return; }
    let snippet = if text.len() > max { &text[..max] } else { text };
    trace_log(format!("{}:\n{}", tag, snippet));
}

fn trace_file_path() -> Option<std::path::PathBuf> {
    if let Ok(fp) = std::env::var("LV_TRACE_FILE") { return Some(std::path::PathBuf::from(fp)); }
    if let Ok(tmp) = std::env::var("TMPDIR") { return Some(std::path::PathBuf::from(tmp).join("lv-trace.log")); }
    Some(std::path::PathBuf::from("/tmp/lv-trace.log"))
}

fn now_millis() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0)
}
