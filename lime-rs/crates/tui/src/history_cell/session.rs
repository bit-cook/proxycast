//! Session header history cells.

use super::*;
use crate::line_truncation::{line_width, truncate_line_with_ellipsis_if_overflow};
use crate::locale::Locale;
use crate::style::{accent_style, attention_style, muted_style};
use crate::text_formatting::center_truncate_path;
use crate::width::display_width;
use ratatui::style::Style;
use ratatui::text::Span;

const SESSION_HEADER_MAX_INNER_WIDTH: usize = 56;

#[derive(Debug)]
pub struct SessionInfoCell(CompositeHistoryCell);

impl SessionInfoCell {
    pub(crate) fn new(parts: Vec<Box<dyn HistoryCell>>) -> Self {
        Self(CompositeHistoryCell::new(parts))
    }
}

impl HistoryCell for SessionInfoCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        self.0.display_lines(width)
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        self.0.raw_lines()
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SessionHeaderHistoryCell {
    pub(crate) version: String,
    pub(crate) model: String,
    pub(crate) reasoning_effort: Option<String>,
    pub(crate) permissions: Option<String>,
    pub(crate) directory: std::path::PathBuf,
    pub(crate) locale: Locale,
}

impl SessionHeaderHistoryCell {
    pub(crate) fn new(
        model: impl Into<String>,
        reasoning_effort: Option<String>,
        permissions: Option<String>,
        directory: std::path::PathBuf,
        version: impl Into<String>,
        locale: Locale,
    ) -> Self {
        Self {
            version: version.into(),
            model: model.into(),
            reasoning_effort,
            permissions,
            directory,
            locale,
        }
    }

    fn format_directory(&self, max_width: Option<usize>) -> String {
        let display = dirs::home_dir()
            .and_then(|home| {
                self.directory.strip_prefix(home).ok().map(|relative| {
                    if relative.as_os_str().is_empty() {
                        "~".to_string()
                    } else {
                        format!("~{}{}", std::path::MAIN_SEPARATOR, relative.display())
                    }
                })
            })
            .unwrap_or_else(|| self.directory.display().to_string());
        max_width.map_or(display.clone(), |width| {
            center_truncate_path(&display, width)
        })
    }
}

impl HistoryCell for SessionHeaderHistoryCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        let Some(inner_width) = card_inner_width(width) else {
            return Vec::new();
        };
        let model_label = self.locale.model_label();
        let directory_label = self.locale.cwd_label();
        let show_danger = self.permissions.as_deref() == Some(":danger-full-access");
        let label_width = [
            display_width(model_label),
            display_width(directory_label),
            if show_danger {
                display_width(self.locale.permissions_label())
            } else {
                0
            },
        ]
        .into_iter()
        .max()
        .unwrap_or_default();

        let model_prefix = padded_label(model_label, label_width);
        let directory_prefix = padded_label(directory_label, label_width);
        let mut model_spans = vec![
            Span::styled(format!("{model_prefix}: "), muted_style()),
            Span::raw(self.model.clone()),
        ];
        if let Some(effort) = self
            .reasoning_effort
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            model_spans.push(Span::raw(format!(" {effort}")));
        }
        model_spans.extend([
            Span::raw("   "),
            Span::styled("/model", accent_style()),
            Span::styled(self.locale.change_model_hint(), muted_style()),
        ]);

        let directory_prefix = format!("{directory_prefix}: ");
        let directory_width = inner_width.saturating_sub(display_width(&directory_prefix));
        let mut lines = vec![
            Line::from(vec![
                Span::styled(">_ ", muted_style()),
                Span::styled("Lime", Style::default().bold()),
                Span::styled(format!(" (v{})", self.version), muted_style()),
            ]),
            Line::default(),
            Line::from(model_spans),
            Line::from(vec![
                Span::styled(directory_prefix, muted_style()),
                Span::raw(self.format_directory(Some(directory_width))),
            ]),
        ];
        if show_danger {
            let permissions_label = padded_label(self.locale.permissions_label(), label_width);
            lines.push(Line::from(vec![
                Span::styled(format!("{permissions_label}: "), muted_style()),
                Span::styled(":danger-full-access", attention_style()),
            ]));
        }
        let lines = lines
            .into_iter()
            .map(|line| truncate_line_with_ellipsis_if_overflow(line, inner_width))
            .collect();
        with_border(lines)
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        let mut model = self.model.clone();
        if let Some(effort) = self
            .reasoning_effort
            .as_deref()
            .filter(|value| !value.is_empty())
        {
            model.push(' ');
            model.push_str(effort);
        }
        let mut lines = vec![
            Line::from(format!("Lime (v{})", self.version)),
            Line::from(format!("{}: {model}", self.locale.model_label())),
            Line::from(format!(
                "{}: {}",
                self.locale.cwd_label(),
                self.format_directory(None)
            )),
        ];
        if self.permissions.as_deref() == Some(":danger-full-access") {
            lines.push(Line::from(format!(
                "{}: :danger-full-access",
                self.locale.permissions_label()
            )));
        }
        lines
    }
}

