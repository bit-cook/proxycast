use super::{cursor_position, visible_prefix, wrapped_lines};
use crate::bottom_pane::textarea::TextArea;
use crate::width::display_width;
use ratatui::layout::Rect;

fn wrapped_rows(text: &str, width: u16) -> Vec<&str> {
    wrapped_lines(text, width)
        .into_iter()
        .map(|range| &text[range.start..range.end.saturating_sub(1)])
        .collect()
}

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

#[test]
fn preserves_textwrap_word_boundaries() {
    for (text, width, expected_rows) in [
        ("a foo-barbaz", 10, vec!["a foo-", "barbaz"]),
        ("a café-barbaz", 10, vec!["a café-", "barbaz"]),
        ("a foo/barbaz", 10, vec!["a foo/", "barbaz"]),
        ("a foo—barbaz", 10, vec!["a foo—", "barbaz"]),
        ("a abc\u{a0}de", 7, vec!["a ", "abc\u{a0}de"]),
        ("a \u{a0}abc", 5, vec!["a ", "\u{a0}abc"]),
    ] {
        assert_eq!(wrapped_rows(text, width), expected_rows, "text={text:?}");
    }
}

#[test]
fn breakable_unicode_spaces_hang_before_following_words() {
    for (text, expected_rows) in [
        ("abad abcde", vec!["abad ", "abcd", "e"]),
        ("abad\u{2003}abcde", vec!["abad\u{2003}", "abcd", "e"]),
        (
            "abad\u{2003}\u{2003}abcde",
            vec!["abad\u{2003}\u{2003}", "abcd", "e"],
        ),
        ("abad\u{3000}abcde", vec!["abad\u{3000}", "abcd", "e"]),
    ] {
        assert_eq!(wrapped_rows(text, 4), expected_rows, "text={text:?}");
    }
}

#[test]
fn explicit_indentation_and_trailing_spaces_do_not_hang() {
    for (text, expected_rows) in [
        ("abad\n a", vec!["abad", "", " a"]),
        ("abad \n a", vec!["abad", " ", " a"]),
        ("    a", vec!["    ", "a"]),
        ("abad  ", vec!["abad", "  "]),
        ("abad\u{2003}", vec!["abad", "\u{2003}"]),
    ] {
        assert_eq!(wrapped_rows(text, 4), expected_rows, "text={text:?}");
    }
}

#[test]
fn nonbreaking_spaces_never_hang() {
    for space in ['\u{a0}', '\u{2007}', '\u{202f}'] {
        let text = format!("abcd{space} x");
        let rows = wrapped_rows(&text, 4);
        assert_eq!(rows.first().copied(), Some("abcd"));
        assert_eq!(rows.get(1).copied(), Some(format!("{space} x").as_str()));
    }
}

#[test]
fn ascii_wrapped_rows_fit_and_preserve_cursor_positions() {
    for text in ["", "a", "a b", "ab-c", "a  b"] {
        for width in 1_u16..=5 {
            let mut textarea = TextArea::new();
            textarea.insert_str(text);
            let ranges = textarea.wrapped_lines(width);
            let mut end = 0;
            for range in ranges.iter() {
                assert_eq!(range.start, end, "text={text:?}, width={width}");
                end = range.end.saturating_sub(1);
                let row = &text[range.start..end];
                assert!(
                    display_width(row.trim_end_matches(' ')) <= usize::from(width),
                    "text={text:?}, width={width}, row={row:?}"
                );
            }
            assert_eq!(end, text.len(), "text={text:?}, width={width}");
            drop(ranges);

            let area = Rect::new(0, 0, width, text.len() as u16 + 1);
            for cursor in 0..=text.len() {
                textarea.set_cursor(cursor);
                assert!(textarea.cursor_pos(area).is_some());
            }
        }
    }
}

#[test]
fn vertical_navigation_preserves_preferred_column_across_short_wrapped_rows() {
    let mut textarea = TextArea::new();
    textarea.insert_str("abcdefghij");
    let _ = textarea.desired_height(/*width*/ 4);

    textarea.set_cursor(/*column*/ 2);
    textarea.move_down();
    assert_eq!(textarea.cursor(), 6);
    textarea.move_down();
    assert_eq!(textarea.cursor(), textarea.text().len());

    // The saved visual column is restored when moving back through a short row.
    textarea.move_up();
    assert_eq!(textarea.cursor(), 6);
    textarea.move_up();
    assert_eq!(textarea.cursor(), 2);
}

#[test]
fn vertical_navigation_preserves_destination_tab_columns() {
    let mut textarea = TextArea::new();
    textarea.insert_str("a\tb\nxyz");
    let area = Rect::new(0, 0, /*width*/ 8, /*height*/ 2);
    let _ = textarea.desired_height(area.width);
    textarea.set_cursor(/*pos*/ 5);

    textarea.move_up();
    assert_eq!(
        (textarea.cursor(), textarea.cursor_pos(area)),
        (1, Some((1, 0)))
    );
    textarea.move_down();
    assert_eq!(
        (textarea.cursor(), textarea.cursor_pos(area)),
        (5, Some((1, 1)))
    );
}

#[test]
fn vertical_navigation_clamps_saved_column_after_resize() {
    let text = "abcdefghij\nabcd  xyz";
    let mut textarea = TextArea::new();
    textarea.insert_str(text);

    let _ = textarea.desired_height(/*width*/ 10);
    textarea.move_up();
    assert_eq!(
        (
            textarea.cursor(),
            textarea.cursor_pos(Rect::new(0, 0, 10, 2))
        ),
        (10, Some((0, 1)))
    );

    let narrow_area = Rect::new(0, 0, 4, textarea.desired_height(/*width*/ 4));
    assert_eq!(textarea.cursor_pos(narrow_area), Some((2, 2)));
    textarea.move_down();
    assert_eq!(
        (textarea.cursor(), textarea.cursor_pos(narrow_area)),
        (14, Some((3, 3)))
    );
    textarea.move_down();
    assert_eq!(
        (textarea.cursor(), textarea.cursor_pos(narrow_area)),
        (20, Some((3, 4)))
    );
    textarea.move_up();
    assert_eq!(textarea.cursor(), 14);
    textarea.move_up();
    assert_eq!(textarea.cursor(), 10);
}
