use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::cell::RefMut;
use unicode_segmentation::UnicodeSegmentation;

mod attachment_state;
mod draft_state;
mod history_search;
mod reconnect;

use self::attachment_state::AttachmentState;
use self::draft_state::DraftState;
use self::history_search::HistorySearchState;
use crate::bottom_pane::textarea::{TextArea, TextAreaState};

const MAX_HISTORY_ENTRIES: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InputResult {
    None,
    Changed,
    Submitted(String),
    Queued(String),
    Interrupt,
    DecreaseEffort,
    IncreaseEffort,
    PreviousPermissions,
    NextPermissions,
    OpenExternalEditor,
    Quit,
}

#[derive(Debug, Default)]
pub(crate) struct ChatComposer {
    draft: DraftState,
    attachments: AttachmentState,
    history: Vec<String>,
    history_index: Option<usize>,
    history_search: Option<HistorySearchState>,
}

impl ChatComposer {
    pub(crate) fn textarea(&self) -> &TextArea {
        &self.draft.textarea
    }

    pub(crate) fn textarea_state_mut(&self) -> RefMut<'_, TextAreaState> {
        self.draft.textarea_state_mut()
    }

    pub(crate) fn text(&self) -> &str {
        self.draft.textarea.text()
    }

    pub(crate) fn cursor(&self) -> usize {
        self.draft.textarea.cursor()
    }

    pub(crate) fn desired_height(&self, width: u16) -> u16 {
        self.draft.textarea.desired_height(width)
    }

    #[allow(dead_code)]
    pub(crate) fn cursor_position(&self, width: u16) -> Option<(usize, usize)> {
        self.draft.textarea.cursor_position(width)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.draft.textarea.is_empty()
    }

    pub(crate) fn pending_image_count(&self) -> usize {
        self.attachments.len()
    }

    pub(crate) fn pending_images(&self) -> &[std::path::PathBuf] {
        self.attachments.paths()
    }

    pub(crate) fn has_pending_images(&self) -> bool {
        !self.attachments.is_empty()
    }

    pub(crate) fn attach_image(&mut self, path: std::path::PathBuf) {
        self.attachments.attach(path);
    }

    pub(crate) fn take_pending_images(&mut self) -> Vec<std::path::PathBuf> {
        self.attachments.take()
    }

    pub(crate) fn restore_pending_images(&mut self, images: Vec<std::path::PathBuf>) {
        self.attachments.restore(images);
    }

    pub(crate) fn remove_last_pending_image(&mut self) -> bool {
        self.attachments.remove_last()
    }

    pub(crate) fn history_search_active(&self) -> bool {
        self.history_search.is_some()
    }

    pub(crate) fn history_search_query(&self) -> Option<&str> {
        self.history_search
            .as_ref()
            .map(|search| search.query.as_str())
    }

    pub(crate) fn replace(&mut self, text: String) {
        self.replace_text(text);
        self.reset_history_navigation();
        self.history_search = None;
    }

    pub(crate) fn load_history<I>(&mut self, entries: I)
    where
        I: IntoIterator<Item = String>,
    {
        self.history = entries
            .into_iter()
            .filter(|entry| !entry.trim().is_empty())
            .collect();
        if self.history.len() > MAX_HISTORY_ENTRIES {
            let keep_from = self.history.len() - MAX_HISTORY_ENTRIES;
            self.history.drain(..keep_from);
        }
        self.history_index = None;
        self.draft.saved_draft = None;
        self.history_search = None;
    }

    pub(crate) fn insert(&mut self, value: &str) {
        self.draft.textarea.insert(value);
        self.reset_history_navigation();
    }

    pub(crate) fn handle_key_event(&mut self, key: KeyEvent) -> InputResult {
        if matches!(key.kind, KeyEventKind::Release) {
            return InputResult::None;
        }
        if self.history_search.is_some() {
            return self.handle_history_search_key(key);
        }

        let editor_key = matches!(key.code, KeyCode::Char('b' | 'f' | 'w' | 'k' | 'u' | 'y'))
            && key.modifiers == KeyModifiers::CONTROL
            || matches!(key.code, KeyCode::Char('d')) && key.modifiers == KeyModifiers::ALT
            || matches!(key.code, KeyCode::Backspace | KeyCode::Delete)
                && key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
        if editor_key {
            self.draft.textarea.input(key);
            self.reset_history_navigation();
            return InputResult::Changed;
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return match key.code {
                KeyCode::Char('c') => InputResult::Interrupt,
                KeyCode::Char('d') if self.is_empty() => InputResult::Quit,
                KeyCode::Char('j') => {
                    self.insert("\n");
                    InputResult::Changed
                }
                KeyCode::Char('a') => {
                    self.draft.textarea.move_line_start();
                    InputResult::Changed
                }
                KeyCode::Char('e') => {
                    self.draft.textarea.move_line_end();
                    InputResult::Changed
                }
                KeyCode::Char('g') => InputResult::OpenExternalEditor,
                KeyCode::Char('r') => self.start_history_search(),
                _ => InputResult::None,
            };
        }

        match key.code {
            KeyCode::Char(',') if key.modifiers.contains(KeyModifiers::ALT) => {
                InputResult::DecreaseEffort
            }
            KeyCode::Char('.') if key.modifiers.contains(KeyModifiers::ALT) => {
                InputResult::IncreaseEffort
            }
            KeyCode::F(7) => InputResult::PreviousPermissions,
            KeyCode::F(8) => InputResult::NextPermissions,
            KeyCode::Tab => self.queue(),
            KeyCode::Enter
                if key
                    .modifiers
                    .intersects(KeyModifiers::ALT | KeyModifiers::SHIFT) =>
            {
                self.insert("\n");
                InputResult::Changed
            }
            KeyCode::Enter => self.submit(),
            KeyCode::Up if !self.draft.textarea.text().contains('\n') => self.history_previous(),
            KeyCode::Down if !self.draft.textarea.text().contains('\n') => self.history_next(),
            KeyCode::Char(_)
            | KeyCode::Backspace
            | KeyCode::Delete
            | KeyCode::Left
            | KeyCode::Right
            | KeyCode::Home
            | KeyCode::End => {
                let before = (self.draft.textarea.text().to_string(), self.cursor());
                self.draft.textarea.input(key);
                let changed = before.0 != self.draft.textarea.text() || before.1 != self.cursor();
                if changed {
                    self.reset_history_navigation();
                    InputResult::Changed
                } else {
                    InputResult::None
                }
            }
            _ => InputResult::None,
        }
    }

    fn submit(&mut self) -> InputResult {
        self.take_submission(InputResult::Submitted)
    }

    fn queue(&mut self) -> InputResult {
        self.take_submission(InputResult::Queued)
    }

    fn take_submission<F>(&mut self, action: F) -> InputResult
    where
        F: FnOnce(String) -> InputResult,
    {
        if self.draft.textarea.text().trim().is_empty() {
            return InputResult::None;
        }
        let submitted = self.draft.textarea.take();
        self.history.push(submitted.clone());
        if self.history.len() > MAX_HISTORY_ENTRIES {
            self.history.remove(0);
        }
        self.history_index = None;
        self.draft.saved_draft = None;
        self.history_search = None;
        action(submitted)
    }

    fn start_history_search(&mut self) -> InputResult {
        self.history_index = None;
        self.draft.saved_draft = None;
        self.history_search = Some(HistorySearchState {
            query: String::new(),
            draft: self.draft.textarea.text().to_string(),
            selected_index: None,
        });
        InputResult::Changed
    }

    fn handle_history_search_key(&mut self, key: KeyEvent) -> InputResult {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return match key.code {
                KeyCode::Char('r') => {
                    self.select_history_match(true);
                    InputResult::Changed
                }
                KeyCode::Char('s') => {
                    self.select_history_match_newer();
                    InputResult::Changed
                }
                KeyCode::Char('c') => {
                    let draft = self
                        .history_search
                        .take()
                        .map(|search| search.draft)
                        .unwrap_or_default();
                    self.replace_text(draft);
                    InputResult::Changed
                }
                _ => InputResult::None,
            };
        }

        match key.code {
            KeyCode::Esc => {
                let draft = self
                    .history_search
                    .take()
                    .map(|search| search.draft)
                    .unwrap_or_default();
                self.replace_text(draft);
                InputResult::Changed
            }
            KeyCode::Enter => {
                if self
                    .history_search
                    .as_ref()
                    .is_some_and(|search| search.selected_index.is_some())
                {
                    self.history_search = None;
                    self.submit()
                } else {
                    let draft = self
                        .history_search
                        .take()
                        .map(|search| search.draft)
                        .unwrap_or_default();
                    self.replace_text(draft);
                    InputResult::Changed
                }
            }
            KeyCode::Backspace => {
                if let Some(search) = self.history_search.as_mut() {
                    if let Some((start, _)) = search.query.grapheme_indices(true).next_back() {
                        search.query.truncate(start);
                        search.selected_index = None;
                    }
                }
                self.select_history_match(false);
                InputResult::Changed
            }
            KeyCode::Char(ch) => {
                if let Some(search) = self.history_search.as_mut() {
                    search.query.push(ch);
                    search.selected_index = None;
                }
                self.select_history_match(false);
                InputResult::Changed
            }
            KeyCode::Up => {
                self.select_history_match(true);
                InputResult::Changed
            }
            KeyCode::Down => {
                self.select_history_match_newer();
                InputResult::Changed
            }
            KeyCode::Left | KeyCode::Right | KeyCode::Home | KeyCode::End => {
                self.history_search = None;
                self.handle_key_event(key)
            }
            _ => InputResult::None,
        }
    }

    fn select_history_match(&mut self, older: bool) {
        let Some(search) = self.history_search.as_ref() else {
            return;
        };
        let selected_index = search.selected_index;
        let found = history_search::find_match(&self.history, &search.query, selected_index, older);
        if let Some(index) = found {
            if let Some(search) = self.history_search.as_mut() {
                search.selected_index = Some(index);
            }
            self.replace_text(self.history[index].clone());
        } else if selected_index.is_none() {
            let draft = self
                .history_search
                .as_ref()
                .map(|search| search.draft.clone())
                .unwrap_or_default();
            self.replace_text(draft);
        }
    }

    fn select_history_match_newer(&mut self) {
        let Some(search) = self.history_search.as_ref() else {
            return;
        };
        let found = history_search::find_newer(&self.history, &search.query, search.selected_index);
        if let Some(index) = found {
            if let Some(search) = self.history_search.as_mut() {
                search.selected_index = Some(index);
            }
            self.replace_text(self.history[index].clone());
        }
    }

    fn history_previous(&mut self) -> InputResult {
        if self.history.is_empty() {
            return InputResult::None;
        }
        if self.history_index.is_none() {
            self.draft.saved_draft = Some(self.draft.textarea.text().to_string());
        }
        let index = self
            .history_index
            .map(|index| index.saturating_sub(1))
            .unwrap_or(self.history.len() - 1);
        self.history_index = Some(index);
        self.replace_text(self.history[index].clone());
        InputResult::Changed
    }

    fn history_next(&mut self) -> InputResult {
        let Some(index) = self.history_index else {
            return InputResult::None;
        };
        if index + 1 < self.history.len() {
            let next = index + 1;
            self.history_index = Some(next);
            self.replace_text(self.history[next].clone());
        } else {
            self.history_index = None;
            let draft = self.draft.saved_draft.take().unwrap_or_default();
            self.replace_text(draft);
        }
        InputResult::Changed
    }

    fn replace_text(&mut self, text: String) {
        self.draft.textarea.replace(text);
    }

    fn reset_history_navigation(&mut self) {
        self.history_index = None;
        self.draft.saved_draft = None;
        self.history_search = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn cursor_and_backspace_respect_extended_graphemes() {
        let mut composer = ChatComposer::default();
        composer.insert("a👩🏽‍💻界");

        composer.handle_key_event(key(KeyCode::Left));
        composer.handle_key_event(key(KeyCode::Backspace));

        assert_eq!(composer.text(), "a界");
        assert_eq!(composer.cursor(), 1);
    }

    #[test]
    fn paste_inserts_at_a_unicode_boundary() {
        let mut composer = ChatComposer::default();
        composer.insert("你好");
        composer.handle_key_event(key(KeyCode::Left));
        composer.insert("\nworld");

        assert_eq!(composer.text(), "你\nworld好");
    }

    #[test]
    fn history_restores_the_unsent_draft() {
        let mut composer = ChatComposer::default();
        composer.insert("first");
        assert!(matches!(
            composer.handle_key_event(key(KeyCode::Enter)),
            InputResult::Submitted(_)
        ));
        composer.insert("draft");

        composer.handle_key_event(key(KeyCode::Up));
        assert_eq!(composer.text(), "first");
        composer.handle_key_event(key(KeyCode::Down));
        assert_eq!(composer.text(), "draft");
    }

    #[test]
    fn modified_enter_adds_a_newline_and_plain_enter_submits() {
        let mut composer = ChatComposer::default();
        composer.insert("first");
        composer.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT));
        composer.insert("second");

        assert_eq!(composer.text(), "first\nsecond");
        assert_eq!(
            composer.handle_key_event(key(KeyCode::Enter)),
            InputResult::Submitted("first\nsecond".to_string())
        );
    }

    #[test]
    fn editor_word_and_kill_shortcuts_share_textarea_semantics() {
        let mut composer = ChatComposer::default();
        composer.insert("alpha beta");

        composer.handle_key_event(KeyEvent::new(KeyCode::Char('w'), KeyModifiers::CONTROL));
        assert_eq!(composer.text(), "alpha ");

        composer.handle_key_event(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL));
        assert_eq!(composer.text(), "alpha beta");

        composer.handle_key_event(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL));
        composer.handle_key_event(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL));
        assert_eq!(composer.text(), "alpha bet");
    }

    #[test]
    fn key_release_does_not_insert_or_submit() {
        let mut composer = ChatComposer::default();
        let mut release = key(KeyCode::Char('x'));
        release.kind = KeyEventKind::Release;

        assert_eq!(composer.handle_key_event(release), InputResult::None);
        assert!(composer.is_empty());

        composer.insert("draft");
        let mut submit_release = key(KeyCode::Enter);
        submit_release.kind = KeyEventKind::Release;
        assert_eq!(composer.handle_key_event(submit_release), InputResult::None);
        assert_eq!(composer.text(), "draft");
    }

    #[test]
    fn tab_queues_non_empty_draft_and_clears_composer() {
        let mut composer = ChatComposer::default();
        composer.insert("follow up");

        assert_eq!(
            composer.handle_key_event(key(KeyCode::Tab)),
            InputResult::Queued("follow up".to_string())
        );
        assert!(composer.is_empty());
    }

    #[test]
    fn loaded_history_is_bounded_to_the_recent_entries() {
        let mut composer = ChatComposer::default();
        composer.load_history((0..205).map(|index| format!("prompt-{index}")));

        composer.handle_key_event(key(KeyCode::Up));
        assert_eq!(composer.text(), "prompt-204");
        for _ in 0..199 {
            composer.handle_key_event(key(KeyCode::Up));
        }
        assert_eq!(composer.text(), "prompt-5");
    }

    #[test]
    fn ctrl_r_searches_history_without_replacing_the_saved_draft() {
        let mut composer = ChatComposer::default();
        composer.load_history([
            "git status".to_string(),
            "cargo test".to_string(),
            "git diff".to_string(),
        ]);
        composer.insert("draft");

        assert_eq!(
            composer.handle_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL)),
            InputResult::Changed
        );
        assert_eq!(composer.text(), "draft");
        assert_eq!(composer.history_search_query(), Some(""));
        composer.handle_key_event(key(KeyCode::Char('g')));
        composer.handle_key_event(key(KeyCode::Char('i')));
        composer.handle_key_event(key(KeyCode::Char('t')));
        assert_eq!(composer.text(), "git diff");
        assert_eq!(composer.history_search_query(), Some("git"));
        composer.handle_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL));
        assert_eq!(composer.text(), "git status");
        composer.handle_key_event(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
        assert_eq!(composer.text(), "git diff");
        composer.handle_key_event(key(KeyCode::Esc));
        assert_eq!(composer.text(), "draft");
        assert!(!composer.history_search_active());
    }

    #[test]
    fn ctrl_r_search_accepts_a_match_with_enter() {
        let mut composer = ChatComposer::default();
        composer.load_history(["first prompt".to_string()]);
        composer.handle_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL));
        for character in "first".chars() {
            composer.handle_key_event(key(KeyCode::Char(character)));
        }

        assert_eq!(
            composer.handle_key_event(key(KeyCode::Enter)),
            InputResult::Submitted("first prompt".to_string())
        );
        assert!(!composer.history_search_active());
    }

    #[test]
    fn history_search_is_case_insensitive_and_restores_on_no_match() {
        let mut composer = ChatComposer::default();
        composer.load_history(["Deploy Lime".to_string()]);
        composer.insert("draft");
        composer.handle_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL));
        for character in "DEP".chars() {
            composer.handle_key_event(key(KeyCode::Char(character)));
        }
        assert_eq!(composer.text(), "Deploy Lime");
        composer.handle_key_event(key(KeyCode::Char('x')));
        assert_eq!(composer.text(), "draft");
        assert_eq!(composer.history_search_query(), Some("DEPx"));
    }
}
