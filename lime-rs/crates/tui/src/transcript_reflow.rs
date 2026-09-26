//! Tracks when the canonical TUI transcript should be rebuilt after resize.
//!
//! This state machine is intentionally renderer agnostic. The App owns the
//! projection; this module only records observed/rebuilt widths and ensures a
//! resize during streaming receives one final source-backed repaint.

use std::cell::{Cell, RefCell};
use std::time::{Duration, Instant};

use ratatui::layout::Rect;

use crate::terminal_hyperlinks::{wrapped_line_starts, HyperlinkLine, HyperlinkParagraph};

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
#[derive(Debug)]
pub(crate) struct TranscriptViewport {
    previous_lines: RefCell<Option<Vec<HyperlinkLine>>>,
    previous_width: Cell<u16>,
    previous_offset: Cell<usize>,
    previous_requested_scroll: Cell<Option<usize>>,
    tail_visible: Cell<bool>,
    unseen_activity: Cell<bool>,
    suppress_next_activity: Cell<bool>,
}

impl Default for TranscriptViewport {
    fn default() -> Self {
        Self {
            previous_lines: RefCell::new(None),
            previous_width: Cell::new(0),
            previous_offset: Cell::new(0),
            previous_requested_scroll: Cell::new(None),
            tail_visible: Cell::new(true),
            unseen_activity: Cell::new(false),
            suppress_next_activity: Cell::new(false),
        }
    }
}

impl TranscriptViewport {
    pub(crate) fn clear(&self) {
        *self.previous_lines.borrow_mut() = None;
        self.previous_width.set(0);
        self.previous_offset.set(0);
        self.previous_requested_scroll.set(None);
        self.tail_visible.set(true);
        self.unseen_activity.set(false);
        self.suppress_next_activity.set(false);
    }

    pub(crate) fn tail_visible(&self) -> bool {
        self.tail_visible.get()
    }

    pub(crate) fn unseen_activity(&self) -> bool {
        self.unseen_activity.get()
    }

    pub(crate) fn suppress_next_activity(&self) {
        self.suppress_next_activity.set(true);
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

        let previous = self.previous_lines.borrow().as_ref().cloned();
        let suppress_activity = self.suppress_next_activity.replace(false);
        let (offset, total_height) = self.requested_offset(
            lines,
            area,
            requested_distance_from_bottom,
            previous.as_deref(),
        );

        self.commit_resolution(
            lines,
            area,
            offset,
            total_height,
            Some(requested_distance_from_bottom),
            previous.as_deref(),
            suppress_activity,
            requested_distance_from_bottom > 0,
        )
    }

    /// Calculate the next top row without committing viewport or unseen-activity state.
    ///
    /// The main transcript uses this for prompt-header reservation, then commits exactly once
    /// against the final body rectangle through [`Self::resolve`].
    pub(crate) fn preview(
        &self,
        lines: &[HyperlinkLine],
        area: Rect,
        requested_distance_from_bottom: usize,
    ) -> u16 {
        if area.width == 0 || area.height == 0 {
            return 0;
        }
        let previous = self.previous_lines.borrow();
        let (offset, _) = self.requested_offset(
            lines,
            area,
            requested_distance_from_bottom,
            previous.as_deref(),
        );
        u16::try_from(offset).unwrap_or(u16::MAX)
    }

