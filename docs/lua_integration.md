# Lua Configuration and Action Flow in lsv

This document explains how Lua configuration integrates with the Rust core. It covers what gets loaded at startup, how key bindings call Lua functions, how those functions communicate intent back to Rust, and how external commands and previewers are executed.

The diagrams use PlantUML (sequence diagrams). You can paste the code blocks into any PlantUML renderer to visualize them.

## Overview

- Defaults are embedded in the binary (`src/lua/defaults.lua`) and loaded first.
- User config (`$LSV_CONFIG_DIR/init.lua`, `$XDG_CONFIG_HOME/lsv/init.lua`, or `~/.config/lsv/init.lua`) is loaded next and overlays values.
  - You may place reusable Lua modules under `lua/` next to `init.lua` and load them via `require("module_name")`.
- Keys are bound via `lsv.map_action(key, description, function(lsv, config) ... end)`.
- At runtime, a key press resolves to either:
  - A Lua action function (recommended), or
  - An internal action (sorting, etc.).
- Lua actions receive:
  - `lsv`: helper functions that can request navigation, output display, interactive/captured command execution, or quit.
  - `config`: a table representing the current configuration and a `context` subtable with current state.
- Lua returns an updated `config` (optionally with flags like `output_text`, `messages`, `redraw`), and Rust applies the changes and repaints the UI.

## Startup Load Sequence

```plantuml
@startuml
actor User
participant "Rust (config::load_config)" as RC
participant "Lua VM" as LUA

User -> RC: Start lsv
RC -> LUA: init Lua VM
RC -> LUA: install lsv API (config, map_action, set_previewer, helpers)
RC -> LUA: load defaults.lua (embedded)
RC -> LUA: load user init.lua (if exists)
RC -> LUA: extract map_action bindings + previewer fn
RC -> RC: produce Config + Keymaps + (LuaEngine, previewer_key, action_keys)
User <- RC: App initialized
@enduml
```

Key points:
- Defaults never hard-code user-specific actions (like external tools). Those belong in user init.lua.
- User config overlays defaults (only provided fields are changed).

## Key Press to Action Function

```plantuml
@startuml
actor User
participant "Rust (input.rs)" as IN
participant "Rust (actions/lua_glue.rs)" as AA
participant "Lua VM" as LUA
IN -> AA: dispatch_action(key sequence)
AA -> AA: resolve to run_lua:index
AA -> LUA: build lsv helpers + config table (with context)
AA -> LUA: call action function(lsv, config)
LUA -> LUA: (mutate config, call lsv helpers)
LUA --> AA: return mutated config table
AA -> AA: parse effects (select, overlays, quit)
AA -> AA: apply effects + config overlay
AA -> IN: Signal redraw/full-redraw as needed
@enduml
```

Notes:
- `config.context` includes: `cwd`, `selected_index`, `current_len`.
- `lsv` helpers can set fields like `output_text`, `output_title`, `messages`, or `redraw` in the config table; Rust applies them.

## lsv Helper Functions

Helpers injected into Lua action functions:

- `lsv.select_item(index)`: select an item by index (0-based).
- `lsv.select_last_item()`: select the last item.
- `lsv.quit()`: set a quit flag.
- `lsv.display_output(text, title?)`: show text in Output panel.
- `lsv.os_run(cmd)`: run a command (captured) and display stdout+stderr in Output.
- `lsv.os_run_interactive(cmd)`: suspend TUI, run command attached to terminal, restore TUI, then request a full redraw.

### Captured Command (os_run)

```plantuml
@startuml
participant "Lua action" as LUA
participant "Rust (AA)" as AA
participant "sh (Command)" as SH
participant "Output Panel" as OP
LUA -> AA: lsv.os_run(cmd)
AA -> SH: spawn sh -lc cmd (captured)
SH --> AA: exit code + stdout/stderr
AA -> AA: set output_text/output_title in config
AA -> OP: display output
@enduml
```

- Use for non-interactive tools (git status, grep, etc.).
- Trace logs show cmd, cwd, env, exit code, and bytes of output when `LSV_TRACE=1`.

### Interactive Command (os_run_interactive)

```plantuml
@startuml
participant "Lua action" as LUA
participant "Rust (AA)" as AA
participant "TUI" as TUI
participant "sh (Command)" as SH
LUA -> AA: lsv.os_run_interactive(cmd)
AA -> TUI: disable_raw_mode + LeaveAlternateScreen
AA -> SH: sh -lc cmd (attached to terminal)
SH --> AA: exit code
AA -> TUI: enable_raw_mode + EnterAlternateScreen
AA -> AA: set redraw=true; if nonzero, set output_text/message
AA -> TUI: force full redraw
@enduml
```

- Use for editors and pagers (nvim, vi, less, tmux, etc.).
- Returning to TUI triggers a full rerender to avoid a blank screen.

## Previewer Flow

```plantuml
@startuml
participant "UI draw" as UI
participant "Previewer (Lua)" as PREV
participant "sh (Command)" as SH
UI -> UI: compute preview pane size
UI -> PREV: call preview fn with ctx
PREV --> UI: returns command or nil
UI -> SH: sh -lc cmd (captured)
SH --> UI: output
UI -> UI: cache by (path,width,height) to avoid reruns
@enduml
```

- The previewer is optional. If nil is returned, lsv falls back to a simple head preview.
- Output is cached by `(selected path, width, height)`. Selection/resize invalidates cache.

## Error and Message Handling

- Command errors:
  - Interactive: on non-zero exit, a short note is recorded and shown in the Output panel.
  - Captured: spawn errors are recorded as messages; stdout/stderr are shown in Output panel.
- Messages panel (toggle `zm`) shows recent messages (e.g., command failures), with rolling history.
- Overlays (which-key, messages, output) are mutually exclusive.

## Tracing (diagnostics)

Set `LSV_TRACE=1` to log to `/tmp/lsv-trace.log` (or `LSV_TRACE_FILE=/path/to/log`).
- Logs include:
  - Lua action calls: start, errors, and runtime in ms
  - os_run/os_run_interactive calls: cmd, cwd, env, exit code, output sizes
  - Preview commands: cmd, exit code, bytes out

This helps identify crashes or slow paths — especially when calling external tools or previewers.

## Security Notes

- lsv executes shell commands only as requested by your Lua actions or preview functions.
- Use quoting helpers in Lua (e.g., `shquote`) to avoid unintended shell expansion.
- Prefer read-only tools in previewers to avoid accidental modification of files.

---

If you have ideas or want to visualize additional flows, add new PlantUML snippets in this document.
