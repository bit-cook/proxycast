//! Bounded, incremental command output retained by an active exec cell.

use std::collections::VecDeque;

use crate::locale::Locale;

const COMMAND_OUTPUT_HEAD_LINES: usize = 50;
const COMMAND_OUTPUT_TAIL_LINES: usize = 50;
pub(crate) const COMMAND_OUTPUT_MAX_LINE_BYTES: usize = 16 * 1024;

#[derive(Debug, Default)]
pub(crate) struct LiveCommandOutput {
    head: Vec<String>,
    tail: VecDeque<String>,
    total_lines: usize,
}

impl LiveCommandOutput {
    pub(crate) fn push_line(&mut self, line: &str, locale: Locale) {
        self.total_lines = self.total_lines.saturating_add(1);
        let line = truncate_command_output_line(line, locale);
        if self.head.len() < COMMAND_OUTPUT_HEAD_LINES {
            self.head.push(line);
        } else {
            if self.tail.len() == COMMAND_OUTPUT_TAIL_LINES {
                self.tail.pop_front();
            }
            self.tail.push_back(line);
        }
    }

    pub(crate) fn total_lines(&self) -> usize {
        self.total_lines
    }

    pub(crate) fn retained_lines(&self) -> usize {
        self.head.len().saturating_add(self.tail.len())
    }

    pub(crate) fn lines(&self) -> Vec<String> {
        let mut lines = self.head.clone();
        lines.extend(self.tail.iter().cloned());
        lines
    }

    pub(crate) fn transcript_lines(&self, locale: Locale) -> Vec<String> {
        let omitted = self
            .total_lines
            .saturating_sub(self.head.len().saturating_add(self.tail.len()));
        let mut lines = self.head.clone();
        if omitted > 0 {
            lines.push(locale.output_omitted_lines(omitted));
        }
        lines.extend(self.tail.iter().cloned());
        lines
    }
}

fn truncate_command_output_line(line: &str, locale: Locale) -> String {
    if line.len() <= COMMAND_OUTPUT_MAX_LINE_BYTES {
        return line.to_string();
    }

    let marker_budget = locale.output_omitted_bytes(line.len());
    let retained_budget = COMMAND_OUTPUT_MAX_LINE_BYTES.saturating_sub(marker_budget.len());
    let head_budget = retained_budget / 2;
    let tail_budget = retained_budget.saturating_sub(head_budget);
    let mut head_end = head_budget.min(line.len());
    while !line.is_char_boundary(head_end) {
        head_end = head_end.saturating_sub(1);
    }
    let mut tail_start = line.len().saturating_sub(tail_budget);
    while !line.is_char_boundary(tail_start) {
        tail_start = tail_start.saturating_add(1);
    }
    let omitted = tail_start.saturating_sub(head_end);

    format!(
        "{}{}{}",
        &line[..head_end],
        locale.output_omitted_bytes(omitted),
        &line[tail_start..]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_command_output_keeps_head_and_tail_with_omitted_line_marker() {
        let mut output = LiveCommandOutput::default();
        for index in 0..101 {
            output.push_line(&format!("line-{index}"), Locale::EnUs);
        }
        let lines = output.transcript_lines(Locale::EnUs);
        assert!(lines.iter().any(|line| line == "line-0"));
        assert!(lines.iter().any(|line| line == "line-49"));
        assert!(lines.iter().any(|line| line == "… 1 lines omitted …"));
        assert!(!lines.iter().any(|line| line == "line-50"));
        assert!(lines.iter().any(|line| line == "line-51"));
        assert!(lines.iter().any(|line| line == "line-100"));
    }

    #[test]
    fn live_command_output_long_line_preserves_utf8_head_and_tail() {
        let line = format!("{}尾", "界".repeat(COMMAND_OUTPUT_MAX_LINE_BYTES));
        let mut output = LiveCommandOutput::default();
        output.push_line(&line, Locale::EnUs);
        let rendered = output.lines().join("\n");
        assert!(rendered.contains("bytes omitted"));
        assert!(rendered.ends_with("尾"));
    }

    #[test]
    fn live_command_output_long_line_includes_marker_in_the_byte_budget() {
        let line = "界".repeat(COMMAND_OUTPUT_MAX_LINE_BYTES);
        let truncated = truncate_command_output_line(&line, Locale::EnUs);
        assert!(truncated.len() <= COMMAND_OUTPUT_MAX_LINE_BYTES);
        assert!(truncated.contains("bytes omitted"));
    }
}
