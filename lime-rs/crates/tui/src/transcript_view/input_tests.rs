use crossterm::event::{KeyEvent, MouseEvent};
use ratatui::layout::Rect;
use ratatui::text::Line;

use super::*;
use crate::terminal_hyperlinks::{HyperlinkLine, TerminalHyperlink};

fn line(text: &str) -> HyperlinkLine {
    HyperlinkLine::new(Line::raw(text.to_string()))
}

fn linked(text: &str, columns: std::ops::Range<usize>, destination: &str) -> HyperlinkLine {
    let mut line = line(text);
    line.hyperlinks
        .push(TerminalHyperlink::web(columns, destination.to_string()));
    line
}

fn key(code: KeyCode, modifiers: KeyModifiers) -> Event {
    Event::Key(KeyEvent::new(code, modifiers))
}

fn mouse(kind: MouseEventKind, column: u16, row: u16, modifiers: KeyModifiers) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers,
    })
}

#[test]
fn ctrl_space_starts_at_viewport_top_and_arrows_move_by_grapheme() {
    let owner = TranscriptSelection::default();
    let lines = vec![line("A👨‍👩‍👧‍👦界e\u{301}Z")];
    owner.update_layout(Rect::new(2, 3, 20, 2), 0, &lines);

    assert_eq!(
        owner.handle_event(&key(KeyCode::Char(' '), KeyModifiers::CONTROL)),
        Some(TranscriptSelectionAction::Consumed)
    );
    for _ in 0..4 {
        assert!(matches!(
            owner.handle_event(&key(KeyCode::Right, KeyModifiers::NONE)),
            Some(TranscriptSelectionAction::RevealRow(0))
        ));
    }
    assert_eq!(owner.selected_text().as_deref(), Some("A👨‍👩‍👧‍👦界e\u{301}"));
}

#[test]
fn horizontal_keyboard_selection_crosses_canonical_line_boundaries() {
    let owner = TranscriptSelection::default();
    let lines = vec![line("a"), line("b")];
    owner.update_layout(Rect::new(0, 0, 20, 2), 0, &lines);
    owner.handle_event(&key(KeyCode::Char(' '), KeyModifiers::CONTROL));

    owner.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    assert_eq!(owner.selected_text().as_deref(), Some("a"));
    owner.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    assert_eq!(owner.selected_text().as_deref(), Some("a\n"));
    owner.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    assert_eq!(owner.selected_text().as_deref(), Some("a\nb"));
    owner.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    owner.handle_event(&key(KeyCode::Left, KeyModifiers::NONE));
    assert_eq!(owner.selected_text().as_deref(), Some("a"));
}

#[test]
fn keyboard_selection_skips_synthetic_disclosure_rows() {
    let owner = TranscriptSelection::default();
    let lines = vec![line("a"), line("+ Show details"), line("b")];
    owner.update_layout_with_exclusions(
        Rect::new(0, 0, 20, 3),
        0,
        &lines,
        &std::collections::HashSet::from([1]),
    );
    owner.handle_event(&key(KeyCode::Char(' '), KeyModifiers::CONTROL));

    owner.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    owner.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    assert_eq!(
        owner.selection.borrow().as_ref().expect("selection").end,
        Anchor { line: 2, offset: 0 }
    );
    owner.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    assert_eq!(owner.selected_text().as_deref(), Some("a\nb"));
}

#[test]
fn vertical_keyboard_selection_keeps_preferred_display_column() {
    let owner = TranscriptSelection::default();
    let lines = vec![line("abcdef"), line("x"), line("123456")];
    owner.update_layout(Rect::new(0, 0, 4, 5), 0, &lines);
    owner.handle_event(&key(KeyCode::Char(' '), KeyModifiers::CONTROL));
    for _ in 0..3 {
        owner.handle_event(&key(KeyCode::Right, KeyModifiers::NONE));
    }
    let original = owner.selection.borrow().as_ref().expect("selection").end;

    for code in [KeyCode::Down, KeyCode::Down, KeyCode::Up, KeyCode::Up] {
        assert!(matches!(
            owner.handle_event(&key(code, KeyModifiers::SHIFT)),
            Some(TranscriptSelectionAction::RevealRow(_))
        ));
    }
    assert_eq!(
        owner.selection.borrow().as_ref().expect("selection").end,
        original
    );
}

