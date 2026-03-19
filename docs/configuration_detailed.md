# Detailed Configuration Guide

This guide documents the current Lua configuration and action API exposed by `lsv`.

## Config File Discovery

`lsv` loads the first matching `init.lua` from:

1. `$LSV_CONFIG_DIR/init.lua`
2. `$XDG_CONFIG_HOME/lsv/init.lua`
3. `~/.config/lsv/init.lua`

Bootstrap a starter config with:

```bash
lsv --init-config
```

## Top-Level Lua API

Available in `init.lua`:

- `lsv.config(table)`
- `lsv.set_previewer(function(ctx) ... end)`
- `lsv.map_action(key_or_list, description, fn)`
- `lsv.mapkey(sequence, action, description?)`
- `lsv.quote(string)`
- `lsv.get_os_name()`
- `lsv.getenv(name, default?)`
- `lsv.trace(text)`

### `lsv.map_action` key binding forms

```lua
lsv.map_action("gs", "Git status", function(lsv, config)
  lsv.os_run("git status")
end)

lsv.map_action({ "gs", "gS" }, "Git status", function(lsv, config)
  lsv.os_run("git status")
end)
```

## `lsv.config({...})` Fields

### `config_version`

- `number`
- Current examples use `1`.

### `keys`

- `keys.sequence_timeout_ms`: number (0 disables timeout)

### `icons`

- `icons.enabled`: boolean
- `icons.preset`: string
- `icons.font`: string
- `icons.default_file`: string
- `icons.default_dir`: string
- `icons.extensions`: table (extension or grouped extensions -> icon)
- `icons.folders`: table (folder name -> icon)
- `icons.mappings.extensions`: table (same mapping shape)
- `icons.mappings.folders`: table (same mapping shape)

Compatibility aliases also accepted:

- `icons.by_ext`
- `icons.by_name`

### `ui`

- `ui.panes.parent`, `ui.panes.current`, `ui.panes.preview`: `u16` percentages
- `ui.show_hidden`: boolean
- `ui.max_list_items`: number
- `ui.date_format`: string (`strftime`-like)
- `ui.display_mode`: string (`"absolute"` or `"friendly"`)
- `ui.sort`: string (`"name"`, `"size"`, etc.)
- `ui.sort_reverse`: boolean
- `ui.show`: info field mode string (`"none"`, `"size"`, `"created"`, etc.)
- `ui.confirm_delete`: boolean

Header and row:

- `ui.header.left`, `ui.header.right`, `ui.header.fg`, `ui.header.bg`
- `ui.header_fg`, `ui.header_bg` (top-level ui aliases)
- `ui.row.icon`, `ui.row.left`, `ui.row.middle`, `ui.row.right`
- `ui.row_widths.icon`, `ui.row_widths.left`, `ui.row_widths.middle`, `ui.row_widths.right`

Theme loading:

- `ui.theme = <table>`
- `ui.theme = "themes.dark"` (Lua module via `require`)
- `ui.theme_path = "/abs/or/relative/path.lua"`

Modals:

- `ui.modals.prompt.width_pct`, `ui.modals.prompt.height_pct`
- `ui.modals.confirm.width_pct`, `ui.modals.confirm.height_pct`
- `ui.modals.theme.width_pct`, `ui.modals.theme.height_pct`

### `actions`

You can declare actions in `lsv.config({ actions = {...} })` as either:

- Lua function actions (`fn = function(lsv, config) ... end`)
- String actions (`action = "cmd:toggle_messages"`, etc.)

Example:

```lua
lsv.config({
  actions = {
    {
      keymap = "gs",
      description = "Git status",
      fn = function(lsv, config)
        lsv.os_run("git status")
      end,
    },
    {
      keymap = "zm",
      description = "Toggle messages",
      action = "cmd:toggle_messages",
    },
  }
})
```

## Action Runtime API (`fn(lsv, config)`)

Inside action callbacks, `lsv` exposes:

Selection and navigation helpers:

- `lsv.select_item(index)`
- `lsv.select_last_item()`
- `lsv.get_selected_paths()`

Clipboard and file operation helpers:

- `lsv.copy_selection()`
- `lsv.move_selection()`
- `lsv.paste_clipboard()`
- `lsv.clear_clipboard()`
- `lsv.delete_selected()`

UI and messaging helpers:

