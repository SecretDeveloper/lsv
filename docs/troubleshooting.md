# Troubleshooting

Common issues, diagnostics, and platform-specific tips for **lsv**.

## General Checklist

1. **Update to the latest build.** `cargo install lsv --force` will refresh from crates.io.
2. **Enable tracing.** Run `LSV_TRACE=1 LSV_TRACE_FILE=/tmp/lsv-trace.log lsv` and reproduce the issue. Inspect the log afterwards.
3. **Verify your Lua config.** Comment out recent changes or run with `LSV_CONFIG_DIR` pointing to an empty folder to rule out configuration errors.

## Preview Commands Not Working (Windows)

- lsv runs preview commands through `cmd.exe /C`. Ensure the command (`bat`, `glow`, `viu`, etc.) is available as a Windows executable and on your `PATH`.
- Check the trace log for entries like:
  
  ```text
  [preview] launching shell='cmd' cmd='bat {path}' cwd='C:\dir' file='...'
  [preview] exit_code=Some(1) success=false bytes_out=0
  ```
  
  A non-zero exit code indicates the command failed. Run the same command manually in `cmd.exe` to debug.
- For WSL-based tools, explicitly call `wsl.exe -- <command>` and ensure quoting is correct.

## Preview Shows Garbled Colours or Question Marks

- Ensure your terminal supports 24-bit colours and UTF-8. On Windows, use Windows Terminal or a modern emulator; on POSIX make sure `TERM` advertises colour support.
- If you enabled icons (`icons.enabled = true`), install a Nerd Font and configure your terminal to use it.

## Lua Errors on Startup

If lsv prints `config load error`, open the referenced log file and check:

- Syntax errors (`unexpected symbol near`) — fix in your `init.lua`.
- Missing modules (`module outside config root`) — Lua only loads modules from the `lua/` directory next to your config. Copy modules there or adjust `LSV_CONFIG_DIR`.

## Actions Not Running

- Press `?` to confirm your keybinding is recognised. If not, check your `lsv.map_action` call for typos.
- Use `lsv.display_output` inside the action to display debug messages.
- Inspect `lsv-trace.log` for `[lua]` or `[actions]` entries showing the action index and any errors.

## Messages/Output Panels Not Appearing

Remember overlays are mutually exclusive. If you programmatically open the Output panel (`lsv.display_output` or `output = "show"`), it hides the Messages panel. Toggle with `zm`/`zo` to verify.

## Getting More Help

- Enable `LSV_TRACE=1` and gather logs along with your `init.lua` when filing an issue.
- Check [Configuration Reference](configuration.md) for API usage and [Lua Integration](lua_integration.md) for deeper architectural notes.
