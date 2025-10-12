//! Marks persistence and navigation for App.

use crate::app::App;

impl App
{
  pub(crate) fn save_marks(&self)
  {
    if let Some(root) = self.theme_root_dir()
    {
      let path = root.join("marks");
      let _ = crate::core::marks::save_marks(&path, &self.marks);
    }
  }

  pub(crate) fn add_mark(
    &mut self,
    ch: char,
  )
  {
    let dir = self.cwd.clone();
    self.marks.insert(ch, dir.clone());
    self.save_marks();
    self.add_message(&format!("Mark '{}' set: {}", ch, dir.display()));
  }

  pub(crate) fn goto_mark(
    &mut self,
    ch: char,
  )
  {
    if let Some(path) = self.marks.get(&ch).cloned()
    {
      if path.is_dir()
      {
        self.set_cwd(&path);
        self.add_message(&format!("Jumped to '{}'", path.display()));
      }
      else
      {
        self.add_message(&format!(
          "Mark '{}' not a directory: {}",
          ch,
          path.display()
        ));
      }
    }
    else
    {
      self.add_message(&format!("No mark '{}'", ch));
    }
  }

  pub(crate) fn list_marks_text(&self) -> String
  {
    let mut keys: Vec<char> = self.marks.keys().copied().collect();
    keys.sort();
    let mut out = String::new();
    for k in keys
    {
      if let Some(p) = self.marks.get(&k)
      {
        out.push_str(&format!("{}  {}\n", k, p.display()));
      }
    }
    if out.is_empty()
    {
      out.push_str("<no marks>\n");
    }
    out
  }
}
