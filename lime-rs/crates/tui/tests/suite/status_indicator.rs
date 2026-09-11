//! Regression test: ensure that `StatusIndicatorWidget` sanitises ANSI escape
//! sequences so that no raw `\x1b` bytes are written into the backing buffer.

use ansi_escape::ansi_escape_line;

#[test]
fn ansi_escape_line_strips_escape_sequences() {
    let text_in_ansi_red = "\x1b[31mRED\x1b[0m";
    let line = ansi_escape_line(text_in_ansi_red);

    let combined: String = line
        .spans
        .iter()
        .map(|span| span.content.to_string())
        .collect();

    assert_eq!(combined, "RED");
    assert!(combined.bytes().all(|byte| byte != 0x1b));
}
