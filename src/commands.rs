// Central list of ":" command palette entries and helpers.

/// Return the full list of command palette entries.
pub fn all() -> &'static [&'static str]
{
    &[
        "add_item",
        "add_mark",
        "cd",
        "change_theme",
        "clear_selected",
        "delete_marks",
        "delete_selected",
        "goto_mark",
        "rename_selected",
        "reverse_sort",
        "search_next",
        "search_prev",
        "search_text",
        "show_marks",
        "sort_created_date",
        "sort_modified_date",
        "sort_name",
        "sort_size",
        "toggle_current_selected",
        "toggle_hidden_files",
        "toggle_messages",
        "toggle_output",
        "view_friendly_units",
        "view_precise_units",
    ]
}
