use std::fs;

#[test]
fn copy_move_remove_paths()
{
  let tmp = tempfile::tempdir().expect("tmp");
  let root = tmp.path();

  // Build a small tree: a/one.txt and a/sub/two.txt
  let a = root.join("a");
  let a_sub = a.join("sub");
  fs::create_dir_all(&a_sub).unwrap();
  let one = a.join("one.txt");
  let two = a_sub.join("two.txt");
  fs::write(&one, b"ONE").unwrap();
  fs::write(&two, b"TWO").unwrap();

  // Copy a -> b
  let b = root.join("b");
  lsv::core::fs_ops::copy_path_recursive(&a, &b).expect("copy");
  assert_eq!(fs::read(b.join("one.txt")).unwrap(), b"ONE");
  assert_eq!(fs::read(b.join("sub").join("two.txt")).unwrap(), b"TWO");

  // Move b -> c (rename or fallback); ensure b removed, c exists
  let c = root.join("c");
  lsv::core::fs_ops::move_path_with_fallback(&b, &c).expect("move");
  assert!(!b.exists());
  assert!(c.join("one.txt").exists());
  assert!(c.join("sub").join("two.txt").exists());

  // Remove c
  lsv::core::fs_ops::remove_path_all(&c).expect("remove");
  assert!(!c.exists());
}
