mod agent_navigation;
pub(crate) mod agent_picker;
pub(crate) mod agents_overview;
pub(crate) mod agents_overview_threads;
pub(crate) mod agents_overview_view;
pub(crate) mod app_server_event_targets;
mod app_server_events;
pub(crate) mod app_server_requests;
pub(crate) mod event_dispatch;
mod input;
mod pending_interactive_replay;
pub(crate) mod reconnect;
mod replay_filter;
mod session_lifecycle;
mod thread_event_buffer;
mod thread_events;
mod thread_settings;

#[cfg(test)]
use app_server_protocol::protocol::v2::ServerNotification;
use app_server_protocol::protocol::v2::{QueuedSubmission, Thread};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use self::agent_navigation::{AgentNavigationDirection, AgentNavigationState};
use self::agent_picker::{AgentPicker, AgentPickerAction};
use self::agents_overview::AgentsOverviewState;
use self::agents_overview_view::AgentsOverviewAction;
use crate::bottom_pane::{AppServerResponse, BottomPane, ChatComposer, InputResult};
use crate::command_popup::{CommandPopup, CommandPopupAction};
use crate::locale::Locale;
use crate::model_catalog::ModelCatalog;
use crate::model_picker::{ModelPicker, ModelPickerAction, ModelSelection};
use crate::pager_overlay::{PagerAction, PagerOverlay, StatusFacts};
use crate::pending_input_preview::can_restore_submission;
use crate::projection::ConversationProjection;
use crate::resume_picker::{PickerAction, PickerState};
use crate::slash_command::{command_from_prompt, SlashCommand};
use crate::tui::TuiEvent;

fn normalize_paste(text: String) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

#[derive(Debug, PartialEq)]
pub(crate) enum AppAction {
    None,
    Submit(String),
    Queue(String),
    Interrupt,
    DecreaseEffort,
    IncreaseEffort,
    PreviousPermissions,
    NextPermissions,
    CopyLastResponse,
    PasteImage,
    EditQueuedSubmission(QueuedSubmission),
    OpenExternalEditor,
    ScrollUp,
    ScrollDown,
    ScrollTop,
    ScrollBottom,
    SelectModel(ModelSelection),
    ChangeCollaborationMode(agent_protocol::CollaborationMode),
    SwitchThread(String),
    RefreshAgentsOverview,
    DispatchAgentsOverviewTask {
        prompt: String,
        cwd: Option<PathBuf>,
    },
    RenameAgentsOverviewThread {
        thread_id: String,
        name: String,
    },
    StopAgentsOverviewThread {
        thread_id: String,
    },
    OpenResumePicker,
    ResumePicker(PickerAction),
    Respond(AppServerResponse),
    Quit,
}

#[derive(Debug, Default)]
pub(crate) struct App {
    pub(crate) bottom_pane: BottomPane,
    pub(crate) composer: ChatComposer,
    pub(crate) projection: ConversationProjection,
    pub(crate) model_picker: Option<ModelPicker>,
    pub(crate) agent_picker: Option<AgentPicker>,
    pub(crate) agents_overview: Option<AgentsOverviewState>,
    pub(crate) resume_picker: Option<PickerState>,
    pub(crate) model_catalog: ModelCatalog,
    pub(crate) collaboration_mode: Option<agent_protocol::CollaborationMode>,
    pub(crate) command_popup: Option<CommandPopup>,
    pub(crate) pager_overlay: Option<PagerOverlay>,
    pub(crate) thread_id: Option<String>,
    pub(crate) primary_thread_id: Option<String>,
    pub(crate) agent_navigation: AgentNavigationState,
    thread_event_channels: HashMap<String, self::thread_events::ThreadEventChannel>,
    pub(crate) model: Option<String>,
    pub(crate) model_provider: Option<String>,
    pub(crate) reasoning_effort: Option<String>,
    pub(crate) permissions: Option<String>,
    pub(crate) permission_profiles: Vec<String>,
    pub(crate) transcript_scroll: usize,
    pub(crate) locale: Locale,
    pub(crate) cwd: PathBuf,
    pub(crate) clipboard_lease: Option<crate::clipboard_copy::ClipboardLease>,
    pub(crate) queued_submissions: Vec<QueuedSubmission>,
    pub(crate) thread_input_states: HashMap<String, String>,
    active_turn_started_at: Option<Instant>,
}

impl App {
    pub(crate) fn set_cwd(&mut self, cwd: PathBuf) {
        self.cwd = cwd;
    }

