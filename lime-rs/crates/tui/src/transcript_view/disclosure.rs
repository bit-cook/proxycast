//! Local transcript activity disclosure keyed by canonical item identities.
//!
//! Controls are synthetic presentation rows. They never become canonical transcript text and are
//! explicitly excluded from selection and search by the pager owner.

use std::cell::RefCell;
use std::collections::HashSet;
use std::ops::Range;

use crossterm::event::{Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind};
use ratatui::layout::{Position, Rect};
use ratatui::style::Modifier;
use ratatui::text::Line;

use crate::line_truncation::truncate_line_with_ellipsis_if_overflow;
use crate::locale::Locale;
use crate::style::{accent_style, muted_style};
use crate::terminal_hyperlinks::{wrapped_line_starts, HyperlinkLine};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct TranscriptContent {
    blocks: Vec<TranscriptBlock>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum TranscriptBlock {
    Source(Vec<HyperlinkLine>),
    Activity {
        ids: Vec<String>,
        compact: Vec<HyperlinkLine>,
        expanded: Vec<HyperlinkLine>,
    },
}

impl TranscriptContent {
    pub(crate) fn from_lines(lines: Vec<HyperlinkLine>) -> Self {
        let mut content = Self::default();
        content.push_lines(lines);
        content
    }

    pub(crate) fn push_line(&mut self, line: HyperlinkLine) {
        self.push_lines(vec![line]);
    }

    pub(crate) fn push_lines(&mut self, lines: Vec<HyperlinkLine>) {
        if lines.is_empty() {
            return;
        }
        match self.blocks.last_mut() {
            Some(TranscriptBlock::Source(current)) => current.extend(lines),
            _ => self.blocks.push(TranscriptBlock::Source(lines)),
        }
    }

    pub(crate) fn push_activity(
        &mut self,
        ids: Vec<String>,
        compact: Vec<HyperlinkLine>,
        expanded: Vec<HyperlinkLine>,
    ) {
        if ids.is_empty() || compact == expanded {
            self.push_lines(expanded);
            return;
        }
        self.blocks.push(TranscriptBlock::Activity {
            ids,
            compact,
            expanded,
        });
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MaterializedTranscript {
    pub(crate) lines: Vec<HyperlinkLine>,
    pub(crate) excluded_lines: HashSet<usize>,
    controls: Vec<MaterializedControl>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MaterializedControl {
    ids: Vec<String>,
    line: usize,
    columns: Range<u16>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RenderedControl {
    ids: Vec<String>,
    row: usize,
    columns: Range<u16>,
}

#[derive(Debug, Default)]
pub(crate) struct TranscriptDisclosure {
    expanded: HashSet<String>,
    focused: Option<Vec<String>>,
    rendered: RefCell<Vec<RenderedControl>>,
    area: RefCell<Rect>,
    scroll: RefCell<usize>,
    pending_anchor: RefCell<Option<(Vec<String>, usize)>>,
}

impl TranscriptDisclosure {
    pub(crate) fn materialize(
        &self,
        content: &TranscriptContent,
        locale: Locale,
        width: u16,
    ) -> MaterializedTranscript {
        let mut lines = Vec::new();
        let mut excluded_lines = HashSet::new();
        let mut controls = Vec::new();
        for block in &content.blocks {
            match block {
                TranscriptBlock::Source(source) => lines.extend(source.iter().cloned()),
                TranscriptBlock::Activity {
                    ids,
                    compact,
                    expanded,
                } => {
                    let is_expanded = self.is_expanded(ids);
                    lines.extend(if is_expanded { expanded } else { compact }.iter().cloned());
                    let focused = self
                        .focused
                        .as_ref()
                        .is_some_and(|focused| shares_identity(focused, ids));
                    let indent = usize::from(width / 4).min(4);
                    let label = if is_expanded {
                        locale.transcript_show_less()
                    } else {
                        locale.transcript_show_details()
                    };
                    let style = if focused {
                        accent_style().add_modifier(Modifier::BOLD)
                    } else {
                        muted_style()
                    };
                    let line = truncate_line_with_ellipsis_if_overflow(
                        Line::styled(format!("{}{label}", " ".repeat(indent)), style),
                        usize::from(width),
                    );
                    let control_line = lines.len();
                    let end = u16::try_from(line.width()).unwrap_or(u16::MAX).min(width);
                    lines.push(HyperlinkLine::new(line));
                    excluded_lines.insert(control_line);
                    controls.push(MaterializedControl {
                        ids: ids.clone(),
                        line: control_line,
                        columns: u16::try_from(indent).unwrap_or(u16::MAX).min(end)..end,
                    });
                }
            }
        }
        MaterializedTranscript {
            lines,
            excluded_lines,
            controls,
        }
    }

    pub(crate) fn update_layout(
        &self,
        materialized: &MaterializedTranscript,
        area: Rect,
        scroll: usize,
    ) {
        let starts = wrapped_line_starts(&materialized.lines, area.width);
        self.rendered.replace(
            materialized
                .controls
                .iter()
                .filter_map(|control| {
                    Some(RenderedControl {
                        ids: control.ids.clone(),
                        row: *starts.get(control.line)?,
                        columns: control.columns.clone(),
                    })
                })
                .collect(),
        );
        self.area.replace(area);
        self.scroll.replace(scroll);
    }

    pub(crate) fn apply_pending_anchor(&self, max_scroll: usize) -> Option<usize> {
        let (ids, row_bias) = self.pending_anchor.borrow_mut().take()?;
        let row = self
            .rendered
            .borrow()
            .iter()
            .find(|control| shares_identity(&control.ids, &ids))?
            .row;
        Some(row.saturating_sub(row_bias).min(max_scroll))
    }

    pub(crate) fn has_controls(&self) -> bool {
        !self.rendered.borrow().is_empty()
    }

    pub(crate) fn is_focused(&self) -> bool {
        self.focused.is_some()
    }

    pub(crate) fn clear_focus(&mut self) {
        self.focused = None;
    }

    pub(crate) fn handle_event(&mut self, event: &Event) -> bool {
        match event {
            Event::Mouse(mouse)
                if mouse.kind == MouseEventKind::Down(MouseButton::Left)
                    && mouse.modifiers.is_empty() =>
            {
                let Some(ids) = self.control_at(mouse.column, mouse.row) else {
                    return false;
                };
                self.toggle(ids);
                true
            }
            Event::Key(key)
                if key.kind == KeyEventKind::Press
                    && key.modifiers.is_empty()
                    && key.code == KeyCode::F(4) =>
            {
                if self.focused.is_some() {
                    self.focused = None;
                } else if let Some(control) = self.last_visible_control() {
                    self.focused = Some(control.ids);
                    self.reveal_focused();
                }
                true
            }
            Event::Key(key) if key.kind == KeyEventKind::Press && self.focused.is_some() => {
                let focused = self.focused.clone().unwrap_or_default();
                match key.code {
                    KeyCode::Esc => self.focused = None,
                    KeyCode::Enter | KeyCode::Char(' ') if key.modifiers.is_empty() => {
                        self.toggle(focused)
                    }
                    KeyCode::Left if key.modifiers.is_empty() => {
                        if self.is_expanded(&focused) {
                            self.toggle(focused);
                        }
                    }
                    KeyCode::Right if key.modifiers.is_empty() => {
                        if !self.is_expanded(&focused) {
                            self.toggle(focused);
                        }
                    }
                    KeyCode::Up
                    | KeyCode::Down
                    | KeyCode::PageUp
                    | KeyCode::PageDown
                    | KeyCode::Home
                    | KeyCode::End
                        if key.modifiers.is_empty() =>
                    {
                        self.move_focus(key.code);
                    }
                    _ => {
                        self.focused = None;
                        return false;
                    }
                }
                true
            }
            _ => false,
        }
    }

    fn is_expanded(&self, ids: &[String]) -> bool {
        ids.iter().any(|id| self.expanded.contains(id))
    }

    fn toggle(&mut self, ids: Vec<String>) {
        if ids.is_empty() {
            return;
        }
        let row_bias = self
            .rendered
            .borrow()
            .iter()
            .find(|control| shares_identity(&control.ids, &ids))
            .map(|control| control.row.saturating_sub(*self.scroll.borrow()))
            .unwrap_or_default();
        if self.is_expanded(&ids) {
            self.expanded.retain(|id| !ids.contains(id));
        } else {
            self.expanded.extend(ids.iter().cloned());
        }
        self.focused = Some(ids.clone());
        self.pending_anchor.replace(Some((ids, row_bias)));
    }

    fn control_at(&self, column: u16, row: u16) -> Option<Vec<String>> {
        let area = *self.area.borrow();
        if !area.contains(Position::new(column, row)) {
            return None;
        }
        let visual_row = self
            .scroll
            .borrow()
            .saturating_add(usize::from(row.saturating_sub(area.y)));
        let column = column.saturating_sub(area.x);
        self.rendered
            .borrow()
            .iter()
            .find(|control| control.row == visual_row && control.columns.contains(&column))
            .map(|control| control.ids.clone())
    }

    fn last_visible_control(&self) -> Option<RenderedControl> {
        let area = *self.area.borrow();
        let scroll = *self.scroll.borrow();
        let end = scroll.saturating_add(usize::from(area.height));
        self.rendered
            .borrow()
            .iter()
            .rev()
            .find(|control| control.row >= scroll && control.row < end)
            .cloned()
            .or_else(|| self.rendered.borrow().last().cloned())
    }

    fn move_focus(&mut self, code: KeyCode) {
        let rendered = self.rendered.borrow();
        if rendered.is_empty() {
            self.focused = None;
            return;
        }
        let current = self
            .focused
            .as_ref()
            .and_then(|focused| {
                rendered
                    .iter()
                    .position(|control| shares_identity(&control.ids, focused))
            })
            .unwrap_or(rendered.len() - 1);
        let next = match code {
            KeyCode::Up => current.saturating_sub(1),
            KeyCode::Down => (current + 1).min(rendered.len() - 1),
            KeyCode::PageUp => current.saturating_sub(5),
            KeyCode::PageDown => (current + 5).min(rendered.len() - 1),
            KeyCode::Home => 0,
            KeyCode::End => rendered.len() - 1,
            _ => current,
        };
        self.focused = Some(rendered[next].ids.clone());
        drop(rendered);
        self.reveal_focused();
    }

    fn reveal_focused(&self) {
        let Some(focused) = self.focused.as_ref() else {
            return;
        };
        let area = *self.area.borrow();
        let scroll = *self.scroll.borrow();
        let Some(row) = self
            .rendered
            .borrow()
            .iter()
            .find(|control| shares_identity(&control.ids, focused))
            .map(|control| control.row)
        else {
            return;
        };
        let page = usize::from(area.height).max(1);
        let next = if row < scroll {
            row
        } else if row >= scroll.saturating_add(page) {
            row.saturating_add(1).saturating_sub(page)
        } else {
            scroll
        };
        self.pending_anchor
            .replace(Some((focused.clone(), row.saturating_sub(next))));
    }
}

fn shares_identity(left: &[String], right: &[String]) -> bool {
    left.iter().any(|id| right.contains(id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn content() -> TranscriptContent {
        let mut content = TranscriptContent::default();
        content.push_activity(
            vec!["entry:tool-1".to_string()],
            vec![HyperlinkLine::from("tool")],
            vec![HyperlinkLine::from("tool"), HyperlinkLine::from("detail")],
        );
        content
    }

    #[test]
    fn controls_are_synthetic_and_localized() {
        for (locale, details, less) in [
            (Locale::ZhCn, "+ 显示详情", "− 收起详情"),
            (Locale::ZhTw, "+ 顯示詳情", "− 收起詳情"),
            (Locale::EnUs, "+ Show details", "− Show less"),
            (Locale::JaJp, "+ 詳細を表示", "− 詳細を閉じる"),
            (Locale::KoKr, "+ 세부 정보 보기", "− 간략히 보기"),
        ] {
            let disclosure = TranscriptDisclosure::default();
            let rendered = disclosure.materialize(&content(), locale, 80);
            assert_eq!(rendered.lines.len(), 2);
            assert!(rendered.lines[1].line.to_string().contains(details));
            assert_eq!(rendered.excluded_lines, HashSet::from([1]));
            assert!(!locale
                .transcript_pager_activity_footer("Ctrl+T·Esc·Q")
                .is_empty());
            assert!(!locale.transcript_activity_focus_footer().is_empty());
            assert_eq!(locale.transcript_show_less(), less);
        }
    }

    #[test]
    fn expansion_is_retained_by_activity_identity_across_prepends() {
        let mut disclosure = TranscriptDisclosure::default();
        let initial = disclosure.materialize(&content(), Locale::EnUs, 80);
        disclosure.update_layout(&initial, Rect::new(0, 0, 80, 10), 0);
        assert!(
            disclosure.handle_event(&Event::Key(crossterm::event::KeyEvent::new(
                KeyCode::F(4),
                KeyModifiers::NONE,
            )))
        );
        assert!(
            disclosure.handle_event(&Event::Key(crossterm::event::KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            )))
        );

        let mut prepended = TranscriptContent::from_lines(vec![HyperlinkLine::from("older")]);
        prepended.blocks.extend(content().blocks);
        let rendered = disclosure.materialize(&prepended, Locale::EnUs, 80);
        assert!(rendered
            .lines
            .iter()
            .any(|line| line.line.to_string() == "detail"));
        assert!(rendered
            .lines
            .iter()
            .any(|line| line.line.to_string().contains("− Show less")));
    }
}
