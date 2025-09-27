-- Built-in defaults for lsv. Loaded before user config.
--
-- config.context passed to Lua actions contains:
--   cwd                        -- current working directory
--   selected_index             -- selected row index (0-based)
--   current_len                -- number of entries in current pane
--   current_file               -- full path of highlighted item (or cwd)
--   current_file_dir           -- parent directory of highlighted item
--   current_file_name          -- basename of highlighted item
--   current_file_extension     -- extension (no dot) of highlighted item
--   current_file_ctime         -- creation time (formatted per ui.date_format)
--   current_file_mtime         -- modified time (formatted per ui.date_format)

-- Provide entries for all config values; users can override any of these.
lsv.config({
	config_version = 1,
	icons = { enabled = false, preset = nil, font = nil },
	keys = { sequence_timeout_ms = 0 },
	ui = {
		panes = { parent = 10, current = 20, preview = 70 },
		show_hidden = false,
		date_format = "%Y-%m-%d %H:%M",
		-- header format strings with placeholders
		header = {
			left = "{username}@{hostname}:{current_file}",
			right = "{current_file_size}  {owner}  {current_file_permissions}  {current_file_ctime}",
		},
		display_mode = "absolute", -- or "friendly"
		max_list_items = 5000,
		confirm_delete = true,
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
		-- Modal window defaults (user can override)
		modals = {
			prompt = { width_pct = 50, height_pct = 10 },
			confirm = { width_pct = 50, height_pct = 10 },
			theme = { width_pct = 60, height_pct = 60 },
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
	lsv.toggle_show_messages()
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
	lsv.add_entry()
end)

-- Toggle last command output panel
lsv.map_action("zo", "Show output", function(lsv, config)
	lsv.toggle_output()
end)

lsv.map_action({ "ut", "Ut" }, "UI Theme picker", function(lsv, config)
	lsv.open_theme_picker()
end)

lsv.map_action("r", "Rename selected", function(lsv, config)
	lsv.rename_item()
end)
lsv.map_action(" ", "Toggle selected", function(lsv, config)
	lsv.toggle_select()
end)
lsv.map_action("c", "Copy selected", function(lsv, config)
	lsv.copy_selection()
end)
lsv.map_action("x", "Move selected", function(lsv, config)
	lsv.move_selection()
end)
lsv.map_action("v", "Paste selected", function(lsv, config)
	lsv.paste_clipboard()
end)
lsv.map_action("u", "Clear selected", function(lsv, config)
	lsv.clear_selection()
end)
lsv.map_action("D", "Delete selected", function(lsv, config)
	lsv.delete_selected_all()
end)
-- Escape key binding (also close overlays via helper)
lsv.map_action("<Esc>", "Cancel Action / Close Popups", function(lsv, config)
	lsv.close_overlays()
end)
