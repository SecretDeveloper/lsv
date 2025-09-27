//! Small utility helpers shared across the codebase.

use std::{
  fs::File,
  io::{
    self,
    BufRead,
    BufReader,
  },
  path::Path,
};

/// Read at most `n` lines from the start of `path`.
pub fn read_file_head(
  path: &Path,
  n: usize,
) -> io::Result<Vec<String>>
{
  let file = File::open(path)?;
  let reader = BufReader::new(file);
  let mut lines = Vec::new();
  for (i, line) in reader.lines().enumerate()
  {
    if i >= n
    {
      break;
    }
    lines.push(line.unwrap_or_default());
  }
  Ok(lines)
}

/// Expand tabs, strip carriage returns, and replace control characters with
/// spaces.
pub fn sanitize_line(s: &str) -> String
{
  let mut out = String::with_capacity(s.len());
  for ch in s.chars()
  {
    match ch
    {
      '\t' => out.push_str("    "),
      '\r' =>
      {}
      c if c.is_control() => out.push(' '),
      c => out.push(c),
    }
  }
  out
}
