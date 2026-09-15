use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::text::Line;
use std::cell::RefMut;
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

mod agents_navigation;
mod attachment_state;
mod completion_target;
mod draft_state;
mod file_search_popup;
mod footer_state;
mod history_search;
mod popup_state;
mod reconnect;
mod skill_popup;
mod slash_input;
mod vim_history;
mod vim_search;

use self::attachment_state::AttachmentState;
use self::draft_state::{ComposerDraft, DraftState};
use self::file_search_popup::FileSearchPopup;
pub(crate) use self::file_search_popup::FileSearchPopupAction;
use self::footer_state::{FooterMode, FooterState};
use self::history_search::HistorySearchState;
use self::popup_state::{ActivePopup, DismissedToken, PopupState};
use self::skill_popup::SkillPopup;
pub(crate) use self::skill_popup::SkillPopupAction;
use self::vim_history::VimHistory;
use super::command_popup::{CommandPopup, CommandPopupAction};
use crate::bottom_pane::textarea::{TextArea, TextAreaState};
use app_server_protocol::protocol::v2::{FuzzyFileSearchResult, SkillMetadata};

const MAX_HISTORY_ENTRIES: usize = 200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileSearchRequest {
    pub(crate) generation: u64,
    pub(crate) query: String,
}

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
    OpenAgentsOverview,
    Quit,
}

#[derive(Debug, Default)]
pub(crate) struct ChatComposer {
    draft: DraftState,
    attachments: AttachmentState,
    popups: PopupState,
    footer: FooterState,
    history: Vec<String>,
    history_index: Option<usize>,
    last_history_text: Option<String>,
    history_search: Option<HistorySearchState>,
    vim_history: VimHistory,
    file_search_generation: u64,
    file_search_request: Option<FileSearchRequest>,
    skills: Vec<SkillMetadata>,
    agents_navigation_enabled: bool,
}

impl ChatComposer {
    fn snapshot_draft(&self) -> ComposerDraft {
        ComposerDraft {
            text: self.text().to_owned(),
            cursor: self.cursor(),
            attachments: self.attachments.clone(),
        }
    }

    fn restore_draft(&mut self, draft: ComposerDraft) {
        self.draft.textarea.replace(draft.text);
        self.draft.textarea.set_cursor(draft.cursor);
        self.attachments = draft.attachments;
        self.history_index = None;
        self.last_history_text = None;
        self.draft.saved_draft = None;
        if self.history_search.is_none() {
            self.footer.mode = if self.is_empty() {
                FooterMode::ComposerEmpty
            } else {
                FooterMode::ComposerHasDraft
            };
        }
        self.sync_command_popup();
    }

    fn draft_content_equals(&self, draft: &ComposerDraft) -> bool {
        self.text() == draft.text && self.attachments == draft.attachments
    }

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

    pub(crate) fn remote_image_urls(&self) -> &[String] {
        self.attachments.remote_image_urls()
    }

    pub(crate) fn has_selected_remote_image(&self) -> bool {
        self.attachments.selected_remote_image_index().is_some()
    }

