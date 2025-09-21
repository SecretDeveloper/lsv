// Public library interface for integration tests and embedding.
pub mod actions;
pub mod app;
pub mod config;
pub mod config_data;
pub mod enums;
pub mod input;
pub mod runtime_util;
pub mod trace;
pub mod util;
// ui and preview are only required for the binary; tests avoid drawing
