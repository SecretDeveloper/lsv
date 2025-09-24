# Getting Started with lsv

This guide gets you from install to your first customised configuration.

## 1. Install

### From crates.io

```bash
cargo install lsv
```

### Build from source

```bash
git clone https://github.com/SecretDeveloper/lsv
cd lsv
cargo +nightly build --release
```

The project currently pins the nightly toolchain (see `rust-toolchain.toml`). Install it with:

```bash
rustup toolchain install nightly
```

## 2. First Run

Run `lsv` in a directory:

```bash
lsv /path/to/directory
```

Use arrow keys or `h/j/k/l` to navigate. Press `?` to view the which-key overlay of available shortcuts.

## 3. Copy the Default Config

Configuration lives in Lua. lsv loads the first file it finds:

1. `$LSV_CONFIG_DIR/init.lua`
2. `$XDG_CONFIG_HOME/lsv/init.lua`
3. `~/.config/lsv/init.lua`

Create the directory if it doesn’t exist and copy a starter config:

```bash
mkdir -p ~/.config/lsv
cp examples/config/init.lua ~/.config/lsv/init.lua
```

## 4. Minimal Customisation

Edit `~/.config/lsv/init.lua` to tweak UI or keybindings. Example:

```lua
lsv.config({
  ui = {
    display_mode = "friendly",
    preview_lines = 60,
    row_widths = { icon = 2, left = 36, right = 16 },
  },
})

lsv.map_action("gs", "Git Status", function(lsv, config)
  local dir = (config.context and config.context.cwd) or "."
  lsv.os_run("git -C " .. shquote(dir) .. " status")
end)
```

See the [Configuration Reference](configuration.md) for all available fields and helpers.

## 5. Enable Tracing (Optional)

If something acts up, enable verbose logging:

```bash
LSV_TRACE=1 LSV_TRACE_FILE=/tmp/lsv-trace.log lsv
```

The log records key actions, preview commands, and external tooling output. Useful when preview commands fail (particularly on Windows).

## 6. Platform Notes

### macOS / Linux

- Preview commands execute via `sh -lc`. Ensure tools like `bat`, `glow`, or `viu` are on your `PATH`.
- Panels default to ANSI colours; install Nerd Font for icons if you enable them.

### Windows

- Preview commands run through `cmd /C`. Use Windows-native equivalents (e.g., `bat.exe`, `glow.exe`).
- Install a terminal emulator with good ANSI support (Windows Terminal or similar).
- Lua configs still live under `%USERPROFILE%\.config\lsv\init.lua` unless you set `LSV_CONFIG_DIR`.

## Next Steps

- Learn the Lua APIs and helper functions in the [Configuration Reference](configuration.md).
- Browse the [Default Keybindings](keybindings.md) to familiarise yourself with shortcuts.
- Dive into [Troubleshooting](troubleshooting.md) for known issues and diagnostic tips.
