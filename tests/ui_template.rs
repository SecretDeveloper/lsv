use ratatui::style::{
  Color,
  Modifier,
};

#[test]
fn template_substitutes_cwd()
{
  use lsv::ui::template::format_header_side;
  let app = lsv::App::new().expect("app");
  let tpl = String::from("cwd: {cwd}");
  let out = format_header_side(&app, Some(&tpl));
  // Should contain the literal prefix and the cwd path
  assert!(out.text.starts_with("cwd: "));
  let cwd = app.get_cwd_path().display().to_string();
  assert!(out.text.contains(&cwd), "out.text={}, cwd={}", out.text, cwd);
}

#[test]
fn template_styles_placeholder()
{
  use lsv::ui::template::format_header_side;
  let app = lsv::App::new().expect("app");
  // Style cwd in red + bold within surrounding plain text
  let tpl = String::from("pre {cwd|fg=red;style=bold} post");
  let out = format_header_side(&app, Some(&tpl));
  // There should be multiple spans: pre, styled cwd, post
  assert!(out.spans.len() >= 3, "spans len={}", out.spans.len());
  let cwd = app.get_cwd_path().display().to_string();
  // Find the styled cwd span
  let styled = out
    .spans
    .iter()
    .find(|s| s.content.as_ref().contains(&cwd))
    .expect("styled cwd span");
  // Styled span should have red fg and bold
  assert_eq!(styled.style.fg, Some(Color::Red));
  let nobold = styled.style.remove_modifier(Modifier::BOLD);
  assert_eq!(nobold.fg, Some(Color::Red));
  // Prefix / suffix exist in the concatenated text
  assert!(out.text.contains("pre "));
  assert!(out.text.contains(" post"));
}
