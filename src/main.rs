mod config;
mod preview;
mod ui;
mod trace;
mod actions;
mod input;
mod enums;
mod config_data;
mod app;
mod runtime;
mod util;

pub use app::App;
// helpers moved to dedicated modules

fn main() -> Result<(), Box<dyn std::error::Error>> {
    trace::install_panic_hook();
    
    trace::log("[main] starting lsv");
    let mut app = App::new()?;
    if let Err(e) = runtime::run_app(&mut app) {
        trace::log(format!("[error] runtime::run_app: {e}"));
        return Err(e.into());
    }
    Ok(())
}
