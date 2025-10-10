use std::{
  fs,
  io,
  path::Path,
};

// Embedded example config files for bootstrapping user config when the
// examples directory is not available alongside the binary (e.g., installed via
// cargo with no extra assets).

const INIT_LUA: &str = include_str!(concat!(
  env!("CARGO_MANIFEST_DIR"),
  "/examples/config/init.lua"
));

const LUA_ICONS: &str = include_str!(concat!(
  env!("CARGO_MANIFEST_DIR"),
  "/examples/config/lua/icons.lua"
));
const LUA_NERDFONT_ICONS: &str = include_str!(concat!(
  env!("CARGO_MANIFEST_DIR"),
  "/examples/config/lua/nerdfont-icons.lua"
));
const LUA_EMOJI_ICONS: &str = include_str!(concat!(
  env!("CARGO_MANIFEST_DIR"),
  "/examples/config/lua/emoji-icons.lua"
));

// Themes
const THEME_CATPPUCCIN: &str = include_str!(concat!(
  env!("CARGO_MANIFEST_DIR"),
  "/examples/config/lua/themes/catppuccin.lua"
));
const THEME_DARK: &str = include_str!(concat!(
  env!("CARGO_MANIFEST_DIR"),
  "/examples/config/lua/themes/dark.lua"
));
const THEME_DRACULA: &str = include_str!(concat!(
  env!("CARGO_MANIFEST_DIR"),
  "/examples/config/lua/themes/dracula.lua"
));
const THEME_EVERFOREST: &str = include_str!(concat!(
  env!("CARGO_MANIFEST_DIR"),
  "/examples/config/lua/themes/everforest.lua"
));
const THEME_GRUVBOX: &str = include_str!(concat!(
  env!("CARGO_MANIFEST_DIR"),
  "/examples/config/lua/themes/gruvbox.lua"
));
const THEME_HORIZON: &str = include_str!(concat!(
  env!("CARGO_MANIFEST_DIR"),
  "/examples/config/lua/themes/horizon.lua"
));
const THEME_KANAGAWA: &str = include_str!(concat!(
  env!("CARGO_MANIFEST_DIR"),
  "/examples/config/lua/themes/kanagawa.lua"
));
const THEME_LIGHT: &str = include_str!(concat!(
  env!("CARGO_MANIFEST_DIR"),
  "/examples/config/lua/themes/light.lua"
));
const THEME_MATERIAL_PALENIGHT: &str = include_str!(concat!(
  env!("CARGO_MANIFEST_DIR"),
  "/examples/config/lua/themes/material_palenight.lua"
));
const THEME_MONOKAI_PRO: &str = include_str!(concat!(
  env!("CARGO_MANIFEST_DIR"),
  "/examples/config/lua/themes/monokai_pro.lua"
));
const THEME_NIGHTFOX: &str = include_str!(concat!(
  env!("CARGO_MANIFEST_DIR"),
  "/examples/config/lua/themes/nightfox.lua"
));
const THEME_NORD: &str = include_str!(concat!(
  env!("CARGO_MANIFEST_DIR"),
  "/examples/config/lua/themes/nord.lua"
));
const THEME_OCEANIC_NEXT: &str = include_str!(concat!(
  env!("CARGO_MANIFEST_DIR"),
  "/examples/config/lua/themes/oceanic_next.lua"
));
const THEME_ONE_LIGHT: &str = include_str!(concat!(
  env!("CARGO_MANIFEST_DIR"),
  "/examples/config/lua/themes/one_light.lua"
));
const THEME_ONEDARK: &str = include_str!(concat!(
  env!("CARGO_MANIFEST_DIR"),
  "/examples/config/lua/themes/onedark.lua"
));
const THEME_ROSE_PINE_MOON: &str = include_str!(concat!(
  env!("CARGO_MANIFEST_DIR"),
  "/examples/config/lua/themes/rose_pine_moon.lua"
));
const THEME_SOLARIZED_LIGHT: &str = include_str!(concat!(
  env!("CARGO_MANIFEST_DIR"),
  "/examples/config/lua/themes/solarized_light.lua"
));
const THEME_SOLARIZED: &str = include_str!(concat!(
  env!("CARGO_MANIFEST_DIR"),
  "/examples/config/lua/themes/solarized.lua"
));
const THEME_TOKYONIGHT_DAY: &str = include_str!(concat!(
  env!("CARGO_MANIFEST_DIR"),
  "/examples/config/lua/themes/tokyonight_day.lua"
));
const THEME_TOKYONIGHT: &str = include_str!(concat!(
  env!("CARGO_MANIFEST_DIR"),
  "/examples/config/lua/themes/tokyonight.lua"
));

pub fn write_all_to(dst_root: &Path) -> io::Result<()>
{
  // Ensure root
  fs::create_dir_all(dst_root)?;

  // Helper to write a file ensuring parent exists
  fn write(
    dst_root: &Path,
    rel: &str,
    contents: &str,
  ) -> io::Result<()>
  {
    let path = dst_root.join(rel);
    if let Some(p) = path.parent()
    {
      fs::create_dir_all(p)?;
    }
    fs::write(path, contents)
  }

  // init.lua
  write(dst_root, "init.lua", INIT_LUA)?;

  // lua/ helpers
  write(dst_root, "lua/icons.lua", LUA_ICONS)?;
  write(dst_root, "lua/nerdfont-icons.lua", LUA_NERDFONT_ICONS)?;
  write(dst_root, "lua/emoji-icons.lua", LUA_EMOJI_ICONS)?;

  // themes
  write(dst_root, "lua/themes/catppuccin.lua", THEME_CATPPUCCIN)?;
  write(dst_root, "lua/themes/dark.lua", THEME_DARK)?;
  write(dst_root, "lua/themes/dracula.lua", THEME_DRACULA)?;
  write(dst_root, "lua/themes/everforest.lua", THEME_EVERFOREST)?;
  write(dst_root, "lua/themes/gruvbox.lua", THEME_GRUVBOX)?;
  write(dst_root, "lua/themes/horizon.lua", THEME_HORIZON)?;
  write(dst_root, "lua/themes/kanagawa.lua", THEME_KANAGAWA)?;
  write(dst_root, "lua/themes/light.lua", THEME_LIGHT)?;
  write(
    dst_root,
    "lua/themes/material_palenight.lua",
    THEME_MATERIAL_PALENIGHT,
  )?;
  write(dst_root, "lua/themes/monokai_pro.lua", THEME_MONOKAI_PRO)?;
  write(dst_root, "lua/themes/nightfox.lua", THEME_NIGHTFOX)?;
  write(dst_root, "lua/themes/nord.lua", THEME_NORD)?;
  write(dst_root, "lua/themes/oceanic_next.lua", THEME_OCEANIC_NEXT)?;
  write(dst_root, "lua/themes/one_light.lua", THEME_ONE_LIGHT)?;
  write(dst_root, "lua/themes/onedark.lua", THEME_ONEDARK)?;
  write(dst_root, "lua/themes/rose_pine_moon.lua", THEME_ROSE_PINE_MOON)?;
  write(dst_root, "lua/themes/solarized_light.lua", THEME_SOLARIZED_LIGHT)?;
  write(dst_root, "lua/themes/solarized.lua", THEME_SOLARIZED)?;
  write(dst_root, "lua/themes/tokyonight_day.lua", THEME_TOKYONIGHT_DAY)?;
  write(dst_root, "lua/themes/tokyonight.lua", THEME_TOKYONIGHT)?;

  Ok(())
}
