use super::*;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn activity_content(prefix: bool) -> TranscriptContent {
    let mut content = TranscriptContent::default();
    if prefix {
        content.push_lines(vec![HyperlinkLine::from("older")]);
    }
    content.push_activity(
        vec!["entry:tool-1".to_string()],
        vec![HyperlinkLine::from("tool")],
        vec![HyperlinkLine::from("tool"), HyperlinkLine::from("detail")],
    );
    content.push_lines(vec![HyperlinkLine::from("after")]);
    content
}

fn key(code: KeyCode) -> Event {
    Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
}

fn mouse(kind: MouseEventKind, column: u16, row: u16) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column,
        row,
        modifiers: KeyModifiers::NONE,
    })
}

fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn mouse_toggles_activity_and_identity_survives_prepend() {
    let mut overlay = PagerOverlay::transcript(Locale::EnUs);
    let mut terminal = Terminal::new(TestBackend::new(40, 8)).expect("terminal");
    terminal
        .draw(|frame| {
            overlay.render_transcript(frame, frame.area(), Locale::EnUs, &activity_content(false))
        })
        .expect("collapsed draw");
    assert!(buffer_text(&terminal).contains("+ Show details"));
    assert!(!buffer_text(&terminal).contains("detail\n"));

    assert_eq!(
        overlay.handle_event(&mouse(MouseEventKind::Down(MouseButton::Left), 5, 2)),
        PagerAction::Consumed
    );
    terminal
        .draw(|frame| {
            overlay.render_transcript(frame, frame.area(), Locale::EnUs, &activity_content(true))
        })
        .expect("expanded draw after prepend");
    let screen = buffer_text(&terminal);
    assert!(screen.contains("detail"), "{screen}");
    assert!(screen.contains("− Show less"), "{screen}");
}

#[test]
fn keyboard_focus_toggles_activity_details() {
    let mut overlay = PagerOverlay::transcript(Locale::EnUs);
    let content = activity_content(false);
    let mut terminal = Terminal::new(TestBackend::new(40, 8)).expect("terminal");
    terminal
        .draw(|frame| overlay.render_transcript(frame, frame.area(), Locale::EnUs, &content))
        .expect("collapsed draw");

    assert_eq!(
        overlay.handle_event(&key(KeyCode::F(4))),
        PagerAction::Consumed
    );
    assert_eq!(
        overlay.handle_event(&key(KeyCode::Enter)),
        PagerAction::Consumed
    );
    terminal
        .draw(|frame| overlay.render_transcript(frame, frame.area(), Locale::EnUs, &content))
        .expect("expanded draw");
    let screen = buffer_text(&terminal);
    assert!(screen.contains("detail"), "{screen}");
    assert!(screen.contains("previous/next"), "{screen}");
}

#[test]
fn disclosure_controls_are_excluded_from_copy_and_search_source() {
    let mut overlay = PagerOverlay::transcript(Locale::EnUs);
    let content = activity_content(false);
    let mut terminal = Terminal::new(TestBackend::new(40, 8)).expect("terminal");
    terminal
        .draw(|frame| overlay.render_transcript(frame, frame.area(), Locale::EnUs, &content))
        .expect("draw");

    for event in [
        mouse(MouseEventKind::Down(MouseButton::Left), 0, 1),
        mouse(MouseEventKind::Drag(MouseButton::Left), 5, 3),
        mouse(MouseEventKind::Up(MouseButton::Left), 5, 3),
    ] {
        assert_eq!(overlay.handle_event(&event), PagerAction::Consumed);
    }
    assert_eq!(
        overlay.handle_event(&Event::Key(KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        ))),
        PagerAction::CopyTranscriptSelection {
            text: "tool\nafter".to_string(),
            follow: false,
        }
    );
    overlay.clear_transcript_selection();

    assert_eq!(
        overlay.handle_event(&key(KeyCode::Char('/'))),
        PagerAction::ScheduleFrame
    );
    for character in "Show details".chars() {
        assert_eq!(
            overlay.handle_event(&key(KeyCode::Char(character))),
            PagerAction::ScheduleFrame
        );
    }
    terminal
        .draw(|frame| overlay.render_transcript(frame, frame.area(), Locale::EnUs, &content))
        .expect("search draw");
    assert_eq!(overlay.search.match_count(), 0);
    assert!(buffer_text(&terminal).contains("No matches"));
}
