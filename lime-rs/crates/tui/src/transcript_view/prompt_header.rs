//! Presentation-only prompt context for the compact main transcript.
//!
//! Composition records canonical entry ranges beside the rendered lines. The header consumes that
//! metadata without becoming transcript content, so selection, copy, search and export keep using
//! the exact canonical projection.

use std::cell::RefCell;
use std::ops::Range;

use ratatui::layout::Rect;
use ratatui::text::Line;

use crate::history_cell::sanitize_user_text;
use crate::line_truncation::truncate_line_with_ellipsis_if_overflow;
use crate::locale::Locale;
use crate::projection::{EntryKind, TranscriptEntry};
use crate::style::history_prompt_style;
use crate::terminal_hyperlinks::{wrapped_line_starts, HyperlinkLine, HyperlinkParagraph};

const MAX_PROMPT_CHARS: usize = 512;
const MIN_HEADER_WIDTH: u16 = 16;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct PromptHeaderSource {
    cells: Vec<PromptCell>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PromptCell {
    key: String,
    lines: Range<usize>,
    prompt: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PromptHeaderCandidate {
    pub(crate) line: Line<'static>,
    position: SuppressedHeader,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PromptHeaderLayout {
    pub(crate) body: Rect,
    pub(crate) scroll: usize,
    pub(crate) line: Option<Line<'static>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SuppressedHeader {
    viewport: Rect,
    key: String,
    row: usize,
}

#[derive(Debug, Default)]
pub(crate) struct TranscriptPromptHeader {
    suppressed: RefCell<Option<SuppressedHeader>>,
    last_source: RefCell<Option<PromptHeaderSource>>,
}

impl PromptHeaderSource {
    pub(crate) fn record_other(&mut self, key: impl Into<String>, lines: Range<usize>) {
        self.record(key.into(), lines, None);
    }

    pub(crate) fn record_entry(
        &mut self,
        entry: &TranscriptEntry,
        locale: Locale,
        lines: Range<usize>,
    ) {
        let prompt = (entry.kind == EntryKind::User)
            .then(|| prompt_text(entry, locale))
            .flatten();
        self.record(format!("entry:{}", entry.id), lines, prompt);
    }

    pub(crate) fn append_shifted(&mut self, mut other: Self, offset: usize) {
        for cell in &mut other.cells {
            cell.lines =
                cell.lines.start.saturating_add(offset)..cell.lines.end.saturating_add(offset);
        }
        self.cells.extend(other.cells);
    }

    pub(crate) fn candidate(
        &self,
        lines: &[HyperlinkLine],
        viewport: Rect,
        offset: usize,
    ) -> Option<PromptHeaderCandidate> {
        if viewport.width < MIN_HEADER_WIDTH || lines.is_empty() {
            return None;
        }
        let starts = wrapped_line_starts(lines, viewport.width);
        let total_height = HyperlinkParagraph::new(lines).line_count(viewport.width);
        let (first_index, first, first_start) =
            self.cells.iter().enumerate().find_map(|(index, cell)| {
                let start = starts.get(cell.lines.start).copied()?;
                let end = starts.get(cell.lines.end).copied().unwrap_or(total_height);
                (end > offset).then_some((index, cell, start))
            })?;
        if first.prompt.is_some() {
            return None;
        }
        let prompt = self.cells[..first_index]
            .iter()
            .rev()
            .find_map(|cell| cell.prompt.as_deref())?;
        Some(PromptHeaderCandidate {
            line: truncate_line_with_ellipsis_if_overflow(
                Line::styled(prompt.to_string(), history_prompt_style()),
                usize::from(viewport.width),
            ),
            position: SuppressedHeader {
                viewport,
                key: first.key.clone(),
                row: offset.saturating_sub(first_start),
            },
        })
    }

    fn record(&mut self, key: String, lines: Range<usize>, prompt: Option<String>) {
        if lines.is_empty() {
            return;
        }
        self.cells.push(PromptCell { key, lines, prompt });
    }
}

impl TranscriptPromptHeader {
    /// Keep entry metadata paired with the frozen selection source until selection ends.
    pub(crate) fn source(
        &self,
        current: &PromptHeaderSource,
        selection_frozen: bool,
    ) -> PromptHeaderSource {
        let mut last = self.last_source.borrow_mut();
        if !selection_frozen || last.is_none() {
            *last = Some(current.clone());
        }
        last.clone().unwrap_or_else(|| current.clone())
    }

    pub(crate) fn layout(
        &self,
        source: &PromptHeaderSource,
        lines: &[HyperlinkLine],
        area: Rect,
        initial_scroll: usize,
        reserved_scroll: usize,
    ) -> PromptHeaderLayout {
        let initial = source.candidate(lines, area, initial_scroll);
        if area.height < 4 {
            self.clear_suppression();
            return PromptHeaderLayout {
                body: area,
                scroll: initial_scroll,
                line: None,
            };
        }
        let Some(initial) = initial else {
            self.clear_suppression();
            return PromptHeaderLayout {
                body: area,
                scroll: initial_scroll,
                line: None,
            };
        };
        if self.is_suppressed(&initial) {
            return PromptHeaderLayout {
                body: area,
                scroll: initial_scroll,
                line: None,
            };
        }
        let body = Rect::new(
            area.x,
            area.y.saturating_add(1),
            area.width,
            area.height.saturating_sub(1),
        );
        if let Some(reserved) = source.candidate(lines, area, reserved_scroll) {
            return PromptHeaderLayout {
                body,
                scroll: reserved_scroll,
                line: Some(reserved.line),
            };
        }
        self.suppress(&initial);
        PromptHeaderLayout {
            body: area,
            scroll: initial_scroll,
            line: None,
        }
    }

    pub(crate) fn is_suppressed(&self, candidate: &PromptHeaderCandidate) -> bool {
        let mut suppressed = self.suppressed.borrow_mut();
        if suppressed.as_ref() == Some(&candidate.position) {
            true
        } else {
            *suppressed = None;
            false
        }
    }

    pub(crate) fn suppress(&self, candidate: &PromptHeaderCandidate) {
        self.suppressed.replace(Some(candidate.position.clone()));
    }

    pub(crate) fn clear_suppression(&self) {
        self.suppressed.replace(None);
    }

    pub(crate) fn clear(&self) {
        self.suppressed.replace(None);
        self.last_source.replace(None);
    }

    #[cfg(test)]
    pub(crate) fn has_suppression(&self) -> bool {
        self.suppressed.borrow().is_some()
    }
}

fn prompt_text(entry: &TranscriptEntry, locale: Locale) -> Option<String> {
    let prefix = entry
        .text
        .chars()
        .take(MAX_PROMPT_CHARS)
        .collect::<String>();
    let message = sanitize_user_text(prefix.into())
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if !message.is_empty() {
        return Some(message);
    }
    entry
        .summary
        .iter()
        .any(|detail| detail.starts_with("image: "))
        .then(|| locale.transcript_attachments_label().to_string())
}

#[cfg(test)]
#[path = "prompt_header_tests.rs"]
mod tests;
