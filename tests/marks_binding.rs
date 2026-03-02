// Keymap smoke test: ensure default bindings exist for marks + vim-style nav.
#[test]
fn default_key_bindings_for_marks_and_vim_nav_exist() {
    let app = lsv::app::App::new().expect("app new");

    assert_eq!(app.get_keymap_action("'"), Some("marks:goto_wait".into()));
    assert_eq!(app.get_keymap_action("m"), Some("marks:add_wait".into()));

    assert_eq!(app.get_keymap_action("h"), Some("nav:parent".into()));
    assert_eq!(app.get_keymap_action("j"), Some("nav:down".into()));
    assert_eq!(app.get_keymap_action("k"), Some("nav:up".into()));
    assert_eq!(app.get_keymap_action("l"), Some("nav:enter".into()));
}
