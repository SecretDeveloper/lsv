-- Override a few UI defaults
lsv.config({
	ui = {
		display_mode = "friendly",
		preview_lines = 80,
		row = { middle = "" },
		row_widths = { icon = 2, left = 40, right = 14 },
		theme = {
			pane_bg = "#0b0d10",
			border_fg = "cyan",
			item_fg = "white",
			selected_item_bg = "magenta",
			dir_fg = "cyan",
			exec_fg = "yellow",
		},
	},
})

-- Previewer: markdown via glow, images via viu, text via bat
lsv.set_previewer(function(ctx)
	if ctx.extension == "md" or ctx.extension == "markdown" then
		return "glow --style=dark --width=" .. tostring(ctx.width) .. " {path}"
	end
	if
		ctx.extension == "jpg"
		or ctx.extension == "jpeg"
		or ctx.extension == "png"
		or ctx.extension == "gif"
		or ctx.extension == "bmp"
		or ctx.extension == "tiff"
	then
		return "viu --width '{width}' --height '{height}' '{path}'"
	end
	if not ctx.is_binary then
		return "bat --color=always --style=numbers --paging=never --wrap=never --line-range=:120 {path}"
	end
	return nil
end)

-- Override an action: make "ss" also show sizes in the info column
lsv.map_action("ss", "Sort by size + show size", function(lsv, config)
	config.ui.sort = "size"
	config.ui.show = "size"
end)
local function shquote(s)
	return "'" .. tostring(s):gsub("'", "'\\''") .. "'"
end

lsv.map_action("t", "New tmux window here", function(lsv, config)
	local dir = (config.context and config.context.cwd) or "."
	lsv.os_run("tmux new-window -c " .. shquote(dir))
end)

lsv.map_action("gs", "Git Status", function(lsv, config)
	local dir = (config.context and config.context.cwd) or "."
	lsv.os_run("git -C " .. shquote(dir) .. " status")
end)

lsv.map_action("E", "Open in tmux pane", function(lsv, config)
  local path = (config.context and config.context.cwd) or "."
  lsv.os_run_interactive("&tmux split-window -h nvim " .. shquote(path))
end)
lsv.map_action("e", "Edit in nvim", function(lsv, config)
  local path = (config.context and config.context.cwd) or "."
  lsv.os_run_interactive("nvim " .. shquote(path))
end)
