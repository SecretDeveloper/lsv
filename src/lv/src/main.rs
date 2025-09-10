use std::cmp::min;
use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Terminal;
use std::process::{Command, Stdio};

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
        };
        // Discover configuration paths (entry not executed yet)
        if let Ok(paths) = crate::config::discover_config_paths() {
            match crate::config::load_config(&paths) {
                Ok((cfg, maps)) => {
                    app.config_paths = Some(paths);
                    app.config = cfg;
                    app.keymaps = maps;
                    app.rebuild_keymap_lookup();
                    app.status_error = None;
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
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
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
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
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
    // Simple template replacements: {path}
    let path_str = selection_path.to_string_lossy().to_string();
    let dir_str = selection_path.parent().unwrap_or(&cwd).to_string_lossy().to_string();
    let name_str = selection_path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    cmd_str = cmd_str.replace("{path}", &shell_escape(&path_str));
    cmd_str = cmd_str.replace("{dir}", &shell_escape(&dir_str));
    cmd_str = cmd_str.replace("{name}", &shell_escape(&name_str));
    // Also support $f shorthand from sample: replace $f with shell-escaped path
    cmd_str = cmd_str.replace("$f", &shell_escape(&path_str));
    // Trim leading '&' (treated as hint)
    let cmd_trimmed = cmd_str.trim_start_matches('&').to_string();

    let is_interactive = looks_interactive(&sc.cmd);
    // in_preview removed: commands are handled via suspend or background spawn

    // If command appears interactive, suspend TUI and run attached to terminal
    if is_interactive {
        // Suspend TUI and run interactive command attached to the terminal
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(stdout, LeaveAlternateScreen, DisableMouseCapture);
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
        let _ = execute!(stdout2, EnterAlternateScreen, EnableMouseCapture);
        app.refresh_lists();
        app.refresh_preview();
        app.force_full_redraw = true;
        let _ = status;
        return;
    }

    // Default: spawn asynchronously (background)
    let _ = Command::new("sh")
        .arg("-lc")
        .arg(&cmd_trimmed)
        .current_dir(&cwd)
        .env("LV_PATH", &path_str)
        .env("LV_DIR", &dir_str)
        .env("LV_NAME", &name_str)
        .spawn();
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
    let block = Block::default()
        .borders(Borders::ALL)
        .title(Line::from(vec![
            Span::styled("Preview", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled(&app.preview_title, Style::default().fg(Color::Gray)),
        ]));

    let text: Vec<Line> = if app.preview_lines.is_empty() {
        vec![Line::from(Span::styled("<no selection>", Style::default().fg(Color::DarkGray)))]
    } else {
        app.preview_lines.iter().map(|l| Line::from(l.clone())).collect()
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
