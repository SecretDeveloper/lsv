#[test]
fn commands_contains_expected_entries()
{
    let cmds = lsv::commands::all();
    // A handful of representative entries
    for expected in [
        "show_marks",
        "delete_marks",
        "search_text",
        "change_theme",
        "toggle_current_selected",
        "toggle_hidden_files",
        "sort_name",
        "view_friendly_units",
        "cd",
        "add_mark",
        "goto_mark",
    ]
    {
        assert!(cmds.iter().any(|c| c == &expected), "missing: {}", expected);
    }
}
