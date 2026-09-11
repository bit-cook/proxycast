//! MCP tool-call history cells.

use super::*;

#[derive(Debug, Clone)]
pub(crate) struct McpInvocation {
    pub(crate) server: String,
    pub(crate) tool: String,
    pub(crate) arguments: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub(crate) struct McpToolCallCell {
    pub(crate) call_id: String,
    pub(crate) invocation: McpInvocation,
    pub(crate) result: Option<String>,
    pub(crate) failed: bool,
}

impl McpToolCallCell {
    pub(crate) fn new(call_id: String, invocation: McpInvocation) -> Self {
        Self {
            call_id,
            invocation,
            result: None,
            failed: false,
        }
    }
}

impl HistoryCell for McpToolCallCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        self.display_hyperlink_lines(width)
            .into_iter()
            .map(|line| line.line)
            .collect()
    }

    fn display_hyperlink_lines(&self, width: u16) -> Vec<HyperlinkLine> {
        let summary = self.result.clone().into_iter().collect();
        TranscriptHistoryCell::new(
            TranscriptEntry {
                id: self.call_id.clone(),
                kind: EntryKind::Mcp,
                text: format!("{}.{}", self.invocation.server, self.invocation.tool),
                streaming: self.result.is_none() && !self.failed,
                status: None,
                summary,
            },
            Locale::default(),
            std::path::PathBuf::new(),
        )
        .display_hyperlink_lines(width)
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        self.display_lines(u16::MAX)
    }
}

#[derive(Debug, Default)]
pub(crate) struct McpInventoryLoadingCell;

impl HistoryCell for McpInventoryLoadingCell {
    fn display_lines(&self, _width: u16) -> Vec<Line<'static>> {
        vec![Line::from("Loading MCP inventory")]
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        self.display_lines(u16::MAX)
    }
}
