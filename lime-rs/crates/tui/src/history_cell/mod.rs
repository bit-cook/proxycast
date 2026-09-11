//! Codex-shaped transcript cells backed by Lime's canonical projection.
//!
//! A history cell owns terminal presentation for one canonical `ThreadItem`. It never owns
//! session state, persistence, provider output, or a second transcript model.

use std::any::Any;
use std::path::PathBuf;

use ratatui::text::{Line, Text};
use ratatui::widgets::{Paragraph, Wrap};

use crate::entry;
use crate::locale::Locale;
use crate::projection::{EntryKind, TranscriptEntry};
use crate::terminal_hyperlinks::{plain_hyperlink_lines, visible_lines_ref, HyperlinkLine};

mod approvals;
mod base;
mod exec;
mod mcp;
mod messages;
mod notices;
mod patches;
mod plans;
mod request_user_input;
mod separators;
mod session;

pub(crate) use approvals::*;
pub(crate) use base::*;
pub(crate) use exec::*;
pub(crate) use mcp::*;
pub(crate) use messages::*;
pub(crate) use notices::*;
pub(crate) use patches::*;
pub(crate) use plans::*;
pub(crate) use request_user_input::*;
pub(crate) use separators::*;
pub(crate) use session::*;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HistoryRenderMode {
    Rich,
    Raw,
}

pub(crate) fn plain_lines(lines: impl IntoIterator<Item = Line<'static>>) -> Vec<Line<'static>> {
    lines
        .into_iter()
        .map(|line| {
            let text = line
                .spans
                .into_iter()
                .map(|span| span.content.into_owned())
                .collect::<String>();
            Line::from(text)
        })
        .collect()
}

pub(crate) trait HistoryCell: std::fmt::Debug + Send + Sync + Any {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>>;

    fn raw_lines(&self) -> Vec<Line<'static>>;

    fn display_hyperlink_lines(&self, width: u16) -> Vec<HyperlinkLine> {
        plain_hyperlink_lines(self.display_lines(width))
    }

    fn display_lines_for_mode(&self, width: u16, mode: HistoryRenderMode) -> Vec<Line<'static>> {
        match mode {
            HistoryRenderMode::Rich => visible_lines_ref(&self.display_hyperlink_lines(width)),
            HistoryRenderMode::Raw => self.raw_lines(),
        }
    }

    fn display_hyperlink_lines_for_mode(
        &self,
        width: u16,
        mode: HistoryRenderMode,
    ) -> Vec<HyperlinkLine> {
        match mode {
            HistoryRenderMode::Rich => self.display_hyperlink_lines(width),
            HistoryRenderMode::Raw => plain_hyperlink_lines(self.raw_lines()),
        }
    }

    fn desired_height(&self, width: u16) -> u16 {
        self.desired_height_for_mode(width, HistoryRenderMode::Rich)
    }

    fn desired_height_for_mode(&self, width: u16, mode: HistoryRenderMode) -> u16 {
        Paragraph::new(Text::from(self.display_lines_for_mode(width, mode)))
            .wrap(Wrap { trim: false })
            .line_count(width)
            .try_into()
            .unwrap_or(0)
    }

    fn transcript_lines(&self, width: u16) -> Vec<Line<'static>> {
        self.display_lines(width)
    }

    fn transcript_hyperlink_lines(&self, width: u16) -> Vec<HyperlinkLine> {
        plain_hyperlink_lines(self.transcript_lines(width))
    }

    fn desired_transcript_height(&self, width: u16) -> u16 {
        Paragraph::new(Text::from(visible_lines_ref(
            &self.transcript_hyperlink_lines(width),
        )))
        .wrap(Wrap { trim: false })
        .line_count(width)
        .try_into()
        .unwrap_or(0)
    }

    fn has_stable_transcript_height(&self) -> bool {
        true
    }

    fn is_stream_continuation(&self) -> bool {
        false
    }

    fn transcript_animation_tick(&self) -> Option<u64> {
        None
    }
}

/// Adapter from the canonical projection to the Codex-shaped cell interface.
#[derive(Clone, Debug)]
pub(crate) struct TranscriptHistoryCell {
    entry: TranscriptEntry,
    locale: Locale,
    cwd: PathBuf,
}

impl TranscriptHistoryCell {
    pub(crate) fn new(entry: TranscriptEntry, locale: Locale, cwd: PathBuf) -> Self {
        Self { entry, locale, cwd }
    }

    pub(crate) fn entry(&self) -> &TranscriptEntry {
        &self.entry
    }
}

impl HistoryCell for TranscriptHistoryCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        self.display_hyperlink_lines(width)
            .into_iter()
            .map(|line| line.line)
            .collect()
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        plain_lines(
            entry::hyperlink_lines_with_locale(&self.entry, self.locale, None, &self.cwd)
                .into_iter()
                .map(|line| line.line),
        )
    }

    fn display_hyperlink_lines(&self, width: u16) -> Vec<HyperlinkLine> {
        entry::hyperlink_lines_with_locale(
            &self.entry,
            self.locale,
            Some(usize::from(width)),
            &self.cwd,
        )
    }

    fn is_stream_continuation(&self) -> bool {
        self.entry.streaming
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection::EntryStatus;

    fn entry(kind: EntryKind, text: &str) -> TranscriptEntry {
        TranscriptEntry {
            id: "entry-1".to_string(),
            kind,
            text: text.to_string(),
            streaming: false,
            status: Some(EntryStatus::Completed),
            summary: Vec::new(),
        }
    }

    #[test]
    fn canonical_projection_adapter_preserves_item_identity_and_text() {
        let cell = TranscriptHistoryCell::new(
            entry(EntryKind::Assistant, "hello"),
            Locale::EnUs,
            PathBuf::from("/workspace"),
        );

        assert_eq!(cell.entry().id, "entry-1");
        assert_eq!(cell.display_lines(80)[0].spans[1].content, "hello");
        assert_eq!(cell.raw_lines()[0].spans[0].content, "  hello [completed]");
    }

    #[test]
    fn streaming_projection_marks_cell_as_stream_continuation() {
        let mut item = entry(EntryKind::Assistant, "partial");
        item.streaming = true;
        let cell = TranscriptHistoryCell::new(item, Locale::EnUs, PathBuf::from("/workspace"));
        assert!(cell.is_stream_continuation());
    }

    #[test]
    fn user_history_cell_wraps_and_prefixes_each_line_snapshot() {
        let cell = TranscriptHistoryCell::new(
            entry(EntryKind::User, "first line\nsecond line"),
            Locale::EnUs,
            PathBuf::from("/workspace"),
        );
        let lines = cell.display_lines(80);
        assert_eq!(lines[0].spans[0].content, "> ");
        assert_eq!(lines[1].spans[0].content, "  ");
        assert!(lines.iter().any(|line| {
            line.spans
                .iter()
                .any(|span| span.content.as_ref() == "second line")
        }));
    }
}
