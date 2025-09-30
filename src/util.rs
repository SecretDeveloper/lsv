//! Small utility helpers shared across the codebase.

use std::{
  fs::File,
  io::{self, Read},
  path::Path,
};

/// Read up to `max_bytes` from the start of `path` and split into at most
/// `max_lines` lines. Uses lossy UTF-8 conversion to avoid panics and limits
/// memory usage for binary or very long single-line files.
pub fn read_file_head_safe(
  path: &Path,
  max_bytes: usize,
  max_lines: usize,
) -> io::Result<Vec<String>>
{
  let mut f = File::open(path)?;
  let mut buf = Vec::with_capacity(std::cmp::min(max_bytes, 1024 * 1024));
  let mut tmp = [0u8; 8192];
  let mut remaining = max_bytes;
  while remaining > 0
  {
    let want = std::cmp::min(remaining, tmp.len());
    let n = f.read(&mut tmp[..want])?;
    if n == 0
    {
      break;
    }
    buf.extend_from_slice(&tmp[..n]);
    remaining -= n;
  }
  let s = String::from_utf8_lossy(&buf).into_owned();
  let mut out = Vec::new();
  for line in s.split_terminator('\n')
  {
    out.push(line.to_string());
    if out.len() >= max_lines
    {
      break;
    }
  }
  Ok(out)
}

/// Heuristic binary detector: reads a small prefix and returns true if it
/// contains a NUL byte or is not valid UTF-8.
pub fn is_binary(path: &Path) -> bool
{
  if let Ok(mut f) = File::open(path)
  {
    let mut buf = [0u8; 4096];
    if let Ok(n) = Read::read(&mut f, &mut buf)
    {
      let slice = &buf[..n];
      if slice.contains(&0)
      {
        return true;
      }
      if std::str::from_utf8(slice).is_err()
      {
        return true;
      }
    }
  }
  false
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
