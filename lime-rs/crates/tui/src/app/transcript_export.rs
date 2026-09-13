//! Complete, Markdown-preserving conversation exports.
//!
//! Export consumes the same canonical `TranscriptEntry` projection rendered by the TUI. It does
//! not read a second history store or reconstruct runtime state locally.

use std::io::Write;
use std::path::{Path, PathBuf};

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, FrameExt as _, Paragraph};
use ratatui::Frame;

use crate::bottom_pane::{TextArea, TextAreaState};
use crate::history_cell::sanitize_user_text;
use crate::line_truncation::truncate_line_with_ellipsis_if_overflow;
use crate::locale::Locale;
use crate::projection::{EntryKind, TranscriptEntry};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExportPickerAction {
    None,
    Cancel,
    Copy,
    Save,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExportPickerMode {
    Destination,
    Filename,
}

/// Codex-shaped destination and filename prompt for `/export`.
///
/// The picker owns only terminal input and selection state. Export rendering and persistence stay
/// on the canonical transcript path below, so choosing a destination cannot create a second
/// history or projection store.
#[derive(Debug)]
pub(crate) struct ExportPicker {
    mode: ExportPickerMode,
    selected: usize,
    filename: TextArea,
    filename_state: TextAreaState,
}

impl ExportPicker {
    pub(crate) fn new(thread_id: Option<&str>) -> Self {
        let filename = thread_id
            .map(|thread_id| format!("codex-session-{thread_id}.md"))
            .unwrap_or_else(|| "codex-session.md".to_string());
        let mut textarea = TextArea::new();
        textarea.replace(filename);
        Self {
            mode: ExportPickerMode::Destination,
            selected: 0,
            filename: textarea,
            filename_state: TextAreaState::default(),
        }
    }

    pub(crate) fn is_filename_prompt(&self) -> bool {
        matches!(self.mode, ExportPickerMode::Filename)
    }

    pub(crate) fn filename(&self) -> &TextArea {
        &self.filename
    }

    pub(crate) fn filename_state(&self) -> TextAreaState {
        self.filename_state
    }

    pub(crate) fn handle_event(&mut self, event: &Event) -> ExportPickerAction {
        match self.mode {
            ExportPickerMode::Destination => self.handle_destination_event(event),
            ExportPickerMode::Filename => self.handle_filename_event(event),
        }
    }

    fn handle_destination_event(&mut self, event: &Event) -> ExportPickerAction {
        let Event::Key(key) = event else {
            return ExportPickerAction::None;
        };
        if key.kind != KeyEventKind::Press {
            return ExportPickerAction::None;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('c' | 'd'))
        {
            return ExportPickerAction::Cancel;
        }
        match key.code {
            KeyCode::Esc => ExportPickerAction::Cancel,
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected = self.selected.saturating_sub(1);
                ExportPickerAction::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.selected = (self.selected + 1).min(1);
                ExportPickerAction::None
            }
            KeyCode::Enter => {
                if self.selected == 0 {
                    ExportPickerAction::Copy
                } else {
                    self.mode = ExportPickerMode::Filename;
                    ExportPickerAction::None
                }
            }
            _ => ExportPickerAction::None,
        }
    }

    fn handle_filename_event(&mut self, event: &Event) -> ExportPickerAction {
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && matches!(key.code, KeyCode::Char('c' | 'd'))
                {
                    return ExportPickerAction::Cancel;
                }
                match key.code {
                    KeyCode::Esc => {
                        self.mode = ExportPickerMode::Destination;
                        ExportPickerAction::None
                    }
                    KeyCode::Enter => {
                        if self.filename.text().trim().is_empty() {
                            ExportPickerAction::None
                        } else {
                            ExportPickerAction::Save
                        }
                    }
                    _ => {
                        self.filename.input(*key);
                        ExportPickerAction::None
                    }
                }
            }
            Event::Paste(text) => {
                let text = text.replace(['\r', '\n'], "");
                self.filename.insert(&text);
                ExportPickerAction::None
            }
            _ => ExportPickerAction::None,
        }
    }

    pub(crate) fn selected_path(&self) -> Option<PathBuf> {
        if self.filename.text().trim().is_empty() {
            None
        } else {
            Some(PathBuf::from(self.filename.text().trim()))
        }
    }
}

