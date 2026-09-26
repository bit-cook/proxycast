//! Main transcript footer composition for Find and text-selection interactions.
//!
//! The query owns the composer gap while Find is active; status owns the ordinary footer row.
//! This keeps the editor caret visible without inserting synthetic transcript rows.

use ratatui::layout::{Position, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use unicode_segmentation::UnicodeSegmentation;

use crate::line_truncation::truncate_line_with_ellipsis_if_overflow;
use crate::locale::Locale;
use crate::width::display_width;

use super::{SearchBoundary, SearchHistoryState, TranscriptSearch};

#[derive(Debug, Default)]
pub(crate) struct TranscriptFooter;

impl TranscriptFooter {
    pub(crate) fn render_search_query(
        &self,
        frame: &mut Frame<'_>,
        area: Option<Rect>,
        locale: Locale,
        search: &TranscriptSearch,
    ) -> bool {
        if !search.is_active() {
            return false;
        }
        let Some(area) = area.filter(|area| !area.is_empty()) else {
            return true;
        };
        let width = usize::from(area.width);
        let label = locale.transcript_search_label();
        let label_width = display_width(label).min(width.saturating_sub(1));
        let visible_label = truncate_text(label, label_width, /*from_end*/ false);
        let query_width = width.saturating_sub(display_width(&visible_label) + 1);
        let visible_query = truncate_text(search.query(), query_width, /*from_end*/ true);
        let cursor_column = display_width(&visible_label)
            .saturating_add(display_width(&visible_query))
            .min(width.saturating_sub(1));
        let line = Line::from(vec![
            Span::styled(visible_label, crate::style::muted_style()),
            Span::raw(visible_query),
            Span::styled("▏", crate::style::accent_style()),
        ]);
        frame.render_widget(
            Paragraph::new(truncate_line_with_ellipsis_if_overflow(line, width)),
            area,
        );
        frame.set_cursor_position(Position::new(
            area.x
                .saturating_add(u16::try_from(cursor_column).unwrap_or(u16::MAX)),
            area.y,
        ));
        true
    }

    pub(crate) fn render_status(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        locale: Locale,
        search: &TranscriptSearch,
        selection_active: bool,
    ) -> bool {
        if !search.is_active() {
            return false;
        }
        if area.is_empty() {
            return true;
        }
        let status = if selection_active {
            locale.transcript_selection_hint().to_string()
        } else {
            search_status(search, locale)
        };
        let limit = search
            .query_truncated()
            .then(|| locale.transcript_search_query_limited());
        let full = match limit {
            Some(limit) => format!("{status}  {limit}"),
            None => status,
        };
        frame.render_widget(
            Paragraph::new(truncate_line_with_ellipsis_if_overflow(
                Line::styled(full, Style::default().patch(crate::style::muted_style())),
                usize::from(area.width),
            )),
            area,
        );
        true
    }
}

pub(crate) fn search_status(search: &TranscriptSearch, locale: Locale) -> String {
    let status = if search.history_state() == SearchHistoryState::Loading {
        locale.transcript_pager_loading().to_string()
    } else if search.history_state() == SearchHistoryState::Failed {
        locale.transcript_pager_retry_footer().to_string()
    } else if search.query().is_empty() {
        locale.transcript_search_prompt().to_string()
    } else if !search.matches_valid() {
        locale.transcript_search_searching().to_string()
    } else if search.boundary() != SearchBoundary::None {
        locale.transcript_search_no_more_matches().to_string()
    } else if search.match_count() == 0 {
        locale.transcript_search_no_matches().to_string()
    } else {
        format!(
            "{} {}/{}",
            locale.transcript_search_matches(),
            search.cursor() + 1,
            search.match_count()
        )
    };
    format!("{status}  {}", locale.transcript_search_hint())
}

fn truncate_text(text: &str, max_width: usize, from_end: bool) -> String {
    if max_width == 0 {
        return String::new();
    }
    if display_width(text) <= max_width {
        return text.to_string();
    }
    let marker = "…";
    let available = max_width.saturating_sub(display_width(marker));
    if available == 0 {
        return marker.to_string();
    }
    if from_end {
        let mut width = 0usize;
        let mut start = text.len();
        for (offset, grapheme) in text.grapheme_indices(true).rev() {
            let next = width.saturating_add(display_width(grapheme));
            if next > available {
                break;
            }
            start = offset;
            width = next;
        }
        format!("{marker}{}", &text[start..])
    } else {
        let mut width = 0usize;
        let mut end = 0usize;
        for (offset, grapheme) in text.grapheme_indices(true) {
            let next = width.saturating_add(display_width(grapheme));
            if next > available {
                break;
            }
            end = offset + grapheme.len();
            width = next;
        }
        format!("{}{marker}", &text[..end])
    }
}

#[cfg(test)]
#[path = "footer_tests.rs"]
mod tests;
