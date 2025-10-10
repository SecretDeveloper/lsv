use ratatui::{
  layout::{
    Constraint,
    Direction,
    Layout,
    Rect,
  },
  style::{
    Color,
    Modifier,
    Style,
  },
  text::{
    Line,
    Span,
  },
  widgets::{
    Block,
    Borders,
    Clear,
    Paragraph,
  },
};
use unicode_width::UnicodeWidthStr;

pub fn draw_whichkey_panel(
  f: &mut ratatui::Frame,
  area: Rect,
  app: &crate::App,
)
{
  fn tokenize_seq(s: &str) -> Vec<String>
  {
    let mut out = Vec::new();
    let mut i = 0;
    let b = s.as_bytes();
    while i < b.len()
    {
      if b[i] == b'<'
        && let Some(j) = s[i + 1..].find('>')
      {
        let end = i + 1 + j + 1;
        out.push(s[i..end].to_string());
        i = end;
        continue;
      }
      let ch = s[i..].chars().next().unwrap();
      out.push(ch.to_string());
      i += ch.len_utf8();
    }
    out
  }

  fn format_token(tok: &str) -> String
  {
    match tok
    {
      " " => "Space".to_string(),
      "\t" => "Tab".to_string(),
      "\n" => "Enter".to_string(),
      "<Esc>" => "Escape".to_string(),
      _ =>
      {
        if tok.starts_with('<') && tok.ends_with('>')
        {
          let inner = &tok[1..tok.len() - 1];
          if let Some(rest) = inner.strip_prefix("C-")
          {
            return format!("Ctrl-{}", rest);
          }
          if let Some(rest) = inner.strip_prefix("M-")
          {
            return format!("Alt-{}", rest);
          }
          if let Some(rest) = inner.strip_prefix("S-")
          {
            return format!("Super-{}", rest);
          }
          if let Some(rest) = inner.strip_prefix("Sh-")
          {
            return format!("Shift-{}", rest);
          }
        }
        tok.to_string()
      }
    }
  }

  fn format_seq_for_display(s: &str) -> String
  {
    let toks = tokenize_seq(s);
    let formatted: Vec<String> =
      toks.into_iter().map(|t| format_token(&t)).collect();
    formatted.join("")
  }

  use std::collections::HashMap;
  let mut map: HashMap<&str, (&str, &str)> = HashMap::new();
  for km in &app.keys.maps
  {
    let label = km.description.as_deref().unwrap_or(km.action.as_str());
    map.insert(km.sequence.as_str(), (km.sequence.as_str(), label));
  }

  let prefix = match app.overlay
  {
    crate::app::Overlay::WhichKey { ref prefix } => prefix.as_str(),
    _ => "",
  };
  let mut buckets: HashMap<String, Vec<(&str, &str)>> = HashMap::new();
  let prefix_toks = tokenize_seq(prefix);
  for (seq, (_, label)) in map.into_iter()
  {
    let seq_toks = tokenize_seq(seq);
    if seq_toks.len() > prefix_toks.len()
    {
      if seq_toks[..prefix_toks.len()] == prefix_toks[..]
      {
        let mut np_toks = prefix_toks.clone();
        np_toks.push(seq_toks[prefix_toks.len()].clone());
        let np = np_toks.concat();
        buckets.entry(np).or_default().push((seq, label));
      }
    }
    else if seq_toks.len() == prefix_toks.len() && seq_toks == prefix_toks
    {
      buckets.entry(seq.to_string()).or_default().push((seq, label));
    }
  }

  #[derive(Clone)]
  struct Entry
  {
    left:     String,
    right:    String,
    is_group: bool,
  }
  let mut entries: Vec<Entry> = Vec::new();
  let mut keys: Vec<String> = buckets.keys().cloned().collect();
  keys.sort();
  for k in keys
  {
    let list = buckets.get(&k).unwrap();
    let mut exact_only = false;
    if list.len() == 1
    {
      let (seq, _) = list[0];
      if seq == k
      {
        exact_only = true;
      }
    }
    if exact_only
    {
      let (_seq, label) = list[0];
      entries.push(Entry {
        left:     format_seq_for_display(&k),
        right:    label.to_string(),
        is_group: false,
      });
    }
    else
    {
      let n = list.len();
      let label = if n == 1
      {
        "(1 binding)".to_string()
      }
      else
      {
        format!("({} bindings)", n)
      };
      entries.push(Entry {
        left:     format_seq_for_display(&k),
        right:    label,
        is_group: true,
      });
    }
  }

  if entries.is_empty()
  {
    return;
  }

  let title_str = if prefix.is_empty()
  {
    "Keys".to_string()
  }
  else
  {
    format!("Keys: prefix '{}'", format_seq_for_display(prefix))
  };
  let mut block = Block::default().borders(Borders::ALL).title(Span::styled(
    title_str,
    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
  ));
  if let Some(th) = app.config.ui.theme.as_ref()
  {
    if let Some(bg) =
      th.pane_bg.as_ref().and_then(|s| crate::ui::colors::parse_color(s))
    {
      block = block.style(Style::default().bg(bg));
    }
    if let Some(bfg) =
      th.border_fg.as_ref().and_then(|s| crate::ui::colors::parse_color(s))
    {
      block = block.border_style(Style::default().fg(bfg));
    }
  }

  let inner_width = area.width.saturating_sub(2) as usize;
  let mut rows = ((area.height as u32 * 20) / 100) as u16;
  if rows < 3
  {
    rows = 3;
  }
  if rows + 2 > area.height
  {
    rows = area.height.saturating_sub(2);
  }
  if rows == 0
  {
    rows = 1;
  }

  let compute_widths = |row_count: usize| -> (Vec<usize>, usize, usize) {
    let rows_usize = row_count.max(1);
    let cols = entries.len().div_ceil(rows_usize).max(1);
    let mut col_widths = vec![0usize; cols];
    for (c, width) in col_widths.iter_mut().enumerate()
    {
      let mut w = 0usize;
      for r in 0..rows_usize
      {
        let idx = c * rows_usize + r;
        if idx >= entries.len()
        {
          break;
        }
        let e = &entries[idx];
        let cell = format!("{}  {}", e.left, e.right);
        let cw = UnicodeWidthStr::width(cell.as_str());
        if cw > w
        {
          w = cw;
        }
      }
      *width = w + 2;
    }
    let total: usize = col_widths.iter().sum();
    (col_widths, total, cols)
  };

  let mut rows_usize = rows as usize;
  let (mut col_widths, mut total_width, _) = compute_widths(rows_usize);
  while total_width > inner_width && (rows_usize as u16) + 2 < area.height
  {
    rows_usize += 1;
    let (new_widths, new_total, _) = compute_widths(rows_usize);
    col_widths = new_widths;
    total_width = new_total;
  }

  let mut lines: Vec<Line> = Vec::new();
  for r in 0..rows_usize
  {
    let mut spans: Vec<Span> = Vec::new();
    let mut consumed_any = false;
    for (c, col_width) in col_widths.iter().enumerate()
    {
      let idx = c * rows_usize + r;
      if idx >= entries.len()
      {
        continue;
      }
      consumed_any = true;
      let e = &entries[idx];
      let left_style =
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD);
      let right_style = if e.is_group
      {
        Style::default().fg(Color::DarkGray)
      }
      else
      {
        Style::default().fg(Color::Gray)
      };
      let cell = format!("{}  {}", e.left, e.right);
      let cw = UnicodeWidthStr::width(cell.as_str());
      spans.push(Span::styled(e.left.clone(), left_style));
      spans.push(Span::raw("  "));
      spans.push(Span::styled(e.right.clone(), right_style));
      let pad = (*col_width).saturating_sub(cw);
      if pad > 0
      {
        let max_pad = 4096usize;
        spans.push(Span::raw(" ".repeat(std::cmp::min(pad, max_pad))));
      }
    }
    if consumed_any
    {
      lines.push(Line::from(spans));
    }
  }

  let panel_height = (rows_usize as u16).saturating_add(2).min(area.height);
  let layout = Layout::default()
    .direction(Direction::Vertical)
    .constraints([Constraint::Min(0), Constraint::Length(panel_height)])
    .split(area);
  let panel = layout[1];
  f.render_widget(Clear, panel);
  let para = Paragraph::new(lines).block(block);
  f.render_widget(para, panel);
}
