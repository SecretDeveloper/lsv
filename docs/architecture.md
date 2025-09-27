# Architecture Overview

This document outlines the high‑level structure of the lsv codebase and where key responsibilities live.

## Core Modules (UI‑agnostic)

- `src/core/fs_ops.rs`
  - `copy_path_recursive(src, dst)`: Recursively copy files/dirs.
  - `move_path_with_fallback(src, dst)`: Rename or copy+remove across devices.
  - `remove_path_all(path)`: Remove file/dir recursively.

- `src/core/listing.rs`
  - `read_dir_sorted(path, show_hidden, sort_key, sort_reverse) -> Vec<app::DirEntryInfo>`:
    Read and sort directory entries according to settings.

- `src/core/selection.rs`
  - `reselect_by_name(app, name)`: Reselect entry by name after resort.

- `src/core/overlays.rs`
  - `open_theme_picker(app)`, `apply_theme_entry(app, entry)`
  - `open_add_entry_prompt(app)`, `open_rename_entry_prompt(app)`
  - `request_delete_selected(app)`
  - `theme_picker_move(app, delta)`, `confirm_theme_picker(app)`

These modules own the application logic for filesystem operations, listing/sorting, selection behavior, and overlay state transitions. The `App` façade delegates to these functions.

## UI Layer

- `src/ui/template.rs`
  - `format_header_side(app, tpl)`: Renders the header (left/right) using placeholders like `{current_file}`, `{date}`, etc. Unknown placeholders are logged.

- `src/ui/panes.rs`, `src/ui/mod.rs`
  - Ratatui drawing code for panes, messages, output, prompts, confirms, theme picker, which‑key.

## Actions

- `src/actions/internal.rs`
  - Internal actions. Some (Quit, GoTop/GoBottom) return `ActionEffects` via `internal_effects`.
  - Others (sort/display toggles) still mutate via `execute_internal_action`.

- `src/actions/dispatcher.rs`
  - Parses action strings. Applies effects when available, otherwise calls internal executors.

- `src/actions/lua_glue.rs`
  - Bridges Lua and the app. Helpers grouped into builder functions:
    - UI helpers (overlays)
    - Selection/prompts
    - Clipboard
    - Process (quit/prompt/delete)

## Key Handling

- `src/keymap/mod.rs`
  - `tokenize_sequence(seq)`: Split sequences into tokens (e.g., `"<C-x>"`).
  - `build_token(ch, modifiers)`: Build token from key/modifiers.

## App

- `src/app.rs`
  - Holds `App` state (cwd, entries, selection, clipboard, overlays, config), and delegates to core modules for listing, FS ops, selection, and overlays.

## Lua Defaults + Config

- `src/lua/defaults.lua`: Default keybindings and UI settings.
- `src/config.rs`, `src/config_data.rs`: Load/merge configs, translate Lua tables to strong types.

## Rationale

- The `core` modules reduce `App` size and make logic testable and reusable.
- Grouped Lua helpers clarify the embedding surface and make future changes atomic.
- Centralized header templating keeps UI code lean and consistent.

## Future Work

- Continue moving overlay helpers fully out of `App` (remaining small methods), and consider extracting more selection/clipboard behaviors.
- Remove any remaining unused helpers and dead code as modules stabilize.

