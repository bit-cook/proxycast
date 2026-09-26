use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use pretty_assertions::assert_eq;
use ratatui::widgets::StatefulWidgetRef;
use unicode_segmentation::UnicodeSegmentation;

fn render(textarea: &TextArea, area: Rect, state: &mut TextAreaState) -> Buffer {
    let mut buffer = Buffer::empty(area);
    StatefulWidgetRef::render_ref(&textarea, area, &mut buffer, state);
    buffer
}

fn mouse(textarea: &mut TextArea, state: TextAreaState, kind: MouseEventKind, x: u16, y: u16) {
    assert!(textarea.handle_mouse(
        MouseEvent {
            kind,
            column: x,
            row: y,
            modifiers: KeyModifiers::NONE,
        },
        state,
    ));
}

#[test]
fn mouse_click_maps_wrapping_unicode_tabs_and_empty_lines() {
    let area = Rect::new(3, 2, 8, 8);
    let mut textarea = TextArea::new();
    let text = "ab 界e\u{301}👩‍💻\n\nx\ty";
    textarea.insert_str(text);
    let mut state = TextAreaState::default();
    render(&textarea, area, &mut state);
    for (pos, _) in text
        .grapheme_indices(true)
        .chain(std::iter::once((text.len(), "")))
    {
        textarea.set_cursor(pos);
        let (x, y) = textarea.cursor_pos_with_state(area, state).expect("cursor");
        mouse(
            &mut textarea,
            state,
            MouseEventKind::Down(MouseButton::Left),
            x,
            y,
        );
        assert_eq!(textarea.cursor(), pos);
    }
    mouse(
        &mut textarea,
        state,
        MouseEventKind::Down(MouseButton::Left),
        7,
        2,
    );
    textarea.insert_str("!");
    assert_eq!(textarea.text(), "ab !界e\u{301}👩‍💻\n\nx\ty");
}

#[test]
fn mouse_drag_selection_renders_and_is_replaced_atomically() {
    let area = Rect::new(0, 1, 8, 2);
    let mut textarea = TextArea::new();
    textarea.insert_str("hello world");
    let mut state = TextAreaState::default();
    render(&textarea, area, &mut state);
    mouse(
        &mut textarea,
        state,
        MouseEventKind::Down(MouseButton::Left),
        1,
        1,
    );
    mouse(
        &mut textarea,
        state,
        MouseEventKind::Drag(MouseButton::Left),
        3,
        2,
    );
    mouse(
        &mut textarea,
        state,
        MouseEventKind::Up(MouseButton::Left),
        3,
        2,
    );
    assert_eq!(textarea.selected_text(), Some("ello wor"));
    let buffer = render(&textarea, area, &mut state);
    assert!(buffer
        .content
        .iter()
        .any(|cell| cell.modifier.contains(Modifier::REVERSED)));

    textarea.input(KeyEvent::new(KeyCode::Char('X'), KeyModifiers::NONE));
    assert_eq!(textarea.text(), "hXld");
    assert_eq!(textarea.cursor(), 2);
    assert!(textarea.selected_text().is_none());
}

#[test]
fn vim_replace_mode_replaces_mouse_selection_atomically() {
    let area = Rect::new(0, 1, 8, 2);
    let mut textarea = TextArea::new();
    textarea.insert_str("hello world");
    textarea.set_vim_enabled(true);
    textarea.input(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::NONE));
    textarea.input(KeyEvent::new(KeyCode::Char('!'), KeyModifiers::NONE));
    textarea.input(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    textarea.input(KeyEvent::new(KeyCode::Char('R'), KeyModifiers::NONE));

    textarea.set_cursor(0);
    let mut state = TextAreaState::default();
    render(&textarea, area, &mut state);
    mouse(
        &mut textarea,
        state,
        MouseEventKind::Down(MouseButton::Left),
        1,
        1,
    );
    mouse(
        &mut textarea,
        state,
        MouseEventKind::Drag(MouseButton::Left),
        3,
        2,
    );
    mouse(
        &mut textarea,
        state,
        MouseEventKind::Up(MouseButton::Left),
        3,
        2,
    );

    textarea.input(KeyEvent::new(KeyCode::Char('X'), KeyModifiers::NONE));
    assert_eq!(textarea.text(), "hXld!");
    assert_eq!(textarea.cursor(), 2);
    assert!(textarea.selected_text().is_none());
}

#[test]
fn mouse_selection_preserves_hyperlink_metadata_and_style() {
    let url = "https://example.test/path";
    let area = Rect::new(0, 0, 40, 2);
    let mut textarea = TextArea::new();
    textarea.insert_str(url);
    let mut state = TextAreaState::default();
    render(&textarea, area, &mut state);
    mouse(
        &mut textarea,
        state,
        MouseEventKind::Down(MouseButton::Left),
        0,
        0,
    );
    mouse(
        &mut textarea,
        state,
        MouseEventKind::Drag(MouseButton::Left),
        7,
        0,
    );
    mouse(
        &mut textarea,
        state,
        MouseEventKind::Up(MouseButton::Left),
        7,
        0,
    );

    let buffer = render(&textarea, area, &mut state);
    for column in 0..7 {
        let cell = &buffer[(column, 0)];
        assert!(cell.modifier.contains(Modifier::REVERSED));
        assert!(cell.symbol().contains(&format!("\x1b]8;;{url}\x07")));
    }
}

#[test]
fn double_and_triple_click_select_word_and_logical_line() {
    let area = Rect::new(0, 0, 8, 4);
    for (prior_clicks, expected) in [(1, "two"), (2, "one two six\n")] {
        let mut textarea = TextArea::new();
        textarea.insert_str("one two six\nnext");
        let mut state = TextAreaState::default();
        render(&textarea, area, &mut state);
        mouse(
            &mut textarea,
            state,
            MouseEventKind::Down(MouseButton::Left),
            5,
            0,
        );
        textarea.last_click = Some((std::time::Instant::now(), 5, 0, prior_clicks));
        mouse(
            &mut textarea,
            state,
            MouseEventKind::Down(MouseButton::Left),
            5,
            0,
        );
        assert_eq!(textarea.selected_text(), Some(expected));
    }
}
