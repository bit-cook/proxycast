mod agent_navigation;
pub(crate) mod agent_picker;
pub(crate) mod agents_overview;
pub(crate) mod agents_overview_threads;
pub(crate) mod agents_overview_view;
pub(crate) mod app_server_event_targets;
mod app_server_events;
pub(crate) mod app_server_requests;
pub(crate) mod event_dispatch;
pub(crate) mod history_pagination;
pub(crate) mod history_ui;
mod input;
mod pending_interactive_replay;
pub(crate) mod reconnect;
mod replay_filter;
mod session_lifecycle;
pub(crate) mod startup;
#[allow(dead_code)]
pub(crate) mod startup_prompts;
mod thread_event_buffer;
mod thread_events;
mod thread_settings;
pub(crate) mod transcript_export;
pub(crate) mod working_directory;

use app_server_protocol::protocol::v2::{QueuedSubmission, Thread};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use self::agent_navigation::{AgentNavigationDirection, AgentNavigationState};
use self::agent_picker::{AgentPicker, AgentPickerAction};
use self::agents_overview::AgentsOverviewState;
use self::agents_overview_view::AgentsOverviewAction;
use self::transcript_export::{ExportPicker, ExportPickerAction};
use crate::bottom_pane::{AppServerResponse, BottomPane, ChatComposer, InputResult};
use crate::command_popup::CommandPopupAction;
use crate::locale::Locale;
use crate::model_catalog::ModelCatalog;
use crate::model_picker::{ModelPicker, ModelPickerAction, ModelSelection};
use crate::pager_overlay::{PagerAction, PagerOverlay, StatusFacts};
use crate::pending_input_preview::can_restore_submission;
use crate::projection::ConversationProjection;
use crate::resume_picker::{PickerAction, PickerState};
use crate::slash_command::SlashCommand;
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
    ExportTranscript {
        path: Option<PathBuf>,
    },
    PasteImage,
    EditQueuedSubmission(QueuedSubmission),
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ExternalEditorState {
    #[default]
    Closed,
    Requested,
    Active,
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
    pub(crate) skill_load_warnings: startup_prompts::SkillLoadWarningState,
    pub(crate) mcp_startup_warnings: startup_prompts::McpStartupWarningState,
    pub(crate) collaboration_mode: Option<agent_protocol::CollaborationMode>,
    pub(crate) pager_overlay: Option<PagerOverlay>,
    pub(crate) export_picker: Option<ExportPicker>,
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
    pub(crate) scrollback_has_older_history: bool,
    pub(crate) locale: Locale,
    pub(crate) cwd: PathBuf,
    pub(crate) clipboard_lease: Option<crate::clipboard_copy::ClipboardLease>,
    pub(crate) queued_submissions: Vec<QueuedSubmission>,
    pub(crate) thread_input_states: HashMap<String, String>,
    /// Keeps terminal input behind a startup request that may open a protected interaction.
    ///
    /// The App Server stream can deliver an approval or user-input request immediately after the
    /// initial thread handshake. Codex quarantines terminal input until that request is visible;
    /// Lime keeps the same boundary while leaving request ownership in `BottomPane`.
    pub(crate) startup_protected_input_boundary: bool,
    pub(crate) startup_pending_protected_request: bool,
    external_editor_state: ExternalEditorState,
    active_turn_started_at: Option<Instant>,
}

impl App {
    pub(crate) fn set_cwd(&mut self, cwd: PathBuf) {
        self.cwd = cwd;
    }

    /// Enable the startup input boundary before the terminal event loop begins.
    pub(crate) fn begin_startup_input_boundary(&mut self) {
        self.startup_protected_input_boundary = true;
        self.startup_pending_protected_request = false;
    }

    /// Returns whether a startup request is waiting in the active pane or a thread buffer.
    ///
    /// Requests are never dropped to make room for ordinary notifications, so this check is
    /// deterministic and does not require a second runtime or a local persistence store.
    pub(crate) fn has_queued_startup_protected_request(&self) -> bool {
        let active_thread_has_buffered_request = self
            .thread_id
            .as_deref()
            .and_then(|thread_id| self.thread_event_channels.get(thread_id))
            .is_some_and(|channel| {
                channel.store.buffer.iter().any(|event| {
                    matches!(event, self::thread_events::ThreadBufferedEvent::Request(_))
                })
            });
        self.startup_protected_input_boundary
            && (self.startup_pending_protected_request || active_thread_has_buffered_request)
    }

    pub(crate) fn note_startup_protected_request(&mut self) {
        if self.startup_protected_input_boundary {
            self.startup_pending_protected_request = true;
        }
    }

    pub(crate) fn end_startup_input_boundary(&mut self) {
        self.startup_protected_input_boundary = false;
        self.startup_pending_protected_request = false;
    }

