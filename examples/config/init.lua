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
	-- Optional config schema/version marker (reserved for future use)
	config_version = 1,

	-- Icons configuration: enable + mappings (preset/font are informational)
	icons = {
		enabled = true, -- set false to disable icons entirely
		preset = nil, -- optional preset label for your setup
		font = "Nerd", -- hint that a Nerd Font is recommended
		default_file = "", -- fallback icon for files
		default_dir = "", -- fallback icon for directories
		mappings = require("nerdfont-icons"), -- combined extensions/folders table
	},

	-- Key handling configuration
	keys = {
		sequence_timeout_ms = 600, -- timeout for multi-key sequences (0=disabled)
	},

	-- UI configuration block
	ui = {
		-- Pane split percentages (parent/current/preview)
		panes = { parent = 30, current = 40, preview = 30 },

		-- Listing and formatting
		show_hidden = false, -- show dotfiles
		max_list_items = 5000, -- soft cap on entries rendered
		date_format = "%Y-%m-%d %H:%M", -- strftime/chrono format used in templates

		-- Header (top bar) formatting + colours
		header = {
			left = "{username|fg=cyan;style=bold}@{hostname|fg=cyan}:{cwd|fg=#ffd866}/{current_file_name|fg=#ffd866;style=bold}",
			right = "{current_file_size|fg=gray}  {owner|fg=gray}  {current_file_permissions|fg=gray}  {current_file_ctime|fg=gray}",
			fg = "gray", -- text colour (overridden by header_fg below)
			bg = "#181825", -- background colour (overridden by header_bg below)
		},
		header_fg = nil, -- optional override for header.fg
		header_bg = nil, -- optional override for header.bg

		-- Row template and optional fixed widths
		row = {
			icon = " ", -- icon cell template (often left as a single space)
			left = "{name}", -- left segment template
			middle = "", -- middle segment template
			right = "{info}", -- right segment template
		},
		row_widths = { icon = 2, left = 40, middle = 0, right = 14 }, -- 0 = flexible

		-- Display/time formatting for dates in templates: "absolute" | "friendly"
		display_mode = "friendly",

		-- Sorting controls applied on startup if provided
		sort = nil, -- one of: "name" | "size" | "mtime" | "created"
		sort_reverse = false, -- reverse the sort order
		-- Which info column to show: "none" | "size" | "created" | "modified"
		show = nil,

		-- Confirmation prompts for destructive actions
		confirm_delete = true,

		-- Theme controls: choose ONE of the following to load a theme
		theme = require("themes/catppuccin"), -- preferred: module name under <config>/lua/themes
		-- theme_path = "themes/dark.lua",     -- legacy: direct Lua file under <config>/themes

		-- Modal sizes (as percentages of the terminal window)
		modals = {
			prompt = { width_pct = 50, height_pct = 40 }, -- add/rename prompt
			confirm = { width_pct = 50, height_pct = 40 }, -- confirmation dialogs
			theme = { width_pct = 60, height_pct = 60 }, -- theme picker
		},
	},
})

-- Helper used by previewer and actions below (OS-aware quoting)
local function shquote(s)
	return lsv.quote(tostring(s))
end
-- Determine OS with backward-compat for older lsv versions
local OS = lsv.get_os_name()

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

-- Git status in current directory
lsv.map_action("gs", "Git Status", function(lsv, config)
	local dir = (config.context and config.context.cwd) or "."
	lsv.os_run(string.format("git -C %s status", lsv.quote(dir)))
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

-- Diff: compare two selected files (fd)
lsv.map_action("fd", "Diff selected files", function(lsv, config)
	local paths = lsv.get_selected_paths()
	local n = #paths
	if n < 2 then
		lsv.show_error("Diff: select 2 files")
		return
	end
	if n > 2 then
		lsv.show_message(string.format("Diff: using first 2 of %d", n))
	end
	local a = shquote(paths[1])
	local b = shquote(paths[2])
	-- Allow overriding via $DIFF_TOOL (e.g., 'delta -s -n' or 'difft')
	local user_tool = lsv.getenv("DIFF_TOOL")
	local cmd
	if user_tool and #user_tool > 0 then
		cmd = string.format("%s %s %s", user_tool, a, b)
	elseif OS == "windows" then
		-- Fallback to git diff on Windows
		cmd = string.format("git --no-pager diff --no-index --color=always %s %s", a, b)
	else
		-- Default: git --no-index to diff arbitrary files
		cmd = string.format("git --no-pager diff --no-index --color=always %s %s", a, b)
	end
	lsv.os_run_interactive(cmd)
end)

-- Example: clear messages (Ctrl+m)
lsv.map_action("<C-m>", "Clear messages", function(lsv, config)
	lsv.clear_messages()
end)
