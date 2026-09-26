use std::cell::{Cell, RefCell};
use std::collections::{HashSet, VecDeque};
use std::ops::Range;
use std::sync::Arc;
use std::time::Instant;

use ratatui::buffer::{Buffer, CellWidth};
use ratatui::layout::{Alignment, Position, Rect};
use ratatui::style::{Modifier, Style};
use unicode_segmentation::UnicodeSegmentation;

use crate::terminal_hyperlinks::HyperlinkLine;
use crate::text_selection::SelectionUnit;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct Anchor {
    pub(super) line: usize,
    pub(super) offset: usize,
}

#[derive(Clone, Debug)]
pub(super) struct Selection {
    pub(super) snapshot: Arc<Vec<HyperlinkLine>>,
    pub(super) excluded_lines: Arc<HashSet<usize>>,
    pub(super) start: Anchor,
    pub(super) end: Anchor,
    pub(super) origin: (Anchor, Anchor),
    pub(super) unit: SelectionUnit,
    pub(super) dragging: bool,
    pub(super) moved: bool,
    pub(super) moved_vertically: bool,
    pub(super) pointer_origin_row: u16,
    pub(super) pointer: Option<Position>,
    pub(super) pressed_link: Option<String>,
    pub(super) preferred_column: Option<u16>,
}

#[derive(Clone, Debug)]
pub(super) struct GraphemeCell {
    pub(super) source: Range<usize>,
    pub(super) width: u16,
    pub(super) whitespace: bool,
    pub(super) destination: Option<String>,
}

#[derive(Clone, Debug)]
pub(super) struct VisualRow {
    pub(super) line: usize,
    pub(super) cells: Vec<GraphemeCell>,
    pub(super) source_cursor: usize,
    pub(super) first_column: u16,
}

#[derive(Clone, Debug)]
pub(super) struct TranscriptLayout {
    pub(super) area: Rect,
    pub(super) scroll: usize,
    pub(super) lines: Arc<Vec<HyperlinkLine>>,
    pub(super) excluded_lines: Arc<HashSet<usize>>,
    pub(super) texts: Vec<String>,
    pub(super) rows: Vec<VisualRow>,
}

impl TranscriptLayout {
    fn new(
        area: Rect,
        scroll: usize,
        lines: Arc<Vec<HyperlinkLine>>,
        excluded_lines: Arc<HashSet<usize>>,
    ) -> Self {
        let texts = lines.iter().map(line_text).collect::<Vec<_>>();
        let rows = if area.width == 0 {
            Vec::new()
        } else {
            texts
                .iter()
                .enumerate()
                .zip(lines.iter())
                .flat_map(|((line_index, text), line)| {
                    wrap_visual_rows(line_index, text, area.width, line)
                })
                .collect()
        };
        let scroll = scroll.min(rows.len().saturating_sub(usize::from(area.height)));
        Self {
            area,
            scroll,
            lines,
            excluded_lines,
            texts,
            rows,
        }
    }

    pub(super) fn hit_test(&self, column: u16, row: u16) -> Option<Anchor> {
        if !self.area.contains(Position::new(column, row)) {
            return None;
        }
        let visual_row = self
            .scroll
            .saturating_add(usize::from(row.saturating_sub(self.area.y)));
        let row = self.rows.get(visual_row)?;
        Some(Anchor {
            line: row.line,
            offset: row.offset_at_column(column.saturating_sub(self.area.x)),
        })
    }

    pub(super) fn is_selectable_line(&self, line: usize) -> bool {
        !self.excluded_lines.contains(&line)
    }

    pub(super) fn link_at(&self, column: u16, row: u16) -> Option<String> {
        if !self.area.contains(Position::new(column, row)) {
            return None;
        }
        let visual_row = self
            .scroll
            .saturating_add(usize::from(row.saturating_sub(self.area.y)));
        self.rows
            .get(visual_row)?
            .link_at(column.saturating_sub(self.area.x))
    }

