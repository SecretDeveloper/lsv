// Central list of ":" command palette entries and helpers.

/// Return the full list of command palette entries.
/// Strings include subcommands/arguments for discoverability (e.g. "sort
/// name").
pub fn all() -> &'static [&'static str]
{
  &[
    "marks",
    "delmark",
    "find",
    "next",
    "prev",
    "messages",
    "output",
    "theme",
    "add",
    "rename",
    "delete",
    "select_toggle",
    "select_clear",
    "show_hidden_toggle",
    "sort name",
    "sort size",
    "sort mtime",
    "sort created",
    "sort_reverse_toggle",
    "display friendly",
    "display absolute",
    "cd",
    "mark",
    "goto",
  ]
}
