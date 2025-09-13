mod config;
mod preview;
mod cmd;
mod ui;
mod trace;
mod actions;
mod input;
mod enums;
mod config_data;
mod app;

pub use app::App;
// Re-export helpers referenced as `crate::...` by sibling modules
pub(crate) use app::{dispatch_action, shell_escape};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new()?;
    app::run_app(&mut app)?;
    Ok(())
}
