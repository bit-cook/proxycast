//! Informational, warning, and recap history cells.

use super::*;

macro_rules! notice_cell {
    ($name:ident) => {
        #[derive(Debug, Clone)]
        pub(crate) struct $name {
            pub(crate) message: String,
        }

        impl $name {
            pub(crate) fn new(message: impl Into<String>) -> Self {
                Self {
                    message: message.into(),
                }
            }
        }

        impl HistoryCell for $name {
            fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
                TranscriptHistoryCell::new(
                    TranscriptEntry {
                        id: String::new(),
                        kind: EntryKind::System,
                        text: self.message.clone(),
                        streaming: false,
                        status: None,
                        summary: Vec::new(),
                    },
                    Locale::default(),
                    std::path::PathBuf::new(),
                )
                .display_lines(width)
            }

            fn raw_lines(&self) -> Vec<Line<'static>> {
                vec![Line::from(self.message.clone())]
            }
        }
    };
}

notice_cell!(UpdateAvailableHistoryCell);
notice_cell!(SafetyAccessBlockCell);
notice_cell!(DeprecationNoticeCell);
notice_cell!(ThreadRecapLoadingCell);
notice_cell!(ThreadRecapHistoryCell);