    pub(super) fn row_for_anchor(&self, anchor: Anchor) -> Option<usize> {
        self.rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row.line == anchor.line && row.start_offset() <= anchor.offset)
            .map(|(index, _)| index)
            .next_back()
    }
}

impl VisualRow {
    pub(super) fn start_offset(&self) -> usize {
        self.cells
            .first()
            .map_or(self.source_cursor, |cell| cell.source.start)
    }

    pub(super) fn end_offset(&self) -> usize {
        self.cells
            .last()
            .map_or(self.source_cursor, |cell| cell.source.end)
    }

    pub(super) fn offset_at_column(&self, column: u16) -> usize {
        if column <= self.first_column {
            return self.start_offset();
        }
        let mut current = self.first_column;
        for cell in &self.cells {
            current = current.saturating_add(cell.width);
            if column < current {
                return cell.source.start;
            }
        }
        self.end_offset()
    }

    pub(super) fn column_for_offset(&self, offset: usize) -> u16 {
        let mut column = self.first_column;
        for cell in &self.cells {
            if offset <= cell.source.start {
                break;
            }
            column = column.saturating_add(cell.width);
            if offset < cell.source.end {
                break;
            }
        }
        column
    }

    fn link_at(&self, column: u16) -> Option<String> {
        if column < self.first_column {
            return None;
        }
        let mut current = self.first_column;
        for cell in &self.cells {
            let end = current.saturating_add(cell.width);
            if column < end {
                return cell.destination.clone();
            }
            current = end;
        }
        None
    }
}

#[derive(Debug, Default)]
pub(crate) struct TranscriptSelection {
    pub(super) selection: RefCell<Option<Selection>>,
    pub(super) layout: RefCell<Option<TranscriptLayout>>,
    pub(super) last_click: RefCell<Option<(Instant, u16, u16, u8)>>,
    pub(super) resume_distance_from_bottom: Cell<Option<usize>>,
}

impl TranscriptSelection {
    pub(crate) fn is_active(&self) -> bool {
        self.selection.borrow().is_some()
    }

    pub(crate) fn snapshot_lines(&self) -> Option<Arc<Vec<HyperlinkLine>>> {
        self.selection
            .borrow()
            .as_ref()
            .map(|selection| Arc::clone(&selection.snapshot))
    }

    pub(crate) fn snapshot_excluded_lines(&self) -> Option<Arc<HashSet<usize>>> {
        self.selection
            .borrow()
            .as_ref()
            .map(|selection| Arc::clone(&selection.excluded_lines))
    }

    pub(crate) fn update_layout(&self, area: Rect, scroll: usize, lines: &[HyperlinkLine]) {
        self.update_layout_with_exclusions(area, scroll, lines, &HashSet::new());
    }

    pub(crate) fn update_layout_with_exclusions(
        &self,
        area: Rect,
        scroll: usize,
        lines: &[HyperlinkLine],
        excluded_lines: &HashSet<usize>,
    ) {
        let (lines, excluded_lines) = self.selection.borrow().as_ref().map_or_else(
            || (Arc::new(lines.to_vec()), Arc::new(excluded_lines.clone())),
            |selection| {
                (
                    Arc::clone(&selection.snapshot),
                    Arc::clone(&selection.excluded_lines),
                )
            },
        );
        self.layout.replace(Some(TranscriptLayout::new(
            area,
            scroll,
            lines,
            excluded_lines,
        )));
        let pointer = self
            .selection
            .borrow()
            .as_ref()
            .filter(|selection| selection.dragging && selection.moved)
            .and_then(|selection| selection.pointer);
        if let Some(pointer) = pointer {
            self.extend(pointer.x, pointer.y);
        }
    }

    pub(crate) fn clear(&self) {
        self.selection.replace(None);
    }

    pub(crate) fn reset(&self) {
        self.selection.replace(None);
        self.layout.replace(None);
        self.last_click.replace(None);
        self.resume_distance_from_bottom.set(None);
    }

