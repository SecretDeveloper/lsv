-- Built-in defaults for lsv. Loaded before user config.

-- Provide entries for all config values; users can override any of these.
lsv.config({
	config_version = 1,
	icons = { enabled = false, preset = nil, font = nil },
	keys = { sequence_timeout_ms = 0 },
	ui = {
		panes = { parent = 20, current = 30, preview = 50 },
		show_hidden = false,
		date_format = "%Y-%m-%d %H:%M",
		display_mode = "absolute", -- or "friendly"
		preview_lines = 100,
		max_list_items = 5000,
		-- initial listing/format defaults
		sort = "name",
		sort_reverse = false,
		show = "none",
		row = {
			icon = "{icon} ",
			left = "{name}",
			middle = "",
			right = "{info}",
		},
		-- fixed widths disabled by default (0 = auto)
		row_widths = { icon = 0, left = 0, middle = 0, right = 0 },
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
lsv.map_action("q", "Quit lsv", function(lsv, config)
	return { quit = true }
end)

-- Sort actions
lsv.map_action("sn", "Sort by name", function(lsv, config)
	config.ui.sort = "name"
end)

lsv.map_action("ss", "Sort by size", function(lsv, config)
	config.ui.sort = "size"
end)

lsv.map_action("sr", "Toggle reverse sort", function(lsv, config)
	config.ui.sort_reverse = not (config.ui.sort_reverse == true)
end)

-- Sort by modified (mtime) and created (ctime)
lsv.map_action("sm", "Sort by modified time", function(lsv, config)
	config.ui.sort = "mtime"
end)

lsv.map_action("sc", "Sort by created time", function(lsv, config)
	config.ui.sort = "created"
end)

-- Navigation (use action tables; runtime interprets nav directive)
lsv.map_action("gg", "Go to top", function(lsv, config)
	lsv.select_item(0)
end)
lsv.map_action("G", "Go to bottom", function(lsv, config)
	lsv.select_last_item()
end)

-- Info field
lsv.map_action("zn", "Info: none", function(lsv, config)
	config.ui.show = "none"
end)
lsv.map_action("zh", "Toggle Show Hidden", function(lsv, config)
	config.ui.show_hidden = not (config.ui.show_hidden == true)
end)

lsv.map_action("zs", "Info: size", function(lsv, config)
	config.ui.show = "size"
end)

lsv.map_action("zc", "Info: created date", function(lsv, config)
	config.ui.show = "created"
end)

lsv.map_action("zm", "Show messages", function(lsv, config)
	return { messages = "toggle" }
end)

-- Display mode
lsv.map_action("zf", "Display: friendly", function(lsv, config)
	config.ui.display_mode = "friendly"
end)

lsv.map_action("za", "Display: absolute", function(lsv, config)
	config.ui.display_mode = "absolute"
end)

-- Add file/folder action
lsv.map_action("a", "Add file/folder", function(lsv, config)
  local cwd = (config.context and config.context.cwd) or "."
  local name = lsv.prompt("Name (end with '/' for folder): ", nil)
  if not name or name == "" then return end
  if name:sub(-1) == "/" then
    lsv.os_run_interactive("mkdir -p " .. cwd .. "/" .. name)
  else
    lsv.os_run_interactive("touch " .. cwd .. "/" .. name)
  end
end)

-- Toggle last command output panel
lsv.map_action("zo", "Show output", function(lsv, config)
	return { output = "toggle" }
end)

lsv.map_action("ut", "UI Theme picker", function(lsv, config)
	lsv.open_theme_picker()
end)
