use super::super::TextArea;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn textarea(text: &str) -> TextArea {
    let mut area = TextArea::new();
    area.insert_str(text);
    area.set_vim_enabled(true);
    area
}

#[test]
fn vim_insert_and_escape_preserve_normal_cursor_contract() {
    let mut area = TextArea::new();
    area.set_vim_enabled(true);
    area.input(key(KeyCode::Char('i')));
    area.input(key(KeyCode::Char('h')));
    area.input(key(KeyCode::Esc));

    assert_eq!(area.text(), "h");
    assert_eq!(
        area.vim_mode_indicator_span()
            .expect("vim indicator")
            .content,
        "Vim: Normal"
    );
    assert_eq!(area.cursor(), 0);
}

#[test]
fn vim_normal_motion_and_delete_respect_grapheme_boundaries() {
    let mut area = textarea("a👩🏽‍💻c");
    area.set_cursor(0);
    area.input(key(KeyCode::Char('l')));
    assert_eq!(area.cursor(), "a".len());
    area.input(key(KeyCode::Char('x')));
    assert_eq!(area.text(), "ac");
    assert_eq!(area.cursor(), "a".len());
}

#[test]
fn vim_operator_pending_supports_word_delete_and_escape_cancel() {
    let mut area = textarea("hello world");
    area.set_cursor(0);
    area.input(key(KeyCode::Char('d')));
    assert!(area.is_vim_operator_pending());
    area.input(key(KeyCode::Char('w')));
    assert_eq!(area.text(), "world");
    assert!(!area.is_vim_operator_pending());

    area.input(key(KeyCode::Char('d')));
    area.input(key(KeyCode::Esc));
    assert_eq!(area.text(), "world");
    assert!(!area.is_vim_operator_pending());
}

#[test]
fn vim_find_and_till_stay_on_current_line_and_grapheme_boundaries() {
    let mut area = textarea("a👩🏽‍💻:b:c\nnext");
    area.set_cursor(0);
    area.input(key(KeyCode::Char('f')));
    area.input(key(KeyCode::Char(':')));
    assert_eq!(area.cursor(), "a👩🏽‍💻".len());

    area.set_cursor("a👩🏽‍💻:".len());
    area.input(key(KeyCode::Char('t')));
    area.input(key(KeyCode::Char('c')));
    assert_eq!(area.cursor(), "a👩🏽‍💻:b".len());

    area.set_cursor("a👩🏽‍💻:b".len());
    area.input(key(KeyCode::Char('F')));
    area.input(key(KeyCode::Char(':')));
    assert_eq!(area.cursor(), "a👩🏽‍💻".len());

    area.set_cursor("a👩🏽‍💻:b:c".len());
    area.input(key(KeyCode::Char('T')));
    area.input(key(KeyCode::Char(':')));
    assert_eq!(area.cursor(), "a👩🏽‍💻:b:".len());

    area.set_cursor(0);
    area.input(key(KeyCode::Char('f')));
    area.input(key(KeyCode::Char('n')));
    assert_eq!(area.cursor(), 0);
}

#[test]
fn vim_find_operators_delete_change_and_yank() {
    let mut area = textarea("one:two:three");
    area.set_cursor(0);
    area.input(key(KeyCode::Char('d')));
    area.input(key(KeyCode::Char('f')));
    area.input(key(KeyCode::Char(':')));
    assert_eq!(area.text(), "two:three");

    let mut area = textarea("abc:def");
    area.set_cursor(0);
    area.input(key(KeyCode::Char('c')));
    area.input(key(KeyCode::Char('t')));
    area.input(key(KeyCode::Char(':')));
    assert_eq!(area.text(), ":def");
    assert_eq!(
        area.vim_mode_indicator_span()
            .expect("vim indicator")
            .content,
        "Vim: Insert"
    );

    let mut area = textarea("abc:def");
    area.set_cursor(0);
    area.input(key(KeyCode::Char('y')));
    area.input(key(KeyCode::Char('f')));
    area.input(key(KeyCode::Char(':')));
    area.input(key(KeyCode::Char('p')));
    assert_eq!(area.text(), "aabc:bc:def");
}

