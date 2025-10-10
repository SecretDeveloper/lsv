# lsv — A Three‑Pane Terminal File Viewer

[![CI](https://github.com/SecretDeveloper/lsv/actions/workflows/ci.yml/badge.svg)](https://github.com/SecretDeveloper/lsv/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/lsv.svg)](https://crates.io/crates/lsv)
[![docs.rs](https://img.shields.io/docsrs/lsv)](https://docs.rs/lsv)
[![MIT licensed](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

![lsv (dark)](docs/lsv-dark.jpg)

lsv is a fast, curses‑based file viewer for the terminal. It presents three panes side by side:

- Parent: shows the contents of the parent directory of your current location.
- Current: shows the contents of the current directory with selection and navigation.
- Preview: shows a preview of the selected file (via a user‑defined preview command), or the entries of a selected directory.

The app is keyboard‑driven, configurable via Lua, and supports rich, ANSI‑colored previews from external tools (e.g., bat, glow).

## Capabilities

- Three‑pane navigation (parent/current/preview) with fast sorting and filtering
- Keyboard‑driven UX with multi‑key sequences and a which‑key overlay
- Lua configuration: themes, keymaps, actions, and a programmable previewer
- External command integration: captured output or fully interactive shells
- File operations: add/rename/delete; multi‑select with copy/move/paste
- Marks: save and jump to directories with single keystrokes
- Display modes: absolute vs. friendly sizes/dates; toggle hidden files
- Command palette (`:`) with suggestions and Tab‑completion
- Cross‑platform support: macOS, Linux, and Windows (see notes under Troubleshooting)

## Install

- From crates.io: `cargo install lsv`

See the [documentation overview](docs/README.md) for setup guides, configuration reference, keybindings, and troubleshooting tips.

### Bootstrap Configuration

Create a ready-to-edit config (init.lua, icons, and themes) in your user config directory with:

```
lsv --init-config
```

- Add `--yes` to skip the prompt and run non-interactively:

```
lsv --init-config --yes
```

The files are copied from the repository’s `examples/config` folder when available; otherwise, lsv writes an embedded copy bundled in the binary. Config is written to the first of `$LSV_CONFIG_DIR`, `$XDG_CONFIG_HOME/lsv`, or `~/.config/lsv` (Windows uses `%LOCALAPPDATA%\lsv` then `%APPDATA%\lsv`).

## Screenshots

![lsv (light)](docs/lsv-light.jpg)

![lsv Which‑Key](docs/lsv-whichkey.jpg)

![Theme Picker](docs/lsv-select-theme.png)

## Build & Run (from source)

- Requires the Rust nightly toolchain (repo pins via `rust-toolchain.toml`). Install with `rustup toolchain install nightly` if you don't have it yet.
- Components `rustfmt` and `clippy` are listed in `rust-toolchain.toml`; `rustup` installs them automatically when you run the commands below.

- Build: `cargo build`
- Run: `cargo run`
- Optional trace logging: `LSV_TRACE=1 LSV_TRACE_FILE=/tmp/lsv-trace.log cargo run` (Windows PowerShell: `$env:LSV_TRACE=1; $env:LSV_TRACE_FILE=$env:TEMP+'\\lsv-trace.log'; cargo run`)

## Git Hooks (format on commit)

To block commits that aren’t rustfmt‑clean, install the provided pre‑commit hook:

```
bash scripts/install-git-hooks.sh
```

This sets `core.hooksPath` to `.githooks` and ensures the hooks are executable.

Pre-commit runs:
- `cargo fmt --all -- --check` (fails commit if formatting is needed)
- `cargo clippy --all-targets --all-features -- -D warnings` (fails on lints)
- `cargo test --all-features --workspace` (fails on test failures)

Optional pre-push hook is also provided (clippy) but redundant if pre-commit passes.

Fix formatting with `cargo fmt --all`. Address clippy warnings and test failures locally. To bypass temporarily (not recommended):
- `git commit --no-verify`
- `git push --no-verify`

## Navigation (defaults)

- Up/Down or k/j: move selection in the Current pane
- Right or Enter: enter selected directory
- Left or Backspace: go to parent directory (reselect the dir you just left)
- q or Esc: quit
 - ?: toggle which‑key overlay (shows grouped keybindings)

## Configuration Overview

lsv loads a Lua config from the first of:

1. `$LSV_CONFIG_DIR/init.lua`
2. `$XDG_CONFIG_HOME/lsv/init.lua`
3. `~/.config/lsv/init.lua`

Quick start: run `lsv --init-config` to create `init.lua` plus example themes and icons.

Top‑level Lua API:

- `lsv.config({ ... })`: core settings (icons, keys, ui, etc.).
- `lsv.set_previewer(function(ctx) ... end)`: return a shell command to render preview.
- `lsv.map_action(key, description, function(lsv, config) ... end)`: bind keys to Lua functions.
- `lsv.quote(s)`: OS‑aware shell quoting for building safe command arguments.
- `lsv.get_os_name()`: returns a platform string (e.g., `windows`, `macos`, `linux`).

Action helper functions available on `lsv` inside actions:

- `lsv.select_item(index)`: set the current selection to `index` (0-based).
- `lsv.select_last_item()`: select the last item in the current list.
- `lsv.quit()`: request the app to exit.
- `lsv.display_output(text, title?)`: show text in a bottom Output panel.
- `lsv.os_run(cmd)`: run a shell command and show its captured output in the Output panel. Env includes `LSV_PATH`, `LSV_DIR`, `LSV_NAME`.

Context data passed to actions via `config.context`:

- `cwd`: current working directory.
- `selected_index`: current selection index (or a sentinel if none).
- `current_len`: number of items in the current list.
- `path`: absolute path of the selected entry (falls back to `cwd`).
- `parent_dir`: parent directory of the selected entry (falls back to `cwd`).
- `name`: file name (basename) of the selected entry, when available.

### Minimal Example: Bind an external tool

```lua
-- Sample lsv config — place in $HOME/.config/lsv/init.lua
lsv.config({
  config_version = 1,
  keys = { sequence_timeout_ms = 0 },

  ui = {
    panes = { parent = 20, current = 30, preview = 50 },
    show_hidden = true,
    date_format = "%Y-%m-%d %H:%M",
    display_mode = "absolute",   -- or "friendly" (affects both dates and sizes)
    -- Prefer module form via <config>/lua/themes/dark.lua
    theme = "themes.dark",           -- or: theme = require("themes.dark")

    -- Optional row layout: icon/left/middle/right with placeholders
    row = {
      icon = "{icon} ",
      left = "{name}",
      middle = "",
      right = "{info}",
    },
  },
})

-- Safe shell quote helper (OS-aware)
local function shquote(s)
  return lsv.quote(tostring(s))
end

-- Example: bind "gs" to git status of the current directory
lsv.map_action("gs", "Git Status", function(lsv, config)
  local dir = (config.context and config.context.cwd) or "."
  lsv.os_run("git -C " .. shquote(dir) .. " status")
end)

-- Previewer function (ctx):
-- ctx = {
--   current_file            = absolute file path (string)
--   current_file_dir        = parent directory (string)
--   current_file_name       = file name (string)
--   current_file_extension  = extension without dot (string, may be empty)
--   is_binary               = boolean (simple heuristic)
--   preview_height          = preview pane height (rows)
--   preview_width           = preview pane width (cols)
--   preview_x, preview_y    = top-left coordinates of preview pane
-- }
-- Return a shell command string to run, or nil to use the built‑in head preview.
lsv.set_previewer(function(ctx)
	-- Render Markdown with glow, respecting pane width
	if ctx.current_file_extension == "md" or ctx.current_file_extension == "markdown" then
		return string.format("glow --style=dark --width %d %s", ctx.preview_width, shquote(ctx.current_file))
	end

	if
		ctx.current_file_extension == "jpg"
		or ctx.current_file_extension == "jpeg"
		or ctx.current_file_extension == "png"
		or ctx.current_file_extension == "gif"
		or ctx.current_file_extension == "bmp"
		or ctx.current_file_extension == "tiff"
	then
		-- image preview using viu (needs installation)
		return string.format("viu --width %d --height %d %s", ctx.preview_width, ctx.preview_height, shquote(ctx.current_file))
	end
	-- For non-binary, colorize with bat (first 120 lines, no wrapping)
	if not ctx.is_binary then
		return string.format("bat --color=always --style=numbers --paging=never --wrap=never --line-range=:120 %s", shquote(ctx.current_file))
	end

	-- Fallback to default preview (first N lines)
	return nil
end)

```

### Full Example Config

Below is a fuller example (see `examples/config/init.lua` in the repo) showing icons, themed header, previewer rules, and custom actions:

```lua
-- About config.context passed to actions:
--   config.context.cwd, selected_index, current_len
--   config.context.current_file, current_file_dir, current_file_name
--   config.context.current_file_extension, current_file_ctime, current_file_mtime

lsv.config({
	icons = {
		enabled = true,
		font = "Nerd",
		default_file = "",
		default_dir = "",
		mappings = require("nerdfont-icons"),
	},
	ui = {
		display_mode = "friendly",
		row = { middle = "" },
		row_widths = { icon = 2, left = 40, right = 14 },
		header = {
			left = "{username|fg=cyan;style=bold}@{hostname|fg=cyan}:{cwd|fg=#ffd866}/{current_file_name|fg=#ffd866;style=bold}",
			right = "{current_file_size|fg=gray}  {owner|fg=gray}  {current_file_permissions|fg=gray}  {current_file_ctime|fg=gray}",
			fg = "gray",
			bg = "#181825",
		},
		theme = require("themes/catppuccin"),
		confirm_delete = true,
	},
})

local function shquote(s)
	return "'" .. tostring(s):gsub("'", "'\\''") .. "'"
end

lsv.set_previewer(function(ctx)
	if ctx.current_file_extension == "md" or ctx.current_file_extension == "markdown" then
		return string.format("glow --style=dark --line-numbers=true --width %d %s", ctx.preview_width - 2, shquote(ctx.current_file))
	elseif
		ctx.current_file_extension == "jpg" or ctx.current_file_extension == "jpeg" or
		ctx.current_file_extension == "png" or ctx.current_file_extension == "gif" or
		ctx.current_file_extension == "bmp" or ctx.current_file_extension == "tiff" then
		return string.format("VIU_NO_KITTY=1 viu --static --width %d --height %d %s", ctx.preview_width - 2, ctx.preview_height - 4, shquote(ctx.current_file))
	elseif not ctx.is_binary then
		return string.format("bat --color=always --style=numbers --paging=never --wrap=never --line-range=:%d %s", ctx.preview_height, shquote(ctx.current_file))
	else
		local bytes = math.max(256, (ctx.preview_height - 4) * 16)
		return string.format("hexyl -n %d %s", bytes, shquote(ctx.current_file))
	end
end)

lsv.map_action("ss", "Sort by size + show size", function(lsv, config)
	config.ui.sort = "size"
	config.ui.show = "size"
end)

lsv.map_action("gs", "Git Status", function(lsv, config)
	local dir = (config.context and config.context.cwd) or "."
	lsv.os_run(string.format("git -C %s status", shquote(dir)))
end)

lsv.map_action("e", "Edit in $EDITOR", function(lsv, config)
	local path = (config.context and config.context.current_file) or "."
	lsv.os_run_interactive(string.format("$EDITOR %s", shquote(path)))
end)

```

### Keybindings: Actions

- Bind with `lsv.map_action(key, description, function(lsv, config) ... end)`.
- Prefer mutating `config` (e.g., `config.ui.sort = "size"`) and using helpers like `lsv.select_item(...)`.

Default action bindings

- Sorting: `sn` (by name), `ss` (by size), `sr` (toggle reverse)
- Info field: `zn` (none), `zs` (size), `zc` (created)
- Display mode: `zf` (friendly), `za` (absolute)
- Navigation: `gg` (top), `G` (bottom)
- Overlays: `zm` (toggle messages), `zo` (toggle last output), `?` (which‑key)

Override example

```lua
-- Change the default for "ss" to also show sizes in the info column
lsv.map_action("ss", "Sort by size + show size", function(lsv, config)
  config.ui.sort = "size"
  config.ui.show = "size"
end)
```

### Which‑Key Overlay and Sequences

- Type `?` to toggle a bottom overlay listing available keys (uses descriptions).
- Composite sequences are supported (e.g., `ss`, `zc`). The overlay opens automatically when you type a registered prefix.
- Timeout: by default there is no timeout for multi‑key sequences (0).
  - To enable a timeout, set `keys.sequence_timeout_ms` in your Lua config:
    
    ```lua
    lsv.config({
      keys = { sequence_timeout_ms = 600 },  -- 600ms timeout for sequences
    })
    ```

### Row Layout (icon/left/right)

Configure row sections under `ui.row`:

- Templates accept placeholders `{icon}`, `{name}`, `{info}`.
- Right column is right‑aligned, left is left‑aligned.

### Rendering Modes and Formats

- Dates: `display:absolute` uses `ui.date_format` (default `%Y-%m-%d %H:%M`); `display:friendly` uses relative strings (e.g., `3d ago`).
- Sizes: `display:absolute` shows raw bytes with `B`; `display:friendly` uses human units (KB/MB/...).

### Command Environment & Helpers

- For commands launched via actions and the previewer, lsv sets these environment variables:
  - `LSV_PATH` (selected file), `LSV_DIR` (directory), `LSV_NAME` (basename)
- Use your Lua `ctx` values to compose commands (see examples above). If you need to inject values via environment expansion, you can also include `$LSV_PATH`, `$LSV_DIR`, `$LSV_NAME` (or `${LSV_*}`) in your string; lsv will expand those before invoking the shell.

Header and row template placeholders (e.g., `{cwd}`, `{current_file_size}`) are specific to UI templates, not shell commands.

### Preview Notes

- lsv captures the command’s output and renders ANSI colors (SGR). If your tool disables color when piped, add `--color=always` (bat) or set styles (glow). lsv sets `FORCE_COLOR=1` and `CLICOLOR_FORCE=1` for preview commands.
- Output is trimmed to fit the preview pane height; older `ui.preview_lines` has been removed.

## Tracing (debugging)

- Enable with `LSV_TRACE=1` (default log path: `$TMPDIR/lsv-trace.log`, `/tmp/lsv-trace.log`, or `%TEMP%\lsv-trace.log` on Windows).
- Override path with `LSV_TRACE_FILE=/path/to/log`.
- Logs include executed commands, exit codes, bytes written, and a snippet of preview output.