    /// Keep the selected source and its top logical row stable across terminal reflow.
    pub(crate) fn frozen_scroll(&self, area: Rect, fallback: usize) -> usize {
        let selection = self.selection.borrow();
        let Some(selection) = selection.as_ref() else {
            return fallback;
        };
        let previous_layout = self.layout.borrow();
        let Some(previous) = previous_layout.as_ref() else {
            return fallback;
        };
        let next = TranscriptLayout::new(
            area,
            /*scroll*/ 0,
            Arc::clone(&selection.snapshot),
            Arc::clone(&selection.excluded_lines),
        );
        let max_scroll = next.rows.len().saturating_sub(usize::from(area.height));
        let Some(top) = previous.rows.get(previous.scroll) else {
            return fallback.min(max_scroll);
        };
        next.rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row.line == top.line && row.start_offset() <= top.start_offset())
            .map(|(index, _)| index)
            .next_back()
            .unwrap_or(fallback)
            .min(max_scroll)
    }

    pub(crate) fn note_resume_distance_from_bottom(&self, distance: usize) {
        if self.is_active() {
            self.resume_distance_from_bottom.set(Some(distance));
        }
    }

    pub(crate) fn take_resume_distance_from_bottom(&self) -> Option<usize> {
        self.resume_distance_from_bottom.take()
    }

    pub(crate) fn scroll_rows(&self, rows: isize) -> bool {
        let mut layout = self.layout.borrow_mut();
        let Some(layout) = layout.as_mut() else {
            return false;
        };
        let max_scroll = layout
            .rows
            .len()
            .saturating_sub(usize::from(layout.area.height));
        let next = layout.scroll.saturating_add_signed(rows).min(max_scroll);
        let changed = next != layout.scroll;
        layout.scroll = next;
        changed
    }

    pub(crate) fn reveal_row(&self, row: usize) -> bool {
        let mut layout = self.layout.borrow_mut();
        let Some(layout) = layout.as_mut() else {
            return false;
        };
        let height = usize::from(layout.area.height).max(1);
        let max_scroll = layout.rows.len().saturating_sub(height);
        let next = if row < layout.scroll {
            row
        } else if row >= layout.scroll.saturating_add(height) {
            row.saturating_add(1).saturating_sub(height)
        } else {
            layout.scroll
        }
        .min(max_scroll);
        let changed = next != layout.scroll;
        layout.scroll = next;
        changed
    }

    pub(crate) fn tick_edge_scroll(&self) -> bool {
        self.edge_scroll_direction()
            .is_some_and(|rows| self.scroll_rows(rows))
    }

    /// End pointer ownership without discarding a non-empty transcript selection.
    pub(crate) fn end_drag(&self) {
        let mut selection = self.selection.borrow_mut();
        let Some(active) = selection.as_mut() else {
            return;
        };
        let was_dragging = active.dragging;
        active.dragging = false;
        active.pointer = None;
        active.pressed_link = None;
        if was_dragging && active.start == active.end {
            *selection = None;
        }
    }

    /// Return the edge-scroll direction for an active vertical pointer gesture.
    pub(crate) fn edge_scroll_direction(&self) -> Option<isize> {
        let selection = self.selection.borrow();
        let selection = selection
            .as_ref()
            .filter(|selection| selection.dragging && selection.moved_vertically)?;
        let pointer = selection.pointer?;
        let layout = self.layout.borrow();
        let area = layout.as_ref()?.area;
        if pointer.y <= area.top() {
            Some(-1)
        } else if pointer.y >= area.bottom().saturating_sub(1) {
            Some(1)
        } else {
            None
        }
    }

    pub(crate) fn render_highlight(&self, buffer: &mut Buffer) {
        let selection = self.selection.borrow();
        let Some(selection) = selection.as_ref() else {
            return;
        };
        let (start, end) = ordered(selection.start, selection.end);
        if start == end {
            return;
        }
        let layout = self.layout.borrow();
        let Some(layout) = layout.as_ref() else {
            return;
        };
        let viewport_end = layout
            .scroll
            .saturating_add(usize::from(layout.area.height));
        for (row_index, row) in layout.rows.iter().enumerate() {
            if row_index < layout.scroll || row_index >= viewport_end {
                continue;
            }
            if !layout.is_selectable_line(row.line) {
                continue;
            }
            let begin = if row.line == start.line {
                start.offset
            } else if row.line > start.line {
                0
            } else {
                continue;
            };
            let finish = if row.line == end.line {
                end.offset
            } else if row.line < end.line {
                layout.texts.get(row.line).map_or(0, String::len)
            } else {
                continue;
            };
            if begin >= finish {
                continue;
            }
            let screen_y = layout
                .area
                .y
                .saturating_add(u16::try_from(row_index - layout.scroll).unwrap_or(u16::MAX));
            let mut column = row.first_column;
            for cell in &row.cells {
                if cell.source.start < finish && begin < cell.source.end {
                    for selected_column in
                        column..column.saturating_add(cell.width).min(layout.area.width)
                    {
                        buffer[(layout.area.x + selected_column, screen_y)]
                            .set_style(Style::default().add_modifier(Modifier::REVERSED));
                    }
                }
                column = column.saturating_add(cell.width);
            }
        }
    }

    pub(super) fn is_dragging(&self) -> bool {
        self.selection
            .borrow()
            .as_ref()
            .is_some_and(|selection| selection.dragging)
    }

    pub(super) fn extend(&self, column: u16, row: u16) {
        let layout = self.layout.borrow();
        let Some(layout) = layout.as_ref() else {
            return;
        };
        let Some(anchor) = layout.hit_test(column, row) else {
            return;
        };
        if !layout.is_selectable_line(anchor.line) {
            return;
        }
        let Some(text) = layout.texts.get(anchor.line) else {
            return;
        };
        let mut selection = self.selection.borrow_mut();
        let Some(selection) = selection.as_mut() else {
            return;
        };
        let range = selection.unit.range(text, anchor.offset);
        let backwards = anchor < selection.origin.0;
        selection.start = if backwards {
            selection.origin.1
        } else {
            selection.origin.0
        };
        selection.end = Anchor {
            offset: if backwards { range.start } else { range.end },
            ..anchor
        };
        selection.moved = true;
        selection.moved_vertically |= selection.pointer_origin_row != row;
        selection.pointer = Some(Position::new(column, row));
        selection.preferred_column = None;
    }

    pub(super) fn selected_text(&self) -> Option<String> {
        let selection = self.selection.borrow();
        let selection = selection.as_ref()?;
        let (start, end) = ordered(selection.start, selection.end);
        if start == end {
            return None;
        }
        let mut text = String::new();
        let mut included = false;
        for line_index in start.line..=end.line {
            if selection.excluded_lines.contains(&line_index) {
                continue;
            }
            if included {
                text.push('\n');
            }
            let line = selection.snapshot.get(line_index).map(line_text)?;
            let begin = if line_index == start.line {
                start.offset
            } else {
                0
            };
            let finish = if line_index == end.line {
                end.offset
            } else {
                line.len()
            };
            text.push_str(line.get(begin..finish)?);
            included = true;
        }
        text.retain(|character| !character.is_control() || matches!(character, '\n' | '\t'));
        (!text.is_empty()).then_some(text)
    }

    #[cfg(test)]
    pub(crate) fn selected_text_for_test(&self) -> Option<String> {
        self.selected_text()
    }
}

