//! Command history cells backed by canonical command projection entries.

use super::*;
use app_server_protocol::protocol::v2::CommandAction;
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;

use crate::line_truncation::truncate_line_with_ellipsis_if_overflow;
use crate::projection::ActivityDetail;
use crate::style::{accent_style, failure_style, muted_style};
use crate::terminal_hyperlinks::prefix_hyperlink_lines;
use crate::wrapping::{adaptive_wrap_lines, RtOptions};

const DETAIL_PREVIEW_LINES: usize = 2;

pub(crate) fn exploration_compact_hyperlink_lines(
    entries: &[TranscriptEntry],
    locale: Locale,
    width: u16,
) -> Vec<HyperlinkLine> {
    let width = usize::from(width.max(1));
    let entries = entries
        .iter()
        .filter(|entry| exploration_detail(entry).is_some())
        .collect::<Vec<_>>();
    let active = entries.iter().any(|entry| {
        entry.streaming || entry.status == Some(crate::projection::EntryStatus::Running)
    });
    let failures = entries
        .iter()
        .filter(|entry| {
            exploration_detail(entry)
                .is_some_and(|(_, exit_code)| exit_code.is_some_and(|code| code != 0))
                || matches!(
                    entry.status,
                    Some(
                        crate::projection::EntryStatus::Failed
                            | crate::projection::EntryStatus::Declined
                            | crate::projection::EntryStatus::Interrupted
                    )
                )
        })
        .count();
    let mut header = Line::from(vec![
        Span::styled("•", muted_style()),
        Span::raw(" "),
        Span::styled(
            locale.transcript_exploration_label(active),
            Style::default().add_modifier(Modifier::BOLD),
        ),
    ]);
    if failures > 0 {
        header.push_span(Span::styled(
            format!(" · {}", locale.transcript_activity_failed_count(failures)),
            failure_style().add_modifier(Modifier::BOLD),
        ));
    }
    let mut lines = adaptive_wrap_lines(
        [header],
        RtOptions::new(width).subsequent_indent("  ".into()),
    )
    .into_iter()
    .map(HyperlinkLine::new)
    .collect::<Vec<_>>();

    let mut details = Vec::new();
    let detail_width = width.saturating_sub(4).max(1);
    let mut index = 0;
    while index < entries.len() {
        let Some((actions, exit_code)) = exploration_detail(entries[index]) else {
            unreachable!("entries were filtered to exploration details");
        };
        let reads_only = actions
            .iter()
            .all(|action| matches!(action, CommandAction::Read { .. }));
        if reads_only && exit_code.is_none_or(|code| code == 0) {
            let mut end = index + 1;
            while end < entries.len()
                && exploration_detail(entries[end]).is_some_and(|(next, next_exit)| {
                    next_exit.is_none_or(|code| code == 0)
                        && next
                            .iter()
                            .all(|action| matches!(action, CommandAction::Read { .. }))
                })
            {
                end += 1;
            }
            let mut names = Vec::new();
            for action in entries[index..end]
                .iter()
                .filter_map(|entry| exploration_detail(entry))
                .flat_map(|(actions, _)| actions)
            {
                if let CommandAction::Read { name, .. } = action {
                    if !names.contains(name) {
                        names.push(name.clone());
                    }
                }
            }
            details.extend(wrapped_activity_line(
                locale,
                locale.transcript_activity_action_label("read"),
                names.join(", "),
                None,
                false,
                false,
                detail_width,
            ));
            index = end;
            continue;
        }

        let has_search = actions
            .iter()
            .any(|action| matches!(action, CommandAction::Search { .. }));
        for (action_index, action) in actions.iter().enumerate() {
            let (label, text) = exploration_action(locale, action);
            let suffix = (action_index + 1 == actions.len())
                .then_some(exit_code)
                .flatten()
                .filter(|code| *code != 0);
            details.extend(wrapped_activity_line(
                locale,
                label,
                text,
                suffix,
                has_search,
                actions.len() > 1,
                detail_width,
            ));
        }
        index += 1;
    }
    details.truncate(DETAIL_PREVIEW_LINES);
    lines.extend(
        prefix_hyperlink_lines(
            details,
            Span::styled("  └ ", muted_style()),
            Span::raw("    "),
        )
        .into_iter()
        .map(|line| HyperlinkLine::new(truncate_line_with_ellipsis_if_overflow(line.line, width))),
    );
    lines
}

