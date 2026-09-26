//! Shared selection gestures and source-text boundaries; each view owns its hit testing and state.

use std::ops::Range;
use std::time::{Duration, Instant};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SelectionUnit {
    Character,
    Word,
    Line,
}

impl SelectionUnit {
    pub(crate) fn from_clicks(clicks: u8) -> Self {
        match clicks {
            2 => Self::Word,
            3 => Self::Line,
            _ => Self::Character,
        }
    }

    pub(crate) fn range(self, text: &str, offset: usize) -> Range<usize> {
        let offset = floor_char_boundary(text, offset.min(text.len()));
        match self {
            Self::Character => offset..offset,
            Self::Word => text
                .split_word_bound_indices()
                .map(|(start, word)| start..start + word.len())
                .find(|range| range.contains(&offset))
                .unwrap_or(text.len()..text.len()),
            // Logical lines include their hard newline, regardless of visual wrapping.
            Self::Line => {
                let start = text[..offset].rfind('\n').map_or(0, |newline| newline + 1);
                let end = text[offset..]
                    .find('\n')
                    .map_or(text.len(), |newline| offset + newline + 1);
                start..end
            }
        }
    }
}

fn floor_char_boundary(text: &str, mut offset: usize) -> usize {
    while offset > 0 && !text.is_char_boundary(offset) {
        offset -= 1;
    }
    offset
}

pub(crate) fn click_count(
    last_click: &mut Option<(Instant, u16, u16, u8)>,
    column: u16,
    row: u16,
) -> u8 {
    let now = Instant::now();
    let clicks = last_click
        .filter(|(at, x, y, _)| {
            now.duration_since(*at) < Duration::from_millis(400) && *x == column && *y == row
        })
        .map_or(1, |(_, _, _, clicks)| clicks % 3 + 1);
    *last_click = Some((now, column, row, clicks));
    clicks
}

pub(crate) fn is_copy_key(key: KeyEvent) -> bool {
    // Kitty reports Cmd+C as Super+C, including over SSH. Crossterm may report
    // Ctrl+Shift+C as uppercase C with only Control set.
    key.kind != KeyEventKind::Release
        && matches!(key.code, KeyCode::Char(value) if value.eq_ignore_ascii_case(&'c'))
        && (matches!(key.modifiers, KeyModifiers::CONTROL | KeyModifiers::SUPER)
            || key.modifiers == (KeyModifiers::CONTROL | KeyModifiers::SHIFT))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_and_line_selection_stay_on_utf8_boundaries() {
        let text = "你好 world\nnext";
        assert_eq!(SelectionUnit::Word.range(text, 1), 0..3);
        assert_eq!(SelectionUnit::Word.range(text, 4), 3..6);
        assert_eq!(SelectionUnit::Line.range(text, 7), 0..13);
    }

    #[test]
    fn copy_shortcuts_accept_control_shift_and_super() {
        assert!(is_copy_key(KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL | KeyModifiers::SHIFT,
        )));
        assert!(is_copy_key(KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::SUPER,
        )));
        assert!(!is_copy_key(KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::NONE,
        )));
    }
}