    /// Observe canonical output while a local selection keeps painting a frozen source snapshot.
    ///
    /// The returned offset follows the same logical canonical row across append, prepend and
    /// width reflow. It is used only to resume reading after the selection ends; the selection
    /// owner keeps its own frozen visual offset.
    pub(crate) fn resolve_frozen_anchor(
        &self,
        lines: &[HyperlinkLine],
        area: Rect,
        fallback_offset: usize,
    ) -> u16 {
        if area.width == 0 || area.height == 0 {
            return 0;
        }

        let previous = self.previous_lines.borrow().as_ref().cloned();
        let suppress_activity = self.suppress_next_activity.replace(false);
        let total_height = HyperlinkParagraph::new(lines).line_count(area.width);
        let max_scroll = total_height.saturating_sub(usize::from(area.height));
        let offset = previous
            .as_ref()
            .and_then(|previous| {
                let old_width = self.previous_width.get();
                (old_width > 0).then(|| {
                    remap_transcript_offset(
                        previous,
                        old_width,
                        self.previous_offset.get(),
                        lines,
                        area.width,
                    )
                })?
            })
            .unwrap_or(fallback_offset)
            .min(max_scroll);
        let reading_away_from_tail = offset.saturating_add(usize::from(area.height)) < total_height;
        self.commit_resolution(
            lines,
            area,
            offset,
            total_height,
            None,
            previous.as_deref(),
            suppress_activity,
            reading_away_from_tail,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn commit_resolution(
        &self,
        lines: &[HyperlinkLine],
        area: Rect,
        offset: usize,
        total_height: usize,
        requested_distance_from_bottom: Option<usize>,
        previous: Option<&[HyperlinkLine]>,
        suppress_activity: bool,
        reading_away_from_tail: bool,
    ) -> u16 {
        *self.previous_lines.borrow_mut() = Some(lines.to_vec());
        self.previous_width.set(area.width);
        self.previous_offset.set(offset);
        self.previous_requested_scroll
            .set(requested_distance_from_bottom);
        let tail_visible = offset.saturating_add(usize::from(area.height)) >= total_height;
        self.tail_visible.set(tail_visible);
        if tail_visible {
            self.unseen_activity.set(false);
        } else if reading_away_from_tail
            && !suppress_activity
            && previous.is_some_and(|previous| {
                previous != lines
                    && !(lines.len() > previous.len() && lines.ends_with(previous))
                    && !is_preserving_insertion_before_offset(
                        previous,
                        self.previous_width.get(),
                        self.previous_offset.get(),
                        lines,
                    )
            })
        {
            self.unseen_activity.set(true);
        }
        u16::try_from(offset).unwrap_or(u16::MAX)
    }

    fn requested_offset(
        &self,
        lines: &[HyperlinkLine],
        area: Rect,
        requested_distance_from_bottom: usize,
        previous: Option<&[HyperlinkLine]>,
    ) -> (usize, usize) {
        let total_height = HyperlinkParagraph::new(lines).line_count(area.width);
        let max_scroll = total_height.saturating_sub(usize::from(area.height));
        let base_offset = max_scroll.saturating_sub(requested_distance_from_bottom.min(max_scroll));
        let requested_is_unchanged =
            self.previous_requested_scroll.get() == Some(requested_distance_from_bottom);
        let mut offset = base_offset;

        if requested_distance_from_bottom > 0 && requested_is_unchanged {
            if let Some(previous) = previous {
                let old_width = self.previous_width.get();
                if old_width > 0 {
                    if let Some(mapped) = remap_transcript_offset(
                        previous,
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
        (offset, total_height)
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
    } else if lines.len() > previous.len()
        && common_prefix < previous.len()
        && common_prefix.saturating_add(common_suffix) == previous.len()
    {
        if old_line < common_prefix {
            old_line
        } else {
            old_line + lines.len() - previous.len()
        }
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

fn is_preserving_insertion_before_offset(
    previous: &[HyperlinkLine],
    old_width: u16,
    old_offset: usize,
    lines: &[HyperlinkLine],
) -> bool {
    if lines.len() <= previous.len() || previous.is_empty() || old_width == 0 {
        return false;
    }
    let common_prefix = previous
        .iter()
        .zip(lines)
        .take_while(|(old, new)| old == new)
        .count();
    if common_prefix == previous.len() {
        return false;
    }
    let common_suffix = previous
        .iter()
        .rev()
        .zip(lines.iter().rev())
        .take_while(|(old, new)| old == new)
        .count()
        .min(previous.len().saturating_sub(common_prefix));
    if common_prefix.saturating_add(common_suffix) != previous.len() {
        return false;
    }
    let old_top_line = wrapped_line_starts(previous, old_width)
        .iter()
        .enumerate()
        .rev()
        .find(|(_, start)| **start <= old_offset)
        .map_or(0, |(index, _)| index);
    common_prefix <= old_top_line
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

    #[test]
    fn viewport_marks_appended_and_revised_tail_activity_while_paused() {
        let viewport = TranscriptViewport::default();
        let area = Rect::new(0, 0, 20, 2);
        let initial = vec![
            HyperlinkLine::from("one"),
            HyperlinkLine::from("two"),
            HyperlinkLine::from("three"),
            HyperlinkLine::from("four"),
        ];
        viewport.resolve(&initial, area, 1);
        assert!(!viewport.tail_visible());
        assert!(!viewport.unseen_activity());

        let mut appended = initial.clone();
        appended.push(HyperlinkLine::from("five"));
        viewport.resolve(&appended, area, 1);
        assert!(viewport.unseen_activity());

        viewport.clear();
        viewport.resolve(&initial, area, 1);
        let mut revised = initial.clone();
        revised[3] = HyperlinkLine::from("four revised");
        viewport.resolve(&revised, area, 1);
        assert!(viewport.unseen_activity());
    }

    #[test]
    fn frozen_selection_anchor_observes_tail_activity_and_remaps_history_prepend() {
        let area = Rect::new(0, 0, 20, 2);
        let initial = vec![
            HyperlinkLine::from("one"),
            HyperlinkLine::from("two"),
            HyperlinkLine::from("three"),
            HyperlinkLine::from("four"),
        ];
        let viewport = TranscriptViewport::default();
        assert_eq!(viewport.resolve(&initial, area, 0), 2);

        let mut appended = initial.clone();
        appended.push(HyperlinkLine::from("five"));
        assert_eq!(viewport.resolve_frozen_anchor(&appended, area, 2), 2);
        assert!(!viewport.tail_visible());
        assert!(viewport.unseen_activity());

        let prepended_viewport = TranscriptViewport::default();
        assert_eq!(prepended_viewport.resolve(&initial, area, 1), 1);
        let mut prepended = vec![HyperlinkLine::from("zero")];
        prepended.extend(initial);
        assert_eq!(
            prepended_viewport.resolve_frozen_anchor(&prepended, area, 1),
            2
        );
        assert!(!prepended_viewport.unseen_activity());
    }

    #[test]
    fn viewport_does_not_treat_older_history_prepend_as_unseen_activity() {
        let viewport = TranscriptViewport::default();
        let area = Rect::new(0, 0, 20, 2);
        let initial = vec![
            HyperlinkLine::from("three"),
            HyperlinkLine::from("four"),
            HyperlinkLine::from("five"),
        ];
        viewport.resolve(&initial, area, 1);
        let prepended = vec![
            HyperlinkLine::from("one"),
            HyperlinkLine::from("two"),
            HyperlinkLine::from("three"),
            HyperlinkLine::from("four"),
            HyperlinkLine::from("five"),
        ];

        viewport.resolve(&prepended, area, 1);

        assert!(!viewport.unseen_activity());
    }

    #[test]
    fn viewport_remaps_history_inserted_after_stable_session_header() {
        let viewport = TranscriptViewport::default();
        let area = Rect::new(0, 0, 20, 2);
        let initial = vec![
            HyperlinkLine::from("session"),
            HyperlinkLine::from("gap"),
            HyperlinkLine::from("current one"),
            HyperlinkLine::from("current two"),
            HyperlinkLine::from("current three"),
        ];
        assert_eq!(viewport.resolve(&initial, area, 1), 2);
        let updated = vec![
            HyperlinkLine::from("session"),
            HyperlinkLine::from("gap"),
            HyperlinkLine::from("older one"),
            HyperlinkLine::from("older two"),
            HyperlinkLine::from("current one"),
            HyperlinkLine::from("current two"),
            HyperlinkLine::from("current three"),
        ];

        assert_eq!(viewport.resolve(&updated, area, 1), 4);
        assert!(!viewport.unseen_activity());
    }

    #[test]
    fn viewport_clears_activity_at_bottom_and_hides_follow_when_tail_fits() {
        let viewport = TranscriptViewport::default();
        let compact = Rect::new(0, 0, 20, 2);
        let initial = vec![
            HyperlinkLine::from("one"),
            HyperlinkLine::from("two"),
            HyperlinkLine::from("three"),
        ];
        viewport.resolve(&initial, compact, 1);
        let mut updated = initial.clone();
        updated.push(HyperlinkLine::from("four"));
        viewport.resolve(&updated, compact, 1);
        assert!(viewport.unseen_activity());

        viewport.resolve(&updated, compact, 0);
        assert!(viewport.tail_visible());
        assert!(!viewport.unseen_activity());

        viewport.resolve(&updated, compact, 1);
        assert!(!viewport.tail_visible());
        viewport.resolve(&updated, Rect::new(0, 0, 20, 6), 1);
        assert!(viewport.tail_visible());
        assert!(!viewport.unseen_activity());
    }
}
