// Centralized helpers to convert between enums and strings

#[inline]
pub(crate) fn sort_key_to_str(k: crate::actions::SortKey) -> &'static str {
  match k {
    crate::actions::SortKey::Name => "name",
    crate::actions::SortKey::Size => "size",
    crate::actions::SortKey::MTime => "mtime",
    crate::actions::SortKey::CTime => "created",
  }
}

pub(crate) fn sort_key_from_str(s: &str) -> Option<crate::actions::SortKey> {
  let low = s.to_ascii_lowercase();
  match low.as_str() {
    "name" | "n" => Some(crate::actions::SortKey::Name),
    "size" | "s" => Some(crate::actions::SortKey::Size),
    "mtime" | "modified" | "time" | "date" | "t" => Some(crate::actions::SortKey::MTime),
    "created" | "ctime" | "birth" | "c" => Some(crate::actions::SortKey::CTime),
    _ => None,
  }
}

#[inline]
pub(crate) fn info_mode_to_str(m: crate::app::InfoMode) -> Option<&'static str> {
  match m {
    crate::app::InfoMode::None => None,
    crate::app::InfoMode::Size => Some("size"),
    crate::app::InfoMode::Created => Some("created"),
    crate::app::InfoMode::Modified => Some("modified"),
  }
}

pub(crate) fn info_mode_from_str(s: &str) -> Option<crate::app::InfoMode> {
  let low = s.to_ascii_lowercase();
  match low.as_str() {
    "none" | "off" => Some(crate::app::InfoMode::None),
    "size" | "bytes" => Some(crate::app::InfoMode::Size),
    "created" | "ctime" | "birth" => Some(crate::app::InfoMode::Created),
    "modified" | "mtime" => Some(crate::app::InfoMode::Modified),
    _ => None,
  }
}

#[inline]
pub(crate) fn display_mode_to_str(d: crate::app::DisplayMode) -> &'static str {
  match d {
    crate::app::DisplayMode::Absolute => "absolute",
    crate::app::DisplayMode::Friendly => "friendly",
  }
}

pub(crate) fn display_mode_from_str(s: &str) -> Option<crate::app::DisplayMode> {
  let low = s.to_ascii_lowercase();
  match low.as_str() {
    "absolute" | "abs" => Some(crate::app::DisplayMode::Absolute),
    "friendly" | "ago" | "human" => Some(crate::app::DisplayMode::Friendly),
    _ => None,
  }
}
