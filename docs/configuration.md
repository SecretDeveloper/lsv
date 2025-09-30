# Configuration Reference

This page documents everything you can configure in **lsv** via Lua. It complements the [Getting Started](getting_started.md) guide and the [Default Keybindings](keybindings.md) table.

## Config File Locations

lsv searches for `init.lua` in the following order:

1. `$LSV_CONFIG_DIR/init.lua`
2. `$XDG_CONFIG_HOME/lsv/init.lua`
3. Platform fallbacks:
   - Windows: `%LOCALAPPDATA%\lsv\init.lua`, then `%APPDATA%\lsv\init.lua`, then `%USERPROFILE%\.config\lsv\init.lua`
   - macOS/Linux: `~/.config/lsv/init.lua`
   - As a last resort, `./.config/lsv/init.lua`

## Lua API Overview

Three entry points are injected into the Lua runtime:

| Function | Purpose |
|----------|---------|
| `lsv.config({ ... })` | Override global configuration fields (UI, keys, icons…). |
| `lsv.map_action(keys, description, fn)` | Bind keys to a Lua function. The function can mutate the config table or call helpers. |
| `lsv.set_previewer(function(ctx) ... end)` | Provide a command to render the preview for the current file. Return `nil` to fall back to the built-in “head” preview. |
| `lsv.open_theme_picker()` | Show the interactive theme picker modal for the current session. |

### Action Helpers (`lsv` table)

Available inside `lsv.map_action` handlers:

| Helper | Description |
|--------|-------------|
| `lsv.select_item(index)` | Select the 0-based item. |
| `lsv.select_last_item()` | Select the last item in the current pane. |
| `lsv.quit()` | Request exit after the action completes. |
| `lsv.display_output(text, title?)` | Show text in the Output panel. |
| `lsv.os_run(cmd)` | Run `cmd` through the system shell (captured output). |
| `lsv.os_run_interactive(cmd)` | Suspend the TUI, run `cmd` attached to the terminal, and resume.

`config.context` exposes runtime information such as `cwd`, `path`, `selected_index`, `current_len`, `parent_dir`, and `name`.

## Configuration Schema

`lsv.config` accepts a nested table matching `src/config.rs`. The most useful fields are summarised here.

```lua
lsv.config({
  icons = { enabled = false, preset = nil, font = nil },
  keys  = { sequence_timeout_ms = 0 },
  ui    = {
    panes         = { parent = 20, current = 30, preview = 50 },
    show_hidden   = false,
    date_format   = "%Y-%m-%d %H:%M",
    display_mode  = "absolute",   -- or "friendly"
    -- preview_lines removed; the viewer uses pane height
    max_list_items = 5000,
    sort          = "name",
    sort_reverse  = false,
    show          = "none",       -- info column (size|created|modified …)
    row = {
      icon   = "{icon} ",
      left   = "{name}",
      middle = "",
      right  = "{info}",
    },
    row_widths = { icon = 0, left = 0, middle = 0, right = 0 },
    theme_path = "themes/dark.lua",  -- load from a Lua module (relative to config root)
    theme = {
      pane_bg = "#101114",
      border_fg = "gray",
      item_fg = "white",
      selected_item_fg = "black",
      selected_item_bg = "cyan",
      title_fg = "gray",
      info_fg = "gray",
      dir_fg = "cyan",
      hidden_fg = "darkgray",
      exec_fg = "green",
      -- *_bg options accept colour names or `#RRGGBB`; use `nil` for default.
    },
  },
})
```

Only provide the fields you want to override; omitted values inherit from the defaults embedded in the binary.

- Theme loading:
  - Prefer `ui.theme` with a module name string (resolved via `require()` under `<config>/lua`), e.g. `ui.theme = "themes.dark"`.
  - You can still inline a theme table: `ui.theme = { item_fg = "white", ... }`.
  - For backward compatibility, `ui.theme_path = "themes/dark.lua"` is supported and loads directly from the config root.
  - Any inline `ui.theme` table is merged on top of the loaded theme.

### Placeholders & Environment

When building commands (either in actions or previewers), you can substitute:

- Placeholders: `{path}`, `{directory}`, `{name}`, `{extension}`, `{width}`, `{height}`, `{preview_x}`, `{preview_y}`.
- Environment variables: `LSV_PATH`, `LSV_DIR`, `LSV_NAME` (set automatically before each command).

Use a quoting helper to avoid shell injection. Example:

```lua
local function shquote(s)
  return "'" .. tostring(s):gsub("'", "'\\''") .. "'"
