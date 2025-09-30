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

- Type `:show` then press `Tab` → completes to `show_hidden_toggle`.
- Type `:sort` + space, then `Tab` → suggestions include `sort name`, `sort size`, `sort mtime`, `sort created`.
- Type `:display` + space, then `Tab` → suggestions include `display friendly`, `display absolute`.

## Built‑in Commands

The palette includes these built‑ins (names are case‑insensitive):

- `marks` — show saved marks
- `delmark <keys...>` — delete marks by key
- `find` — open find prompt; `next` and `prev` to navigate matches
- `messages` — toggle the messages panel
- `output` — toggle the output panel
- `theme` — open the theme picker
- `add` — add file/folder (end with `/` for a folder)
- `rename` — rename the selected entry (or batch rename selected items)
- `delete` — request delete of selected items (respects confirmation setting)
- `select_toggle` — toggle selection of current item
- `select_clear` — clear all selections
- `show_hidden_toggle` — toggle visibility of dotfiles
- `sort <name|size|mtime|created>` — change sort key
- `sort_reverse_toggle` — toggle reverse sort
- `display <absolute|friendly>` — change size/date rendering mode
- `cd <path>` — change directory

Notes

- Commands can be combined with `;` (semicolon). Example: `:sort size; display friendly`.
- Many of these actions are also available as keybindings and Lua actions.

## Tips

- Use the suggestions line to learn available commands and their arguments.
- For frequently used actions, consider binding a key via `lsv.map_action` in your Lua config.
