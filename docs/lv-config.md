# lv Configuration (Lua)

## Overview
- Entry: `${XDG_CONFIG_HOME}/lv/lua/init.lua` (or `~/.config/lv/lua/init.lua`).
- Format: Lua, returning a table via `lv.config({...})`.
- Namespace: all settings live under the `lv` prefix (e.g., `lv.ui.show_hidden`).
- Safety: sandboxed `mlua`; `require()` is allowed but restricted to `~/.config/lv/lua/**`. No C modules, no `io`/`os`.
 - Versioning: declare `lv.config_version = 1` to enable future-safe changes.

## Loading & Reloading
- Discovery: use `${XDG_CONFIG_HOME}/lv/lua/init.lua`; fallback to `~/.config/lv/lua/init.lua`.
- Module root: `${config_dir}/lua` is the only require search path. Dotted names map to files (`require("plugins.foo")` → `${config_dir}/lua/plugins/foo.lua`).
- On startup: load config, validate, apply defaults.
- Reload: action `reload` rebuilds the Lua state, clears module cache, re-runs `init.lua`. Errors are shown non‑blocking.

## Layout Example
```
~/.config/lv/
  lua/
    init.lua
    theme.lua
    config/
      ui.lua
      keymaps.lua
    plugins/
      open_with.lua
      mytool/
        init.lua
```

## Settings Schema (prefix `lv`)
- `lv.config_version`
  - Integer version of the config schema. Current: `1`.
- `lv.theme`
  - `primary`: color for titles/borders. String name, hex ("#rrggbb"), or `{r,g,b}`.
  - `gray`: secondary text color.
  - `highlight_fg`: selected row color; `styles` may add `bold`, `italic`.
  - `preview_title`, `file`, `dir`: colors for specific elements.
  - `styles`: `{ title_bold=true, highlight_bold=true }`.
  - `symbols`: `{ highlight="▶ ", dir_suffix="/" }`.
- `lv.icons`
  - Controls use of icon glyphs (e.g., Nerd Font). The app cannot change your terminal font; set your terminal to a Nerd Font and enable icons here.
  - `enabled`: boolean; draw icons in lists.
  - `preset`: `"nerd" | "ascii"` (nerd uses Unicode glyphs; ascii uses simple symbols).
  - `font`: optional string (e.g., `"JetBrainsMono Nerd Font"`) used to choose glyph set; informational only.
  - `filetypes`: optional map of extension → icon (e.g., `{ rs = "", md = "" }`). Fallbacks: `file`, `dir`, `symlink`.
- `lv.ui`
  - `panes`: `{ parent=30, current=40, preview=30 }` percentages.
  - `show_hidden`: boolean; include dotfiles.
  - `sort`: `{ dirs_first=true, case_insensitive=true }`.
  - `preview_lines`: integer max lines to preview files/dirs.
  - `follow_symlinks`: boolean; list/preview follows symlinks.
  - `binary_preview`: `"none" | "hex" | "strings"`.
- `lv.behavior`
  - `open_on_enter`: `"enter_dir" | "open_file" | "smart"`.
  - `error_overlay`: boolean; show non‑blocking error bar.
- `lv.keys`
  - `sequence_timeout_ms`: integer (default 600). If a prefix is waiting, run only when a full sequence matches or timeout elapses.
- `lv.mapkey(sequence, action, description?)`
  - Register a key or sequence to an action with an optional human-readable description used by which‑key overlays and `:help`.
  - Actions: `up, down, page_up, page_down, enter_dir, parent_dir, quit, reload, command_mode, run_command:<name>, sort:name, sort:size, sort:created`, etc.
  - Sequence grammar: single keys (`"k"`, `"Enter"`, `"Left"`), modifiers (`"<C-r>"`, `"<A-x>"`), or composites like `"ss"`, `"sn"`, `"sc"`.
  - Key names: `"Up","Down","Left","Right","Enter","Backspace","Esc","Tab"`, letters (`"a".."z"`), digits, and modifiers in angle brackets.
  - Command mode: map `":"` to `command_mode` to open the `:` prompt.
  - If `description` is omitted, the action name is shown; you can also override via `which_key.labels`.
- `lv.commands`
  - Named external commands bound to selection.
  - Command spec: `{ cmd, args, cwd, when, interactive, env, confirm }`.
    - `cmd`: string or array; `args`: array or string.
    - `when`: `"file" | "dir" | "any"`.
    - `cwd`: `"selection" | "cwd" | "root"`.
    - `interactive`: bool; if true, suspend TUI; else capture output to a popup/preview.
    - `env`: table of env vars; supports templating.
    - `confirm`: string prompt before run.
  - Templates in `cmd/args/env`: `{path}`, `{name}`, `{dir}`, `{cwd}`, `{home}`.

## Command Mode (`:`)
- Purpose: run configured commands interactively by name with tab completion.
- Enter: press the key bound to `command_mode` (commonly `":"`). A prompt appears at the bottom: `:`.
- Syntax:
  - `run <name>` executes the configured command `lv.commands.<name>` on the current selection.
  - Abbreviation: if no ambiguity, you may type `<name>` directly (e.g., `open_with_editor`).
