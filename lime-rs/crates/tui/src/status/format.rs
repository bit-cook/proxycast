//! Formatting helpers for the borderless `/status` surface.

use ratatui::prelude::*;

use crate::width::display_width;

/// Align status values under one stable column and provide a continuation indent for wrapping.
#[derive(Debug, Clone)]
pub(crate) struct FieldFormatter {
    indent: &'static str,
    label_width: usize,
    value_offset: usize,
    value_indent: String,
}

impl FieldFormatter {
    pub(crate) const INDENT: &'static str = "  ";

    pub(crate) fn from_labels<'a>(labels: impl IntoIterator<Item = &'a str>) -> Self {
        let label_width = labels
            .into_iter()
            .map(display_width)
            .max()
            .unwrap_or_default();
        let indent_width = display_width(Self::INDENT);
        let value_offset = indent_width + label_width + 1 + 2;
        Self {
            indent: Self::INDENT,
            label_width,
            value_offset,
            value_indent: " ".repeat(value_offset),
        }
    }

    pub(crate) fn line(&self, label: &str, value: impl Into<String>) -> Line<'static> {
        Line::from(vec![self.label_span(label), Span::raw(value.into())])
    }

    pub(crate) fn continuation(&self) -> Line<'static> {
        Line::from(Span::raw(self.value_indent.clone()))
    }

    pub(crate) fn value_width(&self, width: usize) -> usize {
        width.saturating_sub(self.value_offset)
    }

    fn label_span(&self, label: &str) -> Span<'static> {
        let mut prefix = String::with_capacity(self.value_offset);
        prefix.push_str(self.indent);
        prefix.push_str(label);
        prefix.push(':');
        let padding = 2 + self.label_width.saturating_sub(display_width(label));
        prefix.extend(std::iter::repeat_n(' ', padding));
        Span::styled(prefix, Style::default().add_modifier(Modifier::DIM))
    }
}
