use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::buffer::Buffer;
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;

use super::*;
use crate::terminal_hyperlinks::{HyperlinkParagraph, TerminalHyperlink};
use crate::transcript_view::TranscriptSelectionAction;

fn line(text: &str) -> HyperlinkLine {
    HyperlinkLine::new(Line::raw(text.to_string()))
}

fn mouse(kind: MouseEventKind, column: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

fn select(owner: &TranscriptSelection, start: (u16, u16), end: (u16, u16)) {
    assert_eq!(
        owner.handle_event(&mouse(
            MouseEventKind::Down(MouseButton::Left),
            start.0,
            start.1,
        )),
        Some(TranscriptSelectionAction::Consumed)
    );
    assert_eq!(
        owner.handle_event(&mouse(
            MouseEventKind::Drag(MouseButton::Left),
            end.0,
            end.1,
        )),
        Some(TranscriptSelectionAction::Consumed)
    );
    assert_eq!(
        owner.handle_event(&mouse(MouseEventKind::Up(MouseButton::Left), end.0, end.1,)),
        Some(TranscriptSelectionAction::Consumed)
    );
}

#[test]
fn wrapped_rows_copy_source_text_without_soft_newlines() {
    let owner = TranscriptSelection::default();
    let lines = vec![line("alpha beta gamma")];
    owner.update_layout(Rect::new(0, 0, 6, 4), 0, &lines);

    select(&owner, (0, 0), (4, 2));

    let Some(TranscriptSelectionAction::Copy { text, follow }) = owner.handle_event(&Event::Key(
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL),
    )) else {
        panic!("copy action");
    };
    assert_eq!(text, "alpha beta gamm");
    assert!(!follow);
    assert!(!text.contains('\n'));
}

#[test]
fn cross_line_selection_keeps_only_canonical_hard_newlines() {
    let owner = TranscriptSelection::default();
    let lines = vec![line("alpha beta"), line("gamma")];
    owner.update_layout(Rect::new(0, 0, 6, 4), 0, &lines);

    select(&owner, (0, 0), (5, 2));

    assert_eq!(owner.selected_text().as_deref(), Some("alpha beta\ngamma"));
}

#[test]
fn selection_geometry_uses_the_same_row_count_as_hyperlink_paragraph() {
    for text in [
        "alpha beta gamma",
        "  leading spaces",
        "word    gap",
        "你好 世界🙂",
        "e\u{301}lan and emoji 👩🏽‍💻",
        "",
    ] {
        let source = line(text);
        for width in 1..=12 {
            let expected = HyperlinkParagraph::new(std::slice::from_ref(&source))
                .line_count(width)
                .max(1);
            let actual = wrap_visual_rows(0, text, width, &source).len();
            assert_eq!(actual, expected, "text={text:?}, width={width}");
        }
    }
}

#[test]
fn hit_testing_stays_on_utf8_grapheme_boundaries_and_keeps_tabs() {
    let owner = TranscriptSelection::default();
    let lines = vec![line("你e\u{301}🙂\t好")];
    owner.update_layout(Rect::new(0, 0, 20, 2), 0, &lines);
    {
        let layout = owner.layout.borrow();
        let layout = layout.as_ref().expect("layout");
        for column in 0..10 {
            let Some(anchor) = layout.hit_test(column, 0) else {
                continue;
            };
            assert!(layout.texts[0].is_char_boundary(anchor.offset));
        }
    }

    owner.selection.replace(Some(Selection {
        snapshot: Arc::new(lines),
        excluded_lines: Arc::new(HashSet::new()),
        start: Anchor { line: 0, offset: 0 },
        end: Anchor {
            line: 0,
            offset: "你e\u{301}🙂\t好".len(),
        },
        origin: (
            Anchor { line: 0, offset: 0 },
            Anchor {
                line: 0,
                offset: "你e\u{301}🙂\t好".len(),
            },
        ),
        unit: SelectionUnit::Character,
        dragging: false,
        moved: false,
        moved_vertically: false,
        pointer_origin_row: 0,
        pointer: None,
        pressed_link: None,
        preferred_column: None,
    }));
    assert_eq!(owner.selected_text().as_deref(), Some("你e\u{301}🙂\t好"));
}

#[test]
fn copied_text_removes_controls_but_preserves_newlines_and_tabs() {
    let owner = TranscriptSelection::default();
    let lines = vec![line("a\u{1b}b\tc"), line("next")];
    owner.selection.replace(Some(Selection {
        snapshot: Arc::new(lines),
        excluded_lines: Arc::new(HashSet::new()),
        start: Anchor { line: 0, offset: 0 },
        end: Anchor {
            line: 1,
            offset: "next".len(),
        },
        origin: (
            Anchor { line: 0, offset: 0 },
            Anchor {
                line: 1,
                offset: "next".len(),
            },
        ),
        unit: SelectionUnit::Character,
        dragging: false,
        moved: false,
        moved_vertically: false,
        pointer_origin_row: 0,
        pointer: None,
        pressed_link: None,
        preferred_column: None,
    }));

    assert_eq!(owner.selected_text().as_deref(), Some("ab\tc\nnext"));
}

