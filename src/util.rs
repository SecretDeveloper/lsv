use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::Path;

pub(crate) fn read_file_head(path: &Path, n: usize) -> io::Result<Vec<String>> {
  let file = File::open(path)?;
  let reader = BufReader::new(file);
  let mut lines = Vec::new();
  for (i, line) in reader.lines().enumerate() {
    if i >= n { break; }
    lines.push(line.unwrap_or_default());
  }
  Ok(lines)
}

pub(crate) fn sanitize_line(s: &str) -> String {
  let mut out = String::with_capacity(s.len());
  for ch in s.chars() {
    match ch {
      '\t' => out.push_str("    "),
      '\r' => {}
      c if c.is_control() => out.push(' '),
      c => out.push(c),
    }
  }
  out
}

pub(crate) fn shell_escape(s: &str) -> String {
  if s.is_empty() {
    "''".to_string()
  } else {
    let mut out = String::from("'");
    for ch in s.chars() {
      if ch == '\'' {
        out.push_str("'\\''");
      } else {
        out.push(ch);
      }
    }
    out.push('\'');
    out
  }
}
