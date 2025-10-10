use ratatui::style::{
  Color,
  Modifier,
  Style,
};

#[test]
fn parse_color_named_and_hex()
{
  use lsv::ui::colors::parse_color;
  assert_eq!(parse_color("red"), Some(Color::Red));
  assert_eq!(parse_color("Gray"), Some(Color::Gray));
  assert_eq!(parse_color("darkgrey"), Some(Color::DarkGray));
  assert_eq!(parse_color("#00ff00"), Some(Color::Rgb(0, 255, 0)));
  assert_eq!(parse_color("#ABCDEF"), Some(Color::Rgb(0xAB, 0xCD, 0xEF)));
  assert_eq!(parse_color("not-a-color"), None);
  assert_eq!(parse_color("#123"), None);
}

#[test]
fn ansi_spans_basic_colors_and_reset()
{
  use lsv::ui::ansi::ansi_spans;
  let s = "\x1b[31mred\x1b[0mX"; // red + reset + plain X
  let spans = ansi_spans(s);
  assert_eq!(spans.len(), 2);
  // First span: "red" in red
  assert_eq!(spans[0].content.as_ref(), "red");
  assert_eq!(spans[0].style.fg, Some(Color::Red));
  // Second span: "X" default style
  assert_eq!(spans[1].content.as_ref(), "X");
  assert_eq!(spans[1].style, Style::default());
}

#[test]
fn ansi_spans_bold_and_blue_then_unbold()
{
  use lsv::ui::ansi::ansi_spans;
  // Bold blue 'A', then remove bold (22) for 'B', color stays
  let s = "\x1b[1;34mA\x1b[22mB";
  let spans = ansi_spans(s);
  assert_eq!(spans.len(), 2);
  // First span A: bold + blue
  assert_eq!(spans[0].content.as_ref(), "A");
  assert_eq!(spans[0].style.fg, Some(Color::Blue));
  // Second span B: blue without bold
  assert_eq!(spans[1].content.as_ref(), "B");
  assert_eq!(spans[1].style.fg, Some(Color::Blue));
  // Adding bold to second span should match first span style
  let s1_bold = spans[1].style.add_modifier(Modifier::BOLD);
  assert_eq!(s1_bold, spans[0].style);
}
