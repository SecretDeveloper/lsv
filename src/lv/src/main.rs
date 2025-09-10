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
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use ratatui::Terminal;

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
    preview_lines_limit: usize,
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
        let preview_lines_limit = env::var("LV_PREVIEW_LINES")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(100);

        let mut app = Self {
            cwd,
            parent,
            current_entries,
            parent_entries,
            list_state,
            preview_lines: Vec::new(),
            preview_title: String::new(),
            preview_lines_limit,
        };
        app.refresh_preview();
        Ok(app)
    }

    fn selected_entry(&self) -> Option<&DirEntryInfo> {
        self.list_state.selected().and_then(|i| self.current_entries.get(i))
    }

    fn refresh_lists(&mut self) {
        self.parent = self.cwd.parent().map(|p| p.to_path_buf());
        self.current_entries = read_dir_sorted(&self.cwd).unwrap_or_default();
        self.parent_entries = if let Some(ref p) = self.parent { read_dir_sorted(p).unwrap_or_default() } else { Vec::new() };
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

        if is_dir {
            self.preview_title = format!("dir: {}", path.display());
            match read_dir_sorted(&path) {
                Ok(list) => {
                    let mut lines = Vec::new();
                    for e in list.into_iter().take(self.preview_lines_limit) {
                        let marker = if e.is_dir { "/" } else { "" };
                        lines.push(format!("{}{}", e.name, marker));
                    }
                    self.preview_lines = lines;
                }
                Err(err) => {
                    self.preview_lines = vec![format!("<error reading directory: {}>", err)];
                }
            }
        } else {
            self.preview_title = format!("file: {}", path.display());
            self.preview_lines = read_file_head(&path, self.preview_lines_limit)
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
                app.cwd = parent.to_path_buf();
                app.refresh_lists();
                app.refresh_preview();
            }
        }
        _ => {}
    }
    Ok(false)
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
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ])
        .split(f.area());

    draw_parent_panel(f, chunks[0], app);
    draw_current_panel(f, chunks[1], app);
    draw_preview_panel(f, chunks[2], app);
}

fn draw_parent_panel(f: &mut ratatui::Frame, area: Rect, app: &App) {
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

    let para = Paragraph::new(text).block(block).wrap(Wrap { trim: false });
    f.render_widget(para, area);
}
