//! Patch history cells.

use super::*;

#[derive(Debug, Clone)]
pub(crate) struct PatchHistoryCell {
    pub(crate) text: String,
}

impl PatchHistoryCell {
    pub(crate) fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

impl HistoryCell for PatchHistoryCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        self.display_hyperlink_lines(width)
            .into_iter()
            .map(|line| line.line)
            .collect()
    }

    fn display_hyperlink_lines(&self, width: u16) -> Vec<HyperlinkLine> {
        TranscriptHistoryCell::new(
            TranscriptEntry {
                id: String::new(),
                kind: EntryKind::Patch,
                text: self.text.clone(),
                streaming: false,
                status: None,
                summary: Vec::new(),
            },
            Locale::default(),
            std::path::PathBuf::new(),
        )
        .display_hyperlink_lines(width)
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        self.display_lines(u16::MAX)
    }
}
