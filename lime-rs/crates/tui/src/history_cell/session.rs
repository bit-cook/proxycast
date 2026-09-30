//! Session header history cells.

use super::*;
use crate::line_truncation::{line_width, truncate_line_with_ellipsis_if_overflow};
use crate::locale::Locale;
use crate::style::{attention_style, muted_style};
use crate::text_formatting::center_truncate_path;
use ratatui::style::Style;
use ratatui::text::Span;

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
        let width = usize::from(width);
        let mut lines = vec![
            Line::default(),
            Line::from(vec![
                Span::raw("  "),
                Span::styled(">_ ", muted_style()),
                Span::styled("Lime", Style::default().bold()),
                Span::styled(format!(" (v{})", self.version), muted_style()),
            ]),
            Line::from(vec![
                Span::raw("     "),
                Span::raw(self.format_directory(Some(width.saturating_sub(5)))),
            ]),
        ];
        if self.permissions.as_deref() == Some(":danger-full-access") {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("  {}: ", self.locale.permissions_label()),
                    muted_style(),
                ),
                Span::styled(":danger-full-access", attention_style()),
            ]));
        }
        lines
            .into_iter()
            .map(|line| truncate_line_with_ellipsis_if_overflow(line, width))
            .collect()
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
    fn session_header_is_borderless_and_width_bounded() {
        for width in [40, 80, 120] {
            let lines =
                cell(Locale::EnUs, "/workspace/a/very/long/project/path").display_lines(width);
            assert!(lines
                .iter()
                .all(|line| line_width(line) <= usize::from(width)));
            assert!(lines.iter().any(|line| line.to_string().contains("Lime")));
            assert!(lines
                .iter()
                .any(|line| line.to_string().contains("very/long")));
            assert!(!lines.iter().any(|line| line.to_string().contains("╭")));
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
            assert!(text.contains("Lime"), "{locale:?}: {text}");
            assert!(text.contains("/workspace"), "{locale:?}: {text}");
            assert!(!text.contains("fixture-model"), "{locale:?}: {text}");
            assert!(!text.contains("/model"), "{locale:?}: {text}");
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