pub(crate) fn render_picker(
    frame: &mut Frame<'_>,
    area: Rect,
    picker: &ExportPicker,
    locale: Locale,
) {
    let width = area.width.saturating_sub(4).min(88);
    let height = if picker.is_filename_prompt() { 5 } else { 6 };
    let height = height.min(area.height);
    if width == 0 || height == 0 {
        return;
    }
    let popup = Rect::new(
        area.x.saturating_add(area.width.saturating_sub(width) / 2),
        area.y
            .saturating_add(area.height.saturating_sub(height) / 2),
        width,
        height,
    );
    frame.render_widget(Clear, popup);
    if picker.is_filename_prompt() {
        let title = Line::from(vec![
            Span::styled("▌ ", Style::default().fg(Color::Cyan)),
            Span::styled(
                locale.export_prompt_title(),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]);
        frame.render_widget(
            Paragraph::new(title),
            Rect::new(popup.x, popup.y, popup.width, 1),
        );
        frame.render_widget(
            Paragraph::new(Line::from("▌")),
            Rect::new(popup.x, popup.y.saturating_add(1), popup.width, 1),
        );
        let input_y = popup.y.saturating_add(2);
        frame.render_widget(
            Paragraph::new(Line::from("▌ ")),
            Rect::new(popup.x, input_y, 2.min(popup.width), 1),
        );
        let input = Rect::new(
            popup.x.saturating_add(2),
            input_y,
            popup.width.saturating_sub(2),
            1,
        );
        let mut state = picker.filename_state();
        frame.render_stateful_widget_ref(picker.filename(), input, &mut state);
        if let Some((x, y)) = picker.filename().cursor_pos_with_state(input, state) {
            frame.set_cursor_position(Position::new(x, y));
        }
        frame.render_widget(
            Paragraph::new(truncate_line_with_ellipsis_if_overflow(
                Line::styled(
                    locale.export_prompt_hint(),
                    Style::default().fg(Color::DarkGray),
                ),
                usize::from(popup.width),
            )),
            Rect::new(popup.x, popup.y.saturating_add(3), popup.width, 1),
        );
        return;
    }

    let lines = [
        Line::styled(
            locale.export_title(),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Line::styled(
            locale.export_subtitle(),
            Style::default().fg(Color::DarkGray),
        ),
        export_option_line(
            0,
            picker.selected == 0,
            locale.export_copy_label(),
            locale.export_copy_description(),
        ),
        export_option_line(
            1,
            picker.selected == 1,
            locale.export_file_label(),
            locale.export_file_description(),
        ),
        Line::from(""),
        Line::styled(
            locale.export_picker_hint(),
            Style::default().fg(Color::DarkGray),
        ),
    ];
    let lines = lines
        .into_iter()
        .map(|line| truncate_line_with_ellipsis_if_overflow(line, usize::from(popup.width)))
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines), popup);
}

fn export_option_line(
    index: usize,
    selected: bool,
    label: &str,
    description: &str,
) -> Line<'static> {
    let style = if selected {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let marker = if selected { '›' } else { ' ' };
    Line::from(vec![
        Span::styled(format!("{marker} {}. ", index + 1), style),
        Span::styled(label.to_string(), style),
        Span::raw("  "),
        Span::styled(
            description.to_string(),
            Style::default().add_modifier(Modifier::DIM),
        ),
    ])
}