    pub(crate) fn remote_image_lines(&self) -> Vec<ratatui::text::Line<'static>> {
        self.attachments.remote_image_lines()
    }

    pub(crate) fn has_pending_images(&self) -> bool {
        !self.attachments.is_empty()
    }

    pub(crate) fn attach_image(&mut self, path: std::path::PathBuf) {
        let started = self.begin_direct_vim_edit();
        self.attachments.attach(path);
        self.reset_history_navigation();
        if started {
            self.finish_vim_edit();
        }
    }

    pub(crate) fn take_pending_images(&mut self) -> Vec<std::path::PathBuf> {
        let images = self.attachments.take();
        if !images.is_empty() {
            self.reset_history_navigation();
        }
        images
    }

    pub(crate) fn restore_pending_images(&mut self, images: Vec<std::path::PathBuf>) {
        self.attachments.restore(images);
        self.reset_history_navigation();
    }

    pub(crate) fn take_remote_image_urls(&mut self) -> Vec<String> {
        let urls = self.attachments.take_remote_image_urls();
        if !urls.is_empty() {
            self.reset_history_navigation();
        }
        urls
    }

    pub(crate) fn set_remote_image_urls(&mut self, urls: Vec<String>) {
        let started = self.begin_direct_vim_edit();
        self.attachments.set_remote_image_urls(urls);
        self.reset_history_navigation();
        if started {
            self.finish_vim_edit();
        }
    }

    pub(crate) fn remove_last_pending_image(&mut self) -> bool {
        let started = self.begin_direct_vim_edit();
        let removed = self.attachments.remove_last();
        if removed {
            self.reset_history_navigation();
        }
        if started {
            self.finish_vim_edit();
        }
        removed
    }

    /// Clear a plain-text draft for Ctrl-C and make it available through local history.
    ///
    /// Lime's composer history currently stores text only. Keep attachment-bearing drafts intact
    /// until history can preserve their canonical image state instead of silently dropping it.
    pub(crate) fn clear_for_ctrl_c(&mut self) -> Option<String> {
        if self.is_empty() || self.has_pending_images() {
            return None;
        }

        let previous = self.draft.textarea.take();
        self.history.push(previous.clone());
        if self.history.len() > MAX_HISTORY_ENTRIES {
            self.history.remove(0);
        }
        self.history_index = None;
        self.last_history_text = None;
        self.draft.saved_draft = None;
        self.history_search = None;
        self.vim_history = VimHistory::default();
        self.footer.mode = FooterMode::ComposerEmpty;
        Some(previous)
    }

    pub(crate) fn history_search_active(&self) -> bool {
        self.history_search.is_some()
    }

    pub(crate) fn footer_flash(&self) -> Option<&ratatui::text::Line<'static>> {
        self.footer.flash_line()
    }

    pub(crate) fn footer_has_draft(&self) -> bool {
        matches!(self.footer.mode, FooterMode::ComposerHasDraft)
    }

    pub(crate) fn set_vim_enabled(&mut self, enabled: bool) {
        self.draft.textarea.set_vim_enabled(enabled);
        self.vim_history = VimHistory::default();
        self.reset_history_navigation();
    }

    pub(crate) fn toggle_vim_enabled(&mut self) -> bool {
        let enabled = !self.draft.textarea.is_vim_enabled();
        self.set_vim_enabled(enabled);
        enabled
    }

    pub(crate) fn is_vim_normal_mode(&self) -> bool {
        self.draft.textarea.is_vim_normal_mode()
    }

    pub(crate) fn should_handle_vim_insert_escape(&self, key: KeyEvent) -> bool {
        self.draft.textarea.should_handle_vim_insert_escape(key)
    }

    pub(crate) fn vim_mode_indicator_span(&self) -> Option<ratatui::text::Span<'static>> {
        self.draft.textarea.vim_mode_indicator_span()
    }

    pub(crate) fn history_search_query(&self) -> Option<&str> {
        self.history_search
            .as_ref()
            .map(|search| search.query.as_str())
    }

    /// 返回当前历史搜索预览中的匹配范围；接受或无匹配时不产生渲染高亮。
    pub(crate) fn history_search_highlight_ranges(&self) -> Vec<Range<usize>> {
        let Some(search) = self.history_search.as_ref() else {
            return Vec::new();
        };
        if search.selected_index.is_none() {
            return Vec::new();
        }
        history_search::match_ranges(self.text(), &search.query)
    }

    /// 返回 footer 历史搜索查询输入框的光标位置。
    pub(crate) fn history_search_cursor_position(
        &self,
        area: Rect,
        label: &str,
    ) -> Option<(u16, u16)> {
        let query = self.history_search_query()?;
        if area.is_empty() {
            return None;
        }
        let prefix_width = Line::from(format!(" {label}")).width() as u16;
        let query_width = Line::from(query.to_owned()).width() as u16;
        let desired_x = area
            .x
            .saturating_add(prefix_width)
            .saturating_add(query_width);
        let max_x = area.x.saturating_add(area.width.saturating_sub(1));
        Some((desired_x.min(max_x), area.y))
    }

    pub(crate) fn replace(&mut self, text: String) {
        self.replace_text(text);
        self.reset_history_navigation();
        self.history_search = None;
        self.sync_command_popup();
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
        self.last_history_text = None;
        self.draft.saved_draft = None;
        self.history_search = None;
    }

    pub(crate) fn insert(&mut self, value: &str) {
        let started = self.begin_direct_vim_edit();
        self.draft.textarea.insert(value);
        self.reset_history_navigation();
        self.sync_command_popup();
        if started {
            self.finish_vim_edit();
        }
    }

    pub(crate) fn command_popup(&self) -> Option<&CommandPopup> {
        match &self.popups.active {
            ActivePopup::Command(popup) => Some(popup),
            ActivePopup::File(_) | ActivePopup::Skill(_) | ActivePopup::None => None,
        }
    }

    pub(crate) fn file_search_popup(&self) -> Option<&FileSearchPopup> {
        match &self.popups.active {
            ActivePopup::File(popup) => Some(popup),
            ActivePopup::Command(_) | ActivePopup::Skill(_) | ActivePopup::None => None,
        }
    }

    pub(crate) fn file_search_popup_active(&self) -> bool {
        matches!(self.popups.active, ActivePopup::File(_))
    }

    pub(crate) fn file_search_popup_has_selection(&self) -> bool {
        matches!(&self.popups.active, ActivePopup::File(popup) if popup.selected_path().is_some())
    }

    pub(crate) fn take_file_search_request(&mut self) -> Option<FileSearchRequest> {
        self.file_search_request.take()
    }

    /// Update the enabled skill catalog received from the App Server startup contract.
    pub(crate) fn set_skills(&mut self, skills: Vec<SkillMetadata>) {
        self.skills = skills.into_iter().filter(|skill| skill.enabled).collect();
        if let ActivePopup::Skill(popup) = &mut self.popups.active {
            popup.set_skills(self.skills.clone());
        }
        self.sync_command_popup();
    }

    pub(crate) fn skills(&self) -> &[SkillMetadata] {
        &self.skills
    }

    pub(crate) fn skill_popup(&self) -> Option<&SkillPopup> {
        match &self.popups.active {
            ActivePopup::Skill(popup) => Some(popup),
            ActivePopup::Command(_) | ActivePopup::File(_) | ActivePopup::None => None,
        }
    }

    pub(crate) fn skill_popup_active(&self) -> bool {
        matches!(self.popups.active, ActivePopup::Skill(_))
    }

    pub(crate) fn handle_skill_popup_event(
        &mut self,
        event: &crossterm::event::Event,
    ) -> SkillPopupAction {
        let action = match &mut self.popups.active {
            ActivePopup::Skill(popup) => popup.handle_event(event),
            ActivePopup::Command(_) | ActivePopup::File(_) | ActivePopup::None => {
                SkillPopupAction::Pass
            }
        };
        match action {
            SkillPopupAction::Cancel => {
                if let Some((range, query)) = self.current_skill_token_range() {
                    self.popups.dismissed_skill_token =
                        Some(DismissedToken::new(self.text(), range, query));
                }
                self.popups.active = ActivePopup::None;
            }
            SkillPopupAction::Complete => {
                let selected_name = match &self.popups.active {
                    ActivePopup::Skill(popup) => {
                        popup.selected_skill().map(|skill| skill.name.clone())
                    }
                    _ => None,
                };
                if let (Some(name), Some((range, _))) =
                    (selected_name, self.current_skill_token_range())
                {
                    let started = self.begin_direct_vim_edit();
                    let start = range.start;
                    let inserted = format!("${name}");
                    self.draft.textarea.replace_range(range, &inserted);
                    let inserted_range = start..start.saturating_add(inserted.len());
                    self.draft.textarea.set_cursor(inserted_range.end);
                    self.advance_past_file_completion_separator();
                    self.popups.dismissed_skill_token =
                        Some(DismissedToken::new(self.text(), inserted_range, name));
                    self.reset_history_navigation();
                    if started {
                        self.finish_vim_edit();
                    }
                }
                self.popups.active = ActivePopup::None;
            }
            SkillPopupAction::Consumed | SkillPopupAction::Pass => {}
        }
        action
    }

    pub(crate) fn on_file_search_result(
        &mut self,
        generation: u64,
        query: &str,
        matches: Vec<FuzzyFileSearchResult>,
    ) {
        if generation != self.file_search_generation {
            return;
        }
        if let ActivePopup::File(popup) = &mut self.popups.active {
            popup.set_matches(query, matches);
        }
    }

    pub(crate) fn handle_file_search_popup_event(
        &mut self,
        event: &crossterm::event::Event,
    ) -> FileSearchPopupAction {
        let action = match &mut self.popups.active {
            ActivePopup::File(popup) => popup.handle_event(event),
            ActivePopup::Command(_) | ActivePopup::Skill(_) | ActivePopup::None => {
                FileSearchPopupAction::Pass
            }
        };
        match action {
            FileSearchPopupAction::Cancel => {
                if let Some((range, query)) = self.current_at_token_range() {
                    self.popups.dismissed_file_token =
                        Some(DismissedToken::new(self.text(), range, query));
                }
                self.popups.active = ActivePopup::None;
                self.file_search_request = None;
            }
            FileSearchPopupAction::Complete => {
                let path = match &self.popups.active {
                    ActivePopup::File(popup) => popup.selected_path().map(str::to_owned),
                    _ => None,
                };
                if let (Some(path), Some((range, _))) = (path, self.current_at_token_range()) {
                    let started = self.begin_direct_vim_edit();
                    let start = range.start;
                    self.draft.textarea.replace_range(range, &path);
                    self.draft
                        .textarea
                        .set_cursor(start.saturating_add(path.len()));
                    self.advance_past_file_completion_separator();
                    self.popups.dismissed_file_token = None;
                    self.popups.active = ActivePopup::None;
                    self.sync_command_popup();
                    self.reset_history_navigation();
                    if started {
                        self.finish_vim_edit();
                    }
                }
            }
            FileSearchPopupAction::Consumed => {
                if matches!(
                    event,
                    crossterm::event::Event::Key(key)
                        if key.code == crossterm::event::KeyCode::Enter
                            && !self.file_search_popup_has_selection()
                ) {
                    self.popups.active = ActivePopup::None;
                    self.file_search_request = None;
                }
            }
            FileSearchPopupAction::Pass => {}
        }
        action
    }

    pub(crate) fn command_popup_active(&self) -> bool {
        self.popups.active()
    }

    pub(crate) fn clear_command_popup(&mut self) {
        self.popups.active = ActivePopup::None;
    }

    pub(crate) fn handle_command_popup_event(
        &mut self,
        event: &crossterm::event::Event,
    ) -> CommandPopupAction {
        let ActivePopup::Command(popup) = &mut self.popups.active else {
            return CommandPopupAction::Pass;
        };
        let action = popup.handle_event(event);
        if matches!(action, CommandPopupAction::Cancel) {
            let first_line = self.text().lines().next().unwrap_or("");
            let token = slash_input::command_popup_filter_text(first_line, self.cursor());
            self.popups.dismiss_command(token.unwrap_or_default());
        }
        action
    }

    pub(crate) fn sync_command_popup(&mut self) {
        if self.history_index.is_some() {
            self.popups.active = ActivePopup::None;
            self.file_search_request = None;
            self.popups.file_search_requested_query = None;
            return;
        }
        if !self.skills.is_empty() {
            if let Some((range, query)) = self.current_skill_token_range() {
                if skill_query_is_candidate(&query, &self.skills) {
                    if self
                        .popups
                        .dismissed_skill_token
                        .as_ref()
                        .is_some_and(|dismissed| dismissed.matches(self.text(), &range, &query))
                    {
                        if matches!(self.popups.active, ActivePopup::Skill(_)) {
                            self.popups.active = ActivePopup::None;
                        }
                        return;
                    }
                    self.popups.dismissed_skill_token = None;
                    match &mut self.popups.active {
                        ActivePopup::Skill(popup) => {
                            popup.set_query(query.clone());
                            popup.set_skills(self.skills.clone());
                        }
                        ActivePopup::Command(_) | ActivePopup::File(_) | ActivePopup::None => {
                            self.popups.active =
                                ActivePopup::Skill(SkillPopup::new(self.skills.clone(), query));
                        }
                    }
                    return;
                }
            }
        }
        self.popups.dismissed_skill_token = None;
        if matches!(self.popups.active, ActivePopup::Skill(_)) {
            self.popups.active = ActivePopup::None;
        }
        if let Some((range, query)) = self.current_at_token_range() {
            if self
                .popups
                .dismissed_file_token
                .as_ref()
                .is_some_and(|dismissed| dismissed.matches(self.text(), &range, &query))
            {
                if matches!(self.popups.active, ActivePopup::File(_)) {
                    self.popups.active = ActivePopup::None;
                }
                return;
            }
            self.popups.dismissed_file_token = None;
            match &mut self.popups.active {
                ActivePopup::File(popup) => popup.set_query(query.clone()),
                ActivePopup::Command(_) | ActivePopup::Skill(_) | ActivePopup::None => {
                    self.popups.active = ActivePopup::File(FileSearchPopup::new(query.clone()));
                }
            }
            if query.is_empty() {
                if let ActivePopup::File(popup) = &mut self.popups.active {
                    popup.set_empty_prompt();
                }
                self.file_search_request = None;
                self.popups.file_search_requested_query = None;
                return;
            }
            let current_query = match &self.popups.active {
                ActivePopup::File(popup) => Some(popup.query()),
                ActivePopup::Command(_) | ActivePopup::Skill(_) | ActivePopup::None => None,
            };
            if let Some(current_query) = current_query {
                if !current_query.is_empty()
                    && self.popups.file_search_requested_query.as_deref() != Some(current_query)
                {
                    self.file_search_generation = self.file_search_generation.wrapping_add(1);
                    self.file_search_request = Some(FileSearchRequest {
                        generation: self.file_search_generation,
                        query: current_query.to_string(),
                    });
                    self.popups.file_search_requested_query = Some(current_query.to_string());
                }
            }
            return;
        }
        self.popups.dismissed_file_token = None;
        if matches!(self.popups.active, ActivePopup::File(_)) {
            self.popups.active = ActivePopup::None;
            self.file_search_request = None;
        }
        self.popups.file_search_requested_query = None;
        let text = self.text().to_string();
        let first_line = text.lines().next().unwrap_or("");
        let Some(filter) = slash_input::command_popup_filter_text(first_line, self.cursor()) else {
            self.popups.clear_dismissal();
            self.popups.active = ActivePopup::None;
            return;
        };
        if self.popups.dismissed_for(Some(&filter)) {
            self.popups.active = ActivePopup::None;
            return;
        } else {
            self.popups.clear_dismissal();
        }
        if self
            .popups
            .active
            .as_command_mut()
            .is_some_and(|popup| popup.update(&filter))
        {
            return;
        }
        self.popups.active = slash_input::command_popup(&filter);
    }

    fn current_at_token_range(&self) -> Option<(Range<usize>, String)> {
        completion_target::current_prefixed_token_range(&self.draft.textarea, '@', false)
    }

    fn current_skill_token_range(&self) -> Option<(Range<usize>, String)> {
        completion_target::current_prefixed_token_range(&self.draft.textarea, '$', true)
    }

    /// Leave the cursor after one horizontal separator following a file completion.
    ///
    /// Existing horizontal whitespace is reused when it already separates the completed path
    /// from a suffix. Newlines are not reused as separators, matching Codex completion behavior.
    fn advance_past_file_completion_separator(&mut self) {
        let cursor = self.draft.textarea.cursor();
        let text = self.draft.textarea.text();
        let Some(next) = text[cursor..].chars().next() else {
            self.draft.textarea.insert_str_at(cursor, " ");
            return;
        };
        let is_horizontal = |ch: char| {
            ch.is_whitespace()
                && !matches!(
                    ch,
                    '\n' | '\r' | '\u{000B}' | '\u{000C}' | '\u{0085}' | '\u{2028}' | '\u{2029}'
                )
        };
        if !is_horizontal(next) {
            self.draft.textarea.insert_str_at(cursor, " ");
            return;
        }
        let separator_len = next.len_utf8();
        let after_separator = cursor.saturating_add(separator_len);
        let suffix_is_non_whitespace = self.draft.textarea.text()[after_separator..]
            .chars()
            .next()
            .is_some_and(|ch| !ch.is_whitespace());
        if suffix_is_non_whitespace {
            self.draft.textarea.insert_str_at(cursor, " ");
        } else {
            self.draft.textarea.set_cursor(after_separator);
        }
    }

    pub(crate) fn command_from_prompt(
        &self,
        prompt: &str,
    ) -> Option<crate::slash_command::SlashCommand> {
        slash_input::command_from_prompt(prompt)
    }

    pub(crate) fn handle_key_event(&mut self, key: KeyEvent) -> InputResult {
        if matches!(key.kind, KeyEventKind::Release) {
            return InputResult::None;
        }
        if self.handle_vim_history_key(key) {
            return InputResult::Changed;
        }
        self.begin_vim_key(key);
        let result = self.handle_key_event_inner(key);
        self.finish_vim_key();
        result
    }

    fn handle_key_event_inner(&mut self, key: KeyEvent) -> InputResult {
        if matches!(key.kind, KeyEventKind::Release) {
            return InputResult::None;
        }
        if self.history_search.is_some() {
            return self.handle_history_search_key(key);
        }

        if self.handle_vim_search_key(key) {
            return InputResult::Changed;
        }

        if self
            .attachments
            .handle_remote_key(key, self.draft.textarea.cursor())
        {
            return InputResult::Changed;
        }

        if crate::keymap::is_editor_key_event(key) {
            self.draft.textarea.input(key);
            self.reset_history_navigation();
            return InputResult::Changed;
        }

        if self.should_handle_vim_insert_escape(key) {
            self.draft.textarea.input(key);
            self.reset_history_navigation();
            return InputResult::Changed;
        }

        if key.code == KeyCode::Left
            && key.modifiers.is_empty()
            && self.agents_navigation_available()
        {
            return InputResult::OpenAgentsOverview;
        }

        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return match key.code {
                KeyCode::Char('c') => InputResult::Interrupt,
                KeyCode::Char('d') if self.is_empty() => InputResult::Quit,
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
            KeyCode::Up
                if !self.is_vim_normal_mode() && self.should_handle_history_navigation(-1) =>
            {
                self.history_previous()
            }
            KeyCode::Down
                if !self.is_vim_normal_mode() && self.should_handle_history_navigation(1) =>
            {
                self.history_next()
            }
            KeyCode::Char(_)
            | KeyCode::Backspace
            | KeyCode::Delete
            | KeyCode::Left
            | KeyCode::Right
            | KeyCode::Up
            | KeyCode::Down
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
        self.last_history_text = None;
        self.draft.saved_draft = None;
        self.history_search = None;
        self.vim_history = VimHistory::default();
        self.footer.mode = FooterMode::ComposerEmpty;
        action(submitted)
    }

    fn start_history_search(&mut self) -> InputResult {
        self.history_index = None;
        self.last_history_text = None;
        self.draft.saved_draft = None;
        self.history_search = Some(HistorySearchState {
            query: String::new(),
            draft: self.snapshot_draft(),
            selected_index: None,
        });
        self.footer.mode = FooterMode::HistorySearch;
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
                    self.restore_draft(draft);
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
                self.restore_draft(draft);
                InputResult::Changed
            }
            KeyCode::Enter => {
                if self
                    .history_search
                    .as_ref()
                    .is_some_and(|search| search.selected_index.is_some())
                {
                    // Codex accepts the preview as an editable draft. Submission remains
                    // an explicit follow-up Enter, so reverse search never starts a turn.
                    self.reset_history_navigation();
                    self.vim_history = VimHistory::default();
                    InputResult::Changed
                } else {
                    // Keep the search session open when there is no match. The original draft
                    // is already restored by the search traversal, and the query can still be
                    // edited to find another entry.
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
            self.restore_draft(draft);
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
            self.draft.saved_draft = Some(self.snapshot_draft());
        }
        let index = self
            .history_index
            .map(|index| index.saturating_sub(1))
            .unwrap_or(self.history.len() - 1);
        self.history_index = Some(index);
        let text = self.history[index].clone();
        self.replace_text(text.clone());
        self.last_history_text = Some(text);
        InputResult::Changed
    }

    fn history_next(&mut self) -> InputResult {
        let Some(index) = self.history_index else {
            return InputResult::None;
        };
        if index + 1 < self.history.len() {
            let next = index + 1;
            self.history_index = Some(next);
            let text = self.history[next].clone();
            self.replace_text(text.clone());
            self.last_history_text = Some(text);
        } else {
            self.history_index = None;
            let draft = self.draft.saved_draft.take().unwrap_or_default();
            self.restore_draft(draft);
        }
        InputResult::Changed
    }

    fn replace_text(&mut self, text: String) {
        self.draft.textarea.replace(text);
        if self.history_search.is_none() {
            self.footer.mode = if self.is_empty() {
                FooterMode::ComposerEmpty
            } else {
                FooterMode::ComposerHasDraft
            };
        }
    }

    fn reset_history_navigation(&mut self) {
        self.history_index = None;
        self.last_history_text = None;
        self.draft.saved_draft = None;
        self.history_search = None;
        self.footer.mode = if self.is_empty() {
            FooterMode::ComposerEmpty
        } else {
            FooterMode::ComposerHasDraft
        };
    }

    /// Return whether a vertical key may enter shell-style history navigation.
    ///
    /// Empty drafts can always start history traversal. Non-empty drafts must be the exact last
    /// recalled entry at a logical cursor boundary. Wrapped single-line drafts add a visual-row
    /// boundary so interior Up/Down keys remain textarea navigation.
    fn should_handle_history_navigation(&self, direction: i8) -> bool {
        if self.history.is_empty() {
            return false;
        }
        // Text-only history cannot safely replace an attachment-bearing draft. Remote-image
        // selection is handled before this gate; local and remote attachments otherwise remain
        // untouched until the richer history entry contract is available.
        if self.has_pending_images() {
            return false;
        }
        let text = self.text();
        if text.is_empty() {
            return true;
        }
        if self.cursor() != 0 && self.cursor() != text.len() {
            return false;
        }
        matches!(self.last_history_text.as_deref(), Some(last) if last == text)
            && self.draft.textarea.is_vertical_boundary(direction)
    }
}