fn ordered(start: Anchor, end: Anchor) -> (Anchor, Anchor) {
    if start <= end {
        (start, end)
    } else {
        (end, start)
    }
}

fn line_text(line: &HyperlinkLine) -> String {
    line.line
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect()
}

fn wrap_visual_rows(
    line: usize,
    text: &str,
    width: u16,
    hyperlink_line: &HyperlinkLine,
) -> Vec<VisualRow> {
    let mut source_column = 0usize;
    let cells = text
        .grapheme_indices(true)
        .map(|(start, grapheme)| {
            let width = if grapheme.chars().any(char::is_control) {
                0
            } else {
                grapheme.cell_width()
            };
            let destination = hyperlink_line.destination_at_column(source_column);
            source_column = source_column.saturating_add(usize::from(width));
            GraphemeCell {
                source: start..start + grapheme.len(),
                width,
                whitespace: grapheme == "\u{200b}"
                    || (grapheme.chars().all(char::is_whitespace) && grapheme != "\u{00a0}"),
                destination,
            }
        })
        .collect::<Vec<_>>();
    let mut rows = word_wrap_cells(cells, width);
    if rows.is_empty() {
        rows.push(Vec::new());
    }
    let mut cursor = 0usize;
    rows.into_iter()
        .map(|cells| {
            let source_cursor = cells.first().map_or(cursor, |cell| cell.source.start);
            if let Some(last) = cells.last() {
                cursor = last.source.end;
            }
            let row_width = cells
                .iter()
                .fold(0u16, |total, cell| total.saturating_add(cell.width));
            let remaining = width.saturating_sub(row_width);
            let first_column = match hyperlink_line.line.alignment.unwrap_or(Alignment::Left) {
                Alignment::Left => 0,
                Alignment::Center => remaining / 2,
                Alignment::Right => remaining,
            };
            VisualRow {
                line,
                cells,
                source_cursor,
                first_column,
            }
        })
        .collect()
}

