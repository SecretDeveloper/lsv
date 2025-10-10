use std::time::{
  Duration,
  SystemTime,
};

#[test]
fn human_size_basic()
{
  use lsv::ui::format::human_size;
  assert_eq!(human_size(0), "0 B");
  assert_eq!(human_size(999), "999 B");
  assert_eq!(human_size(1024), "1.0 KB");
  assert_eq!(human_size(1536), "1.5 KB");
  assert_eq!(human_size(1024 * 1024), "1.0 MB");
}

#[test]
fn format_time_abs_has_expected_pattern()
{
  use lsv::ui::format::format_time_abs;
  let t = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
  let s = format_time_abs(t, "%Y-%m-%d %H:%M");
  // Very light sanity checks (avoid timezone brittleness):
  assert!(s.len() >= 10, "unexpected formatted length: {}", s);
  assert!(s.contains('-') && s.contains(':'), "unexpected format: {}", s);
}

#[test]
fn format_time_ago_buckets()
{
  use lsv::ui::format::format_time_ago;
  let now = SystemTime::now();
  // seconds
  let s = format_time_ago(now - Duration::from_secs(10));
  assert!(s.ends_with("s ago"), "got: {}", s);
  // minutes
  let s = format_time_ago(now - Duration::from_secs(120));
  assert!(s.ends_with("m ago"), "got: {}", s);
  // hours
  let s = format_time_ago(now - Duration::from_secs(7200));
  assert!(s.ends_with("h ago"), "got: {}", s);
  // days
  let s = format_time_ago(now - Duration::from_secs(172_800));
  assert!(s.ends_with("d ago"), "got: {}", s);
  // months (approx)
  let s = format_time_ago(now - Duration::from_secs(86_400 * 40));
  assert!(s.ends_with("mo ago"), "got: {}", s);
  // years (approx)
  let s = format_time_ago(now - Duration::from_secs(86_400 * 800));
  assert!(s.ends_with("y ago"), "got: {}", s);
}
