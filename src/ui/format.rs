use std::time::SystemTime;

pub fn human_size(bytes: u64) -> String
{
  const UNITS: [&str; 7] = ["B", "KB", "MB", "GB", "TB", "PB", "EB"];
  let mut val = bytes as f64;
  let mut idx = 0usize;
  while val >= 1024.0 && idx + 1 < UNITS.len()
  {
    val /= 1024.0;
    idx += 1;
  }
  if idx == 0
  {
    format!("{} {}", bytes, UNITS[idx])
  }
  else
  {
    format!("{:.1} {}", val, UNITS[idx])
  }
}

pub fn format_time_abs(
  t: SystemTime,
  fmt: &str,
) -> String
{
  use chrono::{
    DateTime,
    Local,
  };
  let dt: DateTime<Local> = DateTime::from(t);
  dt.format(fmt).to_string()
}

pub fn format_time_ago(t: SystemTime) -> String
{
  let now = SystemTime::now();
  match now.duration_since(t)
  {
    Ok(d) =>
    {
      let secs = d.as_secs();
      if secs < 60
      {
        format!("{}s ago", secs)
      }
      else if secs < 3600
      {
        format!("{}m ago", secs / 60)
      }
      else if secs < 86400
      {
        format!("{}h ago", secs / 3600)
      }
      else if secs < 86400 * 30
      {
        format!("{}d ago", secs / 86400)
      }
      else if secs < 86400 * 365
      {
        format!("{}mo ago", secs / (86400 * 30))
      }
      else
      {
        format!("{}y ago", secs / (86400 * 365))
      }
    }
    Err(_) => "just now".to_string(),
  }
}
