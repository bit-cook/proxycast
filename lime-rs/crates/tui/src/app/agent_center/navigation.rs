//! Paging uses the rendered rows, including non-selectable group headings and gaps.

use super::*;

pub(super) enum CenterRow {
    Group(usize),
    Task(usize),
    Gap,
    ShowMore,
}

impl AgentsOverviewView {
    pub(super) fn center_rows(&self, rows: &[&AgentsOverviewRow]) -> Vec<CenterRow> {
        let mut entries = Vec::new();
        for (index, task) in rows.iter().enumerate() {
            if index == 0 || !self.same_group(rows[index - 1], task) {
                if index > 0 {
                    entries.push(CenterRow::Gap);
                }
                entries.push(CenterRow::Group(index));
            }
            entries.push(CenterRow::Task(index));
        }
        if self.has_more() {
            entries.push(CenterRow::Gap);
            entries.push(CenterRow::ShowMore);
        }
        entries
    }

    pub(super) fn page_selection(&mut self, forward: bool) {
        if self.input_mode.is_some() {
            return;
        }
        let rows = self.visible_rows();
        let entries = self.center_rows(&rows);
        let tasks = entries
            .iter()
            .enumerate()
            .filter_map(|(position, entry)| match entry {
                CenterRow::Task(index) => Some((position, *index)),
                CenterRow::ShowMore => Some((position, rows.len())),
                _ => None,
            })
            .collect::<Vec<_>>();
        if tasks.is_empty() {
            return;
        }
        let current = tasks
            .iter()
            .find(|(_, index)| *index == self.selected)
            .map(|(position, _)| *position)
            .unwrap_or(tasks[0].0);
        let height = self.page_height.get().max(1);
        let target = if forward {
            current.saturating_add(height)
        } else {
            current.saturating_sub(height)
        };
        let selected = if forward {
            tasks
                .partition_point(|(position, _)| *position < target)
                .min(tasks.len() - 1)
        } else {
            tasks
                .partition_point(|(position, _)| *position <= target)
                .saturating_sub(1)
        };
        let scroll = if forward {
            self.scroll.get().saturating_add(height)
        } else {
            self.scroll.get().saturating_sub(height)
        };
        self.scroll
            .set(scroll.min(entries.len().saturating_sub(height)));
        self.selected = tasks[selected].1;
    }
}
