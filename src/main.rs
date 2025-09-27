mod actions;
mod app;
mod config;
mod config_data;
mod core;
mod enums;
mod input;
mod keymap;
mod preview;
mod runtime;
mod trace;
mod ui;
mod util;

pub use app::App;
// helpers moved to dedicated modules

fn print_version()
{
  println!("lsv {}", env!("CARGO_PKG_VERSION"));
}

fn print_help()
{
  println!(
    "Usage: lsv [OPTIONS] [DIR]\n\n\
     Options:\n\
       -h, --help            Show this help and exit\n\
       -V, --version         Show version and exit\n\
           --config-dir DIR  Use DIR as the config root (sets LSV_CONFIG_DIR)\n\
           --trace[=FILE]    Enable tracing to FILE (default /tmp/lsv-trace.log)\n\
     Arguments:\n\
       DIR                   Start in directory DIR (default: current dir)\n"
  );
}

fn main() -> Result<(), Box<dyn std::error::Error>>
{
  use std::env;
  trace::install_panic_hook();

  // Minimal argument parsing (avoid external deps)
  let mut args = env::args().skip(1);
  let mut dir_arg: Option<String> = None;
  while let Some(a) = args.next()
  {
    match a.as_str()
    {
      "-h" | "--help" =>
      {
        print_help();
        return Ok(());
      }
      "-V" | "--version" =>
      {
        print_version();
        return Ok(());
      }
      s if s == "--trace" || s.starts_with("--trace=") =>
      {
        let file = if let Some(eq) = s.split_once('=')
        {
          eq.1.to_string()
        }
        else
        {
          String::new()
        };
        // Enable trace
        unsafe { env::set_var("LSV_TRACE", "1") };
        if !file.is_empty()
        {
          unsafe { env::set_var("LSV_TRACE_FILE", file) };
        }
      }
      "--config-dir" =>
      {
        if let Some(dir) = args.next()
        {
          unsafe { env::set_var("LSV_CONFIG_DIR", &dir) };
        }
        else
        {
          eprintln!("lsv: --config-dir requires a DIR argument");
          print_help();
          std::process::exit(2);
        }
      }
      s if s.starts_with("--config-dir=") =>
      {
        if let Some((_, dir)) = s.split_once('=')
        {
          unsafe { env::set_var("LSV_CONFIG_DIR", dir) };
        }
      }
      "--" =>
      {
        // Remaining is positional dir (optional); take first if present
        dir_arg = args.next();
        break;
      }
      s if s.starts_with('-') =>
      {
        eprintln!("lsv: unknown option: {}", s);
        print_help();
        std::process::exit(2);
      }
      // Positional directory
      other =>
      {
        if dir_arg.is_none()
        {
          dir_arg = Some(other.to_string());
        }
      }
    }
  }

  if let Some(dir) = dir_arg
    && let Err(e) = std::env::set_current_dir(&dir)
  {
    eprintln!("lsv: failed to change directory to '{}': {}", dir, e);
    std::process::exit(1);
  }

  trace::log("[main] starting lsv");
  let mut app = App::new()?;
  if let Err(e) = runtime::run_app(&mut app)
  {
    trace::log(format!("[error] runtime::run_app: {e}"));
    return Err(e);
  }
  Ok(())
}
