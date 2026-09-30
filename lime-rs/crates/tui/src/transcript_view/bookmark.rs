//! Stable transcript reading bookmarks across pagination, replacement and terminal reflow.

use std::ops::Range;

use crate::terminal_hyperlinks::{wrapped_line_starts, HyperlinkLine, HyperlinkParagraph};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TranscriptAnchorRange {
    pub(crate) keys: Vec<String>,
    pub(crate) lines: Range<usize>,
}

impl TranscriptAnchorRange {
    pub(crate) fn new(keys: Vec<String>, lines: Range<usize>) -> Self {
        Self { keys, lines }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct TranscriptAnchor {
    key: String,
    source_line: usize,
    wrapped_row: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TranscriptBookmark {
    following: bool,
    fallback_scroll: usize,
    anchor: Option<TranscriptAnchor>,
}

impl TranscriptBookmark {
    pub(crate) fn fallback(following: bool, scroll: usize) -> Self {
        Self {
            following,
            fallback_scroll: scroll,
            anchor: None,
        }
    }

    pub(crate) fn capture(frame: &TranscriptFrame, following: bool, scroll: usize) -> Self {
        if following || frame.width == 0 || frame.lines.is_empty() {
            return Self::fallback(following, scroll);
        }
        let starts = wrapped_line_starts(&frame.lines, frame.width);
        let logical_line = starts
            .iter()
            .enumerate()
            .rev()
            .find(|(_, start)| **start <= scroll)
            .map(|(index, start)| (index, scroll.saturating_sub(*start)));
        let anchor = logical_line.and_then(|(line, wrapped_row)| {
            let range = frame
                .anchors
                .iter()
                .find(|range| range.lines.contains(&line))?;
            let key = range.keys.first()?.clone();
            Some(TranscriptAnchor {
                key,
                source_line: line.saturating_sub(range.lines.start),
                wrapped_row,
            })
        });
        Self {
            following,
            fallback_scroll: scroll,
            anchor,
        }
    }

    pub(crate) fn following(&self) -> bool {
        self.following
    }

    pub(crate) fn fallback_scroll(&self) -> usize {
        self.fallback_scroll
    }

    pub(crate) fn resolve(&self, frame: &TranscriptFrame, max_scroll: usize) -> Option<usize> {
        if self.following {
            return Some(max_scroll);
        }
        let anchor = self.anchor.as_ref()?;
        let range = frame
            .anchors
            .iter()
            .find(|range| range.keys.iter().any(|key| key == &anchor.key))?;
        if range.lines.is_empty() {
            return None;
        }
        let logical_line = range
            .lines
            .start
            .saturating_add(anchor.source_line.min(range.lines.len() - 1));
        let starts = wrapped_line_starts(&frame.lines, frame.width);
        let start = *starts.get(logical_line)?;
        let total_rows = HyperlinkParagraph::new(&frame.lines).line_count(frame.width);
        let end = starts.get(logical_line + 1).copied().unwrap_or(total_rows);
        let row_count = end.saturating_sub(start).max(1);
        Some(
            start
                .saturating_add(anchor.wrapped_row.min(row_count - 1))
                .min(max_scroll),
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TranscriptFrame {
    lines: Vec<HyperlinkLine>,
    anchors: Vec<TranscriptAnchorRange>,
    width: u16,
}

impl TranscriptFrame {
    pub(crate) fn new(
        lines: Vec<HyperlinkLine>,
        anchors: Vec<TranscriptAnchorRange>,
        width: u16,
    ) -> Self {
        Self {
            lines,
            anchors,
            width,
        }
    }

    pub(crate) fn lines(&self) -> &[HyperlinkLine] {
        &self.lines
    }

    pub(crate) fn width(&self) -> u16 {
        self.width
    }
}

#[cfg(test)]
#[path = "bookmark_tests.rs"]
mod tests;
