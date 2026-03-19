# Default Keybindings

The table below mirrors the default bindings installed by `src/config/defaults.rs`.
User configs can override or extend these via `lsv.map_action`.

| Keys | Description | Action |
|------|-------------|--------|
| `q` | Quit lsv | `quit` |
| `gg` | Go to top | `nav:top` |
| `G` | Go to bottom | `nav:bottom` |
| `h` | Go to parent directory | `nav:parent` |
| `j` | Move down | `nav:down` |
| `k` | Move up | `nav:up` |
| `l` | Enter directory | `nav:enter` |
| `m` | Set mark (prompt) | `marks:add_wait` |
| `'` | Jump to mark (then type letter) | `marks:goto_wait` |
| `sn` | Sort by name | `sort:name` |
| `ss` | Sort by size | `sort:size` |
| `sm` | Sort by modified time | `sort:mtime` |
| `sc` | Sort by created time | `sort:created` |
| `sr` | Toggle reverse sort | `sort:reverse:toggle` |
| `zh` | Toggle show hidden files | `cmd:toggle_hidden_files` |
| `zn` | Info column: none | `show:none` |
| `zs` | Info column: size | `show:size` |
| `zc` | Info column: created | `show:created` |
| `zf` | Friendly display (relative sizes/dates) | `display:friendly` |
| `za` | Absolute display | `display:absolute` |
| `zm` | Toggle messages panel | `cmd:toggle_messages` |
| `zo` | Toggle output panel | `cmd:toggle_output` |
| `Ut` | UI theme picker | `cmd:change_theme` |
| `/` | Find in current directory | `cmd:search_text` |
| `n` | Find next | `cmd:search_next` |
| `b` | Find previous | `cmd:search_prev` |
| `a` | Add file/folder | `cmd:add_item` |
| `r` | Rename selected | `cmd:rename_selected` |
| `D` | Delete selected | `cmd:delete_selected` |
| `Space` | Toggle selection | `cmd:toggle_current_selected` |
| `u` | Clear selection | `cmd:clear_selected` |
| `c` | Copy selected | `clipboard:copy` |
| `x` | Move selected | `clipboard:move` |
| `v` | Paste clipboard | `clipboard:paste` |
| `<Esc>` | Close overlays (also clears selection) | `overlay:close` |
| `:` | Command palette | built-in handler |
| `?` | Which-key overlay | built-in handler |

## Notes

- Default bindings are action strings; user config can override by mapping keys to Lua via `lsv.map_action`.
- Arrow keys / Enter / Backspace are still handled as built-in fallbacks in `src/input.rs`.
- The shipped defaults avoid destructive operations. To add create/delete features or custom scripts, map new keys in your own `init.lua`.
- On Windows, ensure the terminal supports the `?` which-key overlay (Windows Terminal recommended).

For a runtime view, press `?` while lsv is running to see the overlay sorted by prefix.
