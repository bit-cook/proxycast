use std::path::Path;

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use crate::diff_render;
use crate::exec_cell::{output_lines, CommandOutput, OutputLinesParams};
use crate::locale::Locale;
use crate::markdown_render;
use crate::projection::{EntryKind, EntryStatus, TranscriptEntry};
use crate::terminal_hyperlinks::{prefix_hyperlink_lines, HyperlinkLine};

#[cfg(test)]
pub(crate) fn lines(entry: &TranscriptEntry) -> Vec<Line<'static>> {
    hyperlink_lines_with_locale(entry, Locale::default(), None, Path::new(""))
        .into_iter()
        .map(|line| line.line)
        .collect()
}

pub(crate) fn hyperlink_lines_with_locale(
    entry: &TranscriptEntry,
    locale: Locale,
    width: Option<usize>,
    cwd: &Path,
) -> Vec<HyperlinkLine> {
    let (prefix, prefix_style, text_style) = styles(entry.kind);
    let rich_lines = match entry.kind {
        EntryKind::Assistant | EntryKind::Reasoning => Some(
            markdown_render::render_markdown_lines_with_width(&entry.text, text_style, width),
        ),
        EntryKind::Patch => Some(
            diff_render::render(&entry.text, width, cwd)
                .into_iter()
                .map(HyperlinkLine::new)
                .collect(),
        ),
        _ => None,
    };
    if let Some(mut rendered) = rich_lines {
        if rendered.is_empty() {
            rendered.push(HyperlinkLine::default());
        }
        let status_suffix = entry
            .status
            .map(|status| format!(" [{}]", locale.status(status.label())));
        rendered = prefix_hyperlink_lines(
            rendered,
            Span::styled(prefix, prefix_style),
            Span::styled("  ", prefix_style),
        );
        if let Some(suffix) = status_suffix {
            if let Some(line) = rendered.first_mut() {
                line.push_span(Span::styled(suffix, text_style), None);
            }
        }
        rendered.extend(entry.summary.iter().map(|detail| {
            let detail = formatted_summary(entry.kind, detail, locale);
            HyperlinkLine::new(format_line(
                entry.kind,
                "  ",
                prefix_style,
                continuation_style(entry.kind, text_style),
                &detail,
            ))
        }));
        return rendered;
    }
    let mut source = entry.text.lines();
    let first = source.next().unwrap_or("");
    let first = match entry.kind {
        EntryKind::Tool | EntryKind::System => locale.detail(&localized_tool_detail(first, cwd)),
        EntryKind::MultiAgent => locale.multi_agent(first),
        _ => first.to_string(),
    };
    let first = with_status_suffix(&first, entry.status, locale);
    let mut lines = vec![HyperlinkLine::new(format_line(
        entry.kind,
        prefix,
        prefix_style,
        text_style,
        &first,
    ))];
    if entry.kind == EntryKind::Command {
        let output = CommandOutput::from_lines(source, locale);
        let rendered_output = output_lines(
            Some(&output),
            OutputLinesParams {
                line_limit: 50,
                only_err: false,
                include_angle_pipe: false,
                include_prefix: false,
                locale,
            },
        );
        lines.extend(rendered_output.lines.iter().map(|line| {
            HyperlinkLine::new(format_line(
                entry.kind,
                "  ",
                prefix_style,
                continuation_style(entry.kind, text_style),
                line,
            ))
        }));
    } else {
        lines.extend(source.map(|line| {
            HyperlinkLine::new(format_line(
                entry.kind,
                "  ",
                prefix_style,
                continuation_style(entry.kind, text_style),
                line,
            ))
        }));
    }
    lines.extend(entry.summary.iter().map(|detail| {
        let detail = formatted_summary(entry.kind, detail, locale);
        HyperlinkLine::new(format_line(
            entry.kind,
            "  ",
            prefix_style,
            continuation_style(entry.kind, text_style),
            &detail,
        ))
    }));
    lines
}

fn localized_tool_detail(detail: &str, cwd: &Path) -> String {
    detail
        .strip_prefix("view image: ")
        .map(|path| format!("view image: {}", diff_render::display_path_for(path, cwd)))
        .unwrap_or_else(|| detail.to_string())
}

