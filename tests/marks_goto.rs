use crossterm::event::{
    KeyCode,
    KeyEvent,
    KeyModifiers,
};

#[test]
fn goto_mark_moves_cwd_to_saved_directory()
{
    let tmp1 = tempfile::tempdir().expect("tmp1");
    let tmp2 = tempfile::tempdir().expect("tmp2");
    let dir1 = tmp1.path().to_path_buf();
    let dir2 = tmp2.path().to_path_buf();

    let mut app = lsv::app::App::new().expect("app new");
    app.set_cwd(&dir1);

    // Save mark 'a' for dir1 via key flow: 'm' then 'a' then Enter
    let _ = lsv::input::handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE),
    )
    .expect("handle m");
    let _ = lsv::input::handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
    )
    .expect("handle a for mark");
    let _ = lsv::input::handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    )
    .expect("handle enter to save mark");

    // Change directory to dir2
    app.set_cwd(&dir2);
    assert_eq!(app.get_cwd_path(), dir2);

    // Goto mark 'a' via key flow: '\'' then 'a'
    let _ = lsv::input::handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('\''), KeyModifiers::NONE),
    )
    .expect("handle quote");
    let _ = lsv::input::handle_key(
        &mut app,
        KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE),
    )
    .expect("handle a for goto");

    // Should've moved back to dir1
    assert_eq!(app.get_cwd_path(), dir1);
}
