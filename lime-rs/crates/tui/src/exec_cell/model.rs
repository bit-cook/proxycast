//! Data model for bounded command output previews.

use super::live_output::LiveCommandOutput;
use crate::locale::Locale;

#[derive(Debug, Default)]
pub(crate) struct CommandOutput {
    pub(crate) exit_code: i32,
    aggregated_output: String,
    live_output: Option<LiveCommandOutput>,
}

impl CommandOutput {
    pub(crate) fn new(exit_code: i32, aggregated_output: String) -> Self {
        Self {
            exit_code,
            aggregated_output,
            live_output: None,
        }
    }

    pub(crate) fn from_lines<'a>(source: impl Iterator<Item = &'a str>, locale: Locale) -> Self {
        let mut output = Self::default();
        let live_output = output
            .live_output
            .get_or_insert_with(LiveCommandOutput::default);
        for line in source {
            live_output.push_line(line, locale);
        }
        output
    }

    pub(crate) fn line_counts(&self) -> (usize, usize) {
        match self.live_output.as_ref() {
            Some(output) => (output.total_lines(), output.retained_lines()),
            None => {
                let total = self.aggregated_output.lines().count();
                (total, total)
            }
        }
    }

    pub(crate) fn lines(&self) -> Vec<String> {
        self.live_output
            .as_ref()
            .map(LiveCommandOutput::lines)
            .unwrap_or_else(|| self.aggregated_output.lines().map(str::to_owned).collect())
    }

    pub(crate) fn transcript_lines(&self, locale: Locale) -> Vec<String> {
        self.live_output
            .as_ref()
            .map(|output| output.transcript_lines(locale))
            .unwrap_or_else(|| self.aggregated_output.lines().map(str::to_owned).collect())
    }
}
