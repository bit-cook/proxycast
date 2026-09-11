//! Completed request-user-input transcript rendering.

use std::collections::BTreeMap;

use app_server_protocol::protocol::v2::{ToolRequestUserInputAnswer, ToolRequestUserInputQuestion};

use super::*;

#[derive(Debug, Clone)]
pub(crate) struct RequestUserInputResultCell {
    pub(crate) questions: Vec<ToolRequestUserInputQuestion>,
    pub(crate) answers: BTreeMap<String, ToolRequestUserInputAnswer>,
    pub(crate) interrupted: bool,
}

impl HistoryCell for RequestUserInputResultCell {
    fn display_lines(&self, _width: u16) -> Vec<Line<'static>> {
        let answered = self
            .questions
            .iter()
            .filter(|question| self.answers.contains_key(&question.id))
            .count();
        let suffix = if self.interrupted {
            " (interrupted)"
        } else {
            ""
        };
        let mut lines = vec![Line::from(format!(
            "Questions {answered}/{} answered{suffix}",
            self.questions.len()
        ))];
        lines.extend(self.questions.iter().map(|question| {
            let answer = self
                .answers
                .get(&question.id)
                .map(|answer| answer.answers.join(", "))
                .unwrap_or_else(|| "(unanswered)".to_string());
            Line::from(format!("{}: {answer}", question.question))
        }));
        lines
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        self.display_lines(u16::MAX)
    }
}
