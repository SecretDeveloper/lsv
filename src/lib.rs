//! Library interface for embedding lsv components and driving integration
//! tests.
//!
//! The binary uses these modules internally; consumers can reuse them to
//! configure an [`App`](crate::app::App), dispatch actions, or inspect state in
//! tests. See the documentation under `docs/` for higher-level guides.

pub mod actions;
pub mod app;
pub mod commands;
pub mod config;
// Top-level re-export for convenience in tests and examples
pub use crate::config::load_config_from_code;
// Keep compatibility: re-export config runtime data as `config_data`
pub use crate::config::runtime::data as config_data;
pub mod core;
pub mod enums;
pub mod input;
pub mod keymap;
pub mod runtime_util;
pub mod trace;
pub mod ui;
pub mod util;
pub use app::App;

/// Dispatch a command string (single action or `;`-separated sequence)
/// against an [`App`](crate::app::App) instance.
pub use actions::dispatch_action;