    pub(crate) fn set_thread_id(&mut self, thread_id: String) {
        if self.thread_id.as_deref() != Some(thread_id.as_str()) {
            self.queued_submissions.clear();
        }
        if self.primary_thread_id.is_none() {
            self.primary_thread_id = Some(thread_id.clone());
        }
        if self.agent_navigation.get(&thread_id).is_none() {
            self.agent_navigation
                .upsert(thread_id.clone(), None, None, false);
        }
        self.ensure_thread_channel(&thread_id);
        self.thread_id = Some(thread_id);
    }

    pub(crate) fn capture_current_thread_input(&mut self) {
        let Some(thread_id) = self.thread_id.clone() else {
            return;
        };
        let draft = self.composer.text().to_string();
        self.thread_input_states
            .insert(thread_id.clone(), draft.clone());
        if let Some(overview) = self.agents_overview.as_mut() {
            overview.input_states.insert(thread_id, draft);
        }
    }

    pub(crate) fn restore_thread_input(&mut self, thread_id: &str) {
        let draft = self
            .thread_input_states
            .get(thread_id)
            .cloned()
            .or_else(|| {
                self.agents_overview
                    .as_ref()
                    .and_then(|overview| overview.input_states.get(thread_id))
                    .cloned()
            })
            .unwrap_or_default();
        self.composer.replace(draft);
        self.sync_command_popup();
    }

    pub(crate) fn set_locale(&mut self, locale: Locale) {
        self.locale = locale;
    }

    pub(crate) fn hydrate_thread(&mut self, thread: Thread) {
        self.agent_navigation.upsert(
            thread.id.clone(),
            thread.agent_nickname.clone(),
            thread.agent_role.clone(),
            false,
        );
        if let Some(parent_thread_id) = thread.parent_thread_id.clone() {
            self.agent_navigation.mark_parent_owned(thread.id.clone());
            if self.primary_thread_id.is_none() {
                self.primary_thread_id = Some(parent_thread_id);
            }
        }
        self.projection.hydrate_thread(thread);
        self.active_turn_started_at = self
            .projection
            .active_turn_id()
            .is_some()
            .then(Instant::now);
    }

    pub(crate) fn start_turn(&mut self, turn_id: String) {
        self.projection.start_turn(turn_id);
        self.active_turn_started_at = Some(Instant::now());
    }

    fn adjacent_agent(&self, direction: AgentNavigationDirection) -> Option<String> {
        self.agent_navigation
            .adjacent_thread_id(self.thread_id.as_deref(), direction)
            .filter(|thread_id| self.thread_id.as_deref() != Some(thread_id.as_str()))
    }

    pub(crate) fn active_turn_elapsed(&self, now: Instant) -> Option<Duration> {
        self.projection.active_turn_id()?;
        Some(
            self.active_turn_started_at
                .map(|started_at| now.saturating_duration_since(started_at))
                .unwrap_or_default(),
        )
    }

    pub(crate) fn can_accept_direct_input(&mut self) -> bool {
        if self
            .thread_id
            .as_deref()
            .is_some_and(|thread_id| self.agent_navigation.is_parent_owned(thread_id))
        {
            self.projection
                .set_status("sub-agent thread is parent-owned");
            return false;
        }
        true
    }

    pub(crate) fn replace_composer(&mut self, text: String) {
        self.composer.replace(text);
        self.sync_command_popup();
    }

