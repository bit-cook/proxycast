//! Composer bridge for TextArea-owned Vim query input.

use super::ChatComposer;
use crate::vim_search::SearchDirection;
use crossterm::event::KeyEvent;

impl ChatComposer {
    pub(crate) fn vim_search_query(&self) -> Option<(&str, SearchDirection)> {
        self.draft.textarea.vim_search_query()
    }

    pub(crate) fn vim_search_active(&self) -> bool {
        self.vim_search_query().is_some()
    }

    pub(crate) fn vim_search_wants_key(&self, key: KeyEvent) -> bool {
        self.draft.textarea.wants_vim_search_key(key)
    }

    pub(super) fn handle_vim_search_key(&mut self, key: KeyEvent) -> bool {
        if !self.draft.textarea.wants_vim_search_key(key) {
            return false;
        }
        self.draft.textarea.input(key);
        true
    }
}

#[cfg(test)]
#[path = "vim_search_tests.rs"]
mod tests;
