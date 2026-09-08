use super::{cursor_position, visible_prefix, wrapped_lines};

#[test]
fn wrapped_lines_keep_utf8_boundaries_and_reserve_cursor_rows() {
    let text = "你好 世界";
    let lines = wrapped_lines(text, 5);
    assert!(!lines.is_empty());
    assert!(lines.iter().all(|range| text.is_char_boundary(range.start)));
    assert!(lines
        .iter()
        .all(|range| text.is_char_boundary(range.end.saturating_sub(1).min(text.len()))));
    assert_eq!(
        cursor_position(text, &lines, 5, text.len()),
        Some((lines.len() - 1, 4))
    );
}

#[test]
fn soft_break_spaces_hang_on_the_previous_row() {
    assert_eq!(visible_prefix("a b c d e f", 5), "a b c d e ");
    assert_eq!(visible_prefix("hello ", 5), "hello ");
}

#[test]
fn non_breaking_spaces_stay_inside_the_word() {
    let text = "one\u{a0}two";
    let lines = wrapped_lines(text, 5);
    assert!(lines.len() >= 2);
    assert!(lines.iter().all(|range| text.is_char_boundary(range.start)));
}
