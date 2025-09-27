use crossterm::event::KeyModifiers;

/// Split a key sequence string into tokens, preserving modifier tokens like
/// "<C-x>" as single units.
pub fn tokenize_sequence(seq: &str) -> Vec<String>
{
  let mut toks = Vec::new();
  let mut i = 0;
  let b = seq.as_bytes();
  while i < b.len()
  {
    if b[i] == b'<'
      && let Some(j) = seq[i + 1..].find('>')
    {
      let end = i + 1 + j + 1;
      toks.push(seq[i..end].to_string());
      i = end;
      continue;
    }
    let ch = seq[i..].chars().next().unwrap();
    toks.push(ch.to_string());
    i += ch.len_utf8();
  }
  toks
}

/// Build a key token from a character and its modifiers.
/// Examples: 'x' -> "x", Ctrl-x -> "<C-x>", Alt-Space -> "<M- >"
pub fn build_token(
  ch: char,
  mods: KeyModifiers,
) -> String
{
  let ctrl = mods.contains(KeyModifiers::CONTROL);
  let alt = mods.contains(KeyModifiers::ALT);
  let superm = mods.contains(KeyModifiers::SUPER);
  let shift = mods.contains(KeyModifiers::SHIFT);
  if ctrl || alt || superm || (shift && !ch.is_ascii_alphabetic())
  {
    let mut tok = String::from("<");
    if ctrl
    {
      tok.push_str("C-");
    }
    if alt
    {
      tok.push_str("M-");
    }
    if superm
    {
      tok.push_str("S-");
    }
    if shift && !ch.is_ascii_alphabetic()
    {
      tok.push_str("Sh-");
    }
    tok.push(ch);
    tok.push('>');
    tok
  }
  else
  {
    ch.to_string()
  }
}