fn skill_query_is_candidate(query: &str, skills: &[SkillMetadata]) -> bool {
    match completion_target::dollar_query_kind(query) {
        completion_target::DollarQueryKind::Completable => true,
        completion_target::DollarQueryKind::AmbiguousShellParameter => {
            skills.iter().any(|skill| skill.name == query)
        }
        completion_target::DollarQueryKind::ShellVariable
        | completion_target::DollarQueryKind::DefiniteShellParameter
        | completion_target::DollarQueryKind::Invalid => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn current_at_token_range(text: &str, cursor: usize) -> Option<(Range<usize>, String)> {
        let mut textarea = TextArea::new();
        textarea.insert_str(text);
        textarea.set_cursor(cursor);
        completion_target::current_prefixed_token_range(&textarea, '@', false)
    }

    fn current_dollar_token_range(text: &str, cursor: usize) -> Option<(Range<usize>, String)> {
        let mut textarea = TextArea::new();
        textarea.insert_str(text);
        textarea.set_cursor(cursor);
        completion_target::current_prefixed_token_range(&textarea, '$', true)
    }

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
    fn slash_popup_tracks_cursor_inside_command_with_argument_suffix() {
        let mut composer = ChatComposer::default();
        composer.insert("/effort value");
        assert!(composer.command_popup().is_none());

        composer.draft.textarea.set_cursor("/ef".len());
        composer.sync_command_popup();
        assert_eq!(
            composer.command_popup().and_then(CommandPopup::selected),
            Some(crate::slash_command::SlashCommand::Effort)
        );

        composer.draft.textarea.set_cursor("/effort value".len());
        composer.sync_command_popup();
        assert!(composer.command_popup().is_none());
    }

    #[test]
    fn slash_popup_escape_dismissal_is_scoped_to_cursor_filter() {
        let mut composer = ChatComposer::default();
        composer.insert("/effort value");
        composer.draft.textarea.set_cursor("/ef".len());
        composer.sync_command_popup();
        assert!(composer.command_popup().is_some());

        assert_eq!(
            composer.handle_command_popup_event(&crossterm::event::Event::Key(key(KeyCode::Esc))),
            CommandPopupAction::Cancel
        );
        assert!(composer.command_popup().is_none());
        composer.sync_command_popup();
        assert!(composer.command_popup().is_none());

        composer.draft.textarea.set_cursor("/eff".len());
        composer.sync_command_popup();
        assert!(composer.command_popup().is_some());
    }

    #[test]
    fn history_navigation_does_not_replace_an_arbitrary_draft() {
        let mut composer = ChatComposer::default();
        composer.insert("first");
        assert!(matches!(
            composer.handle_key_event(key(KeyCode::Enter)),
            InputResult::Submitted(_)
        ));
        composer.insert("draft");

        composer.handle_key_event(key(KeyCode::Up));
        assert_eq!(composer.text(), "draft");
        composer.handle_key_event(key(KeyCode::Down));
        assert_eq!(composer.text(), "draft");
    }

    #[test]
    fn history_navigation_clears_completion_popups_for_recalled_text() {
        let mut composer = ChatComposer::default();
        composer.load_history(["/model".to_string()]);

        composer.handle_key_event(key(KeyCode::Up));
        composer.sync_command_popup();

        assert_eq!(composer.text(), "/model");
        assert!(!composer.command_popup_active());
    }

    #[test]
    fn attachment_edit_exits_history_navigation() {
        let mut composer = ChatComposer::default();
        composer.load_history(["recalled prompt".to_string()]);
        composer.handle_key_event(key(KeyCode::Up));
        composer.attach_image(std::path::PathBuf::from("/tmp/recalled.png"));

        composer.handle_key_event(key(KeyCode::Up));

        assert_eq!(composer.text(), "recalled prompt");
        assert_eq!(
            composer.pending_images(),
            &[std::path::PathBuf::from("/tmp/recalled.png")]
        );
    }

    #[test]
    fn attachment_only_draft_does_not_enter_text_history() {
        let mut composer = ChatComposer::default();
        composer.load_history(["older prompt".to_string()]);
        composer.attach_image(std::path::PathBuf::from("/tmp/only-image.png"));

        composer.handle_key_event(key(KeyCode::Up));

        assert!(composer.is_empty());
        assert_eq!(
            composer.pending_images(),
            &[std::path::PathBuf::from("/tmp/only-image.png")]
        );
    }

    #[test]
    fn taking_attachment_state_exits_history_navigation() {
        let mut composer = ChatComposer::default();
        composer.load_history(["recalled prompt".to_string()]);
        composer.handle_key_event(key(KeyCode::Up));
        composer.attach_image(std::path::PathBuf::from("/tmp/recalled.png"));
        assert_eq!(composer.take_pending_images().len(), 1);

        composer.handle_key_event(key(KeyCode::Up));

        assert_eq!(composer.text(), "recalled prompt");
    }

    #[test]
    fn disconnected_submit_keys_clear_history_recall_state() {
        let mut composer = ChatComposer::default();
        composer.load_history(["older prompt".to_string(), "newer prompt".to_string()]);
        composer.handle_key_event(key(KeyCode::Up));
        composer.handle_key_event(key(KeyCode::Up));
        assert_eq!(composer.text(), "older prompt");

        composer.handle_disconnected_key(key(KeyCode::Enter));
        composer.handle_key_event(key(KeyCode::Up));

        assert_eq!(composer.text(), "older prompt");
    }

    #[test]
    fn wrapped_single_line_vertical_navigation_stays_in_editor_until_visual_boundary() {
        let mut composer = ChatComposer::default();
        composer.load_history(["older prompt".to_string()]);
        composer.insert("abcdefghij");
        composer.draft.textarea.set_cursor(6);

        // Seed the same wrap cache used by rendering so Up/Down can distinguish visual rows.
        assert!(composer.desired_height(/*width*/ 4) > 1);
        composer.handle_key_event(key(KeyCode::Up));

        assert_ne!(composer.text(), "older prompt");
        assert!(composer.cursor() < 6);
    }

    #[test]
    fn history_search_cancel_restores_cursor_and_attachments() {
        let mut composer = ChatComposer::default();
        composer.insert("draft");
        composer.handle_key_event(key(KeyCode::Left));
        let original_cursor = composer.cursor();
        composer.attach_image(std::path::PathBuf::from("/tmp/draft.png"));
        composer.load_history(["archived prompt".to_string()]);

        composer.handle_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL));
        for character in "archived".chars() {
            composer.handle_key_event(key(KeyCode::Char(character)));
        }
        assert_eq!(composer.text(), "archived prompt");
        assert!(composer.has_pending_images());

        composer.handle_key_event(key(KeyCode::Esc));
        assert_eq!(composer.text(), "draft");
        assert_eq!(composer.cursor(), original_cursor);
        assert_eq!(
            composer.pending_images(),
            &[std::path::PathBuf::from("/tmp/draft.png")]
        );
        assert!(composer.footer_has_draft());
    }

    #[test]
    fn ctrl_c_clears_plain_text_and_records_it_for_history_recall() {
        let mut composer = ChatComposer::default();
        composer.insert("draft");

        assert_eq!(composer.clear_for_ctrl_c(), Some("draft".to_string()));
        assert!(composer.is_empty());
        assert_eq!(
            composer.handle_key_event(key(KeyCode::Up)),
            InputResult::Changed
        );
        assert_eq!(composer.text(), "draft");
    }

    #[test]
    fn ctrl_c_keeps_attachment_bearing_drafts_until_history_supports_images() {
        let mut composer = ChatComposer::default();
        composer.insert("draft");
        composer.attach_image(std::path::PathBuf::from("/tmp/draft.png"));

        assert_eq!(composer.clear_for_ctrl_c(), None);
        assert_eq!(composer.text(), "draft");
        assert!(composer.has_pending_images());
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
    fn codex_control_newline_and_multiline_navigation_stay_in_composer_editor() {
        let mut composer = ChatComposer::default();
        composer.insert("ab\ncdef");
        composer.draft.textarea.set_cursor(2);

        assert_eq!(
            composer.handle_key_event(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::CONTROL,)),
            InputResult::Changed
        );
        assert_eq!(composer.text(), "ab\n\ncdef");

        composer.replace("ab\ncdef".to_string());
        composer.draft.textarea.set_cursor(2);
        assert_eq!(
            composer.handle_key_event(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL,)),
            InputResult::Changed
        );
        assert_eq!(composer.cursor(), 5);
        assert_eq!(
            composer.handle_key_event(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL,)),
            InputResult::Changed
        );
        assert_eq!(composer.cursor(), 2);
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
            InputResult::Changed
        );
        assert!(!composer.history_search_active());
        assert_eq!(composer.text(), "first prompt");

        composer.insert(" + follow-up");
        assert_eq!(
            composer.handle_key_event(key(KeyCode::Enter)),
            InputResult::Submitted("first prompt + follow-up".to_string())
        );
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

    #[test]
    fn history_search_no_match_enter_keeps_search_open_for_query_edits() {
        let mut composer = ChatComposer::default();
        composer.load_history(["deploy".to_string()]);
        composer.insert("draft");
        composer.handle_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL));
        for character in "zzz".chars() {
            composer.handle_key_event(key(KeyCode::Char(character)));
        }

        assert_eq!(composer.text(), "draft");
        assert_eq!(
            composer.handle_key_event(key(KeyCode::Enter)),
            InputResult::Changed
        );
        assert!(composer.history_search_active());
        assert_eq!(composer.history_search_query(), Some("zzz"));

        for _ in 0..3 {
            composer.handle_key_event(key(KeyCode::Backspace));
        }
        composer.handle_key_event(key(KeyCode::Char('d')));
        assert_eq!(composer.text(), "deploy");
        assert_eq!(composer.history_search_query(), Some("d"));
    }

    #[test]
    fn history_search_highlights_preview_until_enter_accepts_it() {
        let mut composer = ChatComposer::default();
        composer.load_history(["Deploy Lime".to_string()]);
        composer.handle_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL));
        for character in "dep".chars() {
            composer.handle_key_event(key(KeyCode::Char(character)));
        }

        assert_eq!(composer.text(), "Deploy Lime");
        assert_eq!(composer.history_search_highlight_ranges(), vec![0..3]);

        composer.handle_key_event(key(KeyCode::Enter));
        assert!(composer.history_search_highlight_ranges().is_empty());
    }

    #[test]
    fn at_token_range_tracks_cursor_boundaries_without_crossing_whitespace() {
        let text = "@file\tplain\n@next";
        let token_end = "@file".len();
        let next_start = text.find("@next").expect("second token");

        for cursor in [0, 2, token_end] {
            assert_eq!(
                current_at_token_range(text, cursor),
                Some(0..token_end).map(|range| (range, "file".to_string()))
            );
        }
        assert_eq!(current_at_token_range(text, token_end + 1), None);
        assert_eq!(
            current_at_token_range(text, next_start),
            Some(next_start..text.len()).map(|range| (range, "next".to_string()))
        );
    }

    #[test]
    fn at_token_range_rejects_embedded_prefix_and_keeps_repeated_prefixes_in_one_token() {
        assert_eq!(current_at_token_range("foo@bar", "foo@bar".len()), None);
        assert_eq!(
            current_at_token_range("@foo@bar", "@foo@bar".len()),
            Some(0..8).map(|range| (range, "foo@bar".to_string()))
        );
    }

    #[test]
    fn at_token_range_uses_utf8_safe_byte_ranges() {
        let text = "前 @界🙂 后";
        let start = text.find('@').expect("mention prefix");
        let end = text.find(" 后").expect("trailing separator");
        let expected = Some(start..end).map(|range| (range, "界🙂".to_string()));

        assert_eq!(current_at_token_range(text, start), expected);
        assert_eq!(current_at_token_range(text, start + 2), expected);
        assert_eq!(current_at_token_range(text, end), expected);
        assert_eq!(current_at_token_range(text, start + 3), expected);
    }

    #[test]
    fn file_popup_dismissal_does_not_hide_an_identical_later_token() {
        let mut composer = ChatComposer::default();
        composer.insert("@same  @same");

        let first_end = "@same".len();
        composer.draft.textarea.set_cursor(first_end);
        composer.sync_command_popup();
        assert!(composer.file_search_popup_active());
        composer.handle_file_search_popup_event(&crossterm::event::Event::Key(key(KeyCode::Esc)));
        assert!(!composer.file_search_popup_active());

        composer.draft.textarea.set_cursor("@same  @same".len());
        composer.sync_command_popup();
        assert!(composer.file_search_popup_active());
    }

    fn test_skill(name: &str) -> SkillMetadata {
        SkillMetadata {
            name: name.to_string(),
            description: format!("{name} skill"),
            short_description: None,
            interface: None,
            dependencies: None,
            path: std::path::PathBuf::from(format!("/skills/{name}/SKILL.md")),
            scope: app_server_protocol::protocol::v2::SkillScope::User,
            enabled: true,
        }
    }

    #[test]
    fn skill_popup_filters_and_completes_the_active_dollar_token() {
        let mut composer = ChatComposer::default();
        composer.set_skills(vec![test_skill("deploy"), test_skill("code-review")]);
        composer.insert("please $cr");
        assert!(composer.skill_popup_active());
        composer.handle_skill_popup_event(&crossterm::event::Event::Key(key(KeyCode::Enter)));
        assert_eq!(composer.text(), "please $code-review ");
        assert!(!composer.skill_popup_active());
    }

    #[test]
    fn skill_popup_escape_is_scoped_to_the_current_token_occurrence() {
        let mut composer = ChatComposer::default();
        composer.set_skills(vec![test_skill("same")]);
        composer.insert("$same  $same");
        composer.draft.textarea.set_cursor("$same".len());
        composer.sync_command_popup();
        assert!(composer.skill_popup_active());
        composer.handle_skill_popup_event(&crossterm::event::Event::Key(key(KeyCode::Esc)));
        composer.draft.textarea.set_cursor("$same  $same".len());
        composer.sync_command_popup();
        assert!(composer.skill_popup_active());
    }

    #[test]
    fn dollar_token_range_is_utf8_safe_and_rejects_embedded_prefixes() {
        let text = "前 $审查-1 后";
        let start = text.find('$').expect("dollar");
        let end = text.find(" 后").expect("suffix");
        assert_eq!(
            current_dollar_token_range(text, start + 1),
            Some((start..end, "审查-1".to_string()))
        );
        assert_eq!(current_dollar_token_range("foo$deploy", 10), None);
    }

    #[test]
    fn shell_parameters_do_not_open_skill_popup_without_an_exact_skill() {
        let mut composer = ChatComposer::default();
        composer.set_skills(vec![test_skill("deploy")]);
        composer.insert("$HOME");
        assert!(!composer.skill_popup_active());
        composer.replace("$1".to_string());
        assert!(!composer.skill_popup_active());
    }
}
