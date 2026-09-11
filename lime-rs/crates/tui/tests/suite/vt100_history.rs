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
use ratatui::style::Stylize;
use ratatui::text::Line;
use tui::HistoryTerminal;

// Small helper macro to assert a collection contains an item with a clearer
// failure message.
macro_rules! assert_contains {
    ($collection:expr, $item:expr $(,)?) => {
        assert!(
            $collection.contains(&$item),
            "Expected {:?} to contain {:?}",
            $collection,
            $item
        );
    };
    ($collection:expr, $item:expr, $($arg:tt)+) => {
        assert!($collection.contains(&$item), $($arg)+);
    };
}

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

    fn run_insert(&mut self, lines: Vec<Line<'static>>) {
        tui::insert_history_lines(self, lines).expect("Failed to insert history lines in test");
    }

    fn screen(&self) -> &vt100::Screen {
        self.backend.vt100().screen()
    }
}

impl HistoryTerminal for TestScenario {
    fn screen_size(&self) -> Size {
        let (rows, cols) = self.screen().size();
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
fn basic_insertion_no_wrap() {
    // Screen of 20x6; viewport is the last row (height=1 at y=5)
    let area = Rect::new(
        /*x*/ 0, /*y*/ 5, /*width*/ 20, /*height*/ 1,
    );
    let mut scenario = TestScenario::new(/*width*/ 20, /*height*/ 6, area);

    let lines = vec!["first".into(), "second".into()];
    scenario.run_insert(lines);
    let rows = scenario.screen().contents();
    assert_contains!(rows, String::from("first"));
    assert_contains!(rows, String::from("second"));
}

#[test]
fn long_token_wraps() {
    let area = Rect::new(
        /*x*/ 0, /*y*/ 5, /*width*/ 20, /*height*/ 1,
    );
    let mut scenario = TestScenario::new(/*width*/ 20, /*height*/ 6, area);

    let long = "A".repeat(45); // > 2 lines at width 20
    let lines = vec![long.clone().into()];
    scenario.run_insert(lines);
    let screen = scenario.screen();

    // Count total A's on the screen
    let mut count_a = 0usize;
    for row in 0..6 {
        for col in 0..20 {
            if let Some(cell) = screen.cell(row, col) {
                if let Some(ch) = cell.contents().chars().next() {
                    if ch == 'A' {
                        count_a += 1;
                    }
                }
            }
        }
    }

    assert_eq!(
        count_a,
        long.len(),
        "wrapped content did not preserve all characters"
    );
}

#[test]
fn emoji_and_cjk() {
    let area = Rect::new(
        /*x*/ 0, /*y*/ 5, /*width*/ 20, /*height*/ 1,
    );
    let mut scenario = TestScenario::new(/*width*/ 20, /*height*/ 6, area);

    let text = String::from("😀😀😀😀😀 你好世界");
    let lines = vec![text.clone().into()];
    scenario.run_insert(lines);
    let rows = scenario.screen().contents();
    for ch in text.chars().filter(|c| !c.is_whitespace()) {
        assert!(
            rows.contains(ch),
            "missing character {ch:?} in reconstructed screen"
        );
    }
}

#[test]
fn mixed_ansi_spans() {
    let area = Rect::new(
        /*x*/ 0, /*y*/ 5, /*width*/ 20, /*height*/ 1,
    );
    let mut scenario = TestScenario::new(/*width*/ 20, /*height*/ 6, area);

    let line = vec!["red".red(), "+plain".into()].into();
    scenario.run_insert(vec![line]);
    let rows = scenario.screen().contents();
    assert_contains!(rows, String::from("red+plain"));
}

#[test]
fn cursor_restoration() {
    let area = Rect::new(
        /*x*/ 0, /*y*/ 5, /*width*/ 20, /*height*/ 1,
    );
    let mut scenario = TestScenario::new(/*width*/ 20, /*height*/ 6, area);

    let lines = vec!["x".into()];
    scenario.run_insert(lines);
    assert_eq!(scenario.last_known_cursor_pos, (0, 0).into());
}

#[test]
fn word_wrap_no_mid_word_split() {
    // Screen of 40x10; viewport is the last row
    let area = Rect::new(
        /*x*/ 0, /*y*/ 9, /*width*/ 40, /*height*/ 1,
    );
    let mut scenario = TestScenario::new(/*width*/ 40, /*height*/ 10, area);

    let sample = "Years passed, and Willowmere thrived in peace and friendship. Mira's herb garden flourished with both ordinary and enchanted plants, and travelers spoke of the kindness of the woman who tended them.";
    scenario.run_insert(vec![sample.into()]);
    let joined = scenario.screen().contents();
    assert!(
        !joined.contains("bo\nth"),
        "word 'both' should not be split across lines:\n{joined}"
    );
}

#[test]
fn em_dash_and_space_word_wrap() {
    // Repro from report: ensure we break before "inside", not mid-word.
    let area = Rect::new(
        /*x*/ 0, /*y*/ 9, /*width*/ 40, /*height*/ 1,
    );
    let mut scenario = TestScenario::new(/*width*/ 40, /*height*/ 10, area);

    let sample = "Mara found an old key on the shore. Curious, she opened a tarnished box half-buried in sand—and inside lay a single, glowing seed.";
    scenario.run_insert(vec![sample.into()]);
    let joined = scenario.screen().contents();
    assert!(
        !joined.contains("insi\nde"),
        "word 'inside' should not be split across lines:\n{joined}"
    );
}