fn exploration_detail(entry: &TranscriptEntry) -> Option<(&[CommandAction], Option<i32>)> {
    let ActivityDetail::Exploration { actions, exit_code } = entry.activity_detail.as_ref()? else {
        return None;
    };
    Some((actions, *exit_code))
}

fn exploration_action(locale: Locale, action: &CommandAction) -> (&'static str, String) {
    match action {
        CommandAction::Read { name, .. } => (
            locale.transcript_activity_action_label("read"),
            name.clone(),
        ),
        CommandAction::ListFiles { command, path } => (
            locale.transcript_activity_action_label("list"),
            path.clone().unwrap_or_else(|| command.clone()),
        ),
        CommandAction::Search {
            command,
            query,
            path,
        } => (
            locale.transcript_activity_action_label("search"),
            match (query, path) {
                (Some(query), Some(path)) => locale.transcript_activity_search_target(query, path),
                (Some(query), None) => query.clone(),
                _ => command.clone(),
            },
        ),
        CommandAction::Unknown { command } => (
            locale.transcript_activity_action_label("run"),
            command.clone(),
        ),
    }
}

fn wrapped_activity_line(
    locale: Locale,
    label: &'static str,
    text: String,
    exit_code: Option<i32>,
    search: bool,
    compound: bool,
    width: usize,
) -> Vec<HyperlinkLine> {
    let mut spans = vec![Span::raw(text)];
    if let Some(code) = exit_code {
        let suffix = format!(" {}", locale.transcript_activity_exit(code, compound));
        let style = if search && code == 1 {
            muted_style()
        } else {
            failure_style()
        };
        spans.push(Span::styled(suffix, style));
    }
    let initial_indent = Line::from(vec![Span::styled(label, accent_style()), Span::raw(" ")]);
    let subsequent_indent = " ".repeat(initial_indent.width());
    adaptive_wrap_lines(
        [Line::from(spans)],
        RtOptions::new(width)
            .initial_indent(initial_indent)
            .subsequent_indent(subsequent_indent.into()),
    )
    .into_iter()
    .map(HyperlinkLine::new)
    .collect()
}

#[derive(Debug, Default, Clone)]
pub(crate) struct CommandOutput {
    pub(crate) exit_code: i32,
    pub(crate) aggregated_output: String,
}

