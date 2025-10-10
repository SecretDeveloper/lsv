mod actions;
mod app;
mod commands;
mod config;
mod core;
mod embed_examples;
mod enums;
mod input;
mod keymap;
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
          --init-config     Prompt to create user config from examples\n\
          --trace[=FILE]    Enable tracing to FILE (default /tmp/lsv-trace.log)\n\
     Arguments:\n\
      DIR                   Start in directory DIR (default: current dir)\n"
  );
}

fn prompt_yes_no(msg: &str) -> std::io::Result<bool>
{
  use std::io::Write;
  let mut out = std::io::stdout();
  write!(out, "{}", msg)?;
  out.flush()?;
  let mut input = String::new();
  std::io::stdin().read_line(&mut input)?;
  let ans = input.trim();
  Ok(matches!(ans, "y" | "Y" | "yes" | "YES"))
}

fn find_examples_config() -> Option<std::path::PathBuf>
{
  use std::path::PathBuf;
  if let Ok(dir) = std::env::var("LSV_EXAMPLES_DIR")
    && !dir.trim().is_empty()
  {
    let p = PathBuf::from(dir);
    if p.is_dir()
    {
      return Some(p);
    }
  }
  let cwd = std::env::current_dir().ok();
  if let Some(c) = cwd.as_ref()
  {
    let cand = c.join("examples").join("config");
    if cand.is_dir()
    {
      return Some(cand);
    }
  }
  if let Ok(exe) = std::env::current_exe()
  {
    let exe_dir = exe.parent().unwrap_or_else(|| std::path::Path::new("."));
    let cand1 = exe_dir.join("examples").join("config");
    if cand1.is_dir()
    {
      return Some(cand1);
    }
    let cand2 = exe_dir.join("..").join("examples").join("config");
    if cand2.is_dir()
    {
      return Some(cand2);
    }
    let cand3 = exe_dir.join("..").join("..").join("examples").join("config");
    if cand3.is_dir()
    {
      return Some(cand3);
    }
  }
  None
}

fn copy_dir_recursive(
  src: &std::path::Path,
  dst: &std::path::Path,
) -> std::io::Result<()>
{
  use std::fs;
  fs::create_dir_all(dst)?;
  for entry in fs::read_dir(src)?
  {
    let entry = entry?;
    let ty = entry.file_type()?;
    let from = entry.path();
    let to = dst.join(entry.file_name());
    if ty.is_dir()
    {
      copy_dir_recursive(&from, &to)?;
    }
    else if ty.is_file()
    {
      // Create parent dirs just in case
      if let Some(parent) = to.parent()
      {
        fs::create_dir_all(parent)?;
      }
      fs::copy(&from, &to)?;
    }
    // ignore symlinks for now
  }
  Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>>
{
  use std::env;
  trace::install_panic_hook();

  // Minimal argument parsing (avoid external deps)
  let mut args = env::args().skip(1);
  let mut dir_arg: Option<String> = None;
  let mut init_config: bool = false;
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
      "--init-config" =>
      {
        init_config = true;
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

  if init_config
  {
    let paths = config::discover_config_paths()?;
    let root = paths.root;
    let exists = paths.exists;
    if exists
    {
      println!("Config already exists at {}\nNothing to do.", root.display());
      return Ok(());
    }
    println!("This will create lsv config at: {}", root.display());
    if !prompt_yes_no("Proceed? [y/N] ")?
    {
      println!("Aborted.");
      return Ok(());
    }
    if let Some(src_dir) = find_examples_config()
    {
      // Copy everything from examples/config into root
      std::fs::create_dir_all(&root)?;
      copy_dir_recursive(&src_dir, &root)?;
      println!(
        "Created config in {} (source dir: {})",
        root.display(),
        src_dir.display()
      );
    }
    else
    {
      // Fallback to embedded examples
      std::fs::create_dir_all(&root)?;
      embed_examples::write_all_to(&root)?;
      println!("Created config in {} (from embedded examples)", root.display());
    }
    return Ok(());
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