#[test]
fn double_click_selects_word_and_triple_click_selects_logical_line() {
    let owner = TranscriptSelection::default();
    let lines = vec![line("hello world")];
    owner.update_layout(Rect::new(0, 0, 20, 2), 0, &lines);

    for _ in 0..2 {
        owner.handle_event(&mouse(MouseEventKind::Down(MouseButton::Left), 7, 0));
        owner.handle_event(&mouse(MouseEventKind::Up(MouseButton::Left), 7, 0));
    }
    assert_eq!(owner.selected_text().as_deref(), Some("world"));

    owner.handle_event(&mouse(MouseEventKind::Down(MouseButton::Left), 7, 0));
    owner.handle_event(&mouse(MouseEventKind::Up(MouseButton::Left), 7, 0));
    assert_eq!(owner.selected_text().as_deref(), Some("hello world"));
}

#[test]
fn right_click_and_copy_shortcuts_request_copy_but_escape_clears() {
    let owner = TranscriptSelection::default();
    let lines = vec![line("copy me")];
    owner.update_layout(Rect::new(2, 3, 20, 2), 0, &lines);
    select(&owner, (2, 3), (6, 3));

    assert_eq!(
        owner.handle_event(&mouse(MouseEventKind::Down(MouseButton::Right), 3, 3,)),
        Some(TranscriptSelectionAction::Copy {
            text: "copy".to_string(),
            follow: false,
        })
    );
    assert_eq!(
        owner.handle_event(&Event::Key(KeyEvent::new(
            KeyCode::Char('C'),
            KeyModifiers::CONTROL,
        ))),
        Some(TranscriptSelectionAction::Copy {
            text: "copy".to_string(),
            follow: false,
        })
    );
    assert_eq!(
        owner.handle_event(&Event::Key(KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::SUPER,
        ))),
        Some(TranscriptSelectionAction::Copy {
            text: "copy".to_string(),
            follow: false,
        })
    );
    assert_eq!(
        owner.handle_event(&Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        ))),
        Some(TranscriptSelectionAction::Copy {
            text: "copy".to_string(),
            follow: true,
        })
    );
    assert_eq!(
        owner.handle_event(&Event::Key(
            KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE,)
        )),
        Some(TranscriptSelectionAction::Consumed)
    );
    assert!(owner.selected_text().is_none());
}

#[test]
fn selection_snapshot_does_not_drift_when_projection_prepends_history() {
    let owner = TranscriptSelection::default();
    let current = vec![line("stable text")];
    owner.update_layout(Rect::new(0, 0, 20, 2), 0, &current);
    select(&owner, (0, 0), (6, 0));

    let updated = vec![line("older"), line("changed text")];
    owner.update_layout(Rect::new(0, 0, 20, 2), 0, &updated);

    assert_eq!(owner.selected_text().as_deref(), Some("stable"));
    assert_eq!(
        owner.snapshot_lines().as_deref().map(Vec::as_slice),
        Some(current.as_slice())
    );
}

#[test]
fn highlight_preserves_osc8_symbols_and_reverses_selected_cells() {
    let owner = TranscriptSelection::default();
    let mut linked = HyperlinkLine::new(Line::from(Span::raw("link")));
    linked.hyperlinks.push(TerminalHyperlink::web(
        0..4,
        "https://example.com".to_string(),
    ));
    let lines = vec![linked];
    let area = Rect::new(0, 0, 10, 1);
    owner.update_layout(area, 0, &lines);
    select(&owner, (0, 0), (3, 0));

    let mut buffer = Buffer::empty(area);
    HyperlinkParagraph::new(&lines).render(area, &mut buffer);
    owner.render_highlight(&mut buffer);

    assert!(buffer[(0, 0)].symbol().contains("https://example.com"));
    assert!(buffer[(0, 0)].modifier.contains(Modifier::REVERSED));
}

#[test]
fn events_outside_content_fail_closed() {
    let owner = TranscriptSelection::default();
    owner.update_layout(Rect::new(5, 5, 10, 2), 0, &[line("inside")]);
    assert_eq!(
        owner.handle_event(&mouse(MouseEventKind::Down(MouseButton::Left), 0, 0,)),
        None
    );
    assert!(owner.selection.borrow().is_none());
}
