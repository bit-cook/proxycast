use super::*;

fn entry(id: &str, kind: EntryKind, text: &str, summary: &[&str]) -> TranscriptEntry {
    TranscriptEntry {
        id: id.to_string(),
        kind,
        text: text.to_string(),
        streaming: false,
        status: None,
        summary: summary.iter().map(|value| (*value).to_string()).collect(),
        activity_group: None,
        activity_detail: None,
    }
}

fn line(text: &str) -> HyperlinkLine {
    HyperlinkLine::from(text.to_string())
}

#[test]
fn header_tracks_the_nearest_visible_prompt_without_entering_source_lines() {
    let lines = vec![
        line("› first question"),
        line("first answer"),
        line("› second question"),
        line("second answer"),
    ];
    let mut source = PromptHeaderSource::default();
    source.record_entry(
        &entry("user-1", EntryKind::User, "first question", &[]),
        Locale::EnUs,
        0..1,
    );
    source.record_entry(
        &entry("answer-1", EntryKind::Assistant, "first answer", &[]),
        Locale::EnUs,
        1..2,
    );
    source.record_entry(
        &entry("user-2", EntryKind::User, "second question", &[]),
        Locale::EnUs,
        2..3,
    );
    source.record_entry(
        &entry("answer-2", EntryKind::Assistant, "second answer", &[]),
        Locale::EnUs,
        3..4,
    );
    let area = Rect::new(0, 0, 30, 5);

    assert!(source.candidate(&lines, area, 0).is_none());
    assert_eq!(
        source.candidate(&lines, area, 1).unwrap().line.to_string(),
        "first question"
    );
    assert!(source.candidate(&lines, area, 2).is_none());
    assert_eq!(
        source.candidate(&lines, area, 3).unwrap().line.to_string(),
        "second question"
    );
    assert!(source
        .candidate(&lines, Rect::new(0, 0, 15, 5), 3)
        .is_none());
    assert_eq!(lines[3].line.to_string(), "second answer");
}

#[test]
fn header_sanitizes_bounds_and_truncates_prompt_by_display_width() {
    let prompt = format!("\x1b[31m界 café\r\nnext {}TAIL", "x".repeat(600));
    let lines = vec![line("prompt"), line("answer")];
    let mut source = PromptHeaderSource::default();
    source.record_entry(
        &entry("user", EntryKind::User, &prompt, &[]),
        Locale::EnUs,
        0..1,
    );
    source.record_entry(
        &entry("answer", EntryKind::Assistant, "answer", &[]),
        Locale::EnUs,
        1..2,
    );

    let header = source
        .candidate(&lines, Rect::new(0, 0, 18, 4), 1)
        .unwrap()
        .line;
    let rendered = header.to_string();
    assert!(crate::line_truncation::line_width(&header) <= 18);
    assert!(!rendered.contains('\x1b'));
    assert!(!rendered.contains('\r'));
    assert!(!rendered.contains('\n'));
    assert!(!rendered.contains("TAIL"));
    assert!(rendered.starts_with("界 café next"), "{rendered}");
    assert!(rendered.ends_with('…'), "{rendered}");
}

#[test]
fn attachment_only_prompt_uses_fallback_and_control_only_prompt_is_skipped() {
    let lines = vec![
        line("first"),
        line("attachment"),
        line("control"),
        line("answer"),
    ];
    let mut source = PromptHeaderSource::default();
    source.record_entry(
        &entry("first", EntryKind::User, "earlier", &[]),
        Locale::EnUs,
        0..1,
    );
    source.record_entry(
        &entry("attachment", EntryKind::User, "", &["image: 1"]),
        Locale::EnUs,
        1..2,
    );
    source.record_entry(
        &entry("control", EntryKind::User, "\u{0007}", &[]),
        Locale::EnUs,
        2..3,
    );
    source.record_entry(
        &entry("answer", EntryKind::Assistant, "answer", &[]),
        Locale::EnUs,
        3..4,
    );

    assert_eq!(
        source
            .candidate(&lines, Rect::new(0, 0, 30, 4), 3)
            .unwrap()
            .line
            .to_string(),
        "[attachments]"
    );
}

#[test]
fn attachment_fallback_covers_every_product_locale() {
    let attachment = entry("attachment", EntryKind::User, "", &["image: 1"]);
    for (locale, expected) in [
        (Locale::ZhCn, "[附件]"),
        (Locale::ZhTw, "[附件]"),
        (Locale::EnUs, "[attachments]"),
        (Locale::JaJp, "[添付ファイル]"),
        (Locale::KoKr, "[첨부 파일]"),
    ] {
        assert_eq!(prompt_text(&attachment, locale).as_deref(), Some(expected));
    }
}

#[test]
fn suppressed_boundary_is_held_only_for_the_same_viewport_and_top_row() {
    let lines = vec![line("prompt"), line("answer one"), line("answer two")];
    let mut source = PromptHeaderSource::default();
    source.record_entry(
        &entry("user", EntryKind::User, "question", &[]),
        Locale::EnUs,
        0..1,
    );
    source.record_entry(
        &entry("answer", EntryKind::Assistant, "answer", &[]),
        Locale::EnUs,
        1..3,
    );
    let state = TranscriptPromptHeader::default();
    let area = Rect::new(0, 0, 30, 5);
    let candidate = source.candidate(&lines, area, 1).unwrap();

    assert!(!state.is_suppressed(&candidate));
    state.suppress(&candidate);
    assert!(state.is_suppressed(&candidate));

    let moved = source.candidate(&lines, area, 2).unwrap();
    assert!(!state.is_suppressed(&moved));
    assert!(!state.is_suppressed(&candidate));

    state.suppress(&candidate);
    let resized = source.candidate(&lines, Rect::new(0, 0, 31, 5), 1).unwrap();
    assert!(!state.is_suppressed(&resized));
}
