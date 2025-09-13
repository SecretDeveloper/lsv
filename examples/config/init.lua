-- Values-only configuration (good Rust defaults are assumed)
lsv.config({
	config_version = 1,
	keys = { sequence_timeout_ms = 500 },
	ui = {
		panes = { parent = 20, current = 30, preview = 50 },
		show_hidden = true,
		date_format = "%Y-%m-%d %H:%M",
		display_mode = "absolute", -- or "friendly"
		preview_lines = 120,
		max_list_items = 5000,
		row = {
			icon = "{icon} ",
			left = "{name}",
			middle = "",
			right = "{info}",
		},
	},
})

-- External commands (spawned)
lsv.map_command("E", "Open in tmux pane", "&tmux split-window -h nvim '{path}'")
lsv.map_command("e", "Edit in nvim", "nvim '{path}'")
lsv.map_command("t", "New tmux window here", "tmux new-window -c {directory}")

-- Internal actions (Lua functions only; no action strings)
-- Quit
lsv.map_action("y", "Quit lsv", function(lv, config)
	if lv.quit then
		lv.quit()
	end
end)

-- Sorting
lsv.map_action("sn", "Sort by name", function(lv, config)
    config.ui.sort = "name"
end)

lsv.map_action("ss", "Sort by size + show size", function(lv, config)
    config.ui.sort = "size"
    config.ui.show = "size"
end)

lsv.map_action("sr", "Toggle reverse sort", function(lv, config)
    config.ui.sort_reverse = not config.ui.sort_reverse
end)

-- Info field
lsv.map_action("zc", "Info: created date", function(lv, config)
    config.ui.show = "created"
end)

lsv.map_action("zm", "Info: modified date", function(lv, config)
    config.ui.show = "modified"
end)

-- Display mode (affects both dates and sizes)
lsv.map_action("zf", "Display: friendly", function(lv, config)
	config.ui.display_mode = "friendly"
end)

lsv.map_action("za", "Display: absolute", function(lv, config)
	config.ui.display_mode = "absolute"
end)

-- Toggle dotfiles visibility
lsv.map_action("sh", "Toggle show hidden", function(lv, config)
	config.ui.show_hidden = not config.ui.show_hidden
end)

-- Previewer function (ctx):
-- ctx = {
--   path       = absolute file path (string)
--   directory  = parent directory (string)
--   extension  = file extension without dot (string, may be empty)
--   is_binary  = boolean (simple heuristic)
--   height     = preview pane height in rows (number)
--   width      = preview pane width in columns (number)
--   preview_x  = top-left x of preview pane (number)
--   preview_y  = top-left y of preview pane (number)
-- }
-- Return a shell command string (placeholders are expanded: {path},{directory},{name},{extension}), or nil to use default head preview.
lsv.set_previewer(function(ctx)
	-- Render Markdown with glow, respecting pane width
	if ctx.extension == "md" or ctx.extension == "markdown" then
		-- You can build a command with placeholders:
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
		-- image preview using viu (needs installation)
		return "viu --width '{width}' --height '{height}' '{path}'"
	end
	-- For non-binary, colorize with bat (first 120 lines, no wrapping)
	if not ctx.is_binary then
		return "bat --color=always --style=numbers --paging=never --wrap=never --line-range=:120 {path}"
	end

	-- Fallback to default preview (first N lines)
	return nil
end)
