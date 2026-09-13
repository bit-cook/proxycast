//! Bounded Vim undo/redo history for the composer draft.
//!
//! The composer owns this history because text and attachments must restore as one draft
//! transaction. Search query input remains temporary TextArea state and never enters this history.

use std::collections::VecDeque;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{draft_state::ComposerDraft, ChatComposer};

const MAX_VIM_UNDO_STEPS: usize = 64;
const MAX_VIM_UNDO_BYTES: usize = 1024 * 1024;

#[derive(Debug, Default)]
pub(super) struct VimHistory {
    undo: VecDeque<ComposerDraft>,
    redo: VecDeque<ComposerDraft>,
    pending: Option<ComposerDraft>,
}

impl VimHistory {
    fn trim(&mut self) {
        while self.undo.len() > MAX_VIM_UNDO_STEPS || self.total_bytes() > MAX_VIM_UNDO_BYTES {
            if self.undo.pop_front().is_none() {
                self.redo.pop_front();
            }
        }
    }

    fn total_bytes(&self) -> usize {
        self.undo
            .iter()
            .chain(self.redo.iter())
            .map(ComposerDraft::bytes)
            .sum()
    }
}

impl ChatComposer {
    fn begin_vim_edit_transaction(&mut self) {
        let snapshot = self.snapshot_draft();
        if snapshot.bytes() > MAX_VIM_UNDO_BYTES {
            self.vim_history = VimHistory::default();
            return;
        }
        self.vim_history.pending = Some(snapshot);
    }

    pub(super) fn begin_direct_vim_edit(&mut self) -> bool {
        if !self.draft.textarea.is_vim_enabled()
            || self.vim_search_active()
            || self.vim_history.pending.is_some()
        {
            return false;
        }
        self.begin_vim_edit_transaction();
        self.vim_history.pending.is_some()
    }

    pub(super) fn begin_vim_key(&mut self, key: KeyEvent) {
        if !self.draft.textarea.is_vim_enabled() || self.vim_search_active() {
            return;
        }
        if self.vim_history.pending.is_some() {
            return;
        }

        if self.draft.textarea.is_vim_normal_mode() && !starts_vim_edit(key) {
            return;
        }
        self.begin_vim_edit_transaction();
    }

    pub(super) fn finish_vim_key(&mut self) {
        if self.vim_history.pending.is_none()
            || self.draft.textarea.is_vim_operator_pending()
            || self.vim_search_active()
            || !self.draft.textarea.is_vim_normal_mode()
        {
            return;
        }
        self.finish_vim_edit();
    }

    pub(super) fn finish_vim_edit(&mut self) {
        let Some(snapshot) = self.vim_history.pending.take() else {
            return;
        };
        if self.draft_content_equals(&snapshot) {
            return;
        }
        self.vim_history.redo.clear();
        self.vim_history.undo.push_back(snapshot);
        self.vim_history.trim();
    }

    pub(super) fn handle_vim_history_key(&mut self, key: KeyEvent) -> bool {
        if !self.draft.textarea.is_vim_normal_mode()
            || self.draft.textarea.is_vim_operator_pending()
            || self.vim_search_active()
        {
            return false;
        }
        let undo = key.modifiers.is_empty() && key.code == KeyCode::Char('u');
        let redo = key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('r');
        if !undo && !redo {
            return false;
        }

        let snapshot = if redo {
            self.vim_history.redo.pop_back()
        } else {
            self.vim_history.undo.pop_back()
        };
        if let Some(snapshot) = snapshot {
            let current = self.snapshot_draft();
            if redo {
                self.vim_history.undo.push_back(current);
            } else {
                self.vim_history.redo.push_back(current);
            }
            self.restore_draft(snapshot);
            self.vim_history.trim();
        }
        true
    }
}

fn starts_vim_edit(key: KeyEvent) -> bool {
    if key
        .modifiers
        .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        return false;
    }
    !matches!(
        key.code,
        KeyCode::Char('/')
            | KeyCode::Char('?')
            | KeyCode::Char('n' | 'N')
            | KeyCode::Char('h' | 'j' | 'k' | 'l' | 'w' | 'b' | 'e' | '0' | '$')
            | KeyCode::Left
            | KeyCode::Right
            | KeyCode::Up
            | KeyCode::Down
            | KeyCode::Home
            | KeyCode::End
            | KeyCode::Esc
            | KeyCode::Char('y')
    )
}

#[cfg(test)]
#[path = "vim_history_tests.rs"]
mod tests;
