//! Turn separators for transcript history.

use super::*;

#[derive(Debug, Clone, Copy)]
pub struct FinalMessageSeparator {
    elapsed_seconds: Option<u64>,
}

impl FinalMessageSeparator {
    pub(crate) fn new(elapsed_seconds: Option<u64>) -> Self {
        Self { elapsed_seconds }
    }
}

impl HistoryCell for FinalMessageSeparator {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        let label = self
            .elapsed_seconds
            .filter(|seconds| *seconds > 60)
            .map(|seconds| format!(" Worked for {seconds}s "));
        let line = match label {
            Some(label) if label.len() < usize::from(width) => format!(
                "-{label}{}",
                "-".repeat(usize::from(width).saturating_sub(label.len() + 1))
            ),
            _ => "-".repeat(usize::from(width)),
        };
        vec![Line::from(line)]
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        self.elapsed_seconds
            .map(|seconds| vec![Line::from(format!("Worked for {seconds}s"))])
            .unwrap_or_default()
    }
}
