use std::{
  collections::HashMap,
  fs,
  io::{
    self,
    Write,
  },
  path::{
    Path,
    PathBuf,
  },
};

// Simple line-oriented format: "<key>\t<abs_path>\n"
pub fn load_marks(path: &Path) -> HashMap<char, PathBuf>
{
  let mut out = HashMap::new();
  let text = match fs::read_to_string(path)
  {
    Ok(s) => s,
    Err(_) => return out,
  };
  for line in text.lines()
  {
    let l = line.trim();
    if l.is_empty() || l.starts_with('#')
    {
      continue;
    }
    if let Some((k, p)) = l.split_once('\t')
      && let Some(ch) = k.chars().next()
    {
      let pb = PathBuf::from(p);
      out.insert(ch, pb);
    }
  }
  out
}

pub fn save_marks(
  path: &Path,
  marks: &HashMap<char, PathBuf>,
) -> io::Result<()>
{
  if let Some(parent) = path.parent()
  {
    let _ = fs::create_dir_all(parent);
  }
  let mut tmp = path.to_path_buf();
  tmp.set_extension("tmp");
  let mut f = fs::File::create(&tmp)?;
  // stable order
  let mut keys: Vec<char> = marks.keys().copied().collect();
  keys.sort();
  for k in keys
  {
    if let Some(p) = marks.get(&k)
    {
      let _ = writeln!(f, "{}\t{}", k, p.display());
    }
  }
  f.flush()?;
  fs::rename(tmp, path)?;
  Ok(())
}