/// Mirrors ratatui's `WordWrapper` with `trim = false`, while retaining source offsets.
fn word_wrap_cells(cells: Vec<GraphemeCell>, width: u16) -> Vec<Vec<GraphemeCell>> {
    if width == 0 {
        return Vec::new();
    }
    let mut rows = Vec::new();
    let mut pending_line = Vec::new();
    let mut pending_word = Vec::new();
    let mut pending_whitespace = VecDeque::<GraphemeCell>::new();
    let mut line_width = 0u16;
    let mut word_width = 0u16;
    let mut whitespace_width = 0u16;
    let mut non_whitespace_previous = false;

    for cell in cells {
        if cell.width > width {
            continue;
        }
        let is_whitespace = cell.whitespace;
        let word_found = non_whitespace_previous && is_whitespace;
        let untrimmed_overflow = pending_line.is_empty()
            && word_width
                .saturating_add(whitespace_width)
                .saturating_add(cell.width)
                > width;
        if word_found || untrimmed_overflow {
            pending_line.extend(pending_whitespace.drain(..));
            line_width = line_width.saturating_add(whitespace_width);
            pending_line.append(&mut pending_word);
            line_width = line_width.saturating_add(word_width);
            whitespace_width = 0;
            word_width = 0;
        }

        let line_full = line_width >= width;
        let pending_word_overflow = cell.width > 0
            && line_width
                .saturating_add(whitespace_width)
                .saturating_add(word_width)
                >= width;
        if line_full || pending_word_overflow {
            let mut remaining = width.saturating_sub(line_width);
            rows.push(std::mem::take(&mut pending_line));
            line_width = 0;
            while let Some(space) = pending_whitespace.front() {
                if space.width > remaining {
                    break;
                }
                whitespace_width = whitespace_width.saturating_sub(space.width);
                remaining = remaining.saturating_sub(space.width);
                pending_whitespace.pop_front();
            }
            if is_whitespace && pending_whitespace.is_empty() {
                non_whitespace_previous = false;
                continue;
            }
        }

        if is_whitespace {
            whitespace_width = whitespace_width.saturating_add(cell.width);
            pending_whitespace.push_back(cell);
        } else {
            word_width = word_width.saturating_add(cell.width);
            pending_word.push(cell);
        }
        non_whitespace_previous = !is_whitespace;
    }

    pending_line.extend(pending_whitespace);
    pending_line.append(&mut pending_word);
    if !pending_line.is_empty() {
        rows.push(pending_line);
    }
    if rows.is_empty() {
        rows.push(Vec::new());
    }
    rows
}

#[cfg(test)]
#[path = "selection_tests.rs"]
mod tests;
