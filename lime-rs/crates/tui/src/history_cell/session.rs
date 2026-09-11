//! Session header history cells.

use super::*;

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
    pub(crate) directory: std::path::PathBuf,
}

impl SessionHeaderHistoryCell {
    pub(crate) fn new(
        model: impl Into<String>,
        directory: std::path::PathBuf,
        version: impl Into<String>,
    ) -> Self {
        Self {
            version: version.into(),
            model: model.into(),
            directory,
        }
    }
}

impl HistoryCell for SessionHeaderHistoryCell {
    fn display_lines(&self, _width: u16) -> Vec<Line<'static>> {
        vec![
            Line::from(format!("Lime {}", self.version)),
            Line::from(format!("model: {}", self.model)),
            Line::from(format!("directory: {}", self.directory.display())),
        ]
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        self.display_lines(u16::MAX)
    }
}
