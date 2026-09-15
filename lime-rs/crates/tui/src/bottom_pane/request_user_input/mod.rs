use std::collections::BTreeMap;

use app_server_protocol::protocol::v2::{
    ToolRequestUserInputAnswer, ToolRequestUserInputParams, ToolRequestUserInputResponse,
};
use app_server_protocol::RequestId;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use std::time::{Duration, Instant};

use super::{AppServerResponse, ChatComposer, InputResult};
use crate::width::display_width;

pub(super) mod render;

const AUTO_RESOLUTION_HIDDEN_GRACE: Duration = Duration::from_secs(60);
const AUTO_RESOLUTION_VISIBLE_COUNTDOWN: Duration = Duration::from_secs(60);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AutoResolutionTiming {
    Disabled,
    HiddenGrace { remaining: Duration },
    VisibleCountdown { remaining: Duration },
    Due,
}

fn format_auto_resolution_remaining(remaining: Duration) -> String {
    let mut seconds = remaining.as_secs();
    if remaining.subsec_nanos() > 0 {
        seconds = seconds.saturating_add(1);
    }
    if seconds < 60 {
        return format!("{seconds}s");
    }
    format!("{}m {:02}s", seconds / 60, seconds % 60)
}

#[derive(Debug)]
pub(super) struct RequestUserInputOverlay {
    pub(super) id: RequestId,
    pub(super) params: ToolRequestUserInputParams,
    pub(super) question_index: usize,
    pub(super) selected: usize,
    pub(super) editing: bool,
    pub(super) composer: ChatComposer,
    answers: BTreeMap<String, ToolRequestUserInputAnswer>,
    /// Stores one notes draft per question so navigation does not lose input.
    question_drafts: Vec<String>,
    /// Stores option selection per question for reversible focus changes.
    question_selections: Vec<usize>,
    /// Stores the Options/Notes focus per question.
    question_editing: Vec<bool>,
    request_started_at: Instant,
    auto_resolution_snoozed: bool,
}

impl RequestUserInputOverlay {
    pub(super) fn new(id: RequestId, params: ToolRequestUserInputParams) -> Self {
        let question_count = params.questions.len();
        let question_editing = params
            .questions
            .iter()
            .map(|question| question.options.as_ref().is_none_or(Vec::is_empty))
            .collect();
        let editing = params
            .questions
            .first()
            .and_then(|question| question.options.as_ref())
            .is_none_or(Vec::is_empty);
        Self {
            id,
            params,
            question_index: 0,
            selected: 0,
            editing,
            composer: ChatComposer::default(),
            answers: BTreeMap::new(),
            question_drafts: vec![String::new(); question_count],
            question_selections: vec![0; question_count],
            question_editing,
            request_started_at: Instant::now(),
            auto_resolution_snoozed: false,
        }
    }

    fn snooze_auto_resolution(&mut self) {
        if !self.params.is_blocking {
            self.auto_resolution_snoozed = true;
        }
    }

    fn auto_resolution_timing_at(&self, now: Instant) -> AutoResolutionTiming {
        if self.params.is_blocking || self.auto_resolution_snoozed {
            return AutoResolutionTiming::Disabled;
        }

        let elapsed = now.saturating_duration_since(self.request_started_at);
        if elapsed < AUTO_RESOLUTION_HIDDEN_GRACE {
            return AutoResolutionTiming::HiddenGrace {
                remaining: AUTO_RESOLUTION_HIDDEN_GRACE.saturating_sub(elapsed),
            };
        }
        let visible_elapsed = elapsed.saturating_sub(AUTO_RESOLUTION_HIDDEN_GRACE);
        if visible_elapsed < AUTO_RESOLUTION_VISIBLE_COUNTDOWN {
            return AutoResolutionTiming::VisibleCountdown {
                remaining: AUTO_RESOLUTION_VISIBLE_COUNTDOWN.saturating_sub(visible_elapsed),
            };
        }
        AutoResolutionTiming::Due
    }

