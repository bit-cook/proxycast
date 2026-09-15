//! Codex-shaped selection-row layout shared by TUI pickers.
//!
//! This module owns display-only row composition: names, optional descriptions, disabled
//! reasons, and width-aware wrapping. Picker state and selection actions remain owned by their
//! respective App Server-backed surfaces.

use std::borrow::Cow;

use ratatui::layout::Rect;
use ratatui::style::Stylize;
use ratatui::text::{Line, Span};
use unicode_segmentation::UnicodeSegmentation;

use crate::line_truncation::line_width;
use crate::width::display_width;
use crate::wrapping::{word_wrap_line, RtOptions};

/// Maximum number of selectable rows shown by a selection popup.
///
/// This is the same bounded viewport used by Codex's generic list picker. The
/// selected row may move the window, but the popup itself stays anchored.
pub(crate) const MAX_POPUP_ROWS: usize = 8;

/// Center a popup while keeping its rectangle inside the available terminal area.
pub(crate) fn centered_popup(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect::new(
        area.x.saturating_add(area.width.saturating_sub(width) / 2),
        area.y
            .saturating_add(area.height.saturating_sub(height) / 2),
        width,
        height,
    )
}

/// Return the item window that keeps `selected` visible in a bounded popup.
pub(crate) fn visible_item_window(
    selected: usize,
    item_count: usize,
    max_visible: usize,
) -> (usize, usize) {
    if item_count == 0 || max_visible == 0 {
        return (0, 0);
    }
    let visible = item_count.min(max_visible);
    let selected = selected.min(item_count - 1);
    let start = selected.saturating_sub(visible - 1);
    let end = (start + visible).min(item_count);
    (end.saturating_sub(visible), end)
}

/// Controls whether descriptions stay in a column or move below labels when the description
/// column becomes too narrow to read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum SelectionDescriptionLayout {
    #[default]
    Columns,
    StackBelowWhenNarrow {
        min_description_width: u16,
    },
}

impl SelectionDescriptionLayout {
    pub(crate) fn should_stack(self, width: u16, desc_col: usize) -> bool {
        let Self::StackBelowWhenNarrow {
            min_description_width,
        } = self
        else {
            return false;
        };
        width.saturating_sub(desc_col.min(usize::from(width)) as u16) < min_description_width
    }
}

/// A display-only selection row. Prefix spans are supplied by the caller.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SelectionRow {
    pub(crate) name: String,
    pub(crate) name_prefix_spans: Vec<Span<'static>>,
    pub(crate) description: Option<String>,
    pub(crate) disabled_reason: Option<String>,
}

impl SelectionRow {
    pub(crate) fn new(
        name: impl Into<String>,
        description: Option<String>,
        name_prefix_spans: Vec<Span<'static>>,
    ) -> Self {
        Self {
            name: name.into(),
            name_prefix_spans,
            description,
            disabled_reason: None,
        }
    }
}

pub(crate) fn line_to_owned(line: Line<'_>) -> Line<'static> {
    Line {
        style: line.style,
        alignment: line.alignment,
        spans: line
            .spans
            .into_iter()
            .map(|span| Span {
                style: span.style,
                content: Cow::Owned(span.content.into_owned()),
            })
            .collect(),
    }
}

fn combined_description(
    row: &SelectionRow,
    description_layout: SelectionDescriptionLayout,
) -> Option<String> {
    match (&row.description, &row.disabled_reason) {
        (Some(description), Some(reason)) => Some(format!("{description} (disabled: {reason})")),
        (Some(description), None) => Some(description.clone()),
        (None, Some(reason))
            if matches!(
                description_layout,
                SelectionDescriptionLayout::StackBelowWhenNarrow { .. }
            ) =>
        {
            Some(reason.clone())
        }
        (None, Some(reason)) => Some(format!("disabled: {reason}")),
        (None, None) => None,
    }
}

fn stacked_description(row: &SelectionRow) -> Option<String> {
    match (&row.description, &row.disabled_reason) {
        (Some(description), Some(reason)) => Some(format!("{description} (disabled: {reason})")),
        (Some(description), None) => Some(description.clone()),
        (None, Some(reason)) => Some(reason.clone()),
        (None, None) => None,
    }
}

fn build_name_spans(row: &SelectionRow, name_limit: usize) -> Vec<Span<'static>> {
    let mut spans = Vec::with_capacity(row.name.len());
    let mut used_width = 0usize;
    let mut truncated = false;
    for grapheme in row.name.graphemes(true) {
        let next_width = used_width.saturating_add(display_width(grapheme));
        if next_width > name_limit {
            truncated = true;
            break;
        }
        used_width = next_width;
        spans.push(Span::raw(grapheme.to_string()));
    }
    if truncated {
        spans.push(Span::raw("…"));
    }
    if row.disabled_reason.is_some() {
        spans.push(" (disabled)".dim());
    }
    spans
}