    pub(crate) fn handle_tui_event(&mut self, event: TuiEvent, connected: bool) -> AppAction {
        if !connected {
            return match event {
                TuiEvent::Key(key)
                    if key.kind == KeyEventKind::Press
                        && key.modifiers.contains(KeyModifiers::CONTROL)
                        && matches!(key.code, KeyCode::Char(value) if value.eq_ignore_ascii_case(&'c')) =>
                {
                    AppAction::Quit
                }
                TuiEvent::Key(key) => {
                    self.composer.handle_disconnected_key(key);
                    self.command_popup = None;
                    AppAction::None
                }
                TuiEvent::Paste(text) => {
                    self.composer.insert(&normalize_paste(text));
                    self.command_popup = None;
                    AppAction::None
                }
                _ => AppAction::None,
            };
        }

        let event = match event {
            TuiEvent::Key(key) => Event::Key(key),
            TuiEvent::Paste(text) => Event::Paste(normalize_paste(text)),
            TuiEvent::Resize(size) => Event::Resize(size.width, size.height),
            TuiEvent::FocusGained => Event::FocusGained,
            TuiEvent::FocusLost => Event::FocusLost,
            TuiEvent::Draw | TuiEvent::Resume => return AppAction::None,
        };

        if let Some(pager) = self.pager_overlay.as_mut() {
            if pager.handle_event(&event) == PagerAction::Close {
                self.pager_overlay = None;
            }
            return AppAction::None;
        }

        if self.bottom_pane.is_active() {
            return self
                .bottom_pane
                .handle_event(event)
                .map(AppAction::Respond)
                .unwrap_or(AppAction::None);
        }

        if let Some(picker) = self.resume_picker.as_mut() {
            if picker.transcript_pager_is_open() {
                picker.handle_transcript_pager_event(&event);
                return AppAction::None;
            }
            let action = picker.handle_event(event);
            if action == PickerAction::Cancel {
                self.resume_picker = None;
                return AppAction::None;
            }
            return AppAction::ResumePicker(action);
        }

        if let Some(overview) = self.agents_overview.as_mut() {
            let action = overview.view.handle_event(event);
            overview.visible_thread_ids = overview
                .view
                .visible_rows()
                .into_iter()
                .map(|row| row.thread.id.clone())
                .collect();
            overview.sync_view_state();
            return match action {
                AgentsOverviewAction::Select => {
                    let thread_id = overview.view.selected_thread_id().map(str::to_owned);
                    self.agents_overview = None;
                    self.agent_picker = None;
                    thread_id
                        .map(AppAction::SwitchThread)
                        .unwrap_or(AppAction::None)
                }
                AgentsOverviewAction::Cancel => {
                    self.agents_overview = None;
                    self.agent_picker = None;
                    AppAction::None
                }
                AgentsOverviewAction::Refresh => AppAction::RefreshAgentsOverview,
                AgentsOverviewAction::Dispatch { prompt, cwd } => {
                    AppAction::DispatchAgentsOverviewTask { prompt, cwd }
                }
                AgentsOverviewAction::Rename { thread_id, name } => {
                    AppAction::RenameAgentsOverviewThread { thread_id, name }
                }
                AgentsOverviewAction::Stop { thread_id } => {
                    AppAction::StopAgentsOverviewThread { thread_id }
                }
                AgentsOverviewAction::OpenResumePicker => AppAction::OpenResumePicker,
                AgentsOverviewAction::None => AppAction::None,
            };
        }

        if let Some(picker) = self.model_picker.as_mut() {
            return match picker.handle_event(event) {
                ModelPickerAction::Select(index) => {
                    let selection = picker.selected_model(index);
                    self.model_picker = None;
                    selection
                        .map(AppAction::SelectModel)
                        .unwrap_or(AppAction::None)
                }
                ModelPickerAction::Cancel => {
                    self.model_picker = None;
                    AppAction::None
                }
                ModelPickerAction::None => AppAction::None,
            };
        }

        if let Some(picker) = self.agent_picker.as_mut() {
            return match picker.handle_event(event) {
                AgentPickerAction::Select(thread_id) => {
                    self.agent_picker = None;
                    AppAction::SwitchThread(thread_id)
                }
                AgentPickerAction::Cancel => {
                    self.agent_picker = None;
                    AppAction::None
                }
                AgentPickerAction::None => AppAction::None,
            };
        }

        if let Some(popup) = self.command_popup.as_mut() {
            match popup.handle_event(&event) {
                CommandPopupAction::Pass => {}
                CommandPopupAction::Consumed => return AppAction::None,
                CommandPopupAction::Cancel => {
                    self.command_popup = None;
                    return AppAction::None;
                }
                CommandPopupAction::Complete(command) => {
                    self.complete_slash_command(command);
                    return AppAction::None;
                }
                CommandPopupAction::Execute(command) => {
                    self.composer.replace(format!("/{}", command.command()));
                    self.command_popup = None;
                    if let Some(action) = self.run_local_command() {
                        return action;
                    }
                    let action = self
                        .composer
                        .handle_key_event(crossterm::event::KeyEvent::new(
                            KeyCode::Enter,
                            KeyModifiers::NONE,
                        ));
                    return self.map_composer_action(action);
                }
            }
        }

        if self.composer.history_search_active() {
            if let Event::Key(key) = event {
                let action = self.composer.handle_key_event(key);
                return self.map_composer_action(action);
            }
            return AppAction::None;
        }

        match event {
            Event::Key(key) => self.handle_key_event(key),
            Event::Paste(text) => {
                self.composer.insert(&text);
                self.sync_command_popup();
                AppAction::None
            }
            _ => AppAction::None,
        }
    }

    pub(crate) fn scroll_up(&mut self, amount: usize) {
        self.transcript_scroll = self.transcript_scroll.saturating_add(amount);
    }

    pub(crate) fn scroll_down(&mut self, amount: usize) {
        self.transcript_scroll = self.transcript_scroll.saturating_sub(amount);
    }

    pub(crate) fn scroll_top(&mut self) {
        self.transcript_scroll = usize::MAX;
    }

