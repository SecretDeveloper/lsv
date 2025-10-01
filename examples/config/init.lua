--
-- About config.context passed to actions:
--   config.context.cwd                       -- current working directory
--   config.context.selected_index            -- selected row index (0-based)
--   config.context.current_len               -- number of entries in current pane
--   config.context.current_file              -- full path of highlighted item (or cwd)
--   config.context.current_file_dir          -- parent directory of highlighted item
--   config.context.current_file_name         -- basename of highlighted item
--   config.context.current_file_extension    -- extension (no dot) of highlighted item
--   config.context.current_file_ctime        -- creation time (formatted per ui.date_format)
--   config.context.current_file_mtime        -- modified time (formatted per ui.date_format)
--
-- Override a few UI defaults

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
		theme = require("themes/catppuccin"), -- or: theme = require("themes/catppuccin")
		confirm_delete = true,
	},
})

-- Helper used by previewer and actions below (OS-aware quoting)
local function shquote(s)
	return lsv.quote(tostring(s))
end
local OS = lsv.get_os_name() -- "windows", "macos", "linux", ...

-- Previewer: markdown via glow, images via viu, text via bat
lsv.set_previewer(function(ctx)
	if ctx.current_file_extension == "md" or ctx.current_file_extension == "markdown" then
		if OS == "windows" then
			return string.format(
				"glow --style=dark --line-numbers=true --width %d %s",
				ctx.preview_width - 2,
				shquote(ctx.current_file)
			)
		else
			return string.format(
				"head -n %d %s | glow --style=dark --line-numbers=true --width %d",
				ctx.preview_height,
				shquote(ctx.current_file),
				ctx.preview_width - 2
			)
		end
	elseif
		ctx.current_file_extension == "jpg"
		or ctx.current_file_extension == "jpeg"
		or ctx.current_file_extension == "png"
		or ctx.current_file_extension == "gif"
		or ctx.current_file_extension == "bmp"
		or ctx.current_file_extension == "tiff"
	then
		-- Force text/ANSI rendering inside TUI; disable kitty/iterm image protocol
		-- to avoid sequences that won't render within ratatui panels.
		-- You can remove --static if your terminal supports sixel and you add support later.
		return string.format(
			"VIU_NO_KITTY=1 viu --static --width %d --height %d %s",
			ctx.preview_width - 2,
			ctx.preview_height - 4,
			shquote(ctx.current_file)
		)
	elseif not ctx.is_binary then
		return string.format(
			"bat --color=always --style=numbers --paging=never --wrap=never --line-range=:%d %s",
			ctx.preview_height,
			shquote(ctx.current_file)
		)
	else
		-- Binary file: render a compact hex view with hexyl if available
		-- Show roughly 16 bytes per row times the available height
		local bytes = math.max(256, (ctx.preview_height - 4 or 20) * 16)
		return string.format("hexyl -n %d %s", bytes, shquote(ctx.current_file))
	end
end)

-- Override an action: make "ss" also show sizes in the info column
lsv.map_action("ss", "Sort by size + show size", function(lsv, config)
	config.ui.sort = "size"
	config.ui.show = "size"
end)

lsv.map_action("t", "New tmux window here", function(lsv, config)
	local dir = (config.context and config.context.cwd) or "."
	lsv.os_run_interactive(string.format("tmux new-window -c %s", lsv.quote(dir)))
end)

lsv.map_action("gs", "Git Status", function(lsv, config)
	local dir = (config.context and config.context.cwd) or "."
	lsv.os_run_(string.format("git -C %s status", lsv.quote(dir)))
end)

lsv.map_action("E", "Edit in $EDITOR (preview)", function(lsv, config)
	local path = (config.context and config.context.current_file) or "."
	local cmd = string.format("%s %s", "$EDITOR", lsv.quote(path))
	if OS == "windows" then
		cmd = string.format("bat --paging=always %s", lsv.quote(path))
	end
	lsv.os_run_interactive(cmd)
end)
lsv.map_action("e", "Edit in nvim", function(lsv, config)
	local path = (config.context and config.context.current_file) or "."
	lsv.os_run_interactive(string.format("$EDITOR %s", shquote(path)))
end)
lsv.map_action("i", "View file", function(lsv, config)
	local path = (config.context and config.context.current_file) or "."
	lsv.os_run_interactive(string.format("bat --paging=always %s", shquote(path)))
end)

-- Example: clear messages (Ctrl+m)
lsv.map_action("<C-m>", "Clear messages", function(lsv, config)
	lsv.clear_messages()
end)
