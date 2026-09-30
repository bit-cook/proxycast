use super::*;

fn lines(values: &[&str]) -> Vec<HyperlinkLine> {
    values
        .iter()
        .map(|value| HyperlinkLine::from((*value).to_string()))
        .collect()
}

#[test]
fn bookmark_restores_stable_source_after_pagination_and_rewrap() {
    let original = TranscriptFrame::new(
        lines(&[
            "first source line that wraps at a narrow width",
            "first tail",
            "target source line that also wraps",
            "target tail",
        ]),
        vec![
            TranscriptAnchorRange::new(vec!["entry:first".to_string()], 0..2),
            TranscriptAnchorRange::new(vec!["entry:target".to_string()], 2..4),
        ],
        18,
    );
    let starts = wrapped_line_starts(original.lines(), original.width());
    let bookmark = TranscriptBookmark::capture(&original, false, starts[2] + 1);

    let updated = TranscriptFrame::new(
        lines(&[
            "older page",
            "replacement first",
            "replacement tail",
            "target source line that also wraps",
            "target tail",
        ]),
        vec![
            TranscriptAnchorRange::new(vec!["entry:older".to_string()], 0..1),
            TranscriptAnchorRange::new(vec!["entry:first".to_string()], 1..3),
            TranscriptAnchorRange::new(vec!["entry:target".to_string()], 3..5),
        ],
        12,
    );
    let updated_starts = wrapped_line_starts(updated.lines(), updated.width());

    assert_eq!(
        bookmark.resolve(&updated, usize::MAX),
        Some(updated_starts[3] + 1)
    );
}

#[test]
fn bookmark_clamps_source_and_wrapped_offsets_when_replacement_shrinks() {
    let original = TranscriptFrame::new(
        lines(&["one", "two", "a target line that wraps several times"]),
        vec![TranscriptAnchorRange::new(
            vec!["entry:target".to_string()],
            0..3,
        )],
        8,
    );
    let starts = wrapped_line_starts(original.lines(), original.width());
    let bookmark = TranscriptBookmark::capture(&original, false, starts[2] + 3);
    let updated = TranscriptFrame::new(
        lines(&["short"]),
        vec![TranscriptAnchorRange::new(
            vec!["entry:target".to_string()],
            0..1,
        )],
        20,
    );

    assert_eq!(bookmark.resolve(&updated, 10), Some(0));
}

#[test]
fn following_bookmark_returns_to_latest_output() {
    let frame = TranscriptFrame::new(lines(&["one", "two"]), Vec::new(), 20);
    let bookmark = TranscriptBookmark::capture(&frame, true, 0);

    assert_eq!(bookmark.resolve(&frame, 7), Some(7));
}
