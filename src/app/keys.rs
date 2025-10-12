//! Key map utilities and helpers on `App`.

use crate::app::App;

use crate::keymap::tokenize_sequence;

impl App
{
  pub(crate) fn rebuild_keymap_lookup(&mut self)
  {
    self.keys.lookup.clear();
    self.keys.prefixes.clear();
    for m in &self.keys.maps
    {
      self.keys.lookup.insert(m.sequence.clone(), m.action.clone());
      // collect token-based prefixes for sequence matching
      let tokens = tokenize_sequence(&m.sequence);
      let mut acc = String::new();
      for (idx, t) in tokens.iter().enumerate()
      {
        acc.push_str(t);
        if idx + 1 < tokens.len()
        {
          self.keys.prefixes.insert(acc.clone());
        }
      }
    }
  }

  pub fn set_keymaps(
    &mut self,
    maps: Vec<crate::config::KeyMapping>,
  )
  {
    self.keys.maps = maps;
    self.rebuild_keymap_lookup();
  }

  pub fn get_keymap_action(
    &self,
    seq: &str,
  ) -> Option<String>
  {
    self.keys.lookup.get(seq).cloned()
  }

  pub fn has_prefix(
    &self,
    seq: &str,
  ) -> bool
  {
    self.keys.prefixes.contains(seq)
  }
}
