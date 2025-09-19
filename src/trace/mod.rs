use std::fs::OpenOptions;
use std::path::PathBuf;

fn enabled() -> bool {
    std::env::var("LSV_TRACE")
        .map(|v| !v.is_empty() && v != "0")
        .unwrap_or(false)
}

pub fn log<S: AsRef<str>>(s: S) {
    if !enabled() {
        return;
    }
    let line = format!("{} {}\n", now_millis(), s.as_ref());
    if let Some(path) = file_path() {
        let _ = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut f| {
                use std::io::Write;
                f.write_all(line.as_bytes())
            });
    }
}

/// Install a panic hook that logs panic message, location, and backtrace
/// to the trace log and attempts to restore the terminal state so the
/// panic is visible to the user.
pub fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        // Extract panic message and location
        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            String::from("<non-string panic payload>")
        };
        let loc = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "<unknown>".to_string());
        // Capture a backtrace when available
        let bt = std::backtrace::Backtrace::force_capture();
        log(format!("[panic] {msg} @ {loc}"));
        log(format!("[panic] backtrace:\n{bt}"));
        // Best-effort terminal restore so the panic is visible
        let _ = crossterm::terminal::disable_raw_mode();
        let mut out = std::io::stdout();
        let _ = crossterm::execute!(out, crossterm::terminal::LeaveAlternateScreen);
    }));
}

fn file_path() -> Option<PathBuf> {
    if let Ok(fp) = std::env::var("LSV_TRACE_FILE") {
        return Some(PathBuf::from(fp));
    }
    if let Ok(tmp) = std::env::var("TMPDIR") {
        return Some(PathBuf::from(tmp).join("lsv-trace.log"));
    }
    Some(PathBuf::from("/tmp/lsv-trace.log"))
}

fn now_millis() -> u128 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}
