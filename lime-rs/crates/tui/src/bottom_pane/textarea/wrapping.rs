//! Grapheme-safe composer wrapping that preserves textwrap's semantic breakpoints.
//!
//! Source ranges include a sentinel byte for cursor placement. Separators at soft breaks hang off
//! the preceding row without changing the source text. Full logical lines reserve an insertion row.

use super::text_for_display;
use crate::width::display_width;
use std::ops::Range;
use textwrap::Options;
use unicode_segmentation::UnicodeSegmentation;

struct WrappedWord {
    range: Range<usize>,
    width: usize,
}

fn is_breakable_space(ch: char) -> bool {
    ch.is_whitespace() && !matches!(ch, '\u{a0}' | '\u{2007}' | '\u{202f}')
}

fn is_hangable_space(ch: char) -> bool {
    is_breakable_space(ch)
        && !matches!(
            ch,
            '\n' | '\r' | '\u{b}' | '\u{c}' | '\u{85}' | '\u{2028}' | '\u{2029}'
        )
}

pub(super) fn visible_prefix(text: &str, width: u16) -> &str {
    if width == 0 {
        return "";
    }
    if text.len() <= usize::from(width) {
        return text;
    }
    text.grapheme_indices(true)
        .filter(|(_, grapheme)| grapheme.chars().any(is_hangable_space))
        .nth(usize::from(width) - 1)
        .map_or(text, |(start, grapheme)| &text[..start + grapheme.len()])
}

pub(super) fn wrapped_lines(text: &str, width: u16) -> Vec<Range<usize>> {
    let width = usize::from(width);
    let options = Options::new(width).wrap_algorithm(textwrap::WrapAlgorithm::FirstFit);
    let wrapped_lines = crate::wrapping::wrap_ranges(text, &options);
    if width == 0 || (wrapped_lines.len() == 1 && text.len() < width) {
        return wrapped_lines;
    }

    let mut breakpoints = vec![false; text.len() + 1];
    let mut words = Vec::new();
    let mut logical_start = 0;
    for logical_line in text.split('\n') {
        let mut word_start = logical_start;
        for word in options.word_separator.find_words(logical_line) {
            let word_end = word_start + word.word.len();
            for breakpoint in options.word_splitter.split_points(word.word) {
                breakpoints[word_start + breakpoint] = true;
            }
            breakpoints[word_end] = true;
            words.push(WrappedWord {
                range: word_start..word_end,
                width: display_width(word.word),
            });
            word_start = word_end + word.whitespace.len();
        }
        logical_start += logical_line.len() + 1;
    }

    let mut lines = Vec::with_capacity(wrapped_lines.len());
    let mut line_start = 0;
    let mut line_width = 0;
    let mut line_has_text = false;
    let mut line_active = false;
    let mut logical_content_end = 0;
    let mut processed_end = 0;
    let mut previous_was_whitespace = false;
    let mut word_index = 0;
    let mut consumed_word_width = 0;

    for wrapped_line in wrapped_lines {
        let line_end = wrapped_line.end.saturating_sub(1);
        let fragment_start = wrapped_line.start.max(processed_end);
        if fragment_start > line_end {
            continue;
        }

        while word_index < words.len() && words[word_index].range.end <= fragment_start {
            word_index += 1;
            consumed_word_width = 0;
        }

        if !line_active {
            line_start = fragment_start;
            line_active = true;
            let logical_line = text[fragment_start..]
                .split('\n')
                .next()
                .unwrap_or_default();
            logical_content_end =
                fragment_start + logical_line.trim_end_matches(is_hangable_space).len();
        } else if line_has_text && breakpoints[fragment_start] {
            let remaining_word_width = words
                .get(word_index)
                .filter(|word| word.range.contains(&fragment_start))
                .map_or(0, |word| word.width.saturating_sub(consumed_word_width));
            if line_width + remaining_word_width > width {
                lines.push(line_start..fragment_start + 1);
                line_start = fragment_start;
                line_width = 0;
                line_has_text = false;
            }
        }

        for (offset, grapheme) in text[fragment_start..line_end].grapheme_indices(true) {
            let grapheme_start = fragment_start + offset;
            while word_index < words.len() && words[word_index].range.end <= grapheme_start {
                word_index += 1;
                consumed_word_width = 0;
            }

            let grapheme_end = grapheme_start + grapheme.len();
            let is_whitespace = grapheme.chars().all(is_breakable_space);
            if !is_whitespace && line_has_text && previous_was_whitespace {
                let word_end = words
                    .get(word_index)
                    .filter(|word| word.range.contains(&grapheme_start))
                    .map_or(line_end, |word| word.range.end.min(line_end));
                if line_width + display_width(&text[grapheme_start..word_end]) > width {
                    lines.push(line_start..grapheme_start + 1);
                    line_start = grapheme_start;
                    line_width = 0;
                    line_has_text = false;
                }
            }

            let grapheme_width = display_width(grapheme);
            let hanging_space = line_has_text
                && grapheme_end < logical_content_end
                && grapheme.chars().all(is_hangable_space);
            if !hanging_space && line_width > 0 && line_width + grapheme_width > width {
                lines.push(line_start..grapheme_start + 1);
                line_start = grapheme_start;
                line_width = 0;
                line_has_text = false;
            }
            line_width += grapheme_width;
            line_has_text |= !is_whitespace;
            previous_was_whitespace = is_whitespace;

            if words
                .get(word_index)
                .is_some_and(|word| word.range.contains(&grapheme_start))
            {
                consumed_word_width += grapheme_width;
            }
        }

        if matches!(text.as_bytes().get(line_end), None | Some(b'\n')) {
            lines.push(line_start..line_end + 1);
            if line_width >= width {
                lines.push(line_end..line_end + 1);
            }
            line_width = 0;
            line_has_text = false;
            line_active = false;
            previous_was_whitespace = false;
        }
        processed_end = line_end;
    }

    lines
}

pub(super) fn cursor_position(
    text: &str,
    lines: &[Range<usize>],
    width: u16,
    pos: usize,
) -> Option<(usize, usize)> {
    let row = lines
        .partition_point(|line| line.start <= pos)
        .checked_sub(1)?;
    let before_cursor = visible_prefix(&text[lines[row].start..pos], width);
    let before_cursor = text_for_display(before_cursor);
    let col = display_width(before_cursor.as_ref());
    let width = usize::from(width);
    if col >= width
        && lines
            .get(row + 1)
            .is_some_and(|next| next.start == lines[row].end - 1)
    {
        Some((row + 1, 0))
    } else {
        Some((row, col.min(width.saturating_sub(1))))
    }
}

#[cfg(test)]
#[path = "wrapping_tests.rs"]
mod tests;