- Completion: Tab cycles through command names from `lv.commands` (respects `when = file|dir|any` for the current selection). Shift-Tab reverses.
- Execution:
  - Same semantics as keybound commands: interactive commands suspend TUI; non‑interactive output is shown in a popup/preview.
  - Errors are shown in the error overlay; exit status is reported.
- Optional (future): history with Up/Down; `:help` to list available commands.

## Example: `~/.config/lv/lua/init.lua`
```lua
-- ~/.config/lv/lua/init.lua
local theme = require("theme")
local keys = require("config.keymaps")
local extra_cmds = require("plugins.open_with")
-- Apply settings
lv.config({
  config_version = 1,
  theme = theme or {
    primary = "cyan",
    gray = "#888888",
    highlight_fg = "cyan",
    preview_title = "#999999",
    file = "white",
    dir = "yellow",
    styles = { title_bold = true, highlight_bold = true },
    symbols = { highlight = "▶ ", dir_suffix = "/" },
  },
  icons = { enabled = true, preset = "nerd", font = "JetBrainsMono Nerd Font" },
  ui = {
    panes = { parent = 30, current = 40, preview = 30 },
    show_hidden = false,
    sort = { dirs_first = true, case_insensitive = true },
    preview_lines = 120,
    follow_symlinks = true,
    binary_preview = "strings",
  },
  behavior = { open_on_enter = "smart", error_overlay = true },
  keys = { sequence_timeout_ms = 600 },
  commands = {
    open_with_editor = { cmd = "nvim", args = {"{path}"}, when = "any", interactive = true, cwd = "cwd" },
    open_in_finder = { cmd = "open", args = {"-R","{path}"}, when = "any", interactive = false },
    convert_to_pdf = {
      cmd = "/usr/local/bin/convert.sh",
      args = {"--in","{path}","--out","{dir}/{name}.pdf"},
      when = "file", interactive = false, confirm = "Convert file to PDF?" 
    },
  },
})

-- Key mappings (single keys and sequences)
lv.mapkey("k", "up", "Move up")
lv.mapkey("j", "down", "Move down")
lv.mapkey("Up", "up")
lv.mapkey("Down", "down")
lv.mapkey("Left", "parent_dir", "Go to parent directory")
lv.mapkey("Right", "enter_dir", "Enter directory")
lv.mapkey(":", "command_mode", "Open command mode")
lv.mapkey("<C-r>", "reload", "Reload configuration")
lv.mapkey("e", "run_command:open_with_editor", "Open with editor")
lv.mapkey("o", "run_command:open_in_finder", "Reveal in Finder")
-- Composite sort sequences
lv.mapkey("ss", "sort:size")
lv.mapkey("sn", "sort:name")
lv.mapkey("sc", "sort:created")
```

### Example: `~/.config/lv/lua/plugins/open_with.lua`
```lua
return {
  open_with_preview = {
    cmd = "bat",
    args = {"--style","plain","{path}"},
    when = "file", interactive = false
  }
}
```

## Key Sequences & Which‑Key
- Purpose: support multi-key prefixes (e.g., `s s` for sort by size, `s n` by name, `s c` by created). A which‑key overlay shows available continuations after a prefix.
- Settings (under `lv`):
  - `keys.sequence_timeout_ms`: integer (default 600). If no next key within timeout, the sequence cancels.
  - `which_key`: `{ enabled=true, delay_ms=250, position="bottom"|"top"|"overlay", max_height=8, border=true }`.
  - `which_key.labels`: optional map of prefixes/actions to user-friendly labels (e.g., `{ s="Sort", ["s s"]="Sort by Size" }`).
- Binding sequences in `lv.keys`:
  - Use `lv.mapkey("ss", "sort:size")`, `lv.mapkey("sn", "sort:name")`, `lv.mapkey("sc", "sort:created")`.
  - Prefix behavior: a single-key binding that is also a prefix defers until timeout; if no continuation, the single-key action runs.

### Example bindings
```lua
-- in ~/.config/lv/lua/init.lua
lv.config({
  keys = { sequence_timeout_ms = 600 },
  which_key = {
    enabled = true,
    delay_ms = 250,
    position = "bottom",
  },
})

lv.mapkey("sn", "sort:name", "Sort by Name")
lv.mapkey("ss", "sort:size", "Sort by Size")
lv.mapkey("sc", "sort:created", "Sort by Created date")
```

### Behavior
- After pressing a prefix (e.g., `s`), the which‑key overlay lists valid next keys with labels.
- If the next key completes a unique binding, that action runs; otherwise continue or cancel with `Esc`/timeout.
- Unknown sequences do nothing and show a brief hint.

## Implementation Checklist
- [X] Lua runtime: `mlua` sandboxed, `lv.config` API
- [X] Config discovery 
- [ ] Schema + defaults (theme, ui, behavior, keys, commands)
- [ ] Validation + color parsing (names, hex, rgb)
- [ ] Key binding parser → action dispatch
- [ ] Theme wiring to ratatui (colors, styles, symbols)
- [ ] External command runner (templates, env, cwd)
- [ ] TUI suspend/resume for interactive commands
- [ ] Reload action + non‑blocking error overlay
- [ ] Docs and sample `init.lua` under `docs/`