fn formatted_summary(kind: EntryKind, detail: &str, locale: Locale) -> String {
    if kind == EntryKind::User {
        if let Some(index) = detail.strip_prefix("image: ") {
            if !index.is_empty() {
                return locale.numbered_image_label(index);
            }
        }
    }
    format!("- {}", locale.detail(detail))
}

fn with_status_suffix(text: &str, status: Option<EntryStatus>, locale: Locale) -> String {
    status
        .map(|status| format!("{text} [{}]", locale.status(status.label())))
        .unwrap_or_else(|| text.to_string())
}

fn styles(kind: EntryKind) -> (&'static str, Style, Style) {
    match kind {
        EntryKind::User => (
            "> ",
            Style::default().fg(Color::Cyan),
            Style::default().fg(Color::Cyan),
        ),
        EntryKind::Assistant => ("  ", Style::default(), Style::default()),
        EntryKind::Reasoning => (
            "· ",
            Style::default().fg(Color::DarkGray),
            Style::default().fg(Color::DarkGray),
        ),
        EntryKind::Command => (
            "$ ",
            Style::default().fg(Color::Yellow),
            Style::default().fg(Color::Yellow),
        ),
        EntryKind::Patch => ("Δ ", Style::default().fg(Color::Blue), Style::default()),
        EntryKind::Mcp => ("@ ", Style::default().fg(Color::Magenta), Style::default()),
        EntryKind::Plan => (
            "• ",
            Style::default().fg(Color::Cyan),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        EntryKind::MultiAgent => (
            "& ",
            Style::default().fg(Color::LightCyan),
            Style::default(),
        ),
        EntryKind::Tool => ("• ", Style::default().fg(Color::Yellow), Style::default()),
        EntryKind::System => (
            "! ",
            Style::default().fg(Color::Red),
            Style::default().fg(Color::Red),
        ),
    }
}

fn continuation_style(kind: EntryKind, base_style: Style) -> Style {
    if kind == EntryKind::Command {
        Style::default().fg(Color::DarkGray)
    } else {
        base_style
    }
}

fn format_line(
    kind: EntryKind,
    prefix: &'static str,
    prefix_style: Style,
    base_style: Style,
    text: &str,
) -> Line<'static> {
    let text_style = match kind {
        EntryKind::Patch if text.starts_with('+') => Style::default().fg(Color::Green),
        EntryKind::Patch if text.starts_with('-') => Style::default().fg(Color::Red),
        EntryKind::Patch if text.starts_with("@@") => Style::default().fg(Color::Cyan),
        EntryKind::Plan if text.starts_with("[x]") => Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::CROSSED_OUT),
        EntryKind::Plan if text.starts_with("[~]") => Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
        EntryKind::Plan if text.starts_with("[ ]") => Style::default().fg(Color::DarkGray),
        _ => base_style,
    };
    Line::from(vec![
        Span::styled(prefix, prefix_style),
        Span::styled(text.to_string(), text_style),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(kind: EntryKind, text: &str) -> TranscriptEntry {
        TranscriptEntry {
            id: "entry-1".to_string(),
            kind,
            text: text.to_string(),
            streaming: false,
            status: (kind == EntryKind::Command).then_some(EntryStatus::Running),
            summary: Vec::new(),
        }
    }

    #[test]
    fn item_kinds_have_distinct_terminal_layouts() {
        let command = lines(&entry(EntryKind::Command, "cargo test\nfinished"));
        let patch = lines(&entry(EntryKind::Patch, "updated src/lib.rs\n+new"));
        let mcp = lines(&entry(EntryKind::Mcp, "server.tool [Completed]"));
        let plan = lines(&entry(EntryKind::Plan, "[~] run tests"));
        let multi_agent = lines(&entry(EntryKind::MultiAgent, "SpawnAgent [InProgress]"));

        assert_eq!(command[0].spans[0].content.as_ref(), "$ ");
        assert!(command[0].spans[1].content.contains("[running]"));
        assert_eq!(patch[0].spans[0].content.as_ref(), "Δ ");
        assert_eq!(mcp[0].spans[0].content.as_ref(), "@ ");
        assert_eq!(plan[0].spans[0].content.as_ref(), "• ");
        assert_eq!(multi_agent[0].spans[0].content.as_ref(), "& ");
        assert!(patch[1]
            .spans
            .iter()
            .any(|span| span.style.fg == Some(Color::Green)));
        assert_eq!(command[1].spans[1].style.fg, Some(Color::DarkGray));
    }

    #[test]
    fn multi_agent_titles_are_localized_at_the_render_boundary() {
        let entry = entry(EntryKind::MultiAgent, "Spawned agent-1");
        let rendered =
            hyperlink_lines_with_locale(&entry, Locale::ZhCn, Some(80), Path::new("/workspace"));
        let text = rendered
            .iter()
            .flat_map(|line| line.line.spans.iter())
            .map(|span| span.content.as_ref())
            .collect::<String>();
        assert!(text.contains("已启动 agent-1"));
        assert!(!text.contains("Spawned agent-1"));
    }

    #[test]
    fn image_view_paths_are_relative_to_the_current_working_directory() {
        let rendered = hyperlink_lines_with_locale(
            &entry(EntryKind::Tool, "view image: /workspace/assets/result.png"),
            Locale::EnUs,
            Some(80),
            Path::new("/workspace"),
        )
        .into_iter()
        .map(|line| line.line.to_string())
        .collect::<Vec<_>>()
        .join("\n");

        assert!(rendered.contains("view image: assets/result.png"));
        assert!(!rendered.contains("view image: /workspace/assets/result.png"));
    }

    #[test]
    fn user_image_summaries_use_numbered_labels_in_all_product_locales() {
        let mut entry = entry(EntryKind::User, "describe these");
        entry.summary = vec!["image: 1".to_string(), "image: 2".to_string()];

        for (locale, first, second) in [
            (Locale::ZhCn, "[图片 #1]", "[图片 #2]"),
            (Locale::ZhTw, "[圖片 #1]", "[圖片 #2]"),
            (Locale::EnUs, "[Image #1]", "[Image #2]"),
            (Locale::JaJp, "[画像 #1]", "[画像 #2]"),
            (Locale::KoKr, "[이미지 #1]", "[이미지 #2]"),
        ] {
            let rendered =
                hyperlink_lines_with_locale(&entry, locale, Some(80), Path::new("/workspace"))
                    .into_iter()
                    .map(|line| line.line.to_string())
                    .collect::<Vec<_>>()
                    .join("\n");
            assert!(rendered.contains(first), "missing {first}: {rendered}");
            assert!(rendered.contains(second), "missing {second}: {rendered}");
        }
    }

    #[test]
    fn plan_statuses_use_stable_checkbox_styles() {
        let completed = lines(&entry(EntryKind::Plan, "[x] inspect"));
        let running = lines(&entry(EntryKind::Plan, "[~] test"));
        let pending = lines(&entry(EntryKind::Plan, "[ ] ship"));

        assert!(completed[0].spans[1]
            .style
            .add_modifier
            .contains(Modifier::CROSSED_OUT));
        assert_eq!(running[0].spans[1].style.fg, Some(Color::Cyan));
        assert_eq!(pending[0].spans[1].style.fg, Some(Color::DarkGray));
    }

    #[test]
    fn assistant_markdown_is_rendered_at_the_transcript_boundary() {
        let rendered = lines(&entry(
            EntryKind::Assistant,
            "# Result\n\n**completed** with `cargo test`",
        ));
        let text = rendered
            .iter()
            .flat_map(|line| line.spans.iter())
            .map(|span| span.content.as_ref())
            .collect::<String>();
        assert!(text.contains("# Result"));
        assert!(text.contains("completed"));
        assert!(rendered.iter().any(|line| {
            line.spans
                .iter()
                .any(|span| span.style.add_modifier.contains(Modifier::BOLD))
        }));
        assert!(rendered.iter().any(|line| {
            line.spans
                .iter()
                .any(|span| span.style.fg == Some(Color::Cyan))
        }));
    }

    #[test]
    fn reasoning_markdown_keeps_reasoning_tone_and_inline_emphasis() {
        let rendered = lines(&entry(EntryKind::Reasoning, "**Inspecting** `src/lib.rs`"));

        assert!(rendered.iter().flat_map(|line| &line.spans).any(|span| {
            span.style.fg == Some(Color::DarkGray)
                && span.style.add_modifier.contains(Modifier::BOLD)
        }));
        assert!(rendered
            .iter()
            .flat_map(|line| &line.spans)
            .any(|span| span.content == "src/lib.rs" && span.style.fg == Some(Color::Cyan)));
    }
}
