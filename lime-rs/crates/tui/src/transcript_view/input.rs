//! Transcript gestures leave ordinary typing and composer editing with the existing input path.
//! Stationary link clicks open on release; dragging or scrolling keeps the gesture in selection.

use std::sync::Arc;

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::layout::Position;
use unicode_segmentation::UnicodeSegmentation;

use super::selection::{Anchor, Selection};
use super::TranscriptSelection;
use crate::text_selection::{click_count, is_copy_key, SelectionUnit};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum TranscriptSelectionAction {
    Consumed,
    Copy { text: String, follow: bool },
    OpenLink(String),
    Scroll { rows: isize },
    RevealRow(usize),
}

impl TranscriptSelection {
    pub(crate) fn handle_event(&self, event: &Event) -> Option<TranscriptSelectionAction> {
        match event {
            Event::Mouse(mouse) => self.handle_mouse(*mouse),
            Event::Key(key) => self.handle_key(*key),
            Event::FocusLost => {
                self.end_drag();
                Some(TranscriptSelectionAction::Consumed)
            }
            _ => None,
        }
    }

    fn handle_key(&self, key: KeyEvent) -> Option<TranscriptSelectionAction> {
        if key.kind == KeyEventKind::Release {
            return None;
        }
        if self.selection.borrow().is_some() {
            if is_copy_key(key)
                || (key.code == KeyCode::Enter && key.modifiers == KeyModifiers::NONE)
            {
                return self
                    .selected_text()
                    .map(|text| TranscriptSelectionAction::Copy {
                        text,
                        follow: key.code == KeyCode::Enter,
                    })
                    .or(Some(TranscriptSelectionAction::Consumed));
            }
            if key.code == KeyCode::Esc {
                self.clear();
                return Some(TranscriptSelectionAction::Consumed);
            }
            if key.modifiers == KeyModifiers::NONE
                && matches!(key.code, KeyCode::PageUp | KeyCode::PageDown)
            {
                let page = self
                    .layout
                    .borrow()
                    .as_ref()
                    .map_or(1, |layout| usize::from(layout.area.height).max(1));
                let rows = isize::try_from(page).unwrap_or(isize::MAX);
                return Some(TranscriptSelectionAction::Scroll {
                    rows: if key.code == KeyCode::PageUp {
                        -rows
                    } else {
                        rows
                    },
                });
            }
            if matches!(key.modifiers, KeyModifiers::NONE | KeyModifiers::SHIFT)
                && matches!(
                    key.code,
                    KeyCode::Left | KeyCode::Right | KeyCode::Up | KeyCode::Down
                )
            {
                return self
                    .move_endpoint(key.code)
                    .map(TranscriptSelectionAction::RevealRow)
                    .or(Some(TranscriptSelectionAction::Consumed));
            }
            return None;
        }

        if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char(' ') {
            self.begin_keyboard_selection()?;
            return Some(TranscriptSelectionAction::Consumed);
        }
        None
    }

    fn handle_mouse(&self, mouse: MouseEvent) -> Option<TranscriptSelectionAction> {
        let inside = self
            .layout
            .borrow()
            .as_ref()
            .is_some_and(|layout| layout.area.contains(Position::new(mouse.column, mouse.row)));
        let dragging = self.is_dragging();
        if !inside && !dragging {
            return None;
        }

        if matches!(
            mouse.kind,
            MouseEventKind::ScrollUp | MouseEventKind::ScrollDown
        ) {
            if let Some(selection) = self.selection.borrow_mut().as_mut() {
                selection.pointer = None;
                selection.pressed_link = None;
            }
            let rows = if mouse.kind == MouseEventKind::ScrollUp {
                -3
            } else {
                3
            };
            return Some(TranscriptSelectionAction::Scroll { rows });
        }

        match mouse.kind {
            MouseEventKind::Down(MouseButton::Right) if inside => self
                .selected_text()
                .map(|text| TranscriptSelectionAction::Copy {
                    text,
                    follow: false,
                })
                .or(Some(TranscriptSelectionAction::Consumed)),
            MouseEventKind::Down(MouseButton::Left) => self.pointer_down(mouse),
            MouseEventKind::Drag(MouseButton::Left) if dragging => {
                self.extend(mouse.column, mouse.row);
                Some(TranscriptSelectionAction::Consumed)
            }
            MouseEventKind::Up(MouseButton::Left) if dragging => self.pointer_up(mouse, inside),
            _ => None,
        }
    }

