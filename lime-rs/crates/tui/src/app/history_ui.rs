//! Canonical transcript presentation helpers for the TUI app.
//!
//! The app owns projection-to-view composition here; history persistence and turn state remain
//! owned by App Server and `ConversationProjection`.

use crate::app::App;
use crate::history_cell::{HistoryCell, TranscriptHistoryCell};
use crate::locale::Locale;
use crate::projection::TranscriptEntry;
use crate::terminal_hyperlinks::{wrap_hyperlink_line, HyperlinkLine};
use std::path::Path;

/// Render one canonical transcript entry with the same viewport padding used by the app.
pub(crate) fn render_transcript_entry_lines(
    entry: &TranscriptEntry,
    viewport_width: u16,
    locale: Locale,
    cwd: &Path,
) -> Vec<HyperlinkLine> {
    let content_width = viewport_width.saturating_sub(2).max(1);
    TranscriptHistoryCell::new(entry.clone(), locale, cwd.to_path_buf())
        .display_hyperlink_lines(content_width)
}

/// Render one canonical transcript entry with terminal-native wrapping for bounded list rows.
pub(crate) fn render_transcript_entry_lines_wrapped(
    entry: &TranscriptEntry,
    viewport_width: u16,
    locale: Locale,
    cwd: &Path,
) -> Vec<HyperlinkLine> {
    let content_width = viewport_width.saturating_sub(2).max(1);
    render_transcript_entry_lines(entry, viewport_width, locale, cwd)
        .into_iter()
        .flat_map(|line| wrap_hyperlink_line(&line, usize::from(content_width)))
        .collect()
}

/// Render canonical transcript entries for the main viewport or transcript pager.
pub(crate) fn render_transcript_content_lines(
    app: &App,
    viewport_width: u16,
    separate_entries: bool,
) -> Vec<HyperlinkLine> {
    let mut lines = Vec::new();
    for entry in app.projection.entries() {
        if separate_entries && !lines.is_empty() {
            lines.push(HyperlinkLine::default());
        }
        lines.extend(render_transcript_entry_lines(
            entry,
            viewport_width,
            app.locale,
            &app.cwd,
        ));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locale::Locale;

    #[test]
    fn transcript_content_lines_use_canonical_entry_order() {
        let mut app = App {
            locale: Locale::EnUs,
            ..App::default()
        };
        app.projection.apply(
            app_server_protocol::protocol::v2::ServerNotification::Warning(
                app_server_protocol::protocol::v2::WarningNotification {
                    thread_id: None,
                    message: "warning".to_string(),
                    code: None,
                },
            ),
        );
        let lines = render_transcript_content_lines(&app, 80, false);
        assert_eq!(lines.len(), 1);
        assert!(lines[0]
            .line
            .spans
            .iter()
            .any(|span| span.content.contains("warning")));
    }

    #[test]
    fn transcript_entry_lines_share_cell_rendering_and_fit_viewport() {
        let entry = TranscriptEntry {
            id: "assistant-1".to_string(),
            kind: crate::projection::EntryKind::Assistant,
            text: "a long assistant response that must wrap".to_string(),
            streaming: false,
            status: None,
            summary: Vec::new(),
        };
        let lines = render_transcript_entry_lines_wrapped(
            &entry,
            24,
            Locale::EnUs,
            Path::new("/workspace"),
        );

        assert!(!lines.is_empty());
        assert!(lines.iter().all(|line| line.width() <= 24));
        assert!(lines
            .iter()
            .flat_map(|line| line.line.spans.iter())
            .any(|span| span.content.contains("assistant")));
    }
}
