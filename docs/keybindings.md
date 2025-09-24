# Default Keybindings

The table below mirrors the bindings installed by `src/lua/defaults.lua`. User configs can override or extend these via `lsv.map_action`.

| Keys | Description | Action |
|------|-------------|--------|
| `q` | Quit lsv | internal quit |
| `gg` | Select first item | select_item(0) |
| `G` | Select last item | select_last_item() |
| `sn` | Sort by name | set `config.ui.sort` = `"name"` |
| `ss` | Sort by size | set `config.ui.sort` = `"size"` |
| `sm` | Sort by modified time | set `config.ui.sort` = `"mtime"` |
| `sc` | Sort by created time | set `config.ui.sort` = `"created"` |
| `sr` | Toggle reverse sort | toggle `config.ui.sort_reverse` |
| `zh` | Toggle show hidden files | toggle `config.ui.show_hidden` |
| `zn` | Info column: none | set `config.ui.show` = `"none"` |
| `zs` | Info column: size | set `config.ui.show` = `"size"` |
| `zc` | Info column: created | set `config.ui.show` = `"created"` |
| `zf` | Friendly display (relative sizes/dates) | set `config.ui.display_mode` = `"friendly"` |
| `za` | Absolute display | set `config.ui.display_mode` = `"absolute"` |
| `zm` | Toggle messages panel | `messages = "toggle"` |
| `zo` | Toggle output panel | `output = "toggle"` |
| `?` | Show which-key overlay | built-in handler |
| `Up / k` | Move up one item | handled in Rust input loop |
| `Down / j` | Move down one item | handled in Rust input loop |
| `Left / Backspace / h` | Go to parent directory | handled in Rust input loop |
| `Right / Enter / l` | Enter directory / open | handled in Rust input loop |

## Notes

- Actions defined in defaults use the Lua helper functions (`lsv.select_item`, `lsv.os_run`, etc.). Use the [Configuration Reference](configuration.md) to see the full API.
- Some keys (arrows, `h/j/k/l`) are processed directly in Rust; remapping them requires changes in `src/input.rs`.
- The shipped defaults avoid destructive operations. To add create/delete features or custom scripts, map new keys in your own `init.lua`.
- On Windows, ensure the terminal supports the `?` which-key overlay (Windows Terminal recommended).

For a runtime view, press `?` while lsv is running to see the overlay sorted by prefix.
