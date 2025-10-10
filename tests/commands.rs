#[test]
fn commands_contains_expected_entries()
{
  let cmds = lsv::commands::all();
  // A handful of representative entries
  for expected in [
    "marks",
    "delmark",
    "find",
    "theme",
    "select_toggle",
    "show_hidden_toggle",
    "sort name",
    "display friendly",
    "cd",
    "mark",
    "goto",
  ]
  {
    assert!(cmds.iter().any(|c| c == &expected), "missing: {}", expected);
  }
}
