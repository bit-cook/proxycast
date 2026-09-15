//! Tracks when the canonical TUI transcript should be rebuilt after resize.
//!
//! This state machine is intentionally renderer agnostic. The App owns the
//! projection; this module only records observed/rebuilt widths and ensures a
//! resize during streaming receives one final source-backed repaint.

use std::cell::{Cell, RefCell};
use std::time::{Duration, Instant};

use ratatui::layout::Rect;

use crate::terminal_hyperlinks::{HyperlinkLine, HyperlinkParagraph};

pub(crate) const TRANSCRIPT_REFLOW_DEBOUNCE: Duration = Duration::from_millis(75);

#[derive(Debug, Default)]
pub(crate) struct TranscriptReflowState {
    last_observed_width: Option<u16>,
    last_reflow_width: Option<u16>,
    pending_reflow_width: Option<u16>,
    pending_until: Option<Instant>,
    visible_history_rows: Option<u16>,
    ran_during_stream: bool,
    resize_requested_during_stream: bool,
}

impl TranscriptReflowState {
    pub(crate) fn clear(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn set_visible_history_rows(&mut self, rows: u16) {
        self.visible_history_rows = Some(rows.max(1));
    }

    pub(crate) fn visible_history_rows(&self) -> Option<u16> {
        self.visible_history_rows
    }

    pub(crate) fn note_width(&mut self, width: u16) -> TranscriptWidthChange {
        let previous_width = self.last_observed_width.replace(width);
        if previous_width.is_none() {
            self.last_reflow_width = Some(width);
        }
        TranscriptWidthChange {
            changed: previous_width.is_some_and(|previous| previous != width),
            initialized: previous_width.is_none(),
        }
    }

    pub(crate) fn reflow_needed_for_width(&self, width: u16) -> bool {
        self.last_reflow_width != Some(width) && self.pending_reflow_width != Some(width)
    }

    pub(crate) fn schedule_debounced(&mut self, target_width: Option<u16>) -> bool {
        if let Some(target_width) = target_width {
            self.pending_reflow_width = Some(target_width);
        }
        self.pending_until = Some(Instant::now() + TRANSCRIPT_REFLOW_DEBOUNCE);
        false
    }

    pub(crate) fn schedule_immediate(&mut self) {
        self.pending_reflow_width = None;
        self.pending_until = Some(Instant::now());
    }

    #[cfg(test)]
    pub(crate) fn set_due_for_test(&mut self) {
        self.pending_until = Some(Instant::now() - Duration::from_millis(1));
    }

    pub(crate) fn pending_is_due(&self, now: Instant) -> bool {
        self.pending_until.is_some_and(|deadline| now >= deadline)
    }

    pub(crate) fn pending_until(&self) -> Option<Instant> {
        self.pending_until
    }

    pub(crate) fn has_pending_reflow(&self) -> bool {
        self.pending_until.is_some()
    }

    pub(crate) fn clear_pending_reflow(&mut self) {
        self.pending_until = None;
        self.pending_reflow_width = None;
    }

    pub(crate) fn mark_reflowed_width(&mut self, width: u16) -> bool {
        self.last_reflow_width.replace(width) != Some(width)
    }

    pub(crate) fn mark_ran_during_stream(&mut self) {
        self.ran_during_stream = true;
    }

    pub(crate) fn mark_resize_requested_during_stream(&mut self) {
        self.resize_requested_during_stream = true;
    }

    pub(crate) fn take_stream_finish_reflow_needed(&mut self) -> bool {
        let needed = self.ran_during_stream || self.resize_requested_during_stream;
        self.ran_during_stream = false;
        self.resize_requested_during_stream = false;
        needed
    }

    pub(crate) fn clear_stream_flags(&mut self) {
        self.ran_during_stream = false;
        self.resize_requested_during_stream = false;
    }
}

pub(crate) struct TranscriptWidthChange {
    pub(crate) changed: bool,
    pub(crate) initialized: bool,
}

/// Keeps the main transcript viewport anchored while canonical content is replaced or grows.
///
/// `App::transcript_scroll` remains the user-facing distance-from-bottom input contract. This
/// state records the last rendered canonical lines and resolves that relative request into an
/// absolute wrapped-row offset when a stream delta or resize changes the rendered height.
#[derive(Debug, Default)]
pub(crate) struct TranscriptViewport {
    previous_lines: RefCell<Option<Vec<HyperlinkLine>>>,
    previous_width: Cell<u16>,
    previous_offset: Cell<usize>,
    previous_requested_scroll: Cell<Option<usize>>,
}

impl TranscriptViewport {
    pub(crate) fn clear(&self) {
        *self.previous_lines.borrow_mut() = None;
        self.previous_width.set(0);
        self.previous_offset.set(0);
        self.previous_requested_scroll.set(None);
    }

