use super::*;

use crossterm::event::KeyModifiers;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn draw(
    control: &TranscriptFollowControl,
    width: u16,
    tail_visible: bool,
    unseen_activity: bool,
) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(width, 1)).expect("terminal");
    terminal
        .draw(|frame| {
            control.render(
                frame,
                Some(frame.area()),
                Locale::EnUs,
                tail_visible,
                unseen_activity,
            )
        })
        .expect("draw");
    terminal
}

fn text(terminal: &Terminal<TestBackend>) -> String {
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

fn mouse(kind: MouseEventKind, column: u16) -> MouseEvent {
    MouseEvent {
        kind,
        column,
        row: 0,
        modifiers: KeyModifiers::NONE,
    }
}

#[test]
fn control_tracks_pause_activity_hover_resize_and_click() {
    let control = TranscriptFollowControl::default();
    assert!(control.area().is_none());
    assert!(text(&draw(&control, 80, false, false)).contains("Back to bottom"));
    assert!(text(&draw(&control, 80, false, true)).contains("New activity"));
    assert!(text(&draw(&control, 12, false, true)).contains("Bottom"));

    let area = control.area().expect("control area");
    assert_eq!(
        control.handle_mouse(mouse(MouseEventKind::Moved, area.x)),
        Some(TranscriptFollowAction::Consumed)
    );
    assert_eq!(
        control.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), area.x)),
        Some(TranscriptFollowAction::Consumed)
    );
    assert_eq!(
        control.handle_mouse(mouse(MouseEventKind::Up(MouseButton::Left), area.x)),
        Some(TranscriptFollowAction::ReturnToLatest)
    );
}

#[test]
fn hidden_control_does_not_consume_clicks_and_drag_out_cancels() {
    let control = TranscriptFollowControl::default();
    draw(&control, 40, false, false);
    let area = control.area().expect("control area");
    assert_eq!(
        control.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), area.x)),
        Some(TranscriptFollowAction::Consumed)
    );
    assert_eq!(
        control.handle_mouse(mouse(MouseEventKind::Up(MouseButton::Left), 0)),
        Some(TranscriptFollowAction::Consumed)
    );

    draw(&control, 40, true, false);
    assert!(control.area().is_none());
    assert_eq!(
        control.handle_mouse(mouse(MouseEventKind::Down(MouseButton::Left), area.x)),
        None
    );
}