    pub(super) fn next_frame_delay(&self, now: Instant) -> Option<Duration> {
        match self.auto_resolution_timing_at(now) {
            AutoResolutionTiming::Disabled => None,
            AutoResolutionTiming::HiddenGrace { remaining } => Some(remaining),
            AutoResolutionTiming::VisibleCountdown { remaining } => {
                Some(remaining.min(Duration::from_secs(1)))
            }
            AutoResolutionTiming::Due => Some(Duration::ZERO),
        }
    }

    pub(super) fn pre_draw_tick(&mut self, now: Instant) -> Option<AppServerResponse> {
        if !matches!(
            self.auto_resolution_timing_at(now),
            AutoResolutionTiming::Due
        ) {
            return None;
        }
        Some(AppServerResponse::UserInput {
            id: self.id.clone(),
            response: ToolRequestUserInputResponse {
                answers: BTreeMap::new(),
            },
        })
    }

    pub(super) fn auto_resolution_countdown_text(
        &self,
        now: Instant,
        locale: crate::locale::Locale,
    ) -> Option<String> {
        match self.auto_resolution_timing_at(now) {
            AutoResolutionTiming::VisibleCountdown { remaining } => {
                Some(locale.auto_resolution_countdown(&format_auto_resolution_remaining(remaining)))
            }
            AutoResolutionTiming::Disabled
            | AutoResolutionTiming::HiddenGrace { .. }
            | AutoResolutionTiming::Due => None,
        }
    }

    pub(super) fn footer_hint(&self, locale: crate::locale::Locale) -> String {
        let mut hints = Vec::with_capacity(5);
        // The submit/cancel pair is the non-negotiable action set on narrow terminals.
        // Secondary navigation may be clipped, but these two controls must remain visible.
        hints.push(locale.request_submit_hint());
        hints.push(locale.request_cancel_hint());
        if self.has_options() && !self.editing {
            hints.push(locale.request_select_hint());
        }
        if self.has_options() {
            hints.push(locale.request_notes_hint());
        }
        if self.params.questions.len() > 1 {
            hints.push(locale.request_question_nav_hint());
        }
        hints.join(" · ")
    }

    pub(super) fn footer_hint_for_width(
        &self,
        locale: crate::locale::Locale,
        width: usize,
    ) -> String {
        let all = self.footer_hint(locale);
        if display_width(&all) <= width {
            return all;
        }

        // Keep the primary actions first. At very narrow widths use key-only labels instead of
        // truncating the pair in the middle and hiding the cancellation affordance.
        let primary = format!(
            "{} · {}",
            locale.request_submit_hint(),
            locale.request_cancel_hint()
        );
        let compact_primary = [
            primary.as_str(),
            "Enter · Esc",
            "↵ · Esc",
            "↵Esc",
            "Esc",
            "",
        ]
        .into_iter()
        .find(|candidate| display_width(candidate) <= width)
        .unwrap_or("");
        if compact_primary != primary {
            return compact_primary.to_string();
        }

        let mut hints = vec![locale.request_submit_hint(), locale.request_cancel_hint()];
        let secondary = [
            (self.has_options() && !self.editing).then_some(locale.request_select_hint()),
            self.has_options().then_some(locale.request_notes_hint()),
            (self.params.questions.len() > 1).then_some(locale.request_question_nav_hint()),
        ];
        for hint in secondary.into_iter().flatten() {
            let candidate = hints.join(" · ") + " · " + hint;
            if display_width(&candidate) <= width {
                hints.push(hint);
            }
        }
        hints.join(" · ")
    }

    fn save_current_state(&mut self) {
        if let Some(draft) = self.question_drafts.get_mut(self.question_index) {
            *draft = self.composer.text().to_owned();
        }
        if let Some(selection) = self.question_selections.get_mut(self.question_index) {
            *selection = self.selected;
        }
        if let Some(editing) = self.question_editing.get_mut(self.question_index) {
            *editing = self.editing;
        }
    }

    fn restore_current_state(&mut self) {
        self.selected = self
            .question_selections
            .get(self.question_index)
            .copied()
            .unwrap_or(0);
        let draft = self
            .question_drafts
            .get(self.question_index)
            .cloned()
            .unwrap_or_default();
        self.composer.replace(draft);
        self.editing = self
            .question_editing
            .get(self.question_index)
            .copied()
            .unwrap_or_else(|| !self.has_options());
    }