end
```

## Icons

Enable glyph icons and keep the mapping in a separate Lua file:

1) Create `~/.config/lsv/lua/icons.lua` (or under your `LSV_CONFIG_DIR`):

```lua
return {
  md = "", rs = "", lua = "󰢱",
  png = "󰋩", jpg = "󰋩", json = "", toml = "",
}
```

2) Reference it from `init.lua` and enable icons:

```lua
local ext_icons = require("icons")
lsv.config({
  icons = {
    enabled = true,
    font = "Nerd",               -- set your terminal to a Nerd Font
    default_file = "",
    default_dir = "",
    mappings = require("icons"), -- combined extensions/folders table
  },
  ui = {
    row_widths = { icon = 2, left = 40, middle = 0, right = 14 },
  },
})
```

Notes:
- Keys in `icons.extensions` are matched case-insensitively by lowering at load time.
- If no mapping exists for a file, `default_file` is used; directories use `default_dir`.
- Ensure your terminal uses a Nerd Font to render these glyphs.

### Folder-specific icons

You can also assign icons to specific folder names (case-insensitive) via `mappings.folders` in the same module:

```lua
return {
  extensions = { md = "", rs = "" },
  folders    = { src = "󰅪", docs = "󰈙", tests = "󰙨" },
}
```

## Previewer Commands

`lsv.set_previewer(function(ctx) ... end)` receives:

```lua
ctx = {
  path = "/abs/path/to/file",
  directory = "/abs/path",
  name = "file.ext",
  extension = "ext",
  is_binary = true|false,
  width = 80, height = 24,
  preview_x = 40, preview_y = 0,
}
```

Return a shell command string or `nil`. Example with platform-specific fallbacks:

```lua
lsv.set_previewer(function(ctx)
  if ctx.extension == "md" then
    if package.config:sub(1,1) == "\\" then
      return "glow.exe --width=" .. tostring(ctx.width) .. " {path}"
    else
      return "glow --width=" .. tostring(ctx.width) .. " {path}"
    end
  end
  if ctx.extension == "png" or ctx.extension == "jpg" then
    return "viu --width '{width}' --height '{height}' '{path}'"
  end
  if not ctx.is_binary then
    return "bat --color=always --style=numbers --paging=never --wrap=never {path}"
  end
  return nil
end)
```

When tracing is enabled (`LSV_TRACE=1`), lsv logs the resolved command, working directory, exit code, and byte counts. On Windows the command is executed via `cmd /C`; on POSIX it uses `sh -lc`.

## Example: Custom Keybinding

```lua
lsv.map_action("gs", "Git Status", function(lsv, config)
  local dir = (config.context and config.context.cwd) or "."
  local quoted = "'" .. dir:gsub("'", "'\\''") .. "'"
  lsv.os_run("git -C " .. quoted .. " status")
end)
```

`lsv.os_run` captures stdout/stderr and displays it in the Output panel. Use `lsv.display_output` for purely textual messages.

## Context & Effects Returned from Actions

Inside your action function, mutate `config` (it will be merged into the live config) or return direct effect flags, e.g.:

```lua
return {
  messages = "show",            -- toggle Message overlay
  output = "toggle",            -- toggle Output overlay
  output_text = "Finished",     -- also sets the Output panel
  output_title = "My Action",
  quit = true,
  redraw = true,
}
```

See `src/actions/effects.rs` for the full list of flags parsed by the engine.

## Windows-Specific Tips

- Preview commands run under `cmd.exe`; ensure you install Windows builds of CLI tools (`bat.exe`, `glow.exe`, etc.) and they are on `PATH`.
- Paths are passed as UTF-8 strings; wrap them in quotes in Lua (`shquote`) to survive spaces.
- Install a terminal emulator that supports ANSI escape codes (Windows Terminal recommended). 
- If a preview command fails, enable tracing and inspect the `[preview]` logs for exit codes/errors.

## Additional Resources

- [Lua Integration](lua_integration.md) — deep dive into the Rust/Lua bridge and action flow.
- [Default Keybindings](keybindings.md) — full list of shipped shortcuts.
- [Troubleshooting](troubleshooting.md) — platform quirks and diagnostic steps.
