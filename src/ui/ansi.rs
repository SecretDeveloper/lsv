use ratatui::{
  style::{
    Color,
    Modifier,
    Style,
  },
  text::Span,
};

pub fn ansi_spans(s: &str) -> Vec<Span<'_>>
{
  let bytes = s.as_bytes();
  let mut spans: Vec<Span> = Vec::new();
  let mut style = Style::default();
  let mut i: usize = 0;
  let mut seg_start: usize = 0;
  while i < bytes.len()
  {
    if bytes[i] == 0x1B && i + 1 < bytes.len()
    {
      if seg_start < i
      {
        if let Some(seg) = s.get(seg_start..i)
        {
          spans.push(Span::styled(seg.to_string(), style));
        }
      }
      match bytes[i + 1]
      {
        b'[' =>
        {
          i += 2;
          let start = i;
          while i < bytes.len() && !(bytes[i] >= 0x40 && bytes[i] <= 0x7E)
          {
            i += 1;
          }
          if i >= bytes.len()
          {
            break;
          }
          let finalb = bytes[i];
          let params = &s[start..i];
          if finalb == b'm'
          {
            apply_sgr_seq(params, &mut style);
          }
          i += 1;
          seg_start = i;
        }
        b']' =>
        {
          i += 2;
          loop
          {
            if i >= bytes.len()
            {
              break;
            }
            if bytes[i] == 0x07
            {
              i += 1;
              break;
            }
            if bytes[i] == 0x1B && i + 1 < bytes.len() && bytes[i + 1] == b'\\'
            {
              i += 2;
              break;
            }
            i += 1;
          }
          seg_start = i;
        }
        b'(' | b')' | b'*' | b'+' =>
        {
          i += 3;
          seg_start = i;
        }
        _ =>
        {
          i += 2;
          seg_start = i;
        }
      }
    }
    else if bytes[i] == b'\r'
    {
      i += 1;
      seg_start = i;
    }
    else
    {
      i += 1;
    }
  }
  if seg_start < bytes.len()
  {
    if let Some(seg) = s.get(seg_start..bytes.len())
    {
      spans.push(Span::styled(seg.to_string(), style));
    }
  }
  spans
}

fn apply_sgr_seq(
  seq: &str,
  style: &mut Style,
)
{
  let nums: Vec<i32> =
    seq.split(';').filter_map(|t| t.parse::<i32>().ok()).collect();
  if nums.is_empty()
  {
    *style = Style::default();
    return;
  }
  let mut i = 0;
  while i < nums.len()
  {
    match nums[i]
    {
      0 =>
      {
        *style = Style::default();
      }
      1 =>
      {
        *style = style.add_modifier(Modifier::BOLD);
      }
      3 =>
      {
        *style = style.add_modifier(Modifier::ITALIC);
      }
      4 =>
      {
        *style = style.add_modifier(Modifier::UNDERLINED);
      }
      22 =>
      {
        *style = style.remove_modifier(Modifier::BOLD);
      }
      23 =>
      {
        *style = style.remove_modifier(Modifier::ITALIC);
      }
      24 =>
      {
        *style = style.remove_modifier(Modifier::UNDERLINED);
      }
      30..=37 =>
      {
        style.fg = Some(basic_color((nums[i] - 30) as u8, false));
      }
      90..=97 =>
      {
        style.fg = Some(basic_color((nums[i] - 90) as u8, true));
      }
      40..=47 =>
      {
        style.bg = Some(basic_color((nums[i] - 40) as u8, false));
      }
      100..=107 =>
      {
        style.bg = Some(basic_color((nums[i] - 100) as u8, true));
      }
      38 =>
      {
        if i + 1 < nums.len()
        {
          match nums[i + 1]
          {
            5 =>
            {
              if i + 2 < nums.len()
              {
                style.fg = Some(Color::Indexed(nums[i + 2] as u8));
                i += 2;
              }
            }
            2 =>
            {
              if i + 4 < nums.len()
              {
                style.fg = Some(Color::Rgb(
                  nums[i + 2] as u8,
                  nums[i + 3] as u8,
                  nums[i + 4] as u8,
                ));
                i += 4;
              }
            }
            _ =>
            {}
          }
        }
      }
      48 =>
      {
        if i + 1 < nums.len()
        {
          match nums[i + 1]
          {
            5 =>
            {
              if i + 2 < nums.len()
              {
                style.bg = Some(Color::Indexed(nums[i + 2] as u8));
                i += 2;
              }
            }
            2 =>
            {
              if i + 4 < nums.len()
              {
                style.bg = Some(Color::Rgb(
                  nums[i + 2] as u8,
                  nums[i + 3] as u8,
                  nums[i + 4] as u8,
                ));
                i += 4;
              }
            }
            _ =>
            {}
          }
        }
      }
      _ =>
      {}
    }
    i += 1;
  }
}

fn basic_color(
  code: u8,
  bright: bool,
) -> Color
{
  match (code, bright)
  {
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