#[test]
fn vim_text_objects_delete_inner_around_words_and_pairs() {
    let mut area = textarea("alpha beta gamma");
    area.set_cursor("alpha ".len());
    area.input(key(KeyCode::Char('d')));
    area.input(key(KeyCode::Char('i')));
    area.input(key(KeyCode::Char('w')));
    assert_eq!(area.text(), "alpha  gamma");

    let mut area = textarea("alpha beta gamma");
    area.set_cursor("alpha ".len());
    area.input(key(KeyCode::Char('d')));
    area.input(key(KeyCode::Char('a')));
    area.input(key(KeyCode::Char('w')));
    assert_eq!(area.text(), "alpha gamma");

    let mut area = textarea("call(one, [two])");
    area.set_cursor("call(one, [t".len());
    area.input(key(KeyCode::Char('d')));
    area.input(key(KeyCode::Char('i')));
    area.input(key(KeyCode::Char('[')));
    assert_eq!(area.text(), "call(one, [])");

    let mut area = textarea("say(\"hello\")");
    area.set_cursor("say(\"he".len());
    area.input(key(KeyCode::Char('d')));
    area.input(key(KeyCode::Char('a')));
    area.input(key(KeyCode::Char('"')));
    assert_eq!(area.text(), "say()");
}

#[test]
fn vim_replace_mode_overwrites_one_grapheme_and_returns_to_normal() {
    let mut area = textarea("a👩🏽‍💻c");
    area.set_cursor("a".len());
    area.input(key(KeyCode::Char('r')));
    area.input(key(KeyCode::Char('Z')));
    assert_eq!(area.text(), "aZc");
    assert_eq!(
        area.vim_mode_indicator_span()
            .expect("vim indicator")
            .content,
        "Vim: Normal"
    );

    area.input(key(KeyCode::Char('R')));
    area.input(key(KeyCode::Char('X')));
    area.input(key(KeyCode::Esc));
    assert_eq!(area.text(), "aXc");
    assert_eq!(
        area.vim_mode_indicator_span()
            .expect("vim indicator")
            .content,
        "Vim: Normal"
    );
}

#[test]
fn vim_replace_mode_backspace_restores_original_graphemes() {
    let mut area = textarea("a👩🏽‍💻");
    area.set_cursor(0);
    area.input(key(KeyCode::Char('R')));
    area.input(key(KeyCode::Char('X')));
    area.input(key(KeyCode::Char('Y')));
    area.input(key(KeyCode::Char('Z')));
    assert_eq!(area.text(), "XYZ");

    for expected in ["XY", "X👩🏽‍💻", "a👩🏽‍💻", "a👩🏽‍💻"] {
        area.input(key(KeyCode::Backspace));
        assert_eq!(area.text(), expected);
    }
    assert_eq!(area.cursor(), 0);
}

#[test]
fn vim_replace_mode_enter_is_inserted_and_restored_as_one_step() {
    let mut area = textarea("abc");
    area.set_cursor(0);
    area.input(key(KeyCode::Char('R')));
    area.input(key(KeyCode::Enter));
    area.input(key(KeyCode::Char('X')));
    assert_eq!(area.text(), "\nXbc");

    area.input(key(KeyCode::Backspace));
    assert_eq!(area.text(), "\nabc");
    area.input(key(KeyCode::Backspace));
    assert_eq!(area.text(), "abc");
}

#[test]
fn vim_mode_indicator_is_hidden_when_disabled_and_colored_when_enabled() {
    let mut area = TextArea::new();
    assert!(area.vim_mode_indicator_span().is_none());
    area.set_vim_enabled(true);
    let span = area.vim_mode_indicator_span().expect("vim indicator");
    assert_eq!(span.content, "Vim: Normal");
    assert_eq!(span.style.fg, Some(ratatui::style::Color::Magenta));
}