    pub(crate) fn resolve(
        &self,
        lines: &[HyperlinkLine],
        area: Rect,
        requested_distance_from_bottom: usize,
    ) -> u16 {
        if area.width == 0 || area.height == 0 {
            return 0;
        }

        let paragraph = HyperlinkParagraph::new(lines);
        let total_height = paragraph.line_count(area.width);
        let max_scroll = total_height.saturating_sub(usize::from(area.height));
        let base_offset = max_scroll.saturating_sub(requested_distance_from_bottom.min(max_scroll));
        let requested_is_unchanged =
            self.previous_requested_scroll.get() == Some(requested_distance_from_bottom);
        let mut offset = base_offset;

        if requested_distance_from_bottom > 0 && requested_is_unchanged {
            if let Some(previous) = self.previous_lines.borrow().as_ref().cloned() {
                let old_width = self.previous_width.get();
                if old_width > 0 {
                    if let Some(mapped) = remap_transcript_offset(
                        &previous,
                        old_width,
                        self.previous_offset.get(),
                        lines,
                        area.width,
                    ) {
                        offset = mapped.min(max_scroll);
                    } else if previous == lines {
                        // A height-only resize has no logical content change. Keep the same top row
                        // and clamp it to the new page bounds.
                        offset = self.previous_offset.get().min(max_scroll);
                    }
                }
            }
        }

        *self.previous_lines.borrow_mut() = Some(lines.to_vec());
        self.previous_width.set(area.width);
        self.previous_offset.set(offset);
        self.previous_requested_scroll
            .set(Some(requested_distance_from_bottom));
        u16::try_from(offset).unwrap_or(u16::MAX)
    }
}

fn remap_transcript_offset(
    previous: &[HyperlinkLine],
    old_width: u16,
    old_offset: usize,
    lines: &[HyperlinkLine],
    width: u16,
) -> Option<usize> {
    if previous.is_empty() || lines.is_empty() || old_width == 0 || width == 0 {
        return None;
    }

    let old_starts = wrapped_line_starts(previous, old_width);
    let (old_line, intra_line_offset) = old_starts
        .iter()
        .enumerate()
        .rev()
        .find(|(_, start)| **start <= old_offset)
        .map(|(index, start)| (index, old_offset.saturating_sub(*start)))
        .unwrap_or((0, old_offset));

    let common_prefix = previous
        .iter()
        .zip(lines)
        .take_while(|(old, new)| old == new)
        .count();
    let common_suffix = previous
        .iter()
        .rev()
        .zip(lines.iter().rev())
        .take_while(|(old, new)| old == new)
        .count()
        .min(previous.len().saturating_sub(common_prefix));

    let mapped_line = if lines.len() == previous.len()
        && (common_prefix > 0 || common_suffix > 0 || previous.len() == 1)
    {
        old_line
    } else if lines.len() >= previous.len() && lines[lines.len() - previous.len()..] == previous[..]
    {
        old_line + lines.len() - previous.len()
    } else if lines.len() >= previous.len() && lines[..previous.len()] == previous[..] {
        old_line
    } else {
        return None;
    };

    let new_starts = wrapped_line_starts(lines, width);
    let new_start = *new_starts.get(mapped_line)?;
    let new_height = new_starts
        .get(mapped_line + 1)
        .copied()
        .unwrap_or_else(|| HyperlinkParagraph::new(&lines[mapped_line..]).line_count(width));
    Some(new_start + intra_line_offset.min(new_height.saturating_sub(1)))
}

fn wrapped_line_starts(lines: &[HyperlinkLine], width: u16) -> Vec<usize> {
    let mut starts = Vec::with_capacity(lines.len());
    let mut offset = 0usize;
    for line in lines {
        starts.push(offset);
        offset = offset.saturating_add(
            HyperlinkParagraph::new(std::slice::from_ref(line))
                .line_count(width)
                .max(1),
        );
    }
    starts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_width_sets_baseline_without_reflow() {
        let mut state = TranscriptReflowState::default();
        let change = state.note_width(80);
        assert!(change.initialized);
        assert!(!change.changed);
        assert!(!state.reflow_needed_for_width(80));
    }

    #[test]
    fn changed_width_is_debounced_and_due_is_observable() {
        let mut state = TranscriptReflowState::default();
        state.note_width(80);
        let change = state.note_width(100);
        assert!(change.changed);
        state.schedule_debounced(Some(100));
        assert!(state.has_pending_reflow());
        state.set_due_for_test();
        assert!(state.pending_is_due(Instant::now()));
        state.clear_pending_reflow();
        assert!(state.reflow_needed_for_width(100));
    }

    #[test]
    fn stream_reflow_request_is_drained_once() {
        let mut state = TranscriptReflowState::default();
        state.mark_ran_during_stream();
        state.mark_resize_requested_during_stream();
        assert!(state.take_stream_finish_reflow_needed());
        assert!(!state.take_stream_finish_reflow_needed());
    }

    #[test]
    fn viewport_preserves_manual_anchor_when_stream_tail_grows() {
        let viewport = TranscriptViewport::default();
        let area = Rect::new(0, 0, 18, 3);
        let initial = vec![
            HyperlinkLine::from("header"),
            HyperlinkLine::from("anchor"),
            HyperlinkLine::from("tail"),
            HyperlinkLine::from("tail 2"),
            HyperlinkLine::from("tail 3"),
            HyperlinkLine::from("tail 4"),
            HyperlinkLine::from("tail 5"),
            HyperlinkLine::from("tail 6"),
        ];

        assert_eq!(viewport.resolve(&initial, area, 4), 1);

        let updated = vec![
            HyperlinkLine::from("header"),
            HyperlinkLine::from("anchor"),
            HyperlinkLine::from("tail that grew while streaming"),
            HyperlinkLine::from("tail 2"),
            HyperlinkLine::from("tail 3"),
            HyperlinkLine::from("tail 4"),
            HyperlinkLine::from("tail 5"),
            HyperlinkLine::from("tail 6"),
        ];

        assert_eq!(viewport.resolve(&updated, area, 4), 1);
    }

    #[test]
    fn viewport_keeps_pinned_bottom_following_new_content() {
        let viewport = TranscriptViewport::default();
        let area = Rect::new(0, 0, 18, 3);
        let initial = vec![
            HyperlinkLine::from("one"),
            HyperlinkLine::from("two"),
            HyperlinkLine::from("three"),
            HyperlinkLine::from("four"),
        ];
        let first = viewport.resolve(&initial, area, 0);
        let updated = [
            HyperlinkLine::from("one"),
            HyperlinkLine::from("two"),
            HyperlinkLine::from("three"),
            HyperlinkLine::from("four"),
            HyperlinkLine::from("five"),
        ];
        let second = viewport.resolve(&updated, area, 0);
        assert!(second > first);
    }

    #[test]
    fn viewport_remaps_manual_anchor_when_width_reflows() {
        let viewport = TranscriptViewport::default();
        let initial = vec![
            HyperlinkLine::from("a long header that fits on one wide row"),
            HyperlinkLine::from("anchor"),
            HyperlinkLine::from("tail 1"),
            HyperlinkLine::from("tail 2"),
            HyperlinkLine::from("tail 3"),
            HyperlinkLine::from("tail 4"),
            HyperlinkLine::from("tail 5"),
        ];
        let wide = Rect::new(0, 0, 48, 3);
        viewport.resolve(&initial, wide, 3);

        let narrow = Rect::new(0, 0, 16, 3);
        let resolved = viewport.resolve(&initial, narrow, 3);
        assert_eq!(usize::from(resolved), wrapped_line_starts(&initial, 16)[1]);
    }
}
