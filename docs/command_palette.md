# Command Palette (":" Prompt)

The command palette provides a quick, typed interface for actions that are not bound to keys or when you prefer an explicit command. Open it by pressing `:`. A single‑line prompt appears at the bottom; press `Esc` to close.

## Suggestions and Tab‑Completion

- Press `Tab` to show a suggestions line with matching commands.
- Completion behavior:
  - Exactly one match: the input completes to that command.
  - Multiple matches: the input extends to the longest common prefix.
  - Suggestions remain visible after Tab so you can see remaining options.
- Matching is prefix‑based and case‑insensitive (commands are executed in lowercase internally).

Example

- Type `:toggle` then press `Tab` → suggestions include `toggle_hidden_files`, `toggle_messages`, `toggle_output`.
- Type `:sort` then press `Tab` → suggestions include `sort_name`, `sort_size`, `sort_modified_date`, `sort_created_date`.
- Type `:view` then press `Tab` → suggestions include `view_friendly_units`, `view_precise_units`.

## Built‑in Commands

The palette includes these built‑ins (names are case‑insensitive):

- `show_marks` — show saved marks
- `delete_marks <keys...>` — delete marks by key
- `search_text` — open find prompt
- `search_next` — jump to next match
- `search_prev` — jump to previous match
- `toggle_messages` — toggle the messages panel
- `toggle_output` — toggle the output panel
- `change_theme` — open the theme picker
- `add_item` — add file/folder (end with `/` for a folder)
- `rename_selected` — rename the selected entry (or batch rename selected items)
- `delete_selected` — request delete of selected items (respects confirmation setting)
- `toggle_current_selected` — toggle selection of current item
- `clear_selected` — clear all selections
- `toggle_hidden_files` — toggle visibility of dotfiles
- `sort_name` / `sort_size` / `sort_modified_date` / `sort_created_date` — change sort key
- `reverse_sort` — toggle reverse sort
- `view_friendly_units` / `view_precise_units` — change size/date rendering mode
- `add_mark <key>` — set mark by key
- `goto_mark <key>` — jump to mark by key
- `cd <path>` — change directory

Notes

- Legacy names are still accepted as aliases (`:marks`, `:find`, `:messages`, etc.).
- Commands can be combined with `;` (semicolon). Example: `:sort_size; view_friendly_units`.
- Many of these actions are also available as keybindings and Lua actions.

## Tips

- Use the suggestions line to learn available commands and their arguments.
- For frequently used actions, consider binding a key via `lsv.map_action` in your Lua config.