pub(crate) fn render_markdown_transcript(entries: &[TranscriptEntry]) -> Result<String, String> {
    let mut markdown = String::from("# Lime conversation\n");
    for entry in entries {
        let image_labels = if entry.kind == EntryKind::User {
            entry
                .summary
                .iter()
                .filter_map(|detail| detail.strip_prefix("image: "))
                .filter(|index| !index.is_empty())
                .map(|index| Locale::EnUs.numbered_image_label(index))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        if entry.text.trim().is_empty() && image_labels.is_empty() && entry.summary.is_empty() {
            continue;
        }
        let heading = match entry.kind {
            EntryKind::User => "User",
            EntryKind::Assistant => "Assistant",
            EntryKind::Reasoning => "Reasoning",
            EntryKind::Plan => "Plan",
            EntryKind::Command
            | EntryKind::Patch
            | EntryKind::Mcp
            | EntryKind::MultiAgent
            | EntryKind::Tool
            | EntryKind::System => "Activity",
        };
        markdown.push_str(&format!("\n## {heading}\n\n"));
        let text = sanitize_user_text(entry.text.as_str().into());
        if matches!(
            entry.kind,
            EntryKind::Command | EntryKind::Patch | EntryKind::Mcp
        ) {
            if let Some(header) = patch_export_header(entry) {
                markdown.push_str("    ");
                markdown.push_str(&header);
                markdown.push('\n');
            }
            for line in text.lines() {
                markdown.push_str("    ");
                markdown.push_str(line);
                markdown.push('\n');
            }
        } else if !text.trim().is_empty() {
            markdown.push_str(&text);
            if !text.ends_with('\n') {
                markdown.push('\n');
            }
        }
        if !image_labels.is_empty() {
            if !text.trim().is_empty() {
                markdown.push('\n');
            }
            for label in image_labels {
                markdown.push_str(&label);
                markdown.push('\n');
            }
        }
        for detail in entry
            .summary
            .iter()
            .filter(|detail| entry.kind != EntryKind::User || !detail.starts_with("image: "))
        {
            markdown.push_str("    - ");
            markdown.push_str(detail);
            markdown.push('\n');
        }
    }
    if markdown == "# Lime conversation\n" {
        Err("No conversation content to export.".to_string())
    } else {
        Ok(markdown)
    }
}

fn patch_export_header(entry: &TranscriptEntry) -> Option<String> {
    if entry.kind != EntryKind::Patch {
        return None;
    }
    let status = entry.status?;
    let file_count = entry.summary.iter().find_map(|detail| {
        detail
            .strip_prefix("files: ")
            .and_then(|value| value.trim().parse::<usize>().ok())
    });
    Some(match file_count {
        Some(file_count) => {
            format!("file changes: {} · {file_count} changes", status.label())
        }
        None => format!("file changes: {}", status.label()),
    })
}

pub(crate) fn write_transcript(
    cwd: &Path,
    requested_path: &Path,
    markdown: &str,
) -> Result<PathBuf, String> {
    let path = if let Ok(relative) = requested_path.strip_prefix("~") {
        home_dir()
            .ok_or_else(|| "could not determine the home directory".to_string())?
            .join(relative)
    } else if requested_path.is_absolute() {
        requested_path.to_path_buf()
    } else {
        cwd.join(requested_path)
    };
    let mut file = tempfile::NamedTempFile::new_in(path.parent().unwrap_or(cwd))
        .map_err(|error| format!("could not create {}: {error}", path.display()))?;
    file.write_all(markdown.as_bytes())
        .map_err(|error| format!("could not write {}: {error}", path.display()))?;
    file.persist_noclobber(&path)
        .map_err(|error| format!("could not create {}: {error}", path.display()))?;
    Ok(path)
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection::EntryStatus;
    use crossterm::event::{KeyEvent, KeyModifiers};

    fn entry(kind: EntryKind, text: &str) -> TranscriptEntry {
        TranscriptEntry {
            id: format!("{kind:?}"),
            kind,
            text: text.to_string(),
            streaming: false,
            status: None,
            summary: Vec::new(),
        }
    }

    #[test]
    fn export_picker_uses_thread_id_filename_and_supports_both_destinations() {
        let mut picker = ExportPicker::new(Some("thread-123"));
        assert_eq!(picker.filename().text(), "codex-session-thread-123.md");
        assert_eq!(
            picker.handle_event(&Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            ))),
            ExportPickerAction::Copy
        );

        let mut picker = ExportPicker::new(Some("thread-123"));
        assert_eq!(
            picker.handle_event(&Event::Key(KeyEvent::new(
                KeyCode::Down,
                KeyModifiers::NONE,
            ))),
            ExportPickerAction::None
        );
        assert_eq!(
            picker.handle_event(&Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            ))),
            ExportPickerAction::None
        );
        assert!(picker.is_filename_prompt());
        assert_eq!(
            picker.handle_event(&Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            ))),
            ExportPickerAction::Save
        );
        assert_eq!(
            picker.selected_path(),
            Some(PathBuf::from("codex-session-thread-123.md"))
        );
    }

    #[test]
    fn export_picker_escape_returns_to_destination_then_cancels() {
        let mut picker = ExportPicker::new(None);
        picker.handle_event(&Event::Key(KeyEvent::new(
            KeyCode::Down,
            KeyModifiers::NONE,
        )));
        picker.handle_event(&Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )));
        assert!(picker.is_filename_prompt());
        assert_eq!(
            picker.handle_event(&Event::Key(
                KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE,)
            )),
            ExportPickerAction::None
        );
        assert!(!picker.is_filename_prompt());
        assert_eq!(
            picker.handle_event(&Event::Key(
                KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE,)
            )),
            ExportPickerAction::Cancel
        );
    }

    #[test]
    fn export_picker_filename_paste_drops_line_breaks() {
        let mut picker = ExportPicker::new(None);
        picker.handle_event(&Event::Key(KeyEvent::new(
            KeyCode::Down,
            KeyModifiers::NONE,
        )));
        picker.handle_event(&Event::Key(KeyEvent::new(
            KeyCode::Enter,
            KeyModifiers::NONE,
        )));
        picker.filename.replace("report".to_string());
        picker.handle_event(&Event::Paste(".md\r\n".to_string()));
        assert_eq!(picker.filename().text(), "report.md");
    }

    #[test]
    fn markdown_transcript_preserves_messages_and_formats_activity() {
        let markdown = render_markdown_transcript(&[
            entry(EntryKind::User, "Explain the change"),
            entry(EntryKind::Assistant, "**done** with `cargo test`"),
            entry(EntryKind::Command, "$ cargo test\nok"),
        ])
        .expect("exported transcript");

        assert!(markdown.starts_with("# Lime conversation\n"));
        assert!(markdown.contains("## User"));
        assert!(markdown.contains("## Assistant"));
        assert!(markdown.contains("    $ cargo test"));
        assert!(markdown.contains("    ok"));
    }

    #[test]
    fn markdown_transcript_preserves_numbered_images_without_internal_tokens() {
        let mut with_text = entry(EntryKind::User, "describe these");
        with_text.summary = vec!["image: 1".to_string(), "image: 2".to_string()];
        let mut image_only = entry(EntryKind::User, "");
        image_only.summary = vec!["image: 1".to_string()];

        let markdown = render_markdown_transcript(&[with_text, image_only])
            .expect("exported image transcript");

        assert!(markdown.contains("describe these\n\n[Image #1]\n[Image #2]"));
        assert!(markdown.contains("## User\n\n[Image #1]"));
        assert!(!markdown.contains("image: 1"));
    }

    #[test]
    fn empty_transcript_is_rejected() {
        assert!(render_markdown_transcript(&[]).is_err());
        assert!(render_markdown_transcript(&[entry(EntryKind::Assistant, "  \n")]).is_err());
    }

    #[test]
    fn transcript_file_resolves_relative_paths_and_refuses_to_overwrite() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = write_transcript(
            directory.path(),
            Path::new("conversation.md"),
            "# First export\n",
        )
        .expect("write first export");

        assert_eq!(path, directory.path().join("conversation.md"));
        assert!(write_transcript(directory.path(), &path, "# Second export\n").is_err());
        assert_eq!(
            std::fs::read_to_string(&path).expect("read"),
            "# First export\n"
        );
    }

    #[test]
    fn markdown_transcript_includes_activity_details_without_terminal_controls() {
        let mut file = entry(EntryKind::Patch, "\x1b[31m+src/lib.rs\x1b[0m");
        file.summary = vec!["file changes: completed · 1 changes".to_string()];
        let mut mcp = entry(EntryKind::Mcp, "\x1b[32mmatching docs\x1b[0m");
        mcp.summary = vec![
            "mcp tool: docs/search({\"query\":\"export\"}) · completed".to_string(),
            "<image content>".to_string(),
            "embedded resource: file:///report.md".to_string(),
            "structured result: {\"count\":1}".to_string(),
        ];
        let markdown = render_markdown_transcript(&[file, mcp]).expect("exported transcript");

        assert!(markdown.contains("+src/lib.rs"));
        assert!(markdown.contains("matching docs"));
        assert!(markdown.contains("mcp tool: docs/search"));
        assert!(markdown.contains("<image content>"));
        assert!(markdown.contains("embedded resource: file:///report.md"));
        assert!(!markdown.contains('\x1b'));
    }

    #[test]
    fn markdown_transcript_exports_patch_status_and_file_count() {
        let mut patch = entry(EntryKind::Patch, "updated src/lib.rs\n+new");
        patch.status = Some(EntryStatus::Completed);
        patch.summary = vec!["files: 1".to_string(), "updated: 1".to_string()];

        let markdown = render_markdown_transcript(&[patch]).expect("exported patch");

        assert!(markdown.contains("file changes: completed · 1 changes"));
        assert!(markdown.contains("updated src/lib.rs"));
        assert!(markdown.contains("    - files: 1"));
    }

    #[test]
    fn markdown_transcript_keeps_summary_only_activity() {
        let mut patch = entry(EntryKind::Patch, "");
        patch.status = Some(EntryStatus::Failed);
        patch.summary = vec!["files: 0".to_string()];

        let markdown = render_markdown_transcript(&[patch]).expect("summary-only patch");

        assert!(markdown.contains("file changes: failed · 0 changes"));
        assert!(markdown.contains("    - files: 0"));
    }
}
