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

-- Helper used by previewer and actions below
local function shquote(s)
	return "'" .. tostring(s):gsub("'", "'\\''") .. "'"
end

-- Previewer: markdown via glow, images via viu, text via bat
lsv.set_previewer(function(ctx)
	if ctx.current_file_extension == "md" or ctx.current_file_extension == "markdown" then
		return string.format("glow --style=dark --width=%d %s", ctx.preview_width, shquote(ctx.current_file))
	elseif
		ctx.current_file_extension == "jpg"
		or ctx.current_file_extension == "jpeg"
		or ctx.current_file_extension == "png"
		or ctx.current_file_extension == "gif"
		or ctx.current_file_extension == "bmp"
		or ctx.current_file_extension == "tiff"
	then
		return string.format(
			"viu --width %d --height %d %s",
			ctx.preview_width,
			ctx.preview_height,
			shquote(ctx.current_file)
		)
	elseif not ctx.is_binary then
		return string.format(
			"bat --color=always --style=numbers --paging=never --wrap=never --line-range=:%d %s",
			ctx.preview_height,
			shquote(ctx.current_file)
		)
	else
		return string.format(
			"bat --color=always --style=numbers --paging=never --wrap=never --line-range=:%d %s",
			ctx.preview_height,
			shquote(ctx.current_file)
		)
	end
end)

-- Override an action: make "ss" also show sizes in the info column
lsv.map_action("ss", "Sort by size + show size", function(lsv, config)
	config.ui.sort = "size"
	config.ui.show = "size"
end)

lsv.map_action("t", "New tmux window here", function(lsv, config)
	local dir = (config.context and config.context.cwd) or "."
	lsv.os_run(string.format("tmux new-window -c %s", shquote(dir)))
end)

lsv.map_action("gs", "Git Status", function(lsv, config)
	local dir = (config.context and config.context.cwd) or "."
	lsv.os_run(string.format("git -C %s status", shquote(dir)))
end)

lsv.map_action("E", "Open in tmux pane", function(lsv, config)
	local path = (config.context and config.context.current_file) or "."
	lsv.os_run_interactive(string.format("&tmux split-window -h nvim %s", shquote(path)))
end)
lsv.map_action("e", "Edit in nvim", function(lsv, config)
	local path = (config.context and config.context.current_file) or "."
	lsv.os_run_interactive(string.format("$EDITOR %s", shquote(path)))
end)
lsv.map_action("i", "View file", function(lsv, config)
	local path = (config.context and config.context.current_file) or "."
	lsv.os_run_interactive(string.format("bat --paging=always %s", shquote(path)))
end)
