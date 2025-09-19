// Public library interface for integration tests and embedding.
pub mod config;
pub mod app;
pub mod actions;
pub mod enums;
pub mod util;
pub mod trace;
pub mod config_data;
pub mod input;
pub mod runtime_util;
// ui and preview are only required for the binary; tests avoid drawing
