//! Share the composer gap between transcript copy feedback and the follow control.
//!
//! Copy feedback wins while visible. The state is local presentation only and expires through the
//! existing frame scheduler; clipboard truth still comes from `CopyStatus`.

use std::cell::Cell;
use std::time::{Duration, Instant};

use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::clipboard_copy::CopyStatus;
use crate::line_truncation::truncate_line_with_ellipsis_if_overflow;
use crate::locale::Locale;

const COPY_FEEDBACK_DURATION: Duration = Duration::from_secs(5);

#[derive(Clone, Copy, Debug)]
struct CopyFeedback {
    result: Result<CopyStatus, ()>,
    characters: usize,
    expires_at: Instant,
}

#[derive(Debug, Default)]
pub(crate) struct TranscriptComposerGap {
    feedback: Cell<Option<CopyFeedback>>,
}

impl TranscriptComposerGap {
    pub(crate) fn show_copy_feedback(
        &self,
        result: &Result<CopyStatus, String>,
        characters: usize,
    ) {
        self.feedback.set(Some(CopyFeedback {
            result: result.as_ref().copied().map_err(|_| ()),
            characters,
            expires_at: Instant::now() + COPY_FEEDBACK_DURATION,
        }));
    }

    pub(crate) fn clear(&self) {
        self.feedback.set(None);
    }

    /// Clear expired feedback or return the exact delay for the existing frame scheduler.
    pub(crate) fn tick(&self, now: Instant) -> Option<Duration> {
        let feedback = self.feedback.get()?;
        if feedback.expires_at <= now {
            self.feedback.set(None);
            None
        } else {
            Some(feedback.expires_at.saturating_duration_since(now))
        }
    }

    /// Render feedback right-aligned and report whether it owns the composer gap this frame.
    pub(crate) fn render(&self, frame: &mut Frame<'_>, area: Option<Rect>, locale: Locale) -> bool {
        let Some(feedback) = self.feedback.get() else {
            return false;
        };
        let Some(area) = area.filter(|area| !area.is_empty()) else {
            return true;
        };
        let available = area.width.saturating_sub(1);
        if available == 0 {
            return true;
        }
        let text = match feedback.result {
            Ok(CopyStatus::Confirmed) => locale.transcript_copy_confirmed(feedback.characters),
            Ok(CopyStatus::Unconfirmed) => locale.transcript_copy_unconfirmed().to_string(),
            Err(()) => locale.transcript_copy_failed().to_string(),
        };
        let line =
            truncate_line_with_ellipsis_if_overflow(Line::from(text), usize::from(available));
        let width = u16::try_from(line.width())
            .unwrap_or(u16::MAX)
            .min(available);
        let target = Rect::new(
            area.right().saturating_sub(width + 1).max(area.x),
            area.y,
            width,
            1,
        );
        let style = if feedback.result.is_err() {
            Style::default().fg(Color::Red)
        } else {
            crate::style::accent_style()
        };
        frame.render_widget(Paragraph::new(line).style(style), target);
        true
    }

    #[cfg(test)]
    pub(crate) fn expire_for_test(&self) {
        if let Some(mut feedback) = self.feedback.get() {
            feedback.expires_at = Instant::now();
            self.feedback.set(Some(feedback));
        }
    }
}

#[cfg(test)]
#[path = "composer_gap_tests.rs"]
mod tests;
