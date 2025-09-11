use std::process::Command;
use std::io;

use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};

pub fn run_shell_command(app: &mut crate::App, sc: &crate::config::ShellCmd) {
    let selection_path = app
        .selected_entry()
        .map(|e| e.path.clone())
        .unwrap_or_else(|| app.cwd.clone());
    let cwd = app.cwd.clone();

    let mut cmd_str = sc.cmd.clone();
    // Template replacements
    let path_str = selection_path.to_string_lossy().to_string();
    let dir_str = selection_path
        .parent()
        .unwrap_or(&cwd)
        .to_string_lossy()
        .to_string();
    let name_str = selection_path
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    let ext_str = selection_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    cmd_str = cmd_str.replace("{path}", &crate::shell_escape(&path_str));
    cmd_str = cmd_str.replace("{directory}", &crate::shell_escape(&dir_str));
    cmd_str = cmd_str.replace("{dir}", &crate::shell_escape(&dir_str));
    cmd_str = cmd_str.replace("{name}", &crate::shell_escape(&name_str));
    cmd_str = cmd_str.replace("{extension}", &crate::shell_escape(&ext_str));
    cmd_str = cmd_str.replace("$f", &crate::shell_escape(&path_str));
    // Trim leading '&' (treated as hint)
    let cmd_trimmed = cmd_str.trim_start_matches('&').to_string();

    let is_interactive = looks_interactive(&sc.cmd);
    crate::trace::log(format!(
        "[cmd] built cmd='{}' cwd='{}' file='{}'",
        cmd_trimmed,
        cwd.display(),
        path_str
    ));

    if is_interactive {
        // Suspend TUI and run interactive command attached to the terminal
        let _ = disable_raw_mode();
        let mut stdout = io::stdout();
        let _ = execute!(stdout, LeaveAlternateScreen);
        let status = Command::new("sh")
            .arg("-lc")
            .arg(&cmd_trimmed)
            .current_dir(&cwd)
            .env("LV_PATH", &path_str)
            .env("LV_DIR", &dir_str)
            .env("LV_NAME", &name_str)
            .status();
        let _ = enable_raw_mode();
        let mut stdout2 = io::stdout();
        let _ = execute!(stdout2, EnterAlternateScreen);
        app.refresh_lists();
        app.refresh_preview();
        app.force_full_redraw = true;
        crate::trace::log(format!(
            "[cmd] interactive exit={:?}",
            status.as_ref().ok().and_then(|s| s.code())
        ));
        let _ = status;
        return;
    }

    // Default: spawn asynchronously (background)
    match Command::new("sh")
        .arg("-lc")
        .arg(&cmd_trimmed)
        .current_dir(&cwd)
        .env("LV_PATH", &path_str)
        .env("LV_DIR", &dir_str)
        .env("LV_NAME", &name_str)
        .spawn()
    {
        Ok(child) => crate::trace::log(format!("[cmd] spawned pid={}", child.id())),
        Err(e) => crate::trace::log(format!("[cmd] spawn error: {}", e)),
    }
    app.refresh_lists();
    app.refresh_preview();
}

fn looks_interactive(cmd: &str) -> bool {
    let lower = cmd.to_ascii_lowercase();
    let needles = [
        "nvim", "vim", "vi ", "nano", "emacs", "tmux", "less", "more", "ssh ", "top", "htop",
    ];
    needles.iter().any(|n| lower.contains(n))
}
