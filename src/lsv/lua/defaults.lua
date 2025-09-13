-- Built-in defaults for lsv. Loaded before user config.

-- Reasonable UI defaults
lsv.config({
  config_version = 1,
  keys = { sequence_timeout_ms = 600 },
  ui = {
    panes = { parent = 30, current = 40, preview = 30 },
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
    -- Minimal theme; users can override fully in their config
    theme = {
      title_fg = "gray",
      info_fg = "gray",
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
lsv.map_action("za", "Display: friendly", function(lv, config)
  config.ui.display_mode = "friendly"
end)

lsv.map_action("zf", "Display: absolute", function(lv, config)
  config.ui.display_mode = "absolute"
end)

-- Permissions in middle column
lsv.map_action("zd", "Center: permissions", function(lv, config)
  config.ui.row.middle = "{perms}"
end)

-- Previewer: markdown via glow, images via viu, text via bat
lsv.set_previewer(function(ctx)
  if ctx.extension == "md" or ctx.extension == "markdown" then
    return "glow --style=dark --width=" .. tostring(ctx.width) .. " {path}"
  end
  if ctx.extension == "jpg" or ctx.extension == "jpeg" or ctx.extension == "png"
     or ctx.extension == "gif" or ctx.extension == "bmp" or ctx.extension == "tiff" then
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

