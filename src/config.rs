//! Loading and translating configuration between Lua and Rust.
//!
//! The real application consumes Lua configuration files. This module exposes
//! helpers to load them, merge user values with defaults, and convert between
//! Lua tables and strongly typed Rust structures. Integration tests reuse these
//! APIs to fabricate configurations dynamically.

use mlua::{
  Lua,
  LuaOptions,
  StdLib,
};
use std::path::Path;

mod types;
pub use types::*;
mod paths;
pub use paths::{
  ConfigPaths,
  discover_config_paths,
};
mod lsv_api;
pub(crate) use lsv_api::install_lsv_api;
mod theme;
pub(crate) use theme::{
  load_theme_table_from_path,
  merge_theme_table,
  resolve_theme_path,
};
mod require;
pub(crate) use require::install_require;
mod lua_engine;
pub use lua_engine::LuaEngine;
mod loader;
pub mod runtime;
pub use loader::load_config;
#[allow(unused_imports)]
pub use loader::load_config_from_code;

pub mod defaults;

// Theme helpers moved to config/theme.rs

/// Load a standalone theme file from disk.
///
/// The theme is returned as a [`UiTheme`] without mutating any global state.
pub fn load_theme_from_file(path: &Path) -> std::io::Result<UiTheme>
{
  let lua = Lua::new_with(
    StdLib::STRING | StdLib::TABLE | StdLib::MATH,
    LuaOptions::default(),
  )
  .map_err(|e| std::io::Error::other(format!("lua init failed: {e}")))?;
  let tbl = load_theme_table_from_path(&lua, path).map_err(|e| {
    std::io::Error::other(format!("load theme '{}': {e}", path.display()))
  })?;
  let mut theme = UiTheme::default();
  merge_theme_table(&tbl, &mut theme);
  Ok(theme)
}

// LuaEngine moved to config/lua_engine.rs
// Paths moved to config/paths.rs
// lsv_api moved to config/lsv_api.rs
// require() moved to config/require.rs