#[test]
fn keyboard_selection_requests_reveal_when_endpoint_leaves_viewport() {
    let owner = TranscriptSelection::default();
    let lines = vec![line("one"), line("two"), line("three")];
    owner.update_layout(Rect::new(0, 0, 10, 2), 0, &lines);
    owner.handle_event(&key(KeyCode::Char(' '), KeyModifiers::CONTROL));

    assert_eq!(
        owner.handle_event(&key(KeyCode::Down, KeyModifiers::NONE)),
        Some(TranscriptSelectionAction::RevealRow(1))
    );
    assert_eq!(
        owner.handle_event(&key(KeyCode::Down, KeyModifiers::NONE)),
        Some(TranscriptSelectionAction::RevealRow(2))
    );
}

#[test]
fn keyboard_selection_pages_the_frozen_viewport() {
    let owner = TranscriptSelection::default();
    let lines = (0..10)
        .map(|index| line(&format!("line {index}")))
        .collect::<Vec<_>>();
    owner.update_layout(Rect::new(0, 0, 20, 3), 4, &lines);
    owner.handle_event(&key(KeyCode::Char(' '), KeyModifiers::CONTROL));

    assert_eq!(
        owner.handle_event(&key(KeyCode::PageUp, KeyModifiers::NONE)),
        Some(TranscriptSelectionAction::Scroll { rows: -3 })
    );
    assert!(owner.scroll_rows(-3));
    assert_eq!(owner.layout.borrow().as_ref().expect("layout").scroll, 1);
    assert_eq!(
        owner.handle_event(&key(KeyCode::PageDown, KeyModifiers::NONE)),
        Some(TranscriptSelectionAction::Scroll { rows: 3 })
    );
}

#[test]
fn wheel_scrolls_three_rows_inside_transcript() {
    let owner = TranscriptSelection::default();
    owner.update_layout(Rect::new(2, 3, 20, 2), 0, &[line("text")]);

    assert_eq!(
        owner.handle_event(&mouse(MouseEventKind::ScrollUp, 2, 3, KeyModifiers::NONE,)),
        Some(TranscriptSelectionAction::Scroll { rows: -3 })
    );
    assert_eq!(
        owner.handle_event(&mouse(MouseEventKind::ScrollDown, 2, 3, KeyModifiers::NONE,)),
        Some(TranscriptSelectionAction::Scroll { rows: 3 })
    );
    assert!(owner
        .handle_event(&mouse(MouseEventKind::ScrollDown, 0, 0, KeyModifiers::NONE,))
        .is_none());
}

#[test]
fn stationary_link_click_opens_on_release_and_modified_click_opens_immediately() {
    let owner = TranscriptSelection::default();
    let lines = vec![linked("docs", 0..4, "https://example.com/docs")];
    owner.update_layout(Rect::new(2, 1, 20, 2), 0, &lines);

    assert_eq!(
        owner.handle_event(&mouse(
            MouseEventKind::Down(MouseButton::Left),
            3,
            1,
            KeyModifiers::NONE,
        )),
        Some(TranscriptSelectionAction::Consumed)
    );
    assert_eq!(
        owner.handle_event(&mouse(
            MouseEventKind::Up(MouseButton::Left),
            3,
            1,
            KeyModifiers::NONE,
        )),
        Some(TranscriptSelectionAction::OpenLink(
            "https://example.com/docs".to_string()
        ))
    );
    assert!(owner.selection.borrow().is_none());

    assert_eq!(
        owner.handle_event(&mouse(
            MouseEventKind::Down(MouseButton::Left),
            3,
            1,
            KeyModifiers::CONTROL,
        )),
        Some(TranscriptSelectionAction::OpenLink(
            "https://example.com/docs".to_string()
        ))
    );
    assert!(owner.selection.borrow().is_none());
}

#[test]
fn dragging_or_scrolling_cancels_link_activation() {
    for interruption in [
        mouse(
            MouseEventKind::Drag(MouseButton::Left),
            5,
            1,
            KeyModifiers::NONE,
        ),
        mouse(MouseEventKind::ScrollDown, 3, 1, KeyModifiers::NONE),
    ] {
        let owner = TranscriptSelection::default();
        let lines = vec![linked("docs more", 0..4, "https://example.com/docs")];
        owner.update_layout(Rect::new(2, 1, 20, 2), 0, &lines);
        owner.handle_event(&mouse(
            MouseEventKind::Down(MouseButton::Left),
            3,
            1,
            KeyModifiers::NONE,
        ));
        assert!(!matches!(
            owner.handle_event(&interruption),
            Some(TranscriptSelectionAction::OpenLink(_))
        ));
        assert!(!matches!(
            owner.handle_event(&mouse(
                MouseEventKind::Up(MouseButton::Left),
                3,
                1,
                KeyModifiers::NONE,
            )),
            Some(TranscriptSelectionAction::OpenLink(_))
        ));
    }
}

