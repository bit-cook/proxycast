use std::io;
use std::io::Write;

use crate::test_backend::VT100Backend;
use crossterm::cursor::MoveTo;
use crossterm::queue;
use crossterm::terminal::Clear;
use crossterm::terminal::ClearType;
use ratatui::layout::Position;
use ratatui::layout::Rect;
use ratatui::layout::Size;
use ratatui::text::Line;
use tui::HistoryTerminal;

struct TestScenario {
    backend: VT100Backend,
    viewport: Rect,
    last_known_cursor_pos: Position,
}

impl TestScenario {
    fn new(width: u16, height: u16, viewport: Rect) -> Self {
        Self {
            backend: VT100Backend::new(width, height),
            viewport,
            last_known_cursor_pos: Position::ORIGIN,
        }
    }
}

impl HistoryTerminal for TestScenario {
    fn screen_size(&self) -> Size {
        let (rows, cols) = self.backend.vt100().screen().size();
        Size::new(cols, rows)
    }

    fn viewport_area(&self) -> Rect {
        self.viewport
    }

    fn last_known_cursor_position(&mut self) -> Position {
        self.last_known_cursor_pos
    }

    fn clear_after_position(&mut self, position: Position) -> io::Result<()> {
        let mut output = Vec::new();
        queue!(
            output,
            MoveTo(position.x, position.y),
            Clear(ClearType::FromCursorDown)
        )?;
        self.write_ansi(&output)
    }

    fn write_ansi(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.backend.write_all(bytes)
    }

    fn set_viewport_area(&mut self, area: Rect) {
        self.viewport = area;
    }

    fn note_history_rows_inserted(&mut self, _rows: u16) {}
}

#[test]
fn live_001_commit_on_overflow() {
    let area = Rect::new(
        /*x*/ 0, /*y*/ 5, /*width*/ 20, /*height*/ 1,
    );
    let mut term = TestScenario::new(/*width*/ 20, /*height*/ 6, area);

    // Build 5 explicit rows at width 20.
    let mut rb = tui::RowBuilder::new(/*target_width*/ 20);
    rb.push_fragment("one\n");
    rb.push_fragment("two\n");
    rb.push_fragment("three\n");
    rb.push_fragment("four\n");
    rb.push_fragment("five\n");

    // Keep the last 3 in the live ring; commit the first 2.
    let commit_rows = rb.drain_commit_ready(/*max_keep*/ 3);
    let lines: Vec<Line<'static>> = commit_rows.into_iter().map(|r| r.text.into()).collect();

    tui::insert_history_lines(&mut term, lines).expect("Failed to insert history lines in test");

    let screen = term.backend.vt100().screen();

    // The words "one" and "two" should appear above the viewport.
    let joined = screen.contents();
    assert!(
        joined.contains("one"),
        "expected committed 'one' to be visible\n{joined}"
    );
    assert!(
        joined.contains("two"),
        "expected committed 'two' to be visible\n{joined}"
    );
    // The last three (three,four,five) remain in the live ring, not committed here.
}