    fn pointer_down(&self, mouse: MouseEvent) -> Option<TranscriptSelectionAction> {
        if mouse
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::SUPER)
        {
            return self
                .layout
                .borrow()
                .as_ref()?
                .link_at(mouse.column, mouse.row)
                .map(TranscriptSelectionAction::OpenLink);
        }

        let clicks = click_count(&mut self.last_click.borrow_mut(), mouse.column, mouse.row);
        let layout = self.layout.borrow();
        let layout = layout.as_ref()?;
        let anchor = layout.hit_test(mouse.column, mouse.row)?;
        if !layout.is_selectable_line(anchor.line) {
            return None;
        }
        let text = layout.texts.get(anchor.line)?;
        if clicks >= 2 && anchor.offset == text.len() {
            return None;
        }
        let unit = SelectionUnit::from_clicks(clicks);
        let range = unit.range(text, anchor.offset);
        let start = Anchor {
            offset: range.start,
            ..anchor
        };
        let end = Anchor {
            offset: range.end,
            ..anchor
        };
        let pressed_link = (clicks == 1 && mouse.modifiers.is_empty())
            .then(|| layout.link_at(mouse.column, mouse.row))
            .flatten();
        self.selection.replace(Some(Selection {
            snapshot: Arc::clone(&layout.lines),
            excluded_lines: Arc::clone(&layout.excluded_lines),
            start,
            end,
            origin: (start, end),
            unit,
            dragging: true,
            moved: false,
            moved_vertically: false,
            pointer_origin_row: mouse.row,
            pointer: Some(Position::new(mouse.column, mouse.row)),
            pressed_link,
            preferred_column: None,
        }));
        self.resume_distance_from_bottom.set(None);
        Some(TranscriptSelectionAction::Consumed)
    }

    fn pointer_up(&self, mouse: MouseEvent, inside: bool) -> Option<TranscriptSelectionAction> {
        let release = Position::new(mouse.column, mouse.row);
        let link = self.selection.borrow_mut().as_mut().and_then(|selection| {
            (inside
                && !selection.moved
                && selection.pointer == Some(release)
                && mouse.modifiers.is_empty())
            .then(|| selection.pressed_link.take())
            .flatten()
        });
        let link = link.filter(|destination| {
            self.layout
                .borrow()
                .as_ref()
                .and_then(|layout| layout.link_at(mouse.column, mouse.row))
                .as_ref()
                == Some(destination)
        });
        let should_extend = self
            .selection
            .borrow()
            .as_ref()
            .is_some_and(|selection| selection.moved && selection.pointer.is_some());
        if should_extend {
            self.extend(mouse.column, mouse.row);
        }
        let mut selection = self.selection.borrow_mut();
        if let Some(selection) = selection.as_mut() {
            selection.dragging = false;
        }
        if selection
            .as_ref()
            .is_some_and(|selection| selection.start == selection.end)
        {
            *selection = None;
        }
        link.map(TranscriptSelectionAction::OpenLink)
            .or(Some(TranscriptSelectionAction::Consumed))
    }

    fn begin_keyboard_selection(&self) -> Option<()> {
        let layout = self.layout.borrow();
        let layout = layout.as_ref()?;
        let viewport_end = layout
            .scroll
            .saturating_add(usize::from(layout.area.height));
        let row = layout.rows
            [layout.scroll.min(layout.rows.len())..layout.rows.len().min(viewport_end)]
            .iter()
            .find(|row| layout.is_selectable_line(row.line))?;
        let anchor = Anchor {
            line: row.line,
            offset: row.offset_at_column(0),
        };
        self.selection.replace(Some(Selection {
            snapshot: Arc::clone(&layout.lines),
            excluded_lines: Arc::clone(&layout.excluded_lines),
            start: anchor,
            end: anchor,
            origin: (anchor, anchor),
            unit: SelectionUnit::Character,
            dragging: false,
            moved: false,
            moved_vertically: false,
            pointer_origin_row: layout.area.y,
            pointer: None,
            pressed_link: None,
            preferred_column: None,
        }));
        self.resume_distance_from_bottom.set(None);
        Some(())
    }

    fn move_endpoint(&self, code: KeyCode) -> Option<usize> {
        let layout = self.layout.borrow();
        let layout = layout.as_ref()?;
        let mut selection = self.selection.borrow_mut();
        let selection = selection.as_mut()?;
        let end = selection.end;
        let vertical = matches!(code, KeyCode::Up | KeyCode::Down);
        let preferred_column = vertical.then(|| {
            selection.preferred_column.unwrap_or_else(|| {
                layout
                    .row_for_anchor(end)
                    .and_then(|row| layout.rows.get(row))
                    .map_or(0, |row| row.column_for_offset(end.offset))
            })
        });
        let next = if vertical {
            move_vertical(layout, end, code, preferred_column?)
        } else {
            move_horizontal(layout, end, code)
        }?;
        selection.end = next;
        selection.dragging = false;
        selection.moved = true;
        selection.pointer = None;
        selection.pressed_link = None;
        selection.preferred_column = preferred_column;
        layout.row_for_anchor(next)
    }
}

