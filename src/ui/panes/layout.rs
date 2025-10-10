use ratatui::layout::Constraint;

pub fn pane_constraints(app: &crate::App) -> [Constraint; 3]
{
  let (mut p, mut c, mut r) = (30u16, 40u16, 30u16);
  if let Some(panes) = app.config.ui.panes.as_ref()
  {
    p = panes.parent;
    c = panes.current;
    r = panes.preview;
  }
  let total = p.saturating_add(c).saturating_add(r);
  if total == 0
  {
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
