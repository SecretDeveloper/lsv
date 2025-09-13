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