fn card_inner_width(width: u16) -> Option<usize> {
    (width >= 4).then(|| usize::from(width.saturating_sub(4)).min(SESSION_HEADER_MAX_INNER_WIDTH))
}

fn padded_label(label: &str, width: usize) -> String {
    format!(
        "{label}{}",
        " ".repeat(width.saturating_sub(display_width(label)))
    )
}

fn with_border(lines: Vec<Line<'static>>) -> Vec<Line<'static>> {
    let content_width = lines.iter().map(line_width).max().unwrap_or_default();
    let border_width = content_width.saturating_add(2);
    let mut output = Vec::with_capacity(lines.len().saturating_add(2));
    output.push(Line::styled(
        format!("╭{}╮", "─".repeat(border_width)),
        muted_style(),
    ));
    for line in lines {
        let used = line_width(&line);
        let mut spans = vec![Span::styled("│ ", muted_style())];
        spans.extend(line);
        spans.push(Span::raw(" ".repeat(content_width.saturating_sub(used))));
        spans.push(Span::styled(" │", muted_style()));
        output.push(Line::from(spans));
    }
    output.push(Line::styled(
        format!("╰{}╯", "─".repeat(border_width)),
        muted_style(),
    ));
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell(locale: Locale, directory: &str) -> SessionHeaderHistoryCell {
        SessionHeaderHistoryCell::new(
            "fixture-model",
            Some("high".to_string()),
            Some(":workspace".to_string()),
            directory.into(),
            "1.2.3",
            locale,
        )
    }

    #[test]
    fn session_header_is_a_bounded_codex_shaped_box() {
        for width in [40, 80, 120] {
            let lines =
                cell(Locale::EnUs, "/workspace/a/very/long/project/path").display_lines(width);
            assert!(lines
                .first()
                .is_some_and(|line| line.to_string().starts_with('╭')));
            assert!(lines
                .iter()
                .all(|line| line_width(line) <= usize::from(width)));
            assert!(lines.iter().any(|line| line.to_string().contains("Lime")));
            assert!(lines
                .last()
                .is_some_and(|line| line.to_string().starts_with('╰')));
        }
    }

    #[test]
    fn session_header_labels_cover_all_product_locales() {
        for locale in [
            Locale::ZhCn,
            Locale::ZhTw,
            Locale::EnUs,
            Locale::JaJp,
            Locale::KoKr,
        ] {
            let rendered = cell(locale, "/workspace").display_lines(80);
            let text = rendered.iter().map(Line::to_string).collect::<String>();
            assert!(text.contains(locale.model_label()), "{locale:?}: {text}");
            assert!(text.contains(locale.cwd_label()), "{locale:?}: {text}");
            assert!(text.contains("/model"), "{locale:?}: {text}");
            assert!(
                text.contains(locale.change_model_hint()),
                "{locale:?}: {text}"
            );
        }
    }

    #[test]
    fn dangerous_permissions_are_visible_without_becoming_a_fixed_header() {
        let mut header = cell(Locale::EnUs, "/workspace");
        header.permissions = Some(":danger-full-access".to_string());
        let text = header
            .display_lines(80)
            .iter()
            .map(Line::to_string)
            .collect::<String>();
        assert!(text.contains("permissions"));
        assert!(text.contains(":danger-full-access"));
    }
}
