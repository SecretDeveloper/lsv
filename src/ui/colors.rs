use ratatui::style::Color;

pub fn parse_color(s: &str) -> Option<Color> {
  let low = s.trim().to_ascii_lowercase();
  match low.as_str() {
    "black" => Some(Color::Black),
    "red" => Some(Color::Red),
    "green" => Some(Color::Green),
    "yellow" => Some(Color::Yellow),
    "blue" => Some(Color::Blue),
    "magenta" | "purple" => Some(Color::Magenta),
    "cyan" => Some(Color::Cyan),
    "gray" | "grey" => Some(Color::Gray),
    "darkgray" | "darkgrey" => Some(Color::DarkGray),
    "white" => Some(Color::White),
    _ => {
      // Try #RRGGBB
      if let Some(rgb) = parse_hex_rgb(&low) { return Some(rgb); }
      None
    }
  }
}

fn parse_hex_rgb(s: &str) -> Option<Color> {
  let t = s.strip_prefix('#')?;
  if t.len() != 6 { return None; }
  let r = u8::from_str_radix(&t[0..2], 16).ok()?;
  let g = u8::from_str_radix(&t[2..4], 16).ok()?;
  let b = u8::from_str_radix(&t[4..6], 16).ok()?;
  Some(Color::Rgb(r, g, b))
}

