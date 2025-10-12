//! Preview lifecycle control for App.

use crate::app::App;

impl App
{
  pub(crate) fn refresh_preview(&mut self)
  {
    if self.running_preview.is_some()
    {
      // Live process is writing into preview
      return;
    }
    // Avoid borrowing self while mutating by cloning the needed fields first
    let (is_dir, path) = match self.selected_entry()
    {
      Some(e) => (e.is_dir, e.path.clone()),
      None =>
      {
        self.preview.static_lines.clear();
        // Invalidate dynamic preview cache when nothing selected
        self.preview.cache_key = None;
        self.preview.cache_lines = None;
        return;
      }
    };

    const PREVIEW_LINES_LIMIT: usize = 200;
    let preview_limit = PREVIEW_LINES_LIMIT;
    if is_dir
    {
      match self.read_dir_sorted(&path)
      {
        Ok(list) =>
        {
          let mut lines = Vec::new();
          for e in list.into_iter().take(preview_limit)
          {
            let marker = if e.is_dir { "/" } else { "" };
            let formatted = format!("{}{}", e.name, marker);
            lines.push(crate::util::sanitize_line(&formatted));
          }
          self.preview.static_lines = lines;
        }
        Err(err) =>
        {
          self.preview.static_lines =
            vec![format!("<error reading directory: {}>", err)];
        }
      }
    }
    else
    {
      // Detect binary early to avoid rendering junk or huge wrapped lines
      if crate::util::is_binary(&path)
      {
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        self.preview.static_lines = vec![
          String::from("<binary file>"),
          format!("size: {} bytes", size),
          String::from("tip: configure a previewer for this type"),
        ];
      }
      else
      {
        // Cap bytes and lines to avoid runaway previews for huge files
        const HEAD_BYTES_LIMIT: usize = 128 * 1024; // 128 KiB cap
        self.preview.static_lines = crate::util::read_file_head_safe(
          &path,
          HEAD_BYTES_LIMIT,
          preview_limit,
        )
        .map(|v| {
          v.into_iter().map(|s| crate::util::sanitize_line(&s)).collect()
        })
        .unwrap_or_else(|e| vec![format!("<error reading file: {}>", e)]);
      }
      // Invalidate dynamic preview cache when selection changes
      self.preview.cache_key = None;
      self.preview.cache_lines = None;
    }
  }

  pub fn start_preview_process(
    &mut self,
    cmd: &str,
  )
  {
    use std::{
      process::{
        Command,
        Stdio,
      },
      sync::mpsc,
    };
    // Reset preview buffer and caches
    self.preview.static_lines.clear();
    self.preview.cache_key = None;
    self.preview.cache_lines = None;
    // Channel to stream lines
    let (tx, rx) = mpsc::channel::<Option<String>>();
    // Build platform shell
    #[cfg(windows)]
    let mut command = {
      let mut c = Command::new("cmd");
      c.arg("/C").arg(cmd);
      c
    };
    #[cfg(not(windows))]
    let mut command = {
      let mut c = Command::new("sh");
      c.arg("-lc").arg(cmd);
      c
    };
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    match command.spawn()
    {
      Ok(mut child) =>
      {
        let mut stdout = child.stdout.take();
        let stderr = child.stderr.take();
        std::thread::spawn(move || {
          // Helper to read from a pipe and send lines
          let read_out = |s: &mut Option<std::process::ChildStdout>| {
            if let Some(out) = s
            {
              let mut buf = [0u8; 8192];
              let mut acc = Vec::<u8>::new();
              loop
              {
                match std::io::Read::read(out, &mut buf)
                {
                  Ok(0) => break,
                  Ok(n) =>
                  {
                    acc.extend_from_slice(&buf[..n]);
                    while let Some(pos) = acc.iter().position(|&b| b == b'\n')
                    {
                      let chunk = acc.drain(..=pos).collect::<Vec<u8>>();
                      let line = String::from_utf8_lossy(&chunk)
                        .trim_end_matches('\n')
                        .to_string();
                      let _ = tx.send(Some(line));
                    }
                  }
                  Err(_) => break,
                }
              }
              if !acc.is_empty()
              {
                let line = String::from_utf8_lossy(&acc).to_string();
                let _ = tx.send(Some(line));
              }
            }
          };
          read_out(&mut stdout);
          // Read stderr separately (duplicate code for type simplicity)
          if let Some(mut err) = stderr
          {
            let mut buf = [0u8; 8192];
            let mut acc = Vec::<u8>::new();
            loop
            {
              match std::io::Read::read(&mut err, &mut buf)
              {
                Ok(0) => break,
                Ok(n) =>
                {
                  acc.extend_from_slice(&buf[..n]);
                  while let Some(pos) = acc.iter().position(|&b| b == b'\n')
                  {
                    let chunk = acc.drain(..=pos).collect::<Vec<u8>>();
                    let line = String::from_utf8_lossy(&chunk)
                      .trim_end_matches('\n')
                      .to_string();
                    let _ = tx.send(Some(line));
                  }
                }
                Err(_) => break,
              }
            }
            if !acc.is_empty()
            {
              let line = String::from_utf8_lossy(&acc).to_string();
              let _ = tx.send(Some(line));
            }
          }
          let _ = tx.send(None);
        });
        self.running_preview = Some(crate::app::RunningPreview { rx });
        self.force_full_redraw = true;
      }
      Err(e) =>
      {
        self.preview.static_lines = vec![format!("<error: {}>", e)];
      }
    }
  }
}
