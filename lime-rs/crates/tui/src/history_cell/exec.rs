//! Command history cells backed by canonical command projection entries.

use super::*;

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
