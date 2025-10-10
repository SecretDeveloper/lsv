use ratatui::{
  layout::Rect,
  style::{
    Color,
    Style,
  },
  widgets::{
    Block,
    Borders,
    Clear,
    Paragraph,
  },
};

pub fn draw_command_pane(
  f: &mut ratatui::Frame,
  full: Rect,
  app: &crate::App,
)
{
  let (_is_cmd, show_suggest) = match app.overlay
  {
    crate::app::Overlay::CommandPane(ref st) =>
    {
      (st.prompt == ":", st.show_suggestions && st.prompt == ":")
    }
    _ => (false, false),
  };
  let mut want_rows: u16 = 1;
  if full.height >= 2
  {
    want_rows = 2;
  }
  if show_suggest && full.height >= 3
  {
    want_rows = 3;
  }
  let use_two = want_rows >= 2;
  let use_three = want_rows >= 3;
  let height = want_rows;
  let area = Rect {
    x: full.x,
    y: full.y + full.height.saturating_sub(height),
    width: full.width,
    height,
  };
  f.render_widget(Clear, area);
  let mut prompt = String::from(":");
  let mut input = String::new();
  let mut cursor_x = 0u16;
  let cursor_y = area.y + if use_two { 1 } else { 0 };
  if let crate::app::Overlay::CommandPane(ref st_box) = app.overlay
  {
    let st = st_box.as_ref();
    prompt = st.prompt.clone();
    input = st.input.clone();
    cursor_x = area.x + (prompt.len() as u16) + (st.cursor as u16);
  }
  let text = format!("{}{}", prompt, input);
  if use_two
  {
    let mut block = Block::default().borders(Borders::TOP);
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
    f.render_widget(block, area);
    let inner = Rect {
      x:      area.x,
      y:      area.y + 1,
      width:  area.width,
      height: 1,
    };
    let para = Paragraph::new(text);
    f.render_widget(para, inner);
    if use_three
    {
      let inner2 = Rect {
        x:      area.x,
        y:      area.y + 2,
        width:  area.width,
        height: 1,
      };
      let mut line = String::new();
      let prefix = input.trim();
      let cmds = crate::commands::all();
      for c in cmds.iter()
      {
        if prefix.is_empty() || c.starts_with(prefix)
        {
          if !line.is_empty()
          {
            line.push_str("  ");
          }
          line.push_str(c);
        }
      }
      if line.is_empty()
      {
        line.push_str("<no matches>");
      }
      let mut style = Style::default().fg(Color::DarkGray);
      if let Some(th) = app.config.ui.theme.as_ref()
        && let Some(fg) =
          th.info_fg.as_ref().and_then(|s| crate::ui::colors::parse_color(s))
      {
        style = Style::default().fg(fg);
      }
      let para2 = Paragraph::new(line).style(style);
      f.render_widget(para2, inner2);
    }
  }
  else
  {
    let para = Paragraph::new(text);
    f.render_widget(para, area);
  }
  f.set_cursor_position((
    cursor_x.min(area.x + area.width.saturating_sub(1)),
    cursor_y,
  ));
}
