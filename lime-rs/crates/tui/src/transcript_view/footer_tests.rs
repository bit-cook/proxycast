use super::*;

use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .flat_map(|row| {
            (0..buffer.area.width)
                .map(move |column| buffer[(column, row)].symbol().to_string())
                .chain(std::iter::once("\n".to_string()))
        })
        .collect()
}

#[test]
fn query_footer_keeps_end_and_caret_visible_on_narrow_widths() {
    let mut search = TranscriptSearch::default();
    search.begin(0, true);
    search.handle_event(
        &Event::Paste("abcdefghijklmno".to_string()),
        /*older_history_available*/ false,
    );
    let footer = TranscriptFooter;
    let mut terminal = Terminal::new(TestBackend::new(12, 2)).expect("terminal");

    terminal
        .draw(|frame| {
            assert!(footer.render_search_query(
                frame,
                Some(Rect::new(0, 0, 12, 1)),
                Locale::EnUs,
                &search,
            ));
        })
        .expect("draw");

    let text = buffer_text(&terminal);
    assert!(text.contains("…lmno▏"), "{text}");
}

#[test]
fn status_prefers_selection_and_all_locales_fit_without_panicking() {
    for locale in [
        Locale::ZhCn,
        Locale::ZhTw,
        Locale::EnUs,
        Locale::JaJp,
        Locale::KoKr,
    ] {
        let mut search = TranscriptSearch::default();
        search.begin(0, true);
        search.handle_event(
            &Event::Key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE)),
            false,
        );
        let footer = TranscriptFooter;
        let mut terminal = Terminal::new(TestBackend::new(18, 1)).expect("terminal");
        terminal
            .draw(|frame| {
                assert!(footer.render_status(
                    frame,
                    frame.area(),
                    locale,
                    &search,
                    /*selection_active*/ true,
                ));
            })
            .expect("status draw");
        assert_eq!(terminal.backend().buffer().area.width, 18);
    }
}
