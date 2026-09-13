use super::super::TextArea;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn keys(area: &mut TextArea, text: &str) {
    for ch in text.chars() {
        area.input(key(KeyCode::Char(ch)));
    }
}

fn textarea(text: &str) -> TextArea {
    let mut area = TextArea::new();
    area.insert_str(text);
    area.set_vim_enabled(true);
    area
}

#[test]
fn forward_search_accepts_unicode_query_and_repeats_with_n_and_previous() {
    let mut area = textarea("one 👩🏽‍💻 two 👩🏽‍💻");
    area.set_cursor(0);

    area.input(key(KeyCode::Char('/')));
    keys(&mut area, "👩🏽‍💻");
    assert!(area.vim_search_query().is_some());
    area.input(key(KeyCode::Enter));

    let first = "one ".len();
    let second = "one 👩🏽‍💻 two ".len();
    assert_eq!(area.cursor(), first);
    area.input(key(KeyCode::Char('n')));
    assert_eq!(area.cursor(), second);
    area.input(key(KeyCode::Char('N')));
    assert_eq!(area.cursor(), first);
}

#[test]
fn search_cancel_preserves_draft_and_clears_operator_pending() {
    let mut area = textarea("hello world");
    area.input(key(KeyCode::Char('d')));
    area.input(key(KeyCode::Char('/')));
    keys(&mut area, "world");
    area.input(key(KeyCode::Esc));

    assert_eq!(area.text(), "hello world");
    assert!(!area.is_vim_operator_pending());
    assert!(area.vim_search_query().is_none());
}

#[test]
fn search_motion_can_complete_delete_operator() {
    let mut area = textarea("one two three");
    area.set_cursor(0);
    area.input(key(KeyCode::Char('d')));
    area.input(key(KeyCode::Char('/')));
    keys(&mut area, "three");
    area.input(key(KeyCode::Enter));

    assert_eq!(area.text(), "three");
    assert_eq!(area.cursor(), 0);
}

#[test]
fn empty_repeat_search_is_a_noop_and_backward_search_finds_previous_match() {
    let mut area = textarea("alpha beta alpha");
    area.set_cursor(0);
    area.input(key(KeyCode::Char('n')));
    assert_eq!(area.cursor(), 0);

    area.set_cursor(area.text().len());
    area.input(key(KeyCode::Char('?')));
    keys(&mut area, "alpha");
    area.input(key(KeyCode::Enter));
    assert_eq!(area.cursor(), "alpha beta ".len());
}
