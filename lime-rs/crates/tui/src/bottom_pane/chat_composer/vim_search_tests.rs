use super::super::{ChatComposer, InputResult};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn vim_search_query_and_paste_do_not_mutate_the_draft() {
    let mut composer = ChatComposer::default();
    composer.set_vim_enabled(true);
    composer.insert("alpha beta");
    composer.handle_key_event(key(KeyCode::Char('/')));
    composer.handle_paste("beta");

    assert_eq!(composer.text(), "alpha beta");
    assert_eq!(
        composer.vim_search_query().map(|(query, _)| query),
        Some("beta")
    );
}

#[test]
fn empty_vim_search_backspace_cancels_without_submission() {
    let mut composer = ChatComposer::default();
    composer.set_vim_enabled(true);
    composer.insert("draft");
    composer.handle_key_event(key(KeyCode::Char('/')));

    assert_eq!(
        composer.handle_key_event(key(KeyCode::Backspace)),
        InputResult::Changed
    );
    assert!(!composer.vim_search_active());
    assert_eq!(composer.text(), "draft");
}

#[test]
fn disabling_vim_cancels_an_active_query() {
    let mut composer = ChatComposer::default();
    composer.set_vim_enabled(true);
    composer.handle_key_event(key(KeyCode::Char('/')));
    assert!(composer.vim_search_active());

    composer.set_vim_enabled(false);

    assert!(!composer.vim_search_active());
}
