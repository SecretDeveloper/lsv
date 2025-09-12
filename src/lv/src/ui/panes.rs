use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};

pub fn panel_title<'a>(
  label: &'a str,
  path: Option<&std::path::Path>,
) -> Line<'a> {
  let path_str = path
    .map(|p| p.to_string_lossy().to_string())
    .unwrap_or_else(|| String::from("<root>"));
  Line::from(vec![
    Span::styled(
      label,
      Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    ),
    Span::raw("  "),
    Span::styled(path_str, Style::default().fg(Color::Gray)),
  ])
}

pub fn pane_constraints(app: &crate::App) -> [Constraint; 3] {
  let (mut p, mut c, mut r) = (30u16, 40u16, 30u16);
  if let Some(panes) = app.config.ui.panes.as_ref() {
    p = panes.parent;
    c = panes.current;
    r = panes.preview;
  }
  let total = p.saturating_add(c).saturating_add(r);
  if total == 0 {
    return [
      Constraint::Percentage(30),
      Constraint::Percentage(40),
      Constraint::Percentage(30),
    ];
  }
  let p_norm = (p as u32 * 100 / total as u32) as u16;
  let c_norm = (c as u32 * 100 / total as u32) as u16;
  let r_norm = 100u16.saturating_sub(p_norm).saturating_sub(c_norm);
  [
    Constraint::Percentage(p_norm),
    Constraint::Percentage(c_norm),
    Constraint::Percentage(r_norm),
  ]
}

pub fn draw_parent_panel(
  f: &mut ratatui::Frame,
  area: Rect,
  app: &crate::App,
) {
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

pub fn draw_current_panel(
  f: &mut ratatui::Frame,
  area: Rect,
  app: &mut crate::App,
) {
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
    .highlight_style(
      Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("▶ ");

  f.render_stateful_widget(list, area, &mut app.list_state);
}

pub fn draw_error_bar(
  f: &mut ratatui::Frame,
  area: Rect,
  msg: &str,
) {
  let layout = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Min(0), Constraint::Length(1)])
    .split(area);
  let bar = layout[1];
  let text = Line::from(Span::styled(
    msg.to_string(),
    Style::default()
      .fg(Color::Black)
      .bg(Color::Red)
      .add_modifier(Modifier::BOLD),
  ));
  let para = Paragraph::new(text);
  f.render_widget(Clear, bar);
  f.render_widget(para, bar);
}

pub fn draw_whichkey_panel(
  f: &mut ratatui::Frame,
  area: Rect,
  app: &crate::App,
) {
  // Collect matching keymaps for the current prefix
  let prefix = &app.whichkey_prefix;
  let mut entries: Vec<(String, String)> = Vec::new();
  for km in &app.keymaps {
    if km.sequence.starts_with(prefix) {
      let label = km
        .description
        .as_ref()
        .cloned()
        .unwrap_or_else(|| km.action.clone());
      entries.push((km.sequence.clone(), label));
    }
  }
  // If toggled via '?' with empty prefix, just show all entries
  // Limit number of rows
  let max_rows: usize = 12;
  if entries.len() > max_rows {
    entries.truncate(max_rows);
  }

  let title_str = if prefix.is_empty() {
    "Keys".to_string()
  } else {
    format!("Keys: prefix '{}'", prefix)
  };
  let block = Block::default()
    .borders(Borders::ALL)
    .title(Span::styled(
      title_str,
      Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    ));

  let items: Vec<ListItem> = entries
    .into_iter()
    .map(|(seq, label)| {
      let line = Line::from(vec![
        Span::styled(seq, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(label, Style::default().fg(Color::Gray)),
      ]);
      ListItem::new(line)
    })
    .collect();

  // Panel height: items + borders + one padding row
  let height = (items.len() as u16).saturating_add(2).max(3).min(area.height);

  // Place at bottom
  let layout = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Min(0), Constraint::Length(height)])
    .split(area);
  let panel = layout[1];

  f.render_widget(Clear, panel);
  let list = List::new(items).block(block);
  f.render_widget(list, panel);
}
