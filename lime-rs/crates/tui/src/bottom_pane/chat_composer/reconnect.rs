//! Offline composer editing used while the App Server connection is recovering.
//!
//! Enter and Tab deliberately leave the draft untouched. The runtime owns reconnect attempts;
//! this module only provides the Codex-shaped editing boundary for a composer that is offline.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

use super::ChatComposer;

impl ChatComposer {
    /// Apply a basic editor key while disconnected without submitting or queueing the draft.
    #[allow(dead_code)]
    pub(crate) fn handle_disconnected_key(&mut self, key: KeyEvent) {
        if let Some(search) = self.history_search.take() {
            self.replace_text(search.draft);
        }
        self.history_index = None;
        self.draft.saved_draft = None;

        if matches!(key.code, KeyCode::Enter | KeyCode::Tab)
            || !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat)
        {
            return;
        }

        self.draft.textarea.input(key);
        self.reset_history_navigation();
    }
}

#[cfg(test)]
#[path = "reconnect_tests.rs"]
mod tests;