    pub(crate) fn scroll_bottom(&mut self) {
        self.transcript_scroll = 0;
    }

    pub(crate) fn attach_image(&mut self, path: PathBuf) {
        self.composer.attach_image(path);
    }

    pub(crate) fn take_pending_images(&mut self) -> Vec<PathBuf> {
        self.composer.take_pending_images()
    }

    pub(crate) fn restore_pending_images(&mut self, images: Vec<PathBuf>) {
        self.composer.restore_pending_images(images);
    }

    pub(crate) fn set_queued_submissions(&mut self, submissions: Vec<QueuedSubmission>) {
        self.queued_submissions = submissions;
    }

    pub(crate) fn upsert_queued_submission(&mut self, submission: QueuedSubmission) {
        if let Some(existing) = self
            .queued_submissions
            .iter_mut()
            .find(|existing| existing.id == submission.id)
        {
            *existing = submission;
        } else {
            self.queued_submissions.push(submission);
        }
    }

    pub(crate) fn restore_queued_submission_for_edit(
        &mut self,
        submission: QueuedSubmission,
    ) -> bool {
        if !self.composer.is_empty()
            || self.composer.has_pending_images()
            || !can_restore_submission(&submission)
        {
            return false;
        }
        let submission_id = submission.id.clone();
        let mut text = String::new();
        let mut images = Vec::new();
        for input in submission.input {
            match input {
                app_server_protocol::protocol::v2::UserInput::Text { text: value, .. } => {
                    text = value;
                }
                app_server_protocol::protocol::v2::UserInput::LocalImage { path, .. } => {
                    images.push(PathBuf::from(path));
                }
                _ => return false,
            }
        }
        self.queued_submissions
            .retain(|queued| queued.id != submission_id);
        self.replace_composer(text);
        self.composer.restore_pending_images(images);
        true
    }

    fn map_composer_action(&mut self, action: InputResult) -> AppAction {
        match action {
            InputResult::Submitted(text) => {
                self.command_popup = None;
                AppAction::Submit(text)
            }
            InputResult::Queued(text) => {
                self.command_popup = None;
                AppAction::Queue(text)
            }
            InputResult::Interrupt => AppAction::Interrupt,
            InputResult::DecreaseEffort => AppAction::DecreaseEffort,
            InputResult::IncreaseEffort => AppAction::IncreaseEffort,
            InputResult::PreviousPermissions => AppAction::PreviousPermissions,
            InputResult::NextPermissions => AppAction::NextPermissions,
            InputResult::OpenExternalEditor => AppAction::OpenExternalEditor,
            InputResult::Quit => AppAction::Quit,
            InputResult::Changed => {
                if self.composer.history_search_active() {
                    self.command_popup = None;
                } else {
                    self.sync_command_popup();
                }
                AppAction::None
            }
            InputResult::None => AppAction::None,
        }
    }

    fn complete_slash_command(&mut self, command: SlashCommand) {
        let suffix = if command.requires_argument() { " " } else { "" };
        self.composer
            .replace(format!("/{}{suffix}", command.command()));
        self.command_popup = None;
    }

    fn sync_command_popup(&mut self) {
        if self
            .command_popup
            .as_mut()
            .is_some_and(|popup| popup.update(self.composer.text()))
        {
            return;
        }
        self.command_popup = CommandPopup::for_composer(self.composer.text());
    }

    fn run_local_command(&mut self) -> Option<AppAction> {
        if self.composer.text().split_whitespace().count() != 1 {
            return None;
        }
        let command = command_from_prompt(self.composer.text())?;
        let action = match command {
            SlashCommand::Status => {
                self.open_status_pager();
                AppAction::None
            }
            SlashCommand::Copy => AppAction::CopyLastResponse,
            SlashCommand::Agents => {
                self.open_agents_overview();
                AppAction::RefreshAgentsOverview
            }
            SlashCommand::MultiAgents => {
                self.open_agent_picker();
                AppAction::None
            }
            SlashCommand::Resume => AppAction::OpenResumePicker,
            _ => return None,
        };
        self.composer.replace(String::new());
        self.command_popup = None;
        Some(action)
    }

    fn open_status_pager(&mut self) {
        let cwd = self.cwd.to_string_lossy();
        self.pager_overlay = Some(PagerOverlay::status(
            self.locale,
            StatusFacts {
                thread_id: self.thread_id.as_deref(),
                model: self.model.as_deref(),
                provider: self.model_provider.as_deref(),
                effort: self.reasoning_effort.as_deref(),
                permissions: self.permissions.as_deref(),
                cwd: &cwd,
                status: self.projection.status(),
            },
        ));
    }
}

#[cfg(test)]
mod tests;