impl CommandOutput {
    pub(crate) fn new(exit_code: i32, aggregated_output: impl Into<String>) -> Self {
        Self {
            exit_code,
            aggregated_output: aggregated_output.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ExecCall {
    pub(crate) call_id: String,
    pub(crate) command: String,
    pub(crate) output: Option<CommandOutput>,
    pub(crate) streaming: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct ExecCell {
    pub(crate) calls: Vec<ExecCall>,
}

impl ExecCell {
    pub(crate) fn new(call: ExecCall) -> Self {
        Self { calls: vec![call] }
    }

    pub(crate) fn add_call(&mut self, call: ExecCall) {
        self.calls.push(call);
    }

    pub(crate) fn iter_calls(&self) -> impl Iterator<Item = &ExecCall> {
        self.calls.iter()
    }

    fn entry(&self) -> TranscriptEntry {
        let text = self
            .calls
            .iter()
            .map(|call| {
                let mut text = call.command.clone();
                if let Some(output) = &call.output {
                    if !output.aggregated_output.is_empty() {
                        text.push('\n');
                        text.push_str(&output.aggregated_output);
                    }
                }
                text
            })
            .collect::<Vec<_>>()
            .join("\n");
        TranscriptEntry {
            id: self
                .calls
                .first()
                .map(|call| call.call_id.clone())
                .unwrap_or_default(),
            kind: EntryKind::Command,
            text,
            streaming: self.calls.iter().any(|call| call.streaming),
            status: None,
            summary: Vec::new(),
            activity_group: None,
            activity_detail: None,
        }
    }
}

impl HistoryCell for ExecCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        TranscriptHistoryCell::new(self.entry(), Locale::default(), std::path::PathBuf::new())
            .display_lines(width)
    }

    fn display_hyperlink_lines(&self, width: u16) -> Vec<HyperlinkLine> {
        TranscriptHistoryCell::new(self.entry(), Locale::default(), std::path::PathBuf::new())
            .display_hyperlink_lines(width)
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        TranscriptHistoryCell::new(self.entry(), Locale::default(), std::path::PathBuf::new())
            .raw_lines()
    }

    fn is_stream_continuation(&self) -> bool {
        self.calls.iter().any(|call| call.streaming)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection::{ActivityGroupKey, ActivityGroupKind, EntryKind, EntryStatus};

    fn exploration_entry(
        id: &str,
        actions: Vec<CommandAction>,
        exit_code: Option<i32>,
    ) -> TranscriptEntry {
        TranscriptEntry {
            id: id.to_string(),
            kind: EntryKind::Command,
            text: format!("command {id}"),
            streaming: exit_code.is_none(),
            status: Some(if exit_code.is_none() {
                EntryStatus::Running
            } else if exit_code == Some(0) {
                EntryStatus::Completed
            } else {
                EntryStatus::Failed
            }),
            summary: Vec::new(),
            activity_group: Some(ActivityGroupKey::new(
                ActivityGroupKind::Exploration,
                "turn-1",
            )),
            activity_detail: Some(ActivityDetail::Exploration { actions, exit_code }),
        }
    }

    #[test]
    fn compact_exploration_counts_failures_and_keeps_nonzero_read_visible() {
        let entries = [
            exploration_entry(
                "read-1",
                vec![CommandAction::Read {
                    command: "cat first.txt".to_string(),
                    name: "first.txt".to_string(),
                    path: "/workspace/first.txt".to_string(),
                }],
                Some(0),
            ),
            exploration_entry(
                "read-2",
                vec![CommandAction::Read {
                    command: "cat missing.txt".to_string(),
                    name: "missing.txt".to_string(),
                    path: "/workspace/missing.txt".to_string(),
                }],
                Some(1),
            ),
            exploration_entry(
                "search-1",
                vec![CommandAction::Search {
                    command: "rg absent .".to_string(),
                    query: Some("absent".to_string()),
                    path: Some(".".to_string()),
                }],
                Some(1),
            ),
        ];

        let rendered = exploration_compact_hyperlink_lines(&entries, Locale::EnUs, 80)
            .into_iter()
            .map(|line| line.line.to_string())
            .collect::<Vec<_>>();

        assert_eq!(rendered[0], "• Explored · 2 failed");
        assert_eq!(rendered[1], "  └ Read first.txt");
        assert_eq!(rendered[2], "    Read missing.txt (exit 1)");
    }

    #[test]
    fn search_exit_one_is_visible_without_failure_styling() {
        let no_matches = wrapped_activity_line(
            Locale::EnUs,
            "Search",
            "absent in .".to_string(),
            Some(1),
            true,
            false,
            80,
        );
        let error = wrapped_activity_line(
            Locale::EnUs,
            "Search",
            "text in missing.txt".to_string(),
            Some(2),
            true,
            false,
            80,
        );
        let no_matches_exit = no_matches[0]
            .line
            .spans
            .iter()
            .find(|span| span.content.contains("exit 1"))
            .expect("search exit 1 suffix");
        let error_exit = error[0]
            .line
            .spans
            .iter()
            .find(|span| span.content.contains("exit 2"))
            .expect("search exit 2 suffix");

        assert_eq!(no_matches_exit.style, muted_style());
        assert_eq!(error_exit.style, failure_style());
    }
}
