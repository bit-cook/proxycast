//! Plan history cells.

use super::*;

#[derive(Debug, Clone)]
pub(crate) struct PlanUpdateCell {
    pub(crate) text: String,
}

impl PlanUpdateCell {
    pub(crate) fn new(text: impl Into<String>) -> Self {
        Self { text: text.into() }
    }
}

impl HistoryCell for PlanUpdateCell {
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
                kind: EntryKind::Plan,
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

pub(crate) type ProposedPlanCell = PlanUpdateCell;
pub(crate) type ProposedPlanStreamCell = PlanUpdateCell;
pub(crate) type StreamingPlanTailCell = PlanUpdateCell;