#[test]
fn only_a_vertical_edge_drag_requests_auto_scroll() {
    let owner = TranscriptSelection::default();
    let lines = (0..10)
        .map(|index| line(&format!("line {index}")))
        .collect::<Vec<_>>();
    owner.update_layout(Rect::new(0, 0, 20, 3), 3, &lines);

    owner.handle_event(&mouse(
        MouseEventKind::Down(MouseButton::Left),
        0,
        0,
        KeyModifiers::NONE,
    ));
    owner.handle_event(&mouse(
        MouseEventKind::Drag(MouseButton::Left),
        4,
        0,
        KeyModifiers::NONE,
    ));
    assert_eq!(owner.edge_scroll_direction(), None);

    owner.handle_event(&mouse(
        MouseEventKind::Drag(MouseButton::Left),
        4,
        1,
        KeyModifiers::NONE,
    ));
    owner.handle_event(&mouse(
        MouseEventKind::Drag(MouseButton::Left),
        4,
        0,
        KeyModifiers::NONE,
    ));
    assert_eq!(owner.edge_scroll_direction(), Some(-1));
}

#[test]
fn focus_loss_stops_edge_drag_without_discarding_selected_text() {
    let owner = TranscriptSelection::default();
    let lines = vec![line("one"), line("two"), line("three")];
    owner.update_layout(Rect::new(0, 0, 20, 2), 0, &lines);
    owner.handle_event(&mouse(
        MouseEventKind::Down(MouseButton::Left),
        0,
        0,
        KeyModifiers::NONE,
    ));
    owner.handle_event(&mouse(
        MouseEventKind::Drag(MouseButton::Left),
        3,
        1,
        KeyModifiers::NONE,
    ));
    assert_eq!(owner.edge_scroll_direction(), Some(1));
    let selected = owner.selected_text();
    assert_eq!(selected.as_deref(), Some("one\ntwo"));

    assert_eq!(
        owner.handle_event(&Event::FocusLost),
        Some(TranscriptSelectionAction::Consumed)
    );
    assert!(!owner.is_dragging());
    assert_eq!(owner.edge_scroll_direction(), None);
    assert_eq!(owner.selected_text(), selected);
}

#[test]
fn focus_loss_keeps_an_empty_keyboard_selection_ready_to_extend() {
    let owner = TranscriptSelection::default();
    owner.update_layout(Rect::new(0, 0, 20, 2), 0, &[line("text")]);
    owner.handle_event(&key(KeyCode::Char(' '), KeyModifiers::CONTROL));

    owner.handle_event(&Event::FocusLost);
    assert!(owner.selection.borrow().is_some());
    assert_eq!(
        owner.handle_event(&key(KeyCode::Right, KeyModifiers::NONE)),
        Some(TranscriptSelectionAction::RevealRow(0))
    );
    assert_eq!(owner.selected_text().as_deref(), Some("t"));
}

#[test]
fn wheel_pauses_edge_drag_until_another_drag_event() {
    let owner = TranscriptSelection::default();
    let lines = (0..10)
        .map(|index| line(&format!("line {index}")))
        .collect::<Vec<_>>();
    owner.update_layout(Rect::new(0, 0, 20, 3), 2, &lines);
    owner.handle_event(&mouse(
        MouseEventKind::Down(MouseButton::Left),
        0,
        1,
        KeyModifiers::NONE,
    ));
    let edge_drag = mouse(
        MouseEventKind::Drag(MouseButton::Left),
        4,
        2,
        KeyModifiers::NONE,
    );
    owner.handle_event(&edge_drag);
    assert_eq!(owner.edge_scroll_direction(), Some(1));

    assert_eq!(
        owner.handle_event(&mouse(MouseEventKind::ScrollDown, 4, 2, KeyModifiers::NONE,)),
        Some(TranscriptSelectionAction::Scroll { rows: 3 })
    );
    assert_eq!(owner.edge_scroll_direction(), None);

    owner.handle_event(&edge_drag);
    assert_eq!(owner.edge_scroll_direction(), Some(1));
}
