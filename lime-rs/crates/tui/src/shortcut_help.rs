//! Shared shortcut groups align keys and flow into three, two or one column.
//! Surface owners supply resolved bindings, labels and height limits.

use ratatui::style::Stylize;
use ratatui::text::{Line, Span};

const COLUMN_GAP: usize = 4;

pub(crate) struct Shortcut {
    pub(crate) key: String,
    pub(crate) action: &'static str,
}

pub(crate) struct Group {
    pub(crate) title: &'static str,
    pub(crate) entries: Vec<Shortcut>,
}

impl Group {
    pub(crate) fn push(&mut self, key: Option<String>, action: &'static str) {
        if let Some(key) = key.filter(|key| !key.is_empty()) {
            self.entries.push(Shortcut { key, action });
        }
    }

    fn lines(&self) -> Vec<Line<'static>> {
        let key_width = self
            .entries
            .iter()
            .map(|entry| Span::raw(&entry.key).width())
            .max()
            .unwrap_or(0);
        let mut lines = vec![self.title.bold().into()];
        for entry in &self.entries {
            let key = Span::styled(entry.key.clone(), crate::style::accent_style().not_bold());
            let padding = " ".repeat(key_width.saturating_sub(key.width()) + 2);
            lines.push(vec![key, padding.into(), entry.action.into()].into());
        }
        lines
    }
}

pub(crate) fn group_lines(groups: [Group; 3], width: u16) -> Vec<Line<'static>> {
    let groups = groups.map(|group| group.lines());
    let widths = groups
        .iter()
        .map(|group| group.iter().map(Line::width).max().unwrap_or(0))
        .collect::<Vec<_>>();
    let width = usize::from(width.max(1));
    if widths.iter().sum::<usize>() + COLUMN_GAP * 2 <= width {
        columns(&groups, &widths)
    } else if widths[0] + COLUMN_GAP + widths[1].max(widths[2]) <= width {
        let mut right = groups[1].clone();
        right.push(Line::default());
        right.extend(groups[2].clone());
        columns(
            &[groups[0].clone(), right],
            &[widths[0], widths[1].max(widths[2])],
        )
    } else {
        let mut result = Vec::new();
        for (index, group) in groups.into_iter().enumerate() {
            if index > 0 {
                result.push(Line::default());
            }
            result.extend(group);
        }
        result
    }
}

fn columns(groups: &[Vec<Line<'static>>], widths: &[usize]) -> Vec<Line<'static>> {
    let height = groups.iter().map(Vec::len).max().unwrap_or(0);
    (0..height)
        .map(|row| {
            let mut line = Line::default();
            for (column, group) in groups.iter().enumerate() {
                let entry = group.get(row).cloned().unwrap_or_default();
                let padding = widths[column].saturating_sub(entry.width()) + COLUMN_GAP;
                line.extend(entry.spans);
                if column + 1 < groups.len() {
                    line.push_span(" ".repeat(padding));
                }
            }
            line
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_columns_follow_available_width_and_keep_every_bound_action() {
        let groups = || {
            ["Navigate", "Tasks", "View"].map(|title| {
                let mut group = Group {
                    title,
                    entries: Vec::new(),
                };
                group.push(Some("f3".into()), "Open");
                group.push(None, "Unbound");
                group
            })
        };
        for (width, expected_height) in [(40, 2), (22, 5), (12, 8)] {
            let lines = group_lines(groups(), width);
            assert_eq!(lines.len(), expected_height, "help layout at {width}");
            let text = lines
                .iter()
                .map(|line| {
                    line.spans
                        .iter()
                        .map(|span| span.content.as_ref())
                        .collect::<String>()
                })
                .collect::<Vec<_>>()
                .join("\n");
            assert_eq!(text.matches("f3").count(), 3);
            assert!(!text.contains("Unbound"));
        }
    }
}
