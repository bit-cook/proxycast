//! One visual row per task, shared grouping headings and stable metadata columns.

use super::navigation::CenterRow;
use super::render::{line, row};
use super::*;
use crate::locale::Locale;
use crate::style::selection_style;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::Widget;
use unicode_width::UnicodeWidthStr;

fn columns(area: Rect) -> (Rect, Rect, Rect) {
    let metadata = if area.width >= 56 { 24 } else { 0 };
    let gutter = area.width.min(4);
    let title = Rect::new(
        area.x + gutter,
        area.y,
        area.width - gutter - metadata,
        area.height,
    );
    let updated = Rect::new(
        area.right() - metadata.min(9),
        area.y,
        metadata.min(9),
        area.height,
    );
    let status = Rect::new(
        title.right() + metadata.min(2),
        area.y,
        if metadata == 24 { 11 } else { 0 },
        area.height,
    );
    (title, status, updated)
}

pub(in crate::app::agents_overview_view) fn status(
    task: &AgentsOverviewRow,
    locale: Locale,
) -> (&'static str, Span<'static>) {
    match task.group {
        AgentsOverviewGroup::NeedsYou
            if task.thread.status
                == app_server_protocol::protocol::v2::ThreadStatus::SystemError =>
        {
            (locale.agent_center_label("Error"), "!".red())
        }
        AgentsOverviewGroup::NeedsYou => (locale.agent_center_label("Needs input"), "●".red()),
        AgentsOverviewGroup::Working => (locale.agent_center_label("Working"), "●".green()),
        AgentsOverviewGroup::Ready => (locale.agent_center_label("Ready"), "○".cyan()),
        AgentsOverviewGroup::Finished => (locale.agent_center_label("Inactive"), "○".dim()),
    }
}

impl AgentsOverviewView {
    pub(super) fn render_center_rows(
        &self,
        area: Rect,
        buf: &mut Buffer,
        locale: Locale,
        reference: i64,
    ) {
        let row_width = area.width;
        let area = Rect {
            width: row_width.saturating_sub(1),
            ..area
        };
        if area.is_empty() {
            self.page_height.set(0);
            return;
        }
        let rows = self.visible_rows();
        let entries = self.center_rows(&rows);
        let selected_index = self.selected_index();
        if entries.is_empty() {
            line(
                locale
                    .agent_center_label(if self.rows.is_empty() {
                        "No tasks yet"
                    } else {
                        "No matching tasks"
                    })
                    .dim(),
                area,
                buf,
            );
            self.page_height.set(usize::from(area.height));
            return;
        }
        let selected = entries
            .iter()
            .position(|entry| match entry {
                CenterRow::Task(index) => Some(*index) == selected_index,
                CenterRow::ShowMore => self.selected_is_load_more(),
                _ => false,
            })
            .unwrap_or_default();
        let padding = u16::from(area.height >= 3);
        let viewport = row(area, padding, area.height - padding * 2);
        if padding > 0 {
            let (title, status, updated) = columns(row(area, 0, 1));
            line(locale.agent_center_label("Tasks").dim(), title, buf);
            line(locale.agent_center_label("Status").dim(), status, buf);
            line(
                Line::from(locale.agent_center_label("Updated").dim()).right_aligned(),
                updated,
                buf,
            );
        }
        let height = usize::from(viewport.height);
        self.page_height.set(height);
        let mut start = self.scroll.get().min(entries.len().saturating_sub(height));
        if selected < start {
            start = if height > 1
                && selected > 0
                && matches!(entries[selected - 1], CenterRow::Group(_))
            {
                selected - 1
            } else {
                selected
            };
        }
        if selected >= start + height {
            start = selected.saturating_add(1).saturating_sub(height);
        }
        self.scroll.set(start);
        for (offset, entry) in entries.iter().skip(start).take(height).enumerate() {
            let rect = row(viewport, offset as u16, 1);
            let index = match entry {
                CenterRow::Gap => continue,
                CenterRow::ShowMore => {
                    let selected = self.selected_is_load_more();
                    let style = if selected {
                        selection_style()
                    } else {
                        Style::default()
                    };
                    buf.set_style(
                        Rect {
                            width: row_width,
                            ..rect
                        },
                        style,
                    );
                    let label = locale.agent_center_label(if self.loading_more() {
                        "Loading more…"
                    } else if self.load_more_failed() {
                        "Show more (retry)"
                    } else {
                        "Show more"
                    });
                    line(
                        Line::from(format!("{}   {label}", if selected { "›" } else { " " }))
                            .style(style),
                        rect,
                        buf,
                    );
                    continue;
                }
                CenterRow::Group(index) | CenterRow::Task(index) => *index,
            };
            let task = rows[index];
            if matches!(entry, CenterRow::Group(_)) {
                let mut label = match self.grouping {
                    super::super::grouping::AgentsOverviewGrouping::Project => {
                        task.thread.cwd.display().to_string()
                    }
                    super::super::grouping::AgentsOverviewGrouping::Status => {
                        locale.agent_center_label(task.group.label()).to_string()
                    }
                };
                let count = rows.iter().filter(|row| self.same_group(task, row)).count();
                let total = self
                    .rows
                    .iter()
                    .filter(|row| self.same_group(task, row))
                    .count();
                let count = locale.agent_center_count(count, total);
                if matches!(
                    self.grouping,
                    super::super::grouping::AgentsOverviewGrouping::Project
                ) {
                    label = crate::text_formatting::center_truncate_path(
                        &label,
                        usize::from(rect.width)
                            .saturating_sub(count.width() + 2)
                            .min(64),
                    );
                }
                line(format!("{label}  {count}").dim(), rect, buf);
                continue;
            }
            let selected = Some(index) == selected_index;
            let style = if selected {
                selection_style()
            } else {
                Style::default()
            };
            buf.set_style(
                Rect {
                    width: row_width,
                    ..rect
                },
                style,
            );
            let (status, mut dot) = status(task, locale);
            if selected {
                dot.style = style;
            }
            line(
                Line::from(if selected { "›" } else { " " }).style(style),
                rect,
                buf,
            );
            line(
                Line::from(dot),
                Rect::new(
                    rect.x + rect.width.min(2),
                    rect.y,
                    rect.width.saturating_sub(2).min(1),
                    1,
                ),
                buf,
            );
            let (title, status_area, updated) = columns(rect);
            line(
                Line::from(display_title(
                    task,
                    locale.agent_center_label("Untitled task"),
                ))
                .style(style),
                title,
                buf,
            );
            line(Line::from(status).style(style), status_area, buf);
            if !updated.is_empty() {
                let age = locale.agent_center_age(reference, task.thread.updated_at);
                Line::from(age)
                    .style(if selected { style } else { style.dim() })
                    .right_aligned()
                    .render(updated, buf);
            }
        }
        if padding > 0 && start > 0 {
            line("↑".dim(), row(area, 0, 1), buf);
        }
        if padding > 0 && start + height < entries.len() {
            line("↓".dim(), row(area, area.height.saturating_sub(1), 1), buf);
        }
    }
}