fn move_horizontal(
    layout: &super::selection::TranscriptLayout,
    anchor: Anchor,
    code: KeyCode,
) -> Option<Anchor> {
    let text = layout.texts.get(anchor.line)?;
    let offset = floor_grapheme_boundary(text, anchor.offset.min(text.len()));
    match code {
        KeyCode::Left => {
            if offset > 0 {
                let previous = text[..offset]
                    .grapheme_indices(true)
                    .next_back()
                    .map(|(start, _)| start)?;
                Some(Anchor {
                    offset: previous,
                    ..anchor
                })
            } else {
                let line = (0..anchor.line)
                    .rev()
                    .find(|line| layout.is_selectable_line(*line))?;
                Some(Anchor {
                    line,
                    offset: layout.texts.get(line)?.len(),
                })
            }
        }
        KeyCode::Right => {
            if offset < text.len() {
                let grapheme = text[offset..].graphemes(true).next()?;
                Some(Anchor {
                    offset: offset + grapheme.len(),
                    ..anchor
                })
            } else {
                (anchor.line + 1..layout.texts.len())
                    .find(|line| layout.is_selectable_line(*line))
                    .map(|line| Anchor { line, offset: 0 })
            }
        }
        _ => None,
    }
}

fn move_vertical(
    layout: &super::selection::TranscriptLayout,
    anchor: Anchor,
    code: KeyCode,
    preferred_column: u16,
) -> Option<Anchor> {
    let row = layout.row_for_anchor(anchor)?;
    let target = match code {
        KeyCode::Up => (0..row)
            .rev()
            .find(|row| layout.is_selectable_line(layout.rows[*row].line))?,
        KeyCode::Down => (row + 1..layout.rows.len())
            .find(|row| layout.is_selectable_line(layout.rows[*row].line))?,
        _ => return None,
    };
    let row = layout.rows.get(target)?;
    Some(Anchor {
        line: row.line,
        offset: row.offset_at_column(preferred_column),
    })
}

fn floor_grapheme_boundary(text: &str, offset: usize) -> usize {
    text.grapheme_indices(true)
        .map(|(start, _)| start)
        .chain(std::iter::once(text.len()))
        .take_while(|start| *start <= offset)
        .last()
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "input_tests.rs"]
mod tests;