    fn move_question(&mut self, next: bool) {
        let count = self.params.questions.len();
        if count < 2 {
            return;
        }
        self.save_current_state();
        self.question_index = if next {
            (self.question_index + 1) % count
        } else {
            (self.question_index + count - 1) % count
        };
        self.restore_current_state();
    }

    /// Move through the current question's choices with the same wrapping list semantics as
    /// Codex. Notes editing remains a separate mode, so this is only called while options are
    /// focused.
    fn move_option(&mut self, next: bool) {
        let count = self.option_count();
        if count == 0 {
            return;
        }
        let selected = self.selected.min(count - 1);
        self.selected = if next {
            (selected + 1) % count
        } else {
            selected.checked_sub(1).unwrap_or(count - 1)
        };
        self.save_current_state();
    }

    pub(super) fn handle_key_event(&mut self, key: KeyEvent) -> Option<AppServerResponse> {
        if self.params.questions.is_empty() {
            return Some(self.finish());
        }
        self.snooze_auto_resolution();
        match key {
            key if key.kind == KeyEventKind::Press
                && key.modifiers.contains(KeyModifiers::CONTROL)
                && key.code == KeyCode::Char('c') =>
            {
                Some(self.cancel())
            }
            key if key.kind == KeyEventKind::Press => match key.code {
                KeyCode::Esc if self.editing && self.has_options() => {
                    self.clear_notes_and_focus_options();
                    None
                }
                KeyCode::Esc => Some(self.cancel()),
                KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.move_question(false);
                    None
                }
                KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    self.move_question(true);
                    None
                }
                KeyCode::PageUp => {
                    self.move_question(false);
                    None
                }
                KeyCode::PageDown => {
                    self.move_question(true);
                    None
                }
                KeyCode::Left | KeyCode::Char('h')
                    if !self.editing && self.has_options() && key.modifiers.is_empty() =>
                {
                    self.move_question(false);
                    None
                }
                KeyCode::Right | KeyCode::Char('l')
                    if !self.editing && self.has_options() && key.modifiers.is_empty() =>
                {
                    self.move_question(true);
                    None
                }
                KeyCode::Tab if self.has_options() && self.editing => {
                    self.clear_notes_and_focus_options();
                    None
                }
                KeyCode::Tab if self.has_options() => {
                    self.restore_current_state();
                    self.editing = true;
                    self.save_current_state();
                    None
                }
                KeyCode::Up if !self.editing => {
                    self.move_option(false);
                    None
                }
                KeyCode::Char('k')
                    if !self.editing
                        && (key.modifiers.is_empty()
                            || key.modifiers.contains(KeyModifiers::CONTROL)) =>
                {
                    self.move_option(false);
                    None
                }
                KeyCode::Down if !self.editing => {
                    self.move_option(true);
                    None
                }
                KeyCode::Char('j')
                    if !self.editing
                        && (key.modifiers.is_empty()
                            || key.modifiers.contains(KeyModifiers::CONTROL)) =>
                {
                    self.move_option(true);
                    None
                }
                KeyCode::Char(' ')
                    if !self.editing && self.has_options() && key.modifiers.is_empty() =>
                {
                    self.save_current_state();
                    None
                }
                KeyCode::Backspace
                    if self.editing && self.has_options() && self.composer.is_empty() =>
                {
                    self.clear_notes_and_focus_options();
                    None
                }
                _ if self.editing && key.code == KeyCode::Enter && self.composer.is_empty() => {
                    let answer = self.selected_option_label().into_iter().collect();
                    self.commit(answer)
                }
                _ if self.editing => match self.composer.handle_key_event(key) {
                    InputResult::Submitted(text) => {
                        let mut answers =
                            self.selected_option_label().into_iter().collect::<Vec<_>>();
                        let note = text.trim();
                        if !note.is_empty() {
                            if self.has_options() {
                                answers.push(format!("user_note: {note}"));
                            } else {
                                answers.push(note.to_string());
                            }
                        }
                        self.commit(answers)
                    }
                    InputResult::Interrupt | InputResult::Quit => Some(self.cancel()),
                    InputResult::Queued(text) => {
                        self.composer.insert(&text);
                        self.save_current_state();
                        None
                    }
                    InputResult::None
                    | InputResult::Changed
                    | InputResult::DecreaseEffort
                    | InputResult::IncreaseEffort
                    | InputResult::PreviousPermissions
                    | InputResult::NextPermissions
                    | InputResult::OpenExternalEditor
                    | InputResult::OpenAgentsOverview => {
                        self.save_current_state();
                        None
                    }
                },
                KeyCode::Enter => {
                    if self.selected == self.current_options().map_or(usize::MAX, <[_]>::len)
                        && self.other_option_enabled()
                    {
                        self.editing = true;
                        self.save_current_state();
                        return None;
                    }
                    let answer = self.selected_option_label().into_iter().collect();
                    self.commit(answer)
                }
                KeyCode::Char(ch) if ch.is_ascii_digit() && ch != '0' => {
                    let index = ch.to_digit(10).unwrap_or_default() as usize - 1;
                    if index < self.option_count() {
                        self.selected = index;
                        self.save_current_state();
                        let answer = self.selected_option_label().into_iter().collect();
                        self.commit(answer)
                    } else {
                        None
                    }
                }
                _ => None,
            },
            _ => None,
        }
    }

    fn commit(&mut self, answers: Vec<String>) -> Option<AppServerResponse> {
        self.save_current_state();
        if let Some(question) = self.params.questions.get(self.question_index) {
            self.answers
                .insert(question.id.clone(), ToolRequestUserInputAnswer { answers });
        }
        if self.question_index + 1 >= self.params.questions.len() {
            return Some(self.finish());
        }
        self.question_index += 1;
        self.restore_current_state();
        None
    }

    fn clear_notes_and_focus_options(&mut self) {
        if let Some(draft) = self.question_drafts.get_mut(self.question_index) {
            draft.clear();
        }
        self.composer.replace(String::new());
        self.editing = false;
        self.save_current_state();
    }

    fn current_options(
        &self,
    ) -> Option<&[app_server_protocol::protocol::v2::ToolRequestUserInputOption]> {
        self.params
            .questions
            .get(self.question_index)
            .and_then(|question| question.options.as_deref())
    }

    fn has_options(&self) -> bool {
        self.current_options()
            .is_some_and(|options| !options.is_empty())
    }

    fn option_count(&self) -> usize {
        let options = self.current_options().map_or(0, <[_]>::len);
        if self.other_option_enabled() {
            options + 1
        } else {
            options
        }
    }

    fn other_option_enabled(&self) -> bool {
        self.params
            .questions
            .get(self.question_index)
            .is_some_and(|question| question.is_other && self.has_options())
    }

    fn selected_option_label(&self) -> Option<String> {
        let options = self.current_options()?;
        if let Some(option) = options.get(self.selected) {
            return Some(option.label.clone());
        }
        (self.selected == options.len() && self.other_option_enabled()).then(|| "Other".to_string())
    }

    fn finish(&self) -> AppServerResponse {
        AppServerResponse::UserInput {
            id: self.id.clone(),
            response: ToolRequestUserInputResponse {
                answers: self.answers.clone(),
            },
        }
    }

    fn cancel(&self) -> AppServerResponse {
        AppServerResponse::UserInput {
            id: self.id.clone(),
            response: ToolRequestUserInputResponse {
                answers: BTreeMap::new(),
            },
        }
    }

    pub(super) fn handle_paste(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.snooze_auto_resolution();
        self.editing = true;
        self.composer.insert(text);
        self.save_current_state();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_server_protocol::protocol::v2::{
        ToolRequestUserInputOption, ToolRequestUserInputQuestion,
    };
    use crossterm::event::{KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn collects_option_and_freeform_questions_before_responding() {
        let mut request = RequestUserInputOverlay::new(
            RequestId::Integer(9),
            ToolRequestUserInputParams {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                item_id: "question-1".to_string(),
                questions: vec![
                    ToolRequestUserInputQuestion {
                        id: "mode".to_string(),
                        header: "Mode".to_string(),
                        question: "Choose a mode".to_string(),
                        is_other: false,
                        is_secret: false,
                        options: Some(vec![ToolRequestUserInputOption {
                            label: "Fast".to_string(),
                            description: "Continue immediately".to_string(),
                        }]),
                    },
                    ToolRequestUserInputQuestion {
                        id: "note".to_string(),
                        header: "Note".to_string(),
                        question: "Add a note".to_string(),
                        is_other: false,
                        is_secret: false,
                        options: None,
                    },
                ],
                is_blocking: true,
                auto_resolution_ms: None,
            },
        );

        assert_eq!(request.handle_key_event(key(KeyCode::Enter)), None);
        request.handle_key_event(key(KeyCode::Char('好')));
        let response = request.handle_key_event(key(KeyCode::Enter));

        let Some(AppServerResponse::UserInput { response, .. }) = response else {
            panic!("expected user input response");
        };
        assert_eq!(response.answers["mode"].answers, ["Fast"]);
        assert_eq!(response.answers["note"].answers, ["好"]);
    }

    #[test]
    fn escape_returns_an_empty_fail_closed_response() {
        let mut request = RequestUserInputOverlay::new(
            RequestId::Integer(9),
            ToolRequestUserInputParams {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                item_id: "question-1".to_string(),
                questions: Vec::new(),
                is_blocking: true,
                auto_resolution_ms: None,
            },
        );

        let response = request.handle_key_event(key(KeyCode::Esc));
        let Some(AppServerResponse::UserInput { response, .. }) = response else {
            panic!("expected user input response");
        };
        assert!(response.answers.is_empty());
    }

    #[test]
    fn ctrl_c_returns_an_empty_fail_closed_response() {
        let mut request = RequestUserInputOverlay::new(
            RequestId::Integer(10),
            ToolRequestUserInputParams {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                item_id: "question-1".to_string(),
                questions: vec![ToolRequestUserInputQuestion {
                    id: "secret".to_string(),
                    header: "Secret".to_string(),
                    question: "Enter a secret".to_string(),
                    is_other: false,
                    is_secret: true,
                    options: None,
                }],
                is_blocking: true,
                auto_resolution_ms: None,
            },
        );
        request.editing = true;
        request.composer.insert("sensitive");

        let response =
            request.handle_key_event(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
        let Some(AppServerResponse::UserInput { response, .. }) = response else {
            panic!("expected user input response");
        };
        assert!(response.answers.is_empty());
    }

    #[test]
    fn option_notes_follow_codex_answer_shape() {
        let mut request = RequestUserInputOverlay::new(
            RequestId::Integer(11),
            ToolRequestUserInputParams {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                item_id: "question-1".to_string(),
                questions: vec![ToolRequestUserInputQuestion {
                    id: "mode".to_string(),
                    header: "Mode".to_string(),
                    question: "Choose a mode".to_string(),
                    is_other: false,
                    is_secret: false,
                    options: Some(vec![ToolRequestUserInputOption {
                        label: "Fast".to_string(),
                        description: "Continue immediately".to_string(),
                    }]),
                }],
                is_blocking: true,
                auto_resolution_ms: None,
            },
        );

        assert_eq!(request.handle_key_event(key(KeyCode::Tab)), None);
        assert!(request.editing);
        request.composer.insert("keep logs");
        let response = request.handle_key_event(key(KeyCode::Enter));

        let Some(AppServerResponse::UserInput { response, .. }) = response else {
            panic!("expected user input response");
        };
        assert_eq!(
            response.answers["mode"].answers,
            ["Fast", "user_note: keep logs"]
        );
    }

    #[test]
    fn empty_notes_submit_the_selected_option() {
        let mut request = RequestUserInputOverlay::new(
            RequestId::Integer(13),
            ToolRequestUserInputParams {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                item_id: "question-1".to_string(),
                questions: vec![ToolRequestUserInputQuestion {
                    id: "mode".to_string(),
                    header: "Mode".to_string(),
                    question: "Choose a mode".to_string(),
                    is_other: false,
                    is_secret: false,
                    options: Some(vec![ToolRequestUserInputOption {
                        label: "Fast".to_string(),
                        description: "Continue immediately".to_string(),
                    }]),
                }],
                is_blocking: true,
                auto_resolution_ms: None,
            },
        );

        request.handle_key_event(key(KeyCode::Tab));
        let response = request.handle_key_event(key(KeyCode::Enter));

        let Some(AppServerResponse::UserInput { response, .. }) = response else {
            panic!("expected user input response");
        };
        assert_eq!(response.answers["mode"].answers, ["Fast"]);
    }

    #[test]
    fn other_option_is_only_added_when_the_contract_enables_it() {
        let mut request = RequestUserInputOverlay::new(
            RequestId::Integer(12),
            ToolRequestUserInputParams {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                item_id: "question-1".to_string(),
                questions: vec![ToolRequestUserInputQuestion {
                    id: "mode".to_string(),
                    header: "Mode".to_string(),
                    question: "Choose a mode".to_string(),
                    is_other: true,
                    is_secret: false,
                    options: Some(vec![ToolRequestUserInputOption {
                        label: "Fast".to_string(),
                        description: "Continue immediately".to_string(),
                    }]),
                }],
                is_blocking: true,
                auto_resolution_ms: None,
            },
        );

        request.handle_key_event(key(KeyCode::Down));
        assert_eq!(request.handle_key_event(key(KeyCode::Enter)), None);
        assert!(request.editing);
        let response = request.handle_key_event(key(KeyCode::Enter));

        let Some(AppServerResponse::UserInput { response, .. }) = response else {
            panic!("expected user input response");
        };
        assert_eq!(response.answers["mode"].answers, ["Other"]);
    }

    #[test]
    fn question_navigation_preserves_each_question_draft_and_selection() {
        let mut request = RequestUserInputOverlay::new(
            RequestId::Integer(14),
            ToolRequestUserInputParams {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                item_id: "question-1".to_string(),
                questions: vec![
                    ToolRequestUserInputQuestion {
                        id: "first".to_string(),
                        header: "First".to_string(),
                        question: "First note".to_string(),
                        is_other: false,
                        is_secret: false,
                        options: None,
                    },
                    ToolRequestUserInputQuestion {
                        id: "second".to_string(),
                        header: "Second".to_string(),
                        question: "Second note".to_string(),
                        is_other: false,
                        is_secret: false,
                        options: None,
                    },
                ],
                is_blocking: true,
                auto_resolution_ms: None,
            },
        );

        request.composer.insert("draft one");
        assert_eq!(
            request.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE)),
            None
        );
        assert_eq!(request.question_index, 1);
        request.composer.insert("draft two");
        assert_eq!(
            request.handle_key_event(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE)),
            None
        );
        assert_eq!(request.question_index, 0);
        assert_eq!(request.composer.text(), "draft one");
        assert_eq!(
            request.handle_key_event(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE)),
            None
        );
        assert_eq!(request.composer.text(), "draft two");
    }

    #[test]
    fn other_enter_opens_notes_before_submitting_custom_answer() {
        let mut request = RequestUserInputOverlay::new(
            RequestId::Integer(15),
            ToolRequestUserInputParams {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                item_id: "question-1".to_string(),
                questions: vec![ToolRequestUserInputQuestion {
                    id: "mode".to_string(),
                    header: "Mode".to_string(),
                    question: "Choose a mode".to_string(),
                    is_other: true,
                    is_secret: false,
                    options: Some(vec![ToolRequestUserInputOption {
                        label: "Fast".to_string(),
                        description: "Continue immediately".to_string(),
                    }]),
                }],
                is_blocking: true,
                auto_resolution_ms: None,
            },
        );

        request.handle_key_event(key(KeyCode::Down));
        assert_eq!(request.handle_key_event(key(KeyCode::Enter)), None);
        assert!(request.editing);
        request.composer.insert("custom");
        let Some(AppServerResponse::UserInput { response, .. }) =
            request.handle_key_event(key(KeyCode::Enter))
        else {
            panic!("expected user input response");
        };
        assert_eq!(
            response.answers["mode"].answers,
            ["Other", "user_note: custom"]
        );
    }

    #[test]
    fn notes_focus_returns_to_options_without_submitting_on_escape_or_empty_backspace() {
        let mut request = RequestUserInputOverlay::new(
            RequestId::Integer(16),
            ToolRequestUserInputParams {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                item_id: "question-1".to_string(),
                questions: vec![ToolRequestUserInputQuestion {
                    id: "mode".to_string(),
                    header: "Mode".to_string(),
                    question: "Choose a mode".to_string(),
                    is_other: false,
                    is_secret: false,
                    options: Some(vec![ToolRequestUserInputOption {
                        label: "Fast".to_string(),
                        description: "Continue immediately".to_string(),
                    }]),
                }],
                is_blocking: true,
                auto_resolution_ms: None,
            },
        );

        request.handle_key_event(key(KeyCode::Tab));
        request.composer.insert("discarded");
        assert_eq!(request.handle_key_event(key(KeyCode::Esc)), None);
        assert!(!request.editing);
        assert!(request.composer.is_empty());

        request.handle_key_event(key(KeyCode::Tab));
        assert_eq!(request.handle_key_event(key(KeyCode::Backspace)), None);
        assert!(!request.editing);
        assert!(request.composer.is_empty());
    }

    #[test]
    fn options_typing_does_not_open_notes() {
        let mut request = RequestUserInputOverlay::new(
            RequestId::Integer(17),
            ToolRequestUserInputParams {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                item_id: "question-1".to_string(),
                questions: vec![ToolRequestUserInputQuestion {
                    id: "mode".to_string(),
                    header: "Mode".to_string(),
                    question: "Choose a mode".to_string(),
                    is_other: false,
                    is_secret: false,
                    options: Some(vec![
                        ToolRequestUserInputOption {
                            label: "Fast".to_string(),
                            description: "Continue immediately".to_string(),
                        },
                        ToolRequestUserInputOption {
                            label: "Safe".to_string(),
                            description: "Continue carefully".to_string(),
                        },
                    ]),
                }],
                is_blocking: true,
                auto_resolution_ms: None,
            },
        );

        assert_eq!(request.handle_key_event(key(KeyCode::Char('x'))), None);
        assert!(!request.editing);
        assert!(request.composer.is_empty());
        assert_eq!(request.handle_key_event(key(KeyCode::Char('j'))), None);
        assert_eq!(request.selected, 1);
        assert!(!request.editing);
        assert_eq!(request.handle_key_event(key(KeyCode::Char('k'))), None);
        assert_eq!(request.selected, 0);
        assert_eq!(request.handle_key_event(key(KeyCode::Char(' '))), None);
        assert!(!request.editing);
    }

    #[test]
    fn control_j_and_k_navigate_options_without_opening_notes() {
        let mut request = RequestUserInputOverlay::new(
            RequestId::Integer(21),
            ToolRequestUserInputParams {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                item_id: "question-1".to_string(),
                questions: vec![ToolRequestUserInputQuestion {
                    id: "mode".to_string(),
                    header: "Mode".to_string(),
                    question: "Choose a mode".to_string(),
                    is_other: false,
                    is_secret: false,
                    options: Some(vec![
                        ToolRequestUserInputOption {
                            label: "Fast".to_string(),
                            description: "Continue immediately".to_string(),
                        },
                        ToolRequestUserInputOption {
                            label: "Safe".to_string(),
                            description: "Continue carefully".to_string(),
                        },
                    ]),
                }],
                is_blocking: true,
                auto_resolution_ms: None,
            },
        );

        assert_eq!(request.selected, 0);
        request.handle_key_event(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL));
        assert_eq!(request.selected, 1);
        request.handle_key_event(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL));
        assert_eq!(request.selected, 0);
        assert!(!request.editing);
    }

    #[test]
    fn option_navigation_wraps_at_both_ends() {
        let mut request = RequestUserInputOverlay::new(
            RequestId::Integer(22),
            ToolRequestUserInputParams {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                item_id: "question-1".to_string(),
                questions: vec![ToolRequestUserInputQuestion {
                    id: "mode".to_string(),
                    header: "Mode".to_string(),
                    question: "Choose a mode".to_string(),
                    is_other: false,
                    is_secret: false,
                    options: Some(vec![
                        ToolRequestUserInputOption {
                            label: "Fast".to_string(),
                            description: "Continue immediately".to_string(),
                        },
                        ToolRequestUserInputOption {
                            label: "Safe".to_string(),
                            description: "Continue carefully".to_string(),
                        },
                    ]),
                }],
                is_blocking: true,
                auto_resolution_ms: None,
            },
        );

        request.handle_key_event(key(KeyCode::Up));
        assert_eq!(request.selected, 1);
        request.handle_key_event(key(KeyCode::Down));
        assert_eq!(request.selected, 0);
    }

    fn non_blocking_request() -> ToolRequestUserInputParams {
        ToolRequestUserInputParams {
            thread_id: "thread-1".to_string(),
            turn_id: "turn-1".to_string(),
            item_id: "question-1".to_string(),
            questions: vec![ToolRequestUserInputQuestion {
                id: "mode".to_string(),
                header: "Mode".to_string(),
                question: "Choose a mode".to_string(),
                is_other: false,
                is_secret: false,
                options: Some(vec![ToolRequestUserInputOption {
                    label: "Fast".to_string(),
                    description: "Continue immediately".to_string(),
                }]),
            }],
            is_blocking: false,
            auto_resolution_ms: None,
        }
    }

    #[test]
    fn non_blocking_request_uses_hidden_grace_then_visible_countdown() {
        let request = RequestUserInputOverlay::new(RequestId::Integer(18), non_blocking_request());
        let started = request.request_started_at;

        assert_eq!(
            request.auto_resolution_timing_at(started),
            AutoResolutionTiming::HiddenGrace {
                remaining: AUTO_RESOLUTION_HIDDEN_GRACE
            }
        );
        let visible = started + AUTO_RESOLUTION_HIDDEN_GRACE;
        assert!(request
            .auto_resolution_countdown_text(visible, crate::locale::Locale::EnUs)
            .is_some());
        assert_eq!(
            request.next_frame_delay(visible),
            Some(Duration::from_secs(1))
        );
    }

    #[test]
    fn non_blocking_request_expires_with_empty_answers() {
        let mut request =
            RequestUserInputOverlay::new(RequestId::Integer(19), non_blocking_request());
        let due = request.request_started_at
            + AUTO_RESOLUTION_HIDDEN_GRACE
            + AUTO_RESOLUTION_VISIBLE_COUNTDOWN;
        let Some(AppServerResponse::UserInput { response, .. }) = request.pre_draw_tick(due) else {
            panic!("expected automatic user input response");
        };
        assert!(response.answers.is_empty());
        assert_eq!(request.next_frame_delay(due), Some(Duration::ZERO));
    }

    #[test]
    fn non_blocking_request_is_snoozed_by_user_input() {
        let mut request =
            RequestUserInputOverlay::new(RequestId::Integer(20), non_blocking_request());
        let due = request.request_started_at
            + AUTO_RESOLUTION_HIDDEN_GRACE
            + AUTO_RESOLUTION_VISIBLE_COUNTDOWN;
        assert_eq!(request.handle_key_event(key(KeyCode::Down)), None);
        assert_eq!(
            request.auto_resolution_timing_at(due),
            AutoResolutionTiming::Disabled
        );
        assert_eq!(request.pre_draw_tick(due), None);
    }

    #[test]
    fn narrow_footer_keeps_submit_and_cancel_before_secondary_hints() {
        let request = RequestUserInputOverlay::new(RequestId::Integer(21), non_blocking_request());
        let footer = request.footer_hint_for_width(crate::locale::Locale::EnUs, 28);

        assert!(footer.contains("Enter submit"));
        assert!(footer.contains("Esc cancel"));
        assert!(crate::width::display_width(&footer) <= 28);
    }

    #[test]
    fn ultra_narrow_footer_keeps_cancel_action_visible_without_overflow() {
        for locale in [
            crate::locale::Locale::ZhCn,
            crate::locale::Locale::ZhTw,
            crate::locale::Locale::EnUs,
            crate::locale::Locale::JaJp,
            crate::locale::Locale::KoKr,
        ] {
            let request =
                RequestUserInputOverlay::new(RequestId::Integer(23), non_blocking_request());
            for width in [3, 4, 7, 12, 18] {
                let footer = request.footer_hint_for_width(locale, width);
                assert!(
                    crate::width::display_width(&footer) <= width,
                    "{locale:?} at {width}: {footer:?}"
                );
                assert!(footer.contains("Esc"), "{locale:?} at {width}: {footer:?}");
            }
        }
    }
}
