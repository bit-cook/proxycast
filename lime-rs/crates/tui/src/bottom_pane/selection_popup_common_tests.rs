use super::*;
use ratatui::backend::TestBackend;
use ratatui::Terminal;

fn rows() -> Vec<SelectionRow> {
    (0..10)
        .map(|index| {
            SelectionRow::new(
                format!("/option-{index}"),
                Some(
                    "a long description with an exact-fit boundary and several wrapped words"
                        .to_string(),
                ),
                vec![if index == 9 { "› " } else { "  " }.into()],
            )
        })
        .collect()
}

fn text(terminal: &Terminal<TestBackend>) -> String {
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
fn measurement_and_rendering_share_exact_fit_and_continuation_column() {
    let rows = vec![SelectionRow::new(
        "/test",
        Some("Use faster inference".into()),
        vec!["  ".into()],
    )];
    let state = ScrollState {
        selected_idx: Some(0),
        scroll_top: 0,
    };
    assert_eq!(measure_rows_height(&rows, &state, 29), 1);
    assert_eq!(measure_rows_height(&rows, &state, 28), 2);
    let mut terminal = Terminal::new(TestBackend::new(28, 2)).unwrap();
    terminal
        .draw(|frame| render_rows(frame, frame.area(), &rows, &state))
        .unwrap();
    assert_eq!(text(&terminal).lines().nth(1).unwrap().trim(), "inference");
    assert_eq!(text(&terminal).lines().nth(1).unwrap().find('i'), Some(9));
}

#[test]
fn clipped_wrapped_viewport_keeps_selected_row_visible() {
    let rows = rows();
    let state = ScrollState {
        selected_idx: Some(9),
        scroll_top: 2,
    };
    for width in [16, 28, 40, 80, 120] {
        for height in [1, 2, 3, 8] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal
                .draw(|frame| render_rows(frame, frame.area(), &rows, &state))
                .unwrap();
            let visible = text(&terminal);
            assert!(visible.contains("› /opt"), "{width}x{height}: {visible}");
        }
    }
}

#[test]
fn filled_selection_clears_dim_from_secondary_text_and_fills_trailing_cells() {
    let rows = vec![SelectionRow::new(
        "/test",
        Some("details".into()),
        vec!["› ".into()],
    )];
    let state = ScrollState {
        selected_idx: Some(0),
        scroll_top: 0,
    };
    let mut terminal = Terminal::new(TestBackend::new(40, 1)).unwrap();
    terminal
        .draw(|frame| render_rows(frame, frame.area(), &rows, &state))
        .unwrap();
    let style = selection_style();
    let buffer = terminal.backend().buffer();
    for x in 0..40 {
        assert_eq!(buffer[(x, 0)].bg, style.bg.unwrap());
        assert_eq!(
            buffer[(x, 0)].modifier.contains(Modifier::REVERSED),
            style.add_modifier.contains(Modifier::REVERSED)
        );
        assert!(!buffer[(x, 0)].modifier.contains(Modifier::DIM));
    }
}

#[test]
fn short_single_line_viewport_shifts_window_and_reports_both_overflows() {
    let rows = rows();
    let state = ScrollState {
        selected_idx: Some(5),
        scroll_top: 0,
    };
    let mut terminal = Terminal::new(TestBackend::new(40, 3)).unwrap();
    terminal
        .draw(|frame| {
            let rendered = render_rows_single_line(frame, frame.area(), &rows, &state, "empty");
            assert!(rendered.has_above);
            assert!(rendered.has_below);
        })
        .unwrap();
    assert!(text(&terminal).contains("/option-5"));
    assert!(!text(&terminal).contains("/option-0"));
}

#[test]
fn unicode_grapheme_highlighting_and_narrow_category_identity_are_preserved() {
    let mut row = SelectionRow::new(
        "İstanbul e\u{301} 👩‍💻",
        Some("Secondary details".into()),
        vec!["  ".into()],
    );
    row.match_indices = Some(vec![0, 9]);
    row.category_tag = Some("[Skill]".into());
    let state = ScrollState {
        selected_idx: None,
        scroll_top: 0,
    };
    for width in [16, 24, 40, 72] {
        let mut terminal = Terminal::new(TestBackend::new(width, 1)).unwrap();
        terminal
            .draw(|frame| {
                render_rows_single_line(
                    frame,
                    frame.area(),
                    std::slice::from_ref(&row),
                    &state,
                    "empty",
                );
            })
            .unwrap();
        let visible = text(&terminal);
        assert!(visible.contains("[Skill]"), "{width}: {visible}");
        assert_eq!(
            visible.contains("Secondary details"),
            width == 72,
            "{width}: {visible}"
        );
        assert!(terminal.backend().buffer()[(2, 0)]
            .modifier
            .contains(Modifier::BOLD));
    }
}