    /// Release the startup input boundary once the first ordinary input is safe to process.
    ///
    /// Codex keeps startup protection until queued app events and interactive requests have been
    /// drained. The first key or paste event after that point ends the startup-only phase; later
    /// requests are handled by the normal BottomPane lifecycle.
    pub(crate) fn release_startup_input_boundary_if_ready(&mut self, user_input: bool) -> bool {
        if !user_input
            || !self.startup_protected_input_boundary
            || self.bottom_pane.is_active()
            || self.has_queued_startup_protected_request()
        {
            return false;
        }
        self.end_startup_input_boundary();
        true
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

    /// Return the user-visible status while keeping active-turn and explicit command status ahead
    /// of app-scoped MCP startup diagnostics.
    pub(crate) fn status_value(&self) -> String {
        let status = self.projection.status();
        if matches!(status, "" | "ready") {
            return self
                .mcp_startup_warnings
                .status()
                .unwrap_or_else(|| status.to_string());
        }
        status.to_string()
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
        self.scrollback_has_older_history = false;
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

    pub(crate) fn external_editor_state(&self) -> ExternalEditorState {
        self.external_editor_state
    }

    pub(crate) fn request_external_editor_launch(&mut self) {
        if self.external_editor_state == ExternalEditorState::Closed {
            self.external_editor_state = ExternalEditorState::Requested;
        }
    }

    pub(crate) fn set_external_editor_state(&mut self, state: ExternalEditorState) {
        self.external_editor_state = state;
    }

    pub(crate) fn reset_external_editor_state(&mut self) {
        self.external_editor_state = ExternalEditorState::Closed;
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
                    self.clear_command_popup();
                    AppAction::None
                }
                TuiEvent::Paste(text) => {
                    self.composer.handle_paste(&normalize_paste(text));
                    self.clear_command_popup();
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

        if let Some(picker) = self.export_picker.as_mut() {
            let action = picker.handle_event(&event);
            return match action {
                ExportPickerAction::None => AppAction::None,
                ExportPickerAction::Cancel => {
                    self.export_picker = None;
                    AppAction::None
                }
                ExportPickerAction::Copy => {
                    self.export_picker = None;
                    AppAction::ExportTranscript { path: None }
                }
                ExportPickerAction::Save => {
                    let path = picker.selected_path();
                    self.export_picker = None;
                    path.map(|path| AppAction::ExportTranscript { path: Some(path) })
                        .unwrap_or(AppAction::None)
                }
            };
        }

        if self.bottom_pane.is_active() {
            let action = self
                .bottom_pane
                .handle_event(event)
                .map(AppAction::Respond)
                .unwrap_or(AppAction::None);
            if matches!(action, AppAction::Respond(_)) && !self.bottom_pane.is_active() {
                self.startup_pending_protected_request = false;
            }
            return action;
        }

        // A delayed startup approval/user-input request owns the terminal until it is shown.
        // This guard intentionally sits after `BottomPane`: once visible, the pane must receive
        // the key that resolves the request instead of being blocked by its own boundary.
        if self.has_queued_startup_protected_request() {
            return AppAction::None;
        }

        self.release_startup_input_boundary_if_ready(matches!(
            &event,
            Event::Key(_) | Event::Paste(_)
        ));

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

        // Vim query input is owned by the composer and must precede popups, global shortcuts,
        // and submission handling. Query paste edits the ephemeral query editor, never the draft.
        let vim_query_owns_event = self.composer.vim_search_active()
            || matches!(&event, Event::Key(key) if self.composer.vim_search_wants_key(*key));
        if vim_query_owns_event {
            match event {
                Event::Key(key) => {
                    let action = self.composer.handle_key_event(key);
                    return self.map_composer_action(action);
                }
                Event::Paste(text) => {
                    self.composer.handle_paste(&text);
                    return AppAction::None;
                }
                _ => {}
            }
        }

        if self.composer.file_search_popup_active() {
            let action = self.composer.handle_file_search_popup_event(&event);
            match action {
                crate::bottom_pane::FileSearchPopupAction::Pass => {}
                crate::bottom_pane::FileSearchPopupAction::Consumed => {
                    if !matches!(
                        event,
                        Event::Key(key) if key.code == KeyCode::Enter
                    ) {
                        return AppAction::None;
                    }
                }
                crate::bottom_pane::FileSearchPopupAction::Cancel
                | crate::bottom_pane::FileSearchPopupAction::Complete => {
                    return AppAction::None;
                }
            }
        }

        if self.composer.skill_popup_active() {
            let action = self.composer.handle_skill_popup_event(&event);
            match action {
                crate::bottom_pane::SkillPopupAction::Pass => {}
                crate::bottom_pane::SkillPopupAction::Consumed => return AppAction::None,
                crate::bottom_pane::SkillPopupAction::Cancel
                | crate::bottom_pane::SkillPopupAction::Complete => return AppAction::None,
            }
        }

        if self.composer.command_popup_active() {
            let action = self.composer.handle_command_popup_event(&event);
            match action {
                CommandPopupAction::Pass => {}
                CommandPopupAction::Consumed => return AppAction::None,
                CommandPopupAction::Cancel => {
                    return AppAction::None;
                }
                CommandPopupAction::Complete(command) => {
                    self.complete_slash_command(command);
                    return AppAction::None;
                }
                CommandPopupAction::Execute(command) => {
                    self.composer.replace(format!("/{}", command.command()));
                    self.clear_command_popup();
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

        if let Event::Key(key) = event {
            if self.composer.should_handle_vim_insert_escape(key) {
                let action = self.composer.handle_key_event(key);
                return self.map_composer_action(action);
            }
        }

        match event {
            Event::Key(key) => self.handle_key_event(key),
            Event::Paste(text) => {
                self.composer.handle_paste(&text);
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

    pub(crate) fn take_remote_image_urls(&mut self) -> Vec<String> {
        self.composer.take_remote_image_urls()
    }

    pub(crate) fn set_remote_image_urls(&mut self, urls: Vec<String>) {
        self.composer.set_remote_image_urls(urls);
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
        let mut local_images = Vec::new();
        let mut remote_images = Vec::new();
        let mut skills = Vec::new();
        for input in submission.input {
            match input {
                app_server_protocol::protocol::v2::UserInput::Text { text: value, .. } => {
                    text = value;
                }
                app_server_protocol::protocol::v2::UserInput::LocalImage { path, .. } => {
                    local_images.push(PathBuf::from(path));
                }
                app_server_protocol::protocol::v2::UserInput::Image { url, .. } => {
                    remote_images.push(url);
                }
                app_server_protocol::protocol::v2::UserInput::Skill { name, .. } => {
                    skills.push(format!("${name}"));
                }
                _ => return false,
            }
        }
        self.queued_submissions
            .retain(|queued| queued.id != submission_id);
        if !skills.is_empty() {
            let prefix = skills.join(" ");
            text = if text.is_empty() {
                prefix
            } else {
                format!("{prefix} {text}")
            };
        }
        self.replace_composer(text);
        self.composer.restore_pending_images(local_images);
        self.composer.set_remote_image_urls(remote_images);
        self.clear_command_popup();
        true
    }

    fn map_composer_action(&mut self, action: InputResult) -> AppAction {
        match action {
            InputResult::Submitted(text) => {
                self.clear_command_popup();
                AppAction::Submit(text)
            }
            InputResult::Queued(text) => {
                self.clear_command_popup();
                AppAction::Queue(text)
            }
            InputResult::Interrupt => {
                let cleared = self.composer.clear_for_ctrl_c().is_some();
                if cleared {
                    self.clear_command_popup();
                    // Codex treats Ctrl-C as composer cancellation when a draft is present.
                    // Do not also interrupt the active turn: a follow-up Ctrl-C can then be
                    // handled after the terminal projection settles.
                    AppAction::None
                } else {
                    AppAction::Interrupt
                }
            }
            InputResult::DecreaseEffort => AppAction::DecreaseEffort,
            InputResult::IncreaseEffort => AppAction::IncreaseEffort,
            InputResult::PreviousPermissions => AppAction::PreviousPermissions,
            InputResult::NextPermissions => AppAction::NextPermissions,
            InputResult::OpenExternalEditor => {
                self.request_external_editor_launch();
                AppAction::None
            }
            InputResult::Quit => AppAction::Quit,
            InputResult::Changed => {
                if self.composer.history_search_active() || self.composer.vim_search_active() {
                    self.clear_command_popup();
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
        self.clear_command_popup();
    }

    fn sync_command_popup(&mut self) {
        self.composer.sync_command_popup();
    }

    fn clear_command_popup(&mut self) {
        self.composer.clear_command_popup();
    }

    fn run_local_command(&mut self) -> Option<AppAction> {
        let text = self.composer.text().trim();
        let command = self.composer.command_from_prompt(text)?;
        let action = match command {
            SlashCommand::Vim => {
                let enabled = self.composer.toggle_vim_enabled();
                self.projection
                    .set_status(self.locale.vim_mode_message(enabled));
                AppAction::None
            }
            SlashCommand::Status => {
                self.open_status_pager();
                AppAction::None
            }
            SlashCommand::Copy => AppAction::CopyLastResponse,
            SlashCommand::Export => {
                let path = text
                    .strip_prefix("/export")
                    .map(str::trim)
                    .filter(|path| !path.is_empty())
                    .map(PathBuf::from);
                if path.is_none() {
                    self.export_picker = Some(ExportPicker::new(self.thread_id.as_deref()));
                    AppAction::None
                } else {
                    AppAction::ExportTranscript { path }
                }
            }
            SlashCommand::Agents => {
                self.open_agents_overview();
                AppAction::RefreshAgentsOverview
            }
            SlashCommand::MultiAgents => {
                self.open_agent_picker();
                AppAction::None
            }
            SlashCommand::Resume => AppAction::OpenResumePicker,
            SlashCommand::Pwd => {
                if text.split_whitespace().count() != 1 {
                    self.projection.set_status(self.locale.pwd_usage());
                } else {
                    let cwd = self.cwd.to_string_lossy();
                    self.projection
                        .set_status(self.locale.current_working_directory_message(&cwd));
                }
                AppAction::None
            }
            _ => return None,
        };
        self.composer.replace(String::new());
        self.clear_command_popup();
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
                status: &self.status_value(),
            },
        ));
    }
}

#[cfg(test)]
mod tests;
