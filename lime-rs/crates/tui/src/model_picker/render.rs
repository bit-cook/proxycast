//! Borderless bottom picker; wrapped visual rows drive both measurement and scrolling.

use super::*;
use crate::bottom_pane::selection_row_layout::{
    wrap_row, SelectionDescriptionLayout, SelectionRow,
};
use crate::line_truncation::{line_width, truncate_line_with_ellipsis_if_overflow};
use crate::style::{muted_style, selection_style};
use crate::width::display_width;
use ratatui::layout::{Position, Rect};
use ratatui::style::Stylize;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};

fn rows(picker: &ModelPicker, locale: Locale, width: u16) -> Vec<Vec<Line<'static>>> {
    let indices = picker.visible_indices();
    let names = indices
        .iter()
        .enumerate()
        .map(|(visible, index)| {
            let model = &picker.models[*index];
            let flag = if Some(*index) == picker.current {
                Some(locale.picker_current_label())
            } else if model.is_default {
                Some(locale.picker_default_label())
            } else {
                None
            };
            let suffix = flag.map(|flag| format!(" ({flag})")).unwrap_or_default();
            format!("{}. {}{suffix}", visible + 1, model.display_name)
        })
        .collect::<Vec<_>>();
    let row_width = width.saturating_sub(2).max(1);
    let name_width = names
        .iter()
        .map(|name| display_width(name))
        .max()
        .unwrap_or(0);
    let desc_col = name_width
        .saturating_add(2)
        .min((usize::from(row_width) * 70 / 100).max(1))
        .max(1);
    indices
        .into_iter()
        .zip(names)
        .map(|(index, name)| {
            let model = &picker.models[index];
            let description = if model.description.is_empty() {
                format!("[{}]", model.provider_id)
            } else {
                format!("[{}] {}", model.provider_id, model.description)
            };
            wrap_row(
                &SelectionRow::new(name, Some(description), Vec::new()),
                desc_col,
                row_width,
                SelectionDescriptionLayout::StackBelowWhenNarrow {
                    min_description_width: 12,
                },
            )
        })
        .collect()
}

pub(crate) fn desired_height(picker: &ModelPicker, locale: Locale, width: u16) -> u16 {
    let height = rows(picker, locale, width)
        .iter()
        .take(MAX_POPUP_ROWS)
        .map(Vec::len)
        .sum::<usize>()
        .clamp(1, MAX_POPUP_ROWS * 2);
    u16::try_from(height).unwrap_or(u16::MAX).saturating_add(6)
}

fn visible_window(rows: &[Vec<Line<'static>>], selected: usize, height: usize) -> (usize, usize) {
    if rows.is_empty() || height == 0 {
        return (0, 0);
    }
    let selected = selected.min(rows.len() - 1);
    let mut start = selected;
    let mut used = rows[selected].len().min(height);
    while start > 0
        && selected - start + 1 < MAX_POPUP_ROWS
        && used + rows[start - 1].len() <= height
    {
        start -= 1;
        used += rows[start].len();
    }
    let mut end = selected + 1;
    while end < rows.len() && end - start < MAX_POPUP_ROWS && used + rows[end].len() <= height {
        used += rows[end].len();
        end += 1;
    }
    (start, end)
}

fn suffix(text: &str, width: usize) -> &str {
    let mut used = 0;
    let mut start = text.len();
    for (offset, grapheme) in text.grapheme_indices(true).rev() {
        let next = used + display_width(grapheme);
        if next > width {
            break;
        }
        start = offset;
        used = next;
    }
    &text[start..]
}

pub(super) fn render(frame: &mut Frame<'_>, area: Rect, picker: &ModelPicker, locale: Locale) {
    let height = desired_height(picker, locale, area.width).min(area.height);
    let area = Rect::new(
        area.x,
        area.bottom().saturating_sub(height),
        area.width,
        height,
    );
    if area.is_empty() {
        return;
    }
    frame.render_widget(Clear, area);
    // Title/search/result take priority over spacing and wrapped hints at tiny heights.
    let content_x = area.x.saturating_add(2.min(area.width.saturating_sub(1)));
    let content_width = area.right().saturating_sub(content_x).saturating_sub(2);
    let title = Rect::new(content_x, area.y, content_width, 1);
    frame.render_widget(
        Paragraph::new(truncate_line_with_ellipsis_if_overflow(
            Line::from(locale.model_picker_title()).bold(),
            usize::from(title.width),
        )),
        title,
    );
    if height < 2 {
        return;
    }
    let title_gap = u16::from(height >= 6);
    let search_y = area.y + 1 + title_gap;
    let search = Rect::new(content_x, search_y, content_width, 1);
    let query = suffix(picker.query(), usize::from(search.width.saturating_sub(1)));
    let search_line = if picker.query().is_empty() {
        Line::from(locale.model_picker_search_hint()).dim()
    } else {
        Line::from(query.to_string())
    };
    frame.render_widget(
        Paragraph::new(truncate_line_with_ellipsis_if_overflow(
            search_line,
            usize::from(search.width),
        )),
        search,
    );
    if search.width > 0 {
        frame.set_cursor_position(Position::new(
            search.x
                + u16::try_from(display_width(query))
                    .unwrap_or(u16::MAX)
                    .min(search.width - 1),
            search.y,
        ));
    }
    if height < 3 {
        return;
    }
    let footer_height = u16::from(height >= 4);
    let search_gap = u16::from(height >= 7);
    let footer_gap = u16::from(height >= 7);
    let list_y = search_y + 1 + search_gap;
    let list_bottom = area.bottom().saturating_sub(footer_height + footer_gap);
    let list = Rect::new(
        area.x,
        list_y,
        area.width,
        list_bottom.saturating_sub(list_y),
    );
    let rows = rows(picker, locale, list.width);
    let (start, end) = visible_window(&rows, picker.selected, usize::from(list.height));
    picker.page_rows.set(end.saturating_sub(start).max(1));
    let mut y = list.y;
    for (index, row) in rows.iter().enumerate().take(end).skip(start) {
        let selected = index == picker.selected;
        for (line_index, line) in row.iter().enumerate() {
            if y >= list.bottom() {
                break;
            }
            let mut line = line.clone();
            line.spans.insert(
                0,
                Span::raw(if selected && line_index == 0 {
                    "› "
                } else {
                    "  "
                }),
            );
            let line = truncate_line_with_ellipsis_if_overflow(line, usize::from(list.width));
            let style = if selected {
                selection_style()
            } else {
                ratatui::style::Style::default()
            };
            frame.render_widget(
                Paragraph::new(line).style(style),
                Rect::new(list.x, y, list.width, 1),
            );
            y += 1;
        }
    }
    if rows.is_empty() && list.height > 0 {
        frame.render_widget(
            Paragraph::new(locale.picker_empty()).style(muted_style()),
            Rect::new(content_x, list.y, content_width, 1),
        );
    }
    if footer_height > 0 {
        let footer = Rect::new(content_x, area.bottom() - 1, content_width, 1);
        frame.render_widget(
            Paragraph::new(truncate_line_with_ellipsis_if_overflow(
                Line::from(locale.model_picker_footer()).dim(),
                usize::from(footer.width),
            )),
            footer,
        );
    }
    debug_assert!(rows
        .iter()
        .flatten()
        .all(|line| line_width(line) <= usize::from(list.width.saturating_sub(2).max(1))));
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
