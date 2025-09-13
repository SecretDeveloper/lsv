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

-- Override an action: make "ss" also show sizes in the info column
lsv.map_action("ss", "Sort by size + show size", function(lv, config)
	config.ui.sort = "size"
	config.ui.show = "size"
end)

-- Add a new command mapping for testing
lsv.map_command("g", "Git status (background)", "git -C {directory} status")
