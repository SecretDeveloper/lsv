# lsv Configuration Guide

This document explains how to configure lsv via Lua, where the config file lives, and how to define key bindings that change lsv’s behavior or run external tools safely.

## Where the config lives

lsv loads a Lua config from the first location that exists:

1. `$LSV_CONFIG_DIR/init.lua`
2. `$XDG_CONFIG_HOME/lsv/init.lua`
3. `~/.config/lsv/init.lua`

The default configuration shipped with lsv is embedded (see `src/lua/defaults.lua`) and loads before your `init.lua`. Your config overlays and overrides any values from the defaults.

## The configuration shape

Top‑level API you’ll use:

- `lsv.config({ ... })` — set config values (icons, keys, ui, etc.).
- `lsv.set_previewer(function(ctx) ... end)` — return a shell command to render the preview for a file.
- `lsv.map_action(key, description, function(lsv, config) ... end)` — bind keys to Lua functions.

### Useful fields in `config`

- `config.ui.panes = { parent = 20, current = 30, preview = 50 }`
- `config.ui.show_hidden = true|false`
- `config.ui.date_format = "%Y-%m-%d %H:%M"`
- `config.ui.display_mode = "absolute" | "friendly"`
- `config.ui.preview_lines = 100`
- `config.ui.max_list_items = 5000`
- `config.ui.sort = "name" | "size" | "mtime"`
- `config.ui.sort_reverse = true|false`
- `config.ui.show = "none" | "size" | "created" | "modified"`
- `config.ui.row = { icon = "{icon} ", left = "{name}", middle = "", right = "{info}" }`
- `config.ui.row_widths = { icon = 0, left = 0, middle = 0, right = 0 }` (0 = auto)
- `config.ui.theme = { ... }` — colors (names or `#RRGGBB` hex)

Your `init.lua` only needs to set what you want to change — it overlays the defaults.

## Action helpers (available as `lsv` in map_action)

Inside an action function `function(lsv, config) ... end`, these helpers are available:

- `lsv.select_item(index)` — select the 0‑based list index.
- `lsv.select_last_item()` — select the last item in the current list.
- `lsv.quit()` — request the app to exit.
- `lsv.display_output(text, title?)` — show text in a bottom Output panel.
- `lsv.os_run(cmd)` — run a shell command and show the captured output in the Output panel (env: `LSV_PATH`, `LSV_DIR`, `LSV_NAME`).

The current context is available in `config.context`:

- `cwd` — current working directory
- `selected_index` — current selection index
- `current_len` — number of items in the current list

## Minimal example `~/.config/lsv/lua/init.lua`

```lua
-- Overlay a few UI settings
lsv.config({
  ui = {
    display_mode = "friendly",
    preview_lines = 80,
    row = { middle = "" },
    row_widths = { icon = 2, left = 40, right = 14 },
  },
})

-- Change default "ss" to also show sizes in the info column
lsv.map_action("ss", "Sort by size + show size", function(lsv, config)
  config.ui.sort = "size"
  config.ui.show = "size"
end)

-- Safe shell quoting helper
local function shquote(s)
  return "'" .. tostring(s):gsub("'", "'\''") .. "'"
end

-- Open a new tmux window in the current directory
lsv.map_action("t", "New tmux window here", function(lsv, config)
  local dir = (config.context and config.context.cwd) or "."
  lsv.os_run("tmux new-window -c " .. shquote(dir))
end)

-- Git status in the current directory
lsv.map_action("gs", "Git Status", function(lsv, config)
  local dir = (config.context and config.context.cwd) or "."
  lsv.os_run("git -C " .. shquote(dir) .. " status")
end)
```

## Previewer

The previewer receives a `ctx` table and should return a shell command string or `nil` to use the default head preview:

Fields in `ctx`:

- `path`, `directory`, `extension`
- `is_binary` (simple heuristic)
- `height`, `width`, `preview_x`, `preview_y` — pane geometry

Example previewer snippet (Markdown via glow; images via viu; text via bat):

```lua
lsv.set_previewer(function(ctx)
  if ctx.extension == "md" or ctx.extension == "markdown" then
    return "glow --style=dark --width=" .. tostring(ctx.width) .. " {path}"
  end
  if ctx.extension == "png" or ctx.extension == "jpg" or ctx.extension == "jpeg"
     or ctx.extension == "gif" or ctx.extension == "bmp" or ctx.extension == "tiff" then
    return "viu --width '{width}' --height '{height}' '{path}'"
  end
  if not ctx.is_binary then
    return "bat --color=always --style=numbers --paging=never --wrap=never --line-range=:120 {path}"
  end
  return nil
end)
```

## Overlays and panels

- `?` — which‑key overlay (grouped prefixes; multiple columns; 20% default height, expands as needed)
- `zm` — toggle Messages panel (error/info messages; 20% default height)
- `zo` — toggle Output panel (captured output from actions)
- `Esc` — hides any overlay

Overlays are mutually exclusive; opening one hides the others.

## Tips

- Actions are preferred over legacy strings. Bind everything with `lsv.map_action`.
- Use `lsv.os_run` with proper quoting (see `shquote`) to avoid breaking paths.
- Start with the defaults in `src/lua/defaults.lua` to understand all config values you can override.
- Consider using friendly display mode for dates/sizes: `config.ui.display_mode = "friendly"`.

If something doesn’t behave as expected, enable tracing:

```bash
LSV_TRACE=1 LSV_TRACE_FILE=/tmp/lsv-trace.log lsv
```