- `lsv.display_output(text, title?)`
- `lsv.show_message(text)`
- `lsv.show_error(text)`
- `lsv.clear_messages()`
- `lsv.force_redraw()`
- `lsv.set_theme_by_name(name)`
- `lsv.quit()`

Process helpers:

- `lsv.os_run(cmd)`
- `lsv.os_run_interactive(cmd)`

General helpers:

- `lsv.quote(s)`
- `lsv.get_os_name()`
- `lsv.getenv(name, default?)`
- `lsv.trace(text)`

## Action `config.context`

`config.context` includes:

- `cwd`
- `selected_index` (`u64::MAX` sentinel when no selection)
- `current_len`
- `current_file`
- `current_file_dir`
- `current_file_name`
- `current_file_extension`
- `current_file_ctime` (if available)
- `current_file_mtime` (if available)

## Previewer API

Register once:

```lua
lsv.set_previewer(function(ctx)
  -- return shell command string or nil
end)
```

### `ctx` fields

- `current_file`
- `current_file_dir`
- `current_file_name`
- `current_file_extension`
- `is_binary`
- `preview_width`
- `preview_height`
- `preview_x`
- `preview_y`

`preview_width` and `preview_height` are the inner drawable pane area (content area), not including borders.

### Preview behavior notes

- Commands run via `sh -lc` (POSIX) or `cmd /C` (Windows).
- `lsv` captures preview command output and renders text + ANSI SGR colors.
- `FORCE_COLOR=1` and `CLICOLOR_FORCE=1` are set for preview commands.
- Pixel image protocols are not rendered in the pane. Use text/block output for image tools.

Example default `viu` preview command:

```lua
return string.format(
  "viu --width %d --height %d %s",
  ctx.preview_width,
  ctx.preview_height,
  lsv.quote(ctx.current_file)
)
```

Optional workaround for terminals/setups where `viu` image preview does not show (for example, some WezTerm configurations):

```lua
return string.format(
  "VIU_NO_KITTY=1 viu --blocks --static --width %d --height %d %s",
  ctx.preview_width,
  ctx.preview_height,
  lsv.quote(ctx.current_file)
)
```

## Action Effects (Return Table)

In addition to mutating `config`, actions may return effect fields. Common fields:

- `messages = "toggle" | "show" | "hide"`
- `output = "toggle" | "show" | "hide"`
- `output_text`, `output_title`
- `message_text`, `error_text`
- `redraw = true`
- `quit = true`
- `prompt = "add" | "new" | "rename"`
- `confirm = "delete" | "delete_selected"`
- `select = "toggle" | "clear"`
- `clipboard = "copy_arm" | "move_arm" | "paste" | "clear"`
- `find = "open" | "next" | "prev"`
- `marks = "add_wait" | "goto_wait"`
- `theme_picker = "open"`
- `theme_set_name = "..."`
- `clear_messages = true`
- `preview_run_cmd = "..."`
- `select_paths = { "/path/a", "/path/b" }`

## Complete Example

```lua
lsv.config({
  config_version = 1,
  keys = { sequence_timeout_ms = 600 },
  ui = {
    panes = { parent = 20, current = 30, preview = 50 },
    show_hidden = true,
    display_mode = "friendly",
    date_format = "%Y-%m-%d %H:%M",
    theme = "themes.dark",
    row = { icon = "{icon} ", left = "{name}", middle = "", right = "{info}" },
    row_widths = { icon = 2, left = 40, middle = 0, right = 14 },
  },
})

lsv.map_action({ "gs", "gS" }, "Git status", function(lsv, config)
  local cwd = (config.context and config.context.cwd) or "."
  lsv.os_run(string.format("git -C %s status", lsv.quote(cwd)))
end)

lsv.set_previewer(function(ctx)
  local ext = ctx.current_file_extension
  if ext == "png" or ext == "jpg" or ext == "jpeg" or ext == "gif" then
    return string.format(
      "viu --width %d --height %d %s",
      ctx.preview_width,
      ctx.preview_height,
      lsv.quote(ctx.current_file)
    )
  end
  if not ctx.is_binary then
    return string.format(
      "bat --color=always --style=numbers --paging=never --wrap=never --line-range=:%d %s",
      ctx.preview_height,
      lsv.quote(ctx.current_file)
    )
  end
  return nil
end)
```