/// Build a full row with the description beginning at desc_col.
pub(crate) fn build_full_line(
    row: &SelectionRow,
    desc_col: usize,
    description_layout: SelectionDescriptionLayout,
) -> Line<'static> {
    let description = combined_description(row, description_layout);
    let prefix_width = line_width(&Line::from(row.name_prefix_spans.clone()));
    let name_limit = description
        .as_ref()
        .map(|_| desc_col.saturating_sub(2).saturating_sub(prefix_width))
        .unwrap_or(usize::MAX);
    let name_spans = build_name_spans(row, name_limit);
    let name_width = prefix_width + line_width(&Line::from(name_spans.clone()));

    let mut spans = row.name_prefix_spans.clone();
    spans.extend(name_spans);
    if let Some(description) = description {
        let gap = desc_col.saturating_sub(name_width);
        if gap > 0 {
            spans.push(Span::raw(" ".repeat(gap)));
        }
        spans.push(Span::raw(description).dim());
    }
    Line::from(spans)
}

/// Render a row as a label followed by an indented, wrapped description.
pub(crate) fn wrap_stacked_row(row: &SelectionRow, width: u16) -> Vec<Line<'static>> {
    let width = width.max(1);
    let prefix_width = line_width(&Line::from(row.name_prefix_spans.clone()))
        .min(width.saturating_sub(1) as usize);
    let indent = " ".repeat(prefix_width);

    let mut label_spans = row.name_prefix_spans.clone();
    label_spans.extend(build_name_spans(row, usize::MAX));
    let label = Line::from(label_spans);
    let label_options = RtOptions::new(width as usize)
        .initial_indent(Line::default())
        .subsequent_indent(Line::from(indent.clone()));
    let mut lines = word_wrap_line(&label, label_options)
        .into_iter()
        .map(line_to_owned)
        .collect::<Vec<_>>();

    if let Some(description) = stacked_description(row) {
        let description_options = RtOptions::new(width as usize)
            .initial_indent(Line::from(indent.clone()))
            .subsequent_indent(Line::from(indent));
        lines.extend(
            word_wrap_line(&Line::from(description.dim()), description_options)
                .into_iter()
                .map(line_to_owned),
        );
    }
    lines
}

/// Build wrapped rows while preserving the Codex narrow-width stack fallback.
pub(crate) fn wrap_row(
    row: &SelectionRow,
    desc_col: usize,
    width: u16,
    description_layout: SelectionDescriptionLayout,
) -> Vec<Line<'static>> {
    if description_layout.should_stack(width, desc_col) {
        return wrap_stacked_row(row, width);
    }
    let full_line = build_full_line(row, desc_col, description_layout);
    let options = RtOptions::new(width.max(1) as usize)
        .initial_indent(Line::default())
        .subsequent_indent(Line::default());
    word_wrap_line(&full_line, options)
        .into_iter()
        .map(line_to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centered_popup_is_clamped_and_keeps_terminal_anchor() {
        let area = Rect::new(4, 3, 20, 10);
        assert_eq!(centered_popup(area, 12, 6), Rect::new(8, 5, 12, 6));
        assert_eq!(centered_popup(area, 80, 80), area);
    }

    #[test]
    fn visible_item_window_keeps_selection_inside_eight_row_viewport() {
        assert_eq!(visible_item_window(0, 20, MAX_POPUP_ROWS), (0, 8));
        assert_eq!(visible_item_window(7, 20, MAX_POPUP_ROWS), (0, 8));
        assert_eq!(visible_item_window(8, 20, MAX_POPUP_ROWS), (1, 9));
        assert_eq!(visible_item_window(19, 20, MAX_POPUP_ROWS), (12, 20));
        assert_eq!(visible_item_window(0, 0, MAX_POPUP_ROWS), (0, 0));
    }

    #[test]
    fn wide_rows_align_description_with_display_width() {
        let row = SelectionRow::new("模型🙂", Some("provider".to_string()), Vec::new());
        let line = build_full_line(&row, 12, SelectionDescriptionLayout::Columns);
        assert!(line.to_string().contains("provider"));
        assert!(line_width(&line) >= 20);
    }

    #[test]
    fn narrow_rows_stack_description_without_splitting_graphemes() {
        let row = SelectionRow::new("模型🙂", Some("提供方".to_string()), Vec::new());
        let lines = wrap_row(
            &row,
            6,
            12,
            SelectionDescriptionLayout::StackBelowWhenNarrow {
                min_description_width: 8,
            },
        );
        assert!(lines.len() >= 2);
        assert!(lines.iter().any(|line| line.to_string().contains("🙂")));
        assert!(lines.iter().all(|line| line_width(line) <= 12));
    }

    #[test]
    fn disabled_reason_is_visible_and_width_bounded() {
        let row = SelectionRow {
            name: "option".to_string(),
            description: None,
            disabled_reason: Some("later".to_string()),
            ..SelectionRow::default()
        };
        let line = build_full_line(&row, 8, SelectionDescriptionLayout::Columns);
        assert!(line.to_string().contains("disabled: later"));
    }
}
