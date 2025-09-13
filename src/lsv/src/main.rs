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
mod runtime;
mod util;
mod app_actions;

pub use app::App;
// helpers moved to dedicated modules

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut app = App::new()?;
    runtime::run_app(&mut app)?;
    Ok(())
}
