//! Query state and literal search motions for the TextArea Vim owner.

use super::super::TextArea;
use super::vim::{VimMode, VimOperator, VimPending};
use crate::vim_search::{matching_ranges, SearchDirection, SearchQuery};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Default)]
pub(super) struct VimSearch {
    input: Option<Box<SearchInput>>,
    last: SearchQuery,
}

#[derive(Debug)]
struct SearchInput {
    editor: TextArea,
    direction: SearchDirection,
}

impl VimSearch {
    pub(super) fn cancel(&mut self) {
        self.input = None;
    }
}

impl TextArea {
    pub(crate) fn vim_search_query(&self) -> Option<(&str, SearchDirection)> {
        self.vim_search
            .input
            .as_deref()
            .map(|input| (input.editor.text(), input.direction))
    }

    pub(crate) fn insert_vim_search_text(&mut self, text: &str) -> bool {
        let Some(input) = self.vim_search.input.as_deref_mut() else {
            return false;
        };
        input.editor.insert_str(text);
        true
    }

    pub(crate) fn wants_vim_search_key(&self, event: KeyEvent) -> bool {
        self.vim_search.input.is_some() || self.search_command(event).is_some()
    }

    fn search_command(&self, event: KeyEvent) -> Option<SearchCommand> {
        if !self.vim_enabled
            || self.vim_mode != VimMode::Normal
            || !matches!(event.kind, KeyEventKind::Press | KeyEventKind::Repeat)
            || event.modifiers != KeyModifiers::NONE
            || matches!(self.vim_pending, VimPending::ReplaceChar)
        {
            return None;
        }
        match event.code {
            KeyCode::Char('/') => Some(SearchCommand::Start(SearchDirection::Forward)),
            KeyCode::Char('?') => Some(SearchCommand::Start(SearchDirection::Backward)),
            KeyCode::Char('n') => Some(SearchCommand::Next),
            KeyCode::Char('N') => Some(SearchCommand::Previous),
            _ => None,
        }
    }

    pub(super) fn handle_vim_search_key(&mut self, event: KeyEvent) -> bool {
        if let Some(mut input) = self.vim_search.input.take() {
            if event.code == KeyCode::Esc
                || (event.code == KeyCode::Char('c') && event.modifiers == KeyModifiers::CONTROL)
            {
                self.vim_pending = VimPending::None;
                return true;
            }
            if event.code == KeyCode::Backspace && input.editor.is_empty() {
                self.vim_pending = VimPending::None;
                return true;
            }
            if event.code != KeyCode::Enter || event.modifiers != KeyModifiers::NONE {
                input.editor.input(event);
                self.vim_search.input = Some(input);
                return true;
            }

            let text = input.editor.text().to_owned();
            if text.is_empty() && self.vim_search.last.text.is_empty() {
                self.vim_pending = VimPending::None;
                return true;
            }
            let query = SearchQuery {
                text: if text.is_empty() {
                    self.vim_search.last.text.clone()
                } else {
                    text
                },
                direction: input.direction,
            };
            self.vim_search.last = query.clone();
            self.apply_search_with_pending(query);
            return true;
        }

        let Some(command) = self.search_command(event) else {
            return false;
        };
        match command {
            SearchCommand::Start(direction) => {
                self.vim_search.input = Some(Box::new(SearchInput {
                    editor: TextArea::new(),
                    direction,
                }));
            }
            SearchCommand::Next | SearchCommand::Previous => {
                if self.vim_search.last.text.is_empty() {
                    return true;
                }
                let mut query = self.vim_search.last.clone();
                if matches!(command, SearchCommand::Previous) {
                    query.direction = query.direction.reversed();
                }
                self.apply_search_with_pending(query);
            }
        }
        true
    }

    fn apply_search_with_pending(&mut self, query: SearchQuery) {
        let pending = std::mem::replace(&mut self.vim_pending, VimPending::None);
        let operator = match pending {
            VimPending::Operator(operator) => Some(operator),
            _ => None,
        };
        self.apply_vim_search(&query, operator);
    }

    fn apply_vim_search(&mut self, query: &SearchQuery, operator: Option<VimOperator>) -> bool {
        let origin = self.cursor;
        let target = matching_ranges(&self.text, &query.text)
            .map(|range| range.start)
            .min_by_key(|&position| match query.direction {
                SearchDirection::Forward => (position <= origin, position),
                SearchDirection::Backward => (position >= origin, usize::MAX - position),
            });
        let Some(target) = target else {
            return false;
        };
        if let Some(operator) = operator {
            if origin == target {
                return false;
            }
            let range = origin.min(target)..origin.max(target);
            match operator {
                VimOperator::Delete => self.kill_range(range),
                VimOperator::Change => {
                    self.kill_range(range);
                    self.vim_mode = VimMode::Insert;
                }
                VimOperator::Yank => {
                    self.kill_buffer = self.text[range].to_string();
                }
            }
        } else {
            self.cursor = target.min(self.vim_normal_end_cursor_for_search());
        }
        true
    }

    fn vim_normal_end_cursor_for_search(&self) -> usize {
        if self.text.is_empty() {
            return 0;
        }
        self.text
            .grapheme_indices(true)
            .next_back()
            .map(|(index, _)| index)
            .unwrap_or(0)
    }
}

#[derive(Clone, Copy, Debug)]
enum SearchCommand {
    Start(SearchDirection),
    Next,
    Previous,
}

#[cfg(test)]
#[path = "vim_search_tests.rs"]
mod tests;
