//! Shared history-cell building blocks reused by transcript projections.

use super::*;

use ratatui::text::{Line, Span, Text};

use crate::terminal_hyperlinks::{plain_hyperlink_lines, HyperlinkLine};
use crate::wrapping::{adaptive_wrap_lines, RtOptions};

#[derive(Debug)]
pub(crate) struct PlainHistoryCell {
    pub(super) lines: Vec<Line<'static>>,
}

impl PlainHistoryCell {
    pub(crate) fn new(lines: Vec<Line<'static>>) -> Self {
        Self { lines }
    }
}

impl HistoryCell for PlainHistoryCell {
    fn display_lines(&self, _width: u16) -> Vec<Line<'static>> {
        self.lines.clone()
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        plain_lines(self.lines.clone())
    }
}

#[derive(Debug)]
pub(crate) struct WebHyperlinkHistoryCell {
    lines: Vec<HyperlinkLine>,
}

impl WebHyperlinkHistoryCell {
    pub(crate) fn new(lines: Vec<Line<'static>>) -> Self {
        Self {
            lines: plain_hyperlink_lines(lines),
        }
    }

    pub(crate) fn new_hyperlink_lines(lines: Vec<HyperlinkLine>) -> Self {
        Self { lines }
    }
}

impl HistoryCell for WebHyperlinkHistoryCell {
    fn display_lines(&self, _width: u16) -> Vec<Line<'static>> {
        self.lines.iter().map(|line| line.line.clone()).collect()
    }

    fn display_hyperlink_lines(&self, _width: u16) -> Vec<HyperlinkLine> {
        self.lines.clone()
    }

    fn transcript_hyperlink_lines(&self, width: u16) -> Vec<HyperlinkLine> {
        self.display_hyperlink_lines(width)
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        plain_lines(self.display_lines(0))
    }
}

#[derive(Debug)]
pub(crate) struct PrefixedWrappedHistoryCell {
    text: Text<'static>,
    initial_prefix: Line<'static>,
    subsequent_prefix: Line<'static>,
}

impl PrefixedWrappedHistoryCell {
    pub(crate) fn new(
        text: impl Into<Text<'static>>,
        initial_prefix: impl Into<Line<'static>>,
        subsequent_prefix: impl Into<Line<'static>>,
    ) -> Self {
        Self {
            text: text.into(),
            initial_prefix: initial_prefix.into(),
            subsequent_prefix: subsequent_prefix.into(),
        }
    }
}

impl HistoryCell for PrefixedWrappedHistoryCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        if width == 0 {
            return Vec::new();
        }
        adaptive_wrap_lines(
            &self.text,
            RtOptions::new(usize::from(width))
                .initial_indent(self.initial_prefix.clone())
                .subsequent_indent(self.subsequent_prefix.clone()),
        )
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        plain_lines(self.text.clone().lines)
    }
}

#[derive(Debug)]
pub(crate) struct CompositeHistoryCell {
    pub(super) parts: Vec<Box<dyn HistoryCell>>,
}

impl CompositeHistoryCell {
    pub(crate) fn new(parts: Vec<Box<dyn HistoryCell>>) -> Self {
        Self { parts }
    }
}

impl HistoryCell for CompositeHistoryCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        join_parts(self.parts.iter().map(|part| part.display_lines(width)))
    }

    fn display_hyperlink_lines(&self, width: u16) -> Vec<HyperlinkLine> {
        join_parts(
            self.parts
                .iter()
                .map(|part| part.display_hyperlink_lines(width)),
        )
    }

    fn transcript_hyperlink_lines(&self, width: u16) -> Vec<HyperlinkLine> {
        join_parts(
            self.parts
                .iter()
                .map(|part| part.transcript_hyperlink_lines(width)),
        )
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        join_parts(self.parts.iter().map(|part| part.raw_lines()))
    }

    fn has_stable_transcript_height(&self) -> bool {
        false
    }
}

fn join_parts<T: From<&'static str>>(parts: impl Iterator<Item = Vec<T>>) -> Vec<T> {
    let mut out = Vec::new();
    let mut first = true;
    for mut part in parts {
        if part.is_empty() {
            continue;
        }
        if !first {
            out.push(T::from(""));
        }
        out.append(&mut part);
        first = false;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composite_history_cell_separates_non_empty_parts() {
        let cell = CompositeHistoryCell::new(vec![
            Box::new(PlainHistoryCell::new(vec![Line::from("one")])),
            Box::new(PlainHistoryCell::new(vec![Line::from("two")])),
        ]);
        let lines = cell.display_lines(80);
        assert_eq!(lines[0].spans[0].content, "one");
        assert_eq!(lines[1].width(), 0);
        assert_eq!(lines[2].spans[0].content, "two");
    }
}
