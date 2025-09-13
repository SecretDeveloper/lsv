-- Built-in defaults for lsv. Loaded before user config.

-- Reasonable UI defaults
lsv.config({
	config_version = 1,
	keys = { sequence_timeout_ms = 600 },
	ui = {
		panes = { parent = 20, current = 30, preview = 50 },
		show_hidden = false,
		date_format = "%Y-%m-%d %H:%M",
		display_mode = "absolute", -- or "friendly"
		preview_lines = 100,
		max_list_items = 5000,
		row = {
			icon = "{icon} ",
			left = "{name}",
			middle = "",
			right = "{info}",
		},
		-- Default dark theme; users can override fully in their config
		theme = {
			pane_bg = "#101114",
			border_fg = "gray",
			item_fg = "white",
			item_bg = nil,
			selected_item_fg = "black",
			selected_item_bg = "cyan",
			title_fg = "gray",
			title_bg = nil,
			info_fg = "gray",
			-- Per-type accents
			dir_fg = "cyan",
			dir_bg = nil,
			file_fg = "white",
			file_bg = nil,
			hidden_fg = "darkgray",
			hidden_bg = nil,
			exec_fg = "green",
			exec_bg = nil,
		},
	},
})

-- Internal quick quit
lsv.mapkey("q", "quit", "Quit lsv")

-- Sort actions
lsv.map_action("sn", "Sort by name", function(lv, config)
	config.ui.sort = "name"
end)

lsv.map_action("ss", "Sort by size", function(lv, config)
	config.ui.sort = "size"
end)

lsv.map_action("sr", "Toggle reverse sort", function(lv, config)
	config.ui.sort_reverse = not (config.ui.sort_reverse == true)
end)

-- Info field
lsv.map_action("zn", "Info: none", function(lv, config)
	config.ui.show = "none"
end)

lsv.map_action("zs", "Info: size", function(lv, config)
	config.ui.show = "size"
end)

lsv.map_action("zc", "Info: created date", function(lv, config)
	config.ui.show = "created"
end)

lsv.map_action("zm", "Info: modified date", function(lv, config)
	config.ui.show = "modified"
end)

-- Display mode
lsv.map_action("zf", "Display: friendly", function(lv, config)
	config.ui.display_mode = "friendly"
end)

lsv.map_action("za", "Display: absolute", function(lv, config)
	config.ui.display_mode = "absolute"
end)


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

-- A couple of useful command examples
lsv.map_command("E", "Open in tmux pane", "&tmux split-window -h nvim '{path}'")
lsv.map_command("e", "Edit in nvim", "nvim '{path}'")
lsv.map_command("t", "New tmux window here", "tmux new-window -c {directory}")
