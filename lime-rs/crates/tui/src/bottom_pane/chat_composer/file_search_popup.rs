use crate::locale::Locale;
use app_server_protocol::protocol::v2::FuzzyFileSearchResult;
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;

const MAX_ROWS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileSearchPopupAction {
    Pass,
    Consumed,
    Cancel,
    Complete,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct FileSearchPopup {
    query: String,
    matches: Vec<FuzzyFileSearchResult>,
    selected: usize,
    waiting: bool,
}

impl FileSearchPopup {
    pub(crate) fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            waiting: true,
            ..Self::default()
        }
    }

    pub(crate) fn set_query(&mut self, query: impl Into<String>) {
        let query = query.into();
        if self.query != query {
            self.query = query;
            // Keep the previous rows visible until the replacement query resolves.
            self.selected = self.selected.min(self.matches.len().saturating_sub(1));
            self.waiting = true;
        }
    }

    /// Show the idle state for a bare `@` token without starting a filesystem search.
    pub(crate) fn set_empty_prompt(&mut self) {
        self.query.clear();
        self.matches.clear();
        self.selected = 0;
        self.waiting = false;
    }

    pub(crate) fn query(&self) -> &str {
        &self.query
    }

    pub(crate) fn set_matches(&mut self, query: &str, matches: Vec<FuzzyFileSearchResult>) {
        if self.query != query {
            return;
        }
        self.matches = matches.into_iter().take(MAX_ROWS).collect();
        self.selected = self.selected.min(self.matches.len().saturating_sub(1));
        self.waiting = false;
    }

    pub(crate) fn selected_path(&self) -> Option<&str> {
        self.matches
            .get(self.selected)
            .map(|item| item.path.as_str())
    }

    pub(crate) fn handle_event(&mut self, event: &Event) -> FileSearchPopupAction {
        let Event::Key(key) = event else {
            return FileSearchPopupAction::Pass;
        };
        if key.kind != KeyEventKind::Press {
            return FileSearchPopupAction::Pass;
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('p')
                if key.code == KeyCode::Up || key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                if !self.matches.is_empty() {
                    self.selected = self
                        .selected
                        .checked_sub(1)
                        .unwrap_or(self.matches.len() - 1);
                }
                FileSearchPopupAction::Consumed
            }
            KeyCode::Down | KeyCode::Char('n')
                if key.code == KeyCode::Down || key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                if !self.matches.is_empty() {
                    self.selected = (self.selected + 1) % self.matches.len();
                }
                FileSearchPopupAction::Consumed
            }
            KeyCode::Esc => FileSearchPopupAction::Cancel,
            KeyCode::Enter | KeyCode::Tab if self.selected_path().is_some() => {
                FileSearchPopupAction::Complete
            }
            KeyCode::Enter | KeyCode::Tab => FileSearchPopupAction::Consumed,
            _ => FileSearchPopupAction::Pass,
        }
    }

    pub(crate) fn render(&self, frame: &mut Frame<'_>, composer_area: Rect, locale: Locale) {
        if composer_area.y == 0 || composer_area.width == 0 {
            return;
        }
        let rows = self.matches.len().clamp(1, MAX_ROWS);
        let height = u16::try_from(rows).unwrap_or(u16::MAX).min(composer_area.y);
        let area = Rect::new(
            composer_area.x,
            composer_area.y.saturating_sub(height),
            composer_area.width,
            height,
        );
        let lines = if self.matches.is_empty() {
            let message = if self.waiting {
                format!("  {}", locale.file_search_loading())
            } else {
                format!("  {}", locale.file_search_no_matches())
            };
            vec![Line::styled(
                message,
                Style::default().add_modifier(Modifier::DIM),
            )]
        } else {
            self.matches
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    let selected = index == self.selected;
                    let style = if selected {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    Line::from(vec![
                        Span::styled(if selected { "> " } else { "  " }, style),
                        Span::styled(item.path.clone(), style),
                    ])
                })
                .collect()
        };
        frame.render_widget(Clear, area);
        frame.render_widget(Paragraph::new(lines), area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_server_protocol::protocol::v2::{FuzzyFileSearchMatchType, FuzzyFileSearchResult};
    use crossterm::event::{KeyEvent, KeyModifiers};

    fn result(path: &str) -> FuzzyFileSearchResult {
        FuzzyFileSearchResult {
            root: "/tmp".to_string(),
            path: path.to_string(),
            match_type: FuzzyFileSearchMatchType::File,
            file_name: path.to_string(),
            score: 1,
            indices: None,
        }
    }

    #[test]
    fn stale_results_are_ignored_and_selection_wraps() {
        let mut popup = FileSearchPopup::new("src");
        popup.set_matches("old", vec![result("old.rs")]);
        assert!(popup.selected_path().is_none());
        popup.set_matches("src", vec![result("src/lib.rs"), result("src/main.rs")]);
        popup.handle_event(&Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)));
        assert_eq!(popup.selected_path(), Some("src/main.rs"));
    }

    #[test]
    fn ctrl_p_and_ctrl_n_cycle_selection_like_codex() {
        let mut popup = FileSearchPopup::new("src");
        popup.set_matches("src", vec![result("src/lib.rs"), result("src/main.rs")]);

        popup.handle_event(&Event::Key(KeyEvent::new(
            KeyCode::Char('n'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(popup.selected_path(), Some("src/main.rs"));

        popup.handle_event(&Event::Key(KeyEvent::new(
            KeyCode::Char('p'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(popup.selected_path(), Some("src/lib.rs"));
    }

    #[test]
    fn query_changes_keep_previous_rows_visible_while_waiting() {
        let mut popup = FileSearchPopup::new("src");
        popup.set_matches("src", vec![result("src/lib.rs")]);
        popup.set_query("main");

        assert!(popup.waiting);
        assert_eq!(popup.selected_path(), Some("src/lib.rs"));
        popup.set_matches("main", Vec::new());
        assert!(popup.selected_path().is_none());
    }
}
