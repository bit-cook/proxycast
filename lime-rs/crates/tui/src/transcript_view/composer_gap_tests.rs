use super::*;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|row| {
            (0..buffer.area.width)
                .map(|column| buffer[(column, row)].symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn copy_feedback_is_right_aligned_truthful_and_expires_without_sleep() {
    for (result, marker) in [
        (Ok(CopyStatus::Confirmed), "Copied 24 chars"),
        (Ok(CopyStatus::Unconfirmed), "Copy sent"),
        (Err("offline".to_string()), "Copy failed"),
    ] {
        let gap = TranscriptComposerGap::default();
        gap.show_copy_feedback(&result, 24);
        let mut terminal = Terminal::new(TestBackend::new(40, 1)).expect("terminal");
        terminal
            .draw(|frame| {
                assert!(gap.render(frame, Some(frame.area()), Locale::EnUs));
            })
            .expect("draw");
        let rendered = buffer_text(&terminal);
        assert!(rendered.contains(marker), "{result:?}: {rendered}");
        assert!(rendered.ends_with(' '), "{result:?}: {rendered:?}");

        gap.expire_for_test();
        assert_eq!(gap.tick(Instant::now()), None);
        let mut owns_gap = true;
        terminal
            .draw(|frame| {
                owns_gap = gap.render(frame, Some(frame.area()), Locale::EnUs);
            })
            .expect("expired draw");
        assert!(!owns_gap);
    }
}
