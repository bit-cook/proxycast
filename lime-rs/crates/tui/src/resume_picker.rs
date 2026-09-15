use anyhow::{Context, Result};
use app_server_client::RequestHandle;
use app_server_protocol::protocol::v2::{
    SortDirection, Thread, ThreadArchiveParams, ThreadHistoryMode, ThreadListCwdFilter,
    ThreadListParams, ThreadListResponse, ThreadSortKey, ThreadStatus, ThreadUnarchiveParams,
    ThreadUnarchiveResponse, METHOD_THREAD_ARCHIVE, METHOD_THREAD_UNARCHIVE,
};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use futures::StreamExt;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use tokio::sync::mpsc;

use crate::app::history_ui::render_transcript_entry_lines_wrapped;
use crate::clipboard_paste::normalize_pasted_search_query;
use crate::entry;
use crate::locale::Locale;
use crate::pager_overlay::{PagerAction, PagerOverlay};
#[cfg(test)]
use crate::projection::EntryKind;
use crate::projection::TranscriptEntry;
use crate::runtime::{connect_session, TuiOptions};
use crate::terminal_hyperlinks::HyperlinkLine;
use crate::text_formatting::center_truncate_path;
use crate::tui::{Tui, TuiEvent};
use crate::width::display_width;

mod archive;
mod page_loading;

use page_loading::{PageCursor, PageLoadMode, PaginationState};

#[path = "resume_picker_transcript_preview.rs"]
mod transcript_preview;

const MAX_THREADS: u32 = 100;
const PICKER_LIST_HORIZONTAL_INSET: u16 = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct SessionTarget {
    pub(crate) path: Option<PathBuf>,
    pub(crate) thread_id: String,
    pub(crate) history_mode: Option<ThreadHistoryMode>,
}

#[allow(dead_code)]
impl SessionTarget {
    pub(crate) fn display_label(&self) -> String {
        self.path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| format!("thread {}", self.thread_id))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum SessionSelection {
    StartFresh,
    AgentsOverview,
    Resume(SessionTarget),
    Fork(SessionTarget),
    Exit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum SessionPickerAction {
    Resume,
    Fork,
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum SessionPickerLaunchContext {
    Startup,
    ExistingSession { current_thread_id: Option<String> },
}

#[allow(dead_code)]
impl SessionPickerAction {
    pub(crate) fn title(self) -> &'static str {
        match self {
            Self::Resume => "Resume a previous session",
            Self::Fork => "Fork a previous session",
        }
    }

    pub(crate) fn action_label(self) -> &'static str {
        match self {
            Self::Resume => "resume",
            Self::Fork => "fork",
        }
    }

    pub(crate) fn selection(self, target_session: SessionTarget) -> SessionSelection {
        match self {
            Self::Resume => SessionSelection::Resume(target_session),
            Self::Fork => SessionSelection::Fork(target_session),
        }
    }
}

/// Codex-shaped resume picker entry point backed by App Server's Thread list.
pub(crate) async fn run_resume_picker_with_app_server(
    options: &TuiOptions,
) -> Result<Option<String>> {
    run_session_picker_with_action(options, SessionPickerAction::Resume).await
}

#[allow(dead_code)]
pub(crate) async fn run_fork_picker_with_app_server(
    options: &TuiOptions,
) -> Result<Option<String>> {
    let source = run_session_picker_with_action(options, SessionPickerAction::Fork).await?;
    let Some(source) = source else {
        return Ok(None);
    };
    let session = connect_session(options).await?;
    let result = session
        .fork_thread(
            source,
            Some(options.cwd.clone()),
            options.model.clone(),
            options.model_provider.clone(),
        )
        .await;
    let shutdown = session.shutdown().await;
    let response = result?;
    shutdown?;
    Ok(Some(response.thread.id))
}

#[derive(Debug)]
pub(crate) struct ThreadPage {
    threads: Vec<Thread>,
    next_cursor: Option<String>,
}

async fn load_thread_page_with_handle(
    request_handle: RequestHandle,
    status: SessionStatus,
    filter_cwd: Option<&std::path::Path>,
    query: &str,
    sort_key: ThreadSortKey,
    cursor: Option<String>,
) -> Result<ThreadPage> {
    // Lime's current App Server owns the canonical index; do not switch to
    // Codex's private local state DB mode for later pages.
    let page: ThreadListResponse = request_handle
        .request(
            app_server_protocol::protocol::v2::METHOD_THREAD_LIST,
            ThreadListParams {
                cursor,
                limit: Some(MAX_THREADS),
                sort_key: Some(sort_key),
                sort_direction: Some(SortDirection::Desc),
                archived: Some(status == SessionStatus::Archived),
                cwd: filter_cwd
                    .map(|cwd| ThreadListCwdFilter::One(cwd.to_string_lossy().into_owned())),
                search_term: (!query.trim().is_empty()).then(|| query.trim().to_string()),
                ..ThreadListParams::default()
            },
        )
        .await?;
    Ok(ThreadPage {
        threads: filter_threads(page.data, query),
        next_cursor: page.next_cursor,
    })
}

fn filter_threads(mut threads: Vec<Thread>, query: &str) -> Vec<Thread> {
    threads.retain(|thread| !thread.ephemeral);
    let query = query.trim().to_ascii_lowercase();
    if query.is_empty() {
        return threads;
    }
    threads
        .into_iter()
        .filter(|thread| {
            thread
                .name
                .as_deref()
                .unwrap_or_default()
                .to_ascii_lowercase()
                .contains(&query)
                || thread.preview.to_ascii_lowercase().contains(&query)
                || thread
                    .cwd
                    .to_string_lossy()
                    .to_ascii_lowercase()
                    .contains(&query)
                || thread.id.to_ascii_lowercase().contains(&query)
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PickerAction {
    None,
    Reload,
    MoveUp,
    MoveDown,
    Select,
    Archive,
    Restore,
    ToggleStatus,
    ToggleFilter,
    ToggleSort,
    ToggleDensity,
    ToggleExpanded,
    OpenTranscript,
    Cancel,
}

pub(crate) enum PickerLoadEvent {
    Threads {
        token: usize,
        result: Result<ThreadPage>,
    },
    Preview {
        thread_id: String,
        result: std::io::Result<Vec<transcript_preview::TranscriptPreviewLine>>,
    },
    Transcript {
        thread_id: String,
        result: std::io::Result<Vec<TranscriptEntry>>,
    },
    Archive {
        thread_id: String,
        result: Result<()>,
    },
    Unarchive {
        thread_id: String,
        result: Box<Result<ThreadUnarchiveResponse>>,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum SessionStatus {
    #[default]
    Active,
    Archived,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SessionListDensity {
    #[default]
    Comfortable,
    Dense,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SessionTranscriptState {
    Loading,
    Loaded(Vec<TranscriptEntry>),
    Failed,
}

#[derive(Debug)]
struct PickerTranscriptPager {
    thread_id: String,
    overlay: PagerOverlay,
}

impl PickerTranscriptPager {
    fn new(thread_id: String, locale: Locale) -> Self {
        Self {
            thread_id,
            overlay: PagerOverlay::transcript(locale),
        }
    }
}

impl SessionListDensity {
    fn toggle(self) -> Self {
        match self {
            Self::Comfortable => Self::Dense,
            Self::Dense => Self::Comfortable,
        }
    }
}

#[derive(Debug)]
pub(crate) struct PickerState {
    action: SessionPickerAction,
    pub(crate) threads: Vec<Thread>,
    selected: usize,
    query: String,
    status: SessionStatus,
    filter_cwd: Option<PathBuf>,
    show_all: bool,
    sort_key: ThreadSortKey,
    density: SessionListDensity,
    transcript_previews: HashMap<String, Vec<transcript_preview::TranscriptPreviewLine>>,
    preview_loading: HashSet<String>,
    expanded_thread_id: Option<String>,
    transcripts: HashMap<String, SessionTranscriptState>,
    transcript_pager: Option<PickerTranscriptPager>,
    pagination: PaginationState,
    seen_cursors: HashSet<String>,
    archive_state: archive::ArchiveState,
    status_message: Option<String>,
    loading: bool,
    pub(crate) load_token: usize,
}

impl PickerState {
    pub(crate) fn new(
        threads: Vec<Thread>,
        action: SessionPickerAction,
        status: SessionStatus,
        filter_cwd: Option<PathBuf>,
        show_all: bool,
    ) -> Self {
        Self {
            action,
            threads: filter_threads(threads, ""),
            selected: 0,
            query: String::new(),
            status,
            filter_cwd,
            show_all,
            sort_key: ThreadSortKey::UpdatedAt,
            density: SessionListDensity::Comfortable,
            transcript_previews: HashMap::new(),
            preview_loading: HashSet::new(),
            expanded_thread_id: None,
            transcripts: HashMap::new(),
            transcript_pager: None,
            pagination: PaginationState::new(),
            seen_cursors: HashSet::new(),
            archive_state: archive::ArchiveState::Idle,
            status_message: None,
            loading: false,
            load_token: 0,
        }
    }

    pub(crate) fn selected_thread_id(&self) -> Option<&str> {
        self.threads
            .get(self.selected)
            .map(|thread| thread.id.as_str())
    }

    pub(crate) fn set_transcript_preview(
        &mut self,
        thread_id: String,
        preview: Vec<transcript_preview::TranscriptPreviewLine>,
    ) {
        self.preview_loading.remove(&thread_id);
        self.transcript_previews.insert(thread_id, preview);
    }

    fn preview_needs_load(&self, thread_id: &str) -> bool {
        !self.transcript_previews.contains_key(thread_id)
            && !self.preview_loading.contains(thread_id)
    }

    fn mark_preview_loading(&mut self, thread_id: impl Into<String>) {
        self.preview_loading.insert(thread_id.into());
    }

    pub(crate) fn toggle_selected_expansion(&mut self) -> Option<String> {
        let thread_id = self.selected_thread_id()?.to_string();
        if self.expanded_thread_id.as_deref() == Some(thread_id.as_str()) {
            self.expanded_thread_id = None;
            return None;
        }
        self.expanded_thread_id = Some(thread_id.clone());
        if !matches!(
            self.transcripts.get(&thread_id),
            Some(SessionTranscriptState::Loaded(_))
        ) {
            self.transcripts
                .insert(thread_id.clone(), SessionTranscriptState::Loading);
            return Some(thread_id);
        }
        None
    }

    pub(crate) fn open_transcript_pager(&mut self, locale: Locale) -> Option<String> {
        let thread_id = self.selected_thread_id()?.to_string();
        let should_load = if self.transcripts.contains_key(&thread_id) {
            false
        } else {
            self.transcripts
                .insert(thread_id.clone(), SessionTranscriptState::Loading);
            true
        };
        self.transcript_pager = Some(PickerTranscriptPager::new(thread_id.clone(), locale));
        should_load.then_some(thread_id)
    }

    fn transcript_lines(&self, thread_id: &str, width: u16, locale: Locale) -> Vec<HyperlinkLine> {
        let Some(state) = self.transcripts.get(thread_id) else {
            return vec![HyperlinkLine::new(Line::styled(
                locale.resume_transcript_loading(),
                Style::default().fg(Color::DarkGray),
            ))];
        };
        match state {
            SessionTranscriptState::Loading => vec![HyperlinkLine::new(Line::styled(
                locale.resume_transcript_loading(),
                Style::default().fg(Color::DarkGray),
            ))],
            SessionTranscriptState::Failed => vec![HyperlinkLine::new(Line::styled(
                locale.resume_transcript_failed(),
                Style::default().fg(Color::Red),
            ))],
            SessionTranscriptState::Loaded(entries) if entries.is_empty() => {
                vec![HyperlinkLine::new(Line::styled(
                    locale.resume_transcript_empty(),
                    Style::default().fg(Color::DarkGray),
                ))]
            }
            SessionTranscriptState::Loaded(entries) => {
                let content_width = Some(usize::from(width.saturating_sub(2).max(1)));
                let cwd = self
                    .threads
                    .iter()
                    .find(|thread| thread.id == thread_id)
                    .map(|thread| thread.cwd.as_path());
                let cwd = cwd.unwrap_or_else(|| std::path::Path::new(""));
                let mut lines = Vec::new();
                for entry in entries {
                    if !lines.is_empty() {
                        lines.push(HyperlinkLine::default());
                    }
                    lines.extend(entry::hyperlink_lines_with_locale(
                        entry,
                        locale,
                        content_width,
                        cwd,
                    ));
                }
                lines
            }
        }
    }

    pub(crate) fn set_transcript(
        &mut self,
        thread_id: String,
        result: std::io::Result<Vec<TranscriptEntry>>,
    ) {
        self.transcripts.insert(
            thread_id,
            match result {
                Ok(entries) => SessionTranscriptState::Loaded(entries),
                Err(_) => SessionTranscriptState::Failed,
            },
        );
    }

    fn begin_load(&mut self) -> usize {
        self.load_token = self.load_token.wrapping_add(1);
        self.loading = true;
        self.status_message = None;
        self.pagination
            .start_load(self.load_token, None, PageLoadMode::StoreDefault);
        self.load_token
    }

    #[cfg(test)]
    fn apply_threads(&mut self, token: usize, threads: Vec<Thread>) {
        self.apply_thread_page(
            token,
            ThreadPage {
                threads,
                next_cursor: None,
            },
        );
    }

    pub(crate) fn apply_thread_page(&mut self, token: usize, page: ThreadPage) {
        if token != self.load_token || self.pagination.finish_load(token).is_none() {
            return;
        }
        self.loading = false;
        let page_len = page.threads.len();
        let next_cursor = page.next_cursor;
        let mut seen_thread_ids = self
            .threads
            .iter()
            .map(|thread| thread.id.clone())
            .collect::<HashSet<_>>();
        self.threads.extend(
            page.threads
                .into_iter()
                .filter(|thread| seen_thread_ids.insert(thread.id.clone())),
        );
        self.pagination
            .complete_page(next_cursor.map(PageCursor::AppServer), page_len, false);
        self.selected = self.selected.min(self.threads.len().saturating_sub(1));
    }

    pub(crate) fn fail_thread_load(&mut self, token: usize, error: anyhow::Error) {
        if token != self.load_token {
            return;
        }
        let _ = self.pagination.finish_load(token);
        self.loading = false;
        self.status_message = Some(error.to_string());
    }

    pub(crate) fn should_load_more(&self) -> bool {
        !self.pagination.is_loading()
            && self.pagination.next_page().is_some()
            && !self.threads.is_empty()
            && self.selected + 1 >= self.threads.len()
    }

    pub(crate) fn has_more_pages(&self) -> bool {
        self.pagination.next_page().is_some()
    }

    fn transcript_preview_text(&self, thread_id: &str) -> Option<String> {
        let lines = self.transcript_previews.get(thread_id)?;
        (!lines.is_empty()).then(|| {
            lines
                .iter()
                .map(|line| {
                    let prefix = match line.speaker {
                        transcript_preview::TranscriptPreviewSpeaker::User => "you: ",
                        transcript_preview::TranscriptPreviewSpeaker::Assistant => "assistant: ",
                    };
                    format!("{prefix}{}", line.text)
                })
                .collect::<Vec<_>>()
                .join(" | ")
        })
    }

    pub(crate) fn handle_event(&mut self, event: Event) -> PickerAction {
        if let Event::Paste(text) = event {
            if let Some(text) = normalize_pasted_search_query(&text) {
                if !self.query.is_empty() && !self.query.ends_with(char::is_whitespace) {
                    self.query.push(' ');
                }
                self.query.push_str(&text);
                self.invalidate_thread_list();
            }
            return PickerAction::Reload;
        }
        let Event::Key(key) = event else {
            return PickerAction::None;
        };
        if key.kind != KeyEventKind::Press {
            return PickerAction::None;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('d'))
        {
            return PickerAction::Cancel;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && key.code == KeyCode::Char('a')
            && self.status == SessionStatus::Active
            && self.archive_shortcut_available()
        {
            return PickerAction::Archive;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('s') => return PickerAction::ToggleStatus,
                KeyCode::Char('f') => return PickerAction::ToggleFilter,
                KeyCode::Char('r') => return PickerAction::ToggleSort,
                KeyCode::Char('o') => return PickerAction::ToggleDensity,
                KeyCode::Char('e') => return PickerAction::ToggleExpanded,
                KeyCode::Char('t') => return PickerAction::OpenTranscript,
                _ => {}
            }
        }
        if key.modifiers.is_empty() && key.code == KeyCode::Char('\u{0014}') {
            return PickerAction::OpenTranscript;
        }
        if key.modifiers.is_empty() && key.code == KeyCode::Char('\u{0005}') {
            return PickerAction::ToggleExpanded;
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('p') | KeyCode::Char('k')
                if key.code == KeyCode::Up || key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.selected = self.selected.saturating_sub(1);
                PickerAction::MoveUp
            }
            KeyCode::PageUp | KeyCode::Char('b')
                if key.code == KeyCode::PageUp || key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.selected = self.selected.saturating_sub(10);
                PickerAction::MoveUp
            }
            KeyCode::Down | KeyCode::Char('n') | KeyCode::Char('j')
                if key.code == KeyCode::Down || key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                if !self.threads.is_empty() {
                    self.selected = (self.selected + 1).min(self.threads.len() - 1);
                }
                PickerAction::MoveDown
            }
            KeyCode::PageDown
                if key.code == KeyCode::PageDown
                    || key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                if !self.threads.is_empty() {
                    self.selected = (self.selected + 10).min(self.threads.len() - 1);
                }
                PickerAction::MoveDown
            }
            KeyCode::Home => {
                self.selected = 0;
                PickerAction::MoveUp
            }
            KeyCode::End => {
                if !self.threads.is_empty() {
                    self.selected = self.threads.len() - 1;
                }
                PickerAction::MoveDown
            }
            KeyCode::Enter if self.status == SessionStatus::Archived => PickerAction::Restore,
            KeyCode::Enter => PickerAction::Select,
            KeyCode::Backspace => {
                self.query.pop();
                self.invalidate_thread_list();
                PickerAction::Reload
            }
            KeyCode::Esc if self.query.is_empty() => PickerAction::Cancel,
            KeyCode::Esc => {
                self.query.clear();
                self.invalidate_thread_list();
                PickerAction::Reload
            }
            KeyCode::Char(c)
                if !key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                self.query.push(c);
                self.invalidate_thread_list();
                PickerAction::Reload
            }
            _ => PickerAction::None,
        }
    }

    pub(crate) fn toggle_status(&mut self) {
        self.status = match self.status {
            SessionStatus::Active => SessionStatus::Archived,
            SessionStatus::Archived => SessionStatus::Active,
        };
        self.selected = 0;
        self.transcript_previews.clear();
        self.preview_loading.clear();
        self.expanded_thread_id = None;
        self.transcripts.clear();
        self.transcript_pager = None;
        self.pagination.reset();
        self.seen_cursors.clear();
    }

    pub(crate) fn toggle_filter(&mut self) {
        self.show_all = !self.show_all;
        self.selected = 0;
        self.transcript_previews.clear();
        self.preview_loading.clear();
        self.expanded_thread_id = None;
        self.transcripts.clear();
        self.transcript_pager = None;
        self.pagination.reset();
        self.seen_cursors.clear();
    }

    pub(crate) fn toggle_sort(&mut self) {
        self.sort_key = match self.sort_key {
            ThreadSortKey::UpdatedAt => ThreadSortKey::CreatedAt,
            ThreadSortKey::CreatedAt
            | ThreadSortKey::RecencyAt
            | ThreadSortKey::SectionPosition => ThreadSortKey::UpdatedAt,
        };
        self.selected = 0;
        self.transcript_previews.clear();
        self.preview_loading.clear();
        self.expanded_thread_id = None;
        self.transcripts.clear();
        self.transcript_pager = None;
        self.pagination.reset();
        self.seen_cursors.clear();
    }

    #[cfg(test)]
    fn set_threads(&mut self, threads: Vec<Thread>) {
        self.threads = filter_threads(threads, &self.query);
        self.selected = self.selected.min(self.threads.len().saturating_sub(1));
        self.transcript_previews.clear();
        self.preview_loading.clear();
        self.expanded_thread_id = None;
        self.transcripts.clear();
        self.transcript_pager = None;
        self.pagination.reset();
        self.seen_cursors.clear();
    }

    pub(crate) fn invalidate_thread_list(&mut self) {
        self.pagination.reset();
        self.seen_cursors.clear();
    }

    pub(crate) fn toggle_density(&mut self) {
        self.density = self.density.toggle();
    }

    pub(crate) fn transcript_pager_is_open(&self) -> bool {
        self.transcript_pager.is_some()
    }

    pub(crate) fn handle_transcript_pager_event(&mut self, event: &Event) {
        if let Some(pager) = self.transcript_pager.as_mut() {
            match pager.overlay.handle_event(event) {
                PagerAction::Close => self.transcript_pager = None,
                PagerAction::Consumed | PagerAction::LoadOlderHistory => {}
            }
        }
    }
}

pub(crate) fn spawn_thread_load(
    request_handle: RequestHandle,
    sender: &mpsc::UnboundedSender<PickerLoadEvent>,
    picker: &mut PickerState,
) {
    let cursor = picker
        .pagination
        .next_cursor
        .as_ref()
        .map(|cursor| match cursor {
            PageCursor::AppServer(cursor) => cursor.clone(),
        });
    if cursor.is_none() {
        picker.threads.clear();
        picker.selected = 0;
        picker.transcript_previews.clear();
        picker.preview_loading.clear();
        picker.expanded_thread_id = None;
        picker.transcripts.clear();
        picker.transcript_pager = None;
        picker.seen_cursors.clear();
    } else if let Some(cursor) = cursor.as_deref() {
        if !picker.seen_cursors.insert(cursor.to_string()) {
            picker.status_message =
                Some(format!("thread list pagination repeated cursor {cursor}"));
            return;
        }
    }
    let token = picker.begin_load();
    let status = picker.status;
    let cwd = (!picker.show_all)
        .then(|| picker.filter_cwd.clone())
        .flatten();
    let query = picker.query.clone();
    let sort_key = picker.sort_key;
    let sender = sender.clone();
    tokio::spawn(async move {
        let result = load_thread_page_with_handle(
            request_handle,
            status,
            cwd.as_deref(),
            &query,
            sort_key,
            cursor,
        )
        .await;
        let _ = sender.send(PickerLoadEvent::Threads { token, result });
    });
}

pub(crate) fn spawn_preview_load(
    request_handle: RequestHandle,
    sender: &mpsc::UnboundedSender<PickerLoadEvent>,
    picker: &mut PickerState,
    thread_id: String,
) {
    if !picker.preview_needs_load(&thread_id) {
        return;
    }
    picker.mark_preview_loading(thread_id.clone());
    let sender = sender.clone();
    tokio::spawn(async move {
        let result = load_transcript_preview_with_handle(request_handle, thread_id.clone()).await;
        let _ = sender.send(PickerLoadEvent::Preview { thread_id, result });
    });
}

pub(crate) fn spawn_transcript_load(
    request_handle: RequestHandle,
    sender: &mpsc::UnboundedSender<PickerLoadEvent>,
    thread_id: String,
) {
    let sender = sender.clone();
    tokio::spawn(async move {
        let result = crate::thread_transcript::load_session_transcript_with_handle(
            request_handle,
            thread_id.clone(),
        )
        .await;
        let _ = sender.send(PickerLoadEvent::Transcript { thread_id, result });
    });
}

async fn load_transcript_preview_with_handle(
    request_handle: RequestHandle,
    thread_id: String,
) -> std::io::Result<Vec<transcript_preview::TranscriptPreviewLine>> {
    let loaded_entries =
        crate::thread_transcript::load_session_transcript_with_handle(request_handle, thread_id)
            .await?;
    let preview_entries = loaded_entries
        .into_iter()
        .filter_map(|entry| {
            let speaker = match entry.kind {
                crate::projection::EntryKind::User => {
                    transcript_preview::TranscriptPreviewSpeaker::User
                }
                crate::projection::EntryKind::Assistant => {
                    transcript_preview::TranscriptPreviewSpeaker::Assistant
                }
                _ => return None,
            };
            Some((speaker, entry.text))
        })
        .collect();
    transcript_preview::preview_from_entries(preview_entries)
}

pub(crate) fn spawn_archive_request(
    request_handle: RequestHandle,
    sender: &mpsc::UnboundedSender<PickerLoadEvent>,
    thread_id: String,
) {
    let sender = sender.clone();
    tokio::spawn(async move {
        let result = request_handle
            .request::<_, app_server_protocol::protocol::v2::ThreadArchiveResponse>(
                METHOD_THREAD_ARCHIVE,
                ThreadArchiveParams {
                    thread_id: thread_id.clone(),
                },
            )
            .await
            .map(|_| ())
            .map_err(anyhow::Error::from);
        let _ = sender.send(PickerLoadEvent::Archive { thread_id, result });
    });
}

pub(crate) fn spawn_unarchive_request(
    request_handle: RequestHandle,
    sender: &mpsc::UnboundedSender<PickerLoadEvent>,
    thread_id: String,
) {
    let sender = sender.clone();
    tokio::spawn(async move {
        let result = request_handle
            .request::<_, ThreadUnarchiveResponse>(
                METHOD_THREAD_UNARCHIVE,
                ThreadUnarchiveParams {
                    thread_id: thread_id.clone(),
                },
            )
            .await
            .map_err(anyhow::Error::from);
        let _ = sender.send(PickerLoadEvent::Unarchive {
            thread_id,
            result: Box::new(result),
        });
    });
}

#[allow(dead_code)]
pub(crate) async fn run_session_picker_with_app_server(
    options: &TuiOptions,
) -> Result<Option<String>> {
    run_session_picker_with_action(options, SessionPickerAction::Resume).await
}

async fn run_session_picker_with_action(
    options: &TuiOptions,
    action: SessionPickerAction,
) -> Result<Option<String>> {
    let session = connect_session(options).await?;
    let request_handle = session.request_handle();
    let (load_tx, mut load_rx) = mpsc::unbounded_channel();
    let mut picker = PickerState::new(
        Vec::new(),
        action,
        SessionStatus::Active,
        Some(options.cwd.clone()),
        false,
    );
    spawn_thread_load(request_handle.clone(), &load_tx, &mut picker);
    let locale = Locale::resolve(options.locale.as_deref());
    let mut terminal = match Tui::enter().context("failed to initialize terminal") {
        Ok(terminal) => terminal,
        Err(error) => {
            let _ = session.shutdown().await;
            return Err(error);
        }
    };
    let mut input = terminal.event_stream();
    let selected = loop {
        if let Some(thread_id) = picker.selected_thread_id().map(ToOwned::to_owned) {
            spawn_preview_load(request_handle.clone(), &load_tx, &mut picker, thread_id);
        }
        terminal
            .terminal_mut()
            .draw(|frame| render_with_locale(frame, &picker, locale))
            .context("failed to render session picker")?;
        tokio::select! {
            load_event = load_rx.recv() => {
                let Some(load_event) = load_event else { break None; };
                match load_event {
                    PickerLoadEvent::Threads { token, result } => match result {
                        Ok(page) => {
                            picker.apply_thread_page(token, page);
                            if picker.threads.is_empty() && picker.pagination.next_page().is_some()
                            {
                                spawn_thread_load(request_handle.clone(), &load_tx, &mut picker);
                            }
                        }
                        Err(error) if token == picker.load_token => {
                            picker.fail_thread_load(token, error);
                        }
                        Err(_) => {}
                    },
                    PickerLoadEvent::Preview { thread_id, result } => {
                        picker.set_transcript_preview(thread_id, result.unwrap_or_default());
                    }
                    PickerLoadEvent::Transcript { thread_id, result } => {
                        picker.set_transcript(thread_id, result);
                    }
                    PickerLoadEvent::Archive { thread_id, result } => {
                        picker.handle_archive_result(thread_id, result);
                    }
                    PickerLoadEvent::Unarchive { thread_id, result } => {
                        let selection = picker.handle_unarchive_result(thread_id, *result);
                        if let Some(crate::resume_picker::SessionSelection::Resume(target)) = selection {
                            break Some(target.thread_id);
                        }
                    }
                }
            }
            event = input.next() => {
                let Some(event) = event else { break None; };
                let event = match event {
                    TuiEvent::Key(key) => Event::Key(key),
                    TuiEvent::Paste(text) => Event::Paste(text),
                    TuiEvent::Resize(size) => Event::Resize(size.width, size.height),
                    TuiEvent::FocusGained => Event::FocusGained,
                    TuiEvent::FocusLost => Event::FocusLost,
                    TuiEvent::Draw | TuiEvent::Resume => continue,
                };
                if let Some(pager) = picker.transcript_pager.as_mut() {
                    match pager.overlay.handle_event(&event) {
                        PagerAction::Close => picker.transcript_pager = None,
                        PagerAction::Consumed | PagerAction::LoadOlderHistory => {}
                    }
                    continue;
                }
                let action = picker.handle_event(event);
                match action {
                    PickerAction::Select => break picker.selected_thread_id().map(ToOwned::to_owned),
                    PickerAction::Restore => {
                        if let Some(thread_id) = picker.request_unarchive_for_selected_session() {
                            spawn_unarchive_request(request_handle.clone(), &load_tx, thread_id);
                        }
                    }
                    PickerAction::Archive => {
                        if let Some(thread_id) = picker.request_archive_for_selected_session() {
                            spawn_archive_request(request_handle.clone(), &load_tx, thread_id);
                        }
                    }
                    PickerAction::ToggleStatus | PickerAction::ToggleFilter | PickerAction::ToggleSort | PickerAction::Reload => {
                        match action {
                            PickerAction::ToggleStatus => picker.toggle_status(),
                            PickerAction::ToggleFilter => picker.toggle_filter(),
                            PickerAction::ToggleSort => picker.toggle_sort(),
                            PickerAction::Reload => {}
                            _ => unreachable!(),
                        }
                        spawn_thread_load(request_handle.clone(), &load_tx, &mut picker);
                    }
                    PickerAction::ToggleDensity => {
                        picker.density = picker.density.toggle();
                    }
                    PickerAction::ToggleExpanded => {
                        if let Some(thread_id) = picker.toggle_selected_expansion() {
                            spawn_transcript_load(request_handle.clone(), &load_tx, thread_id);
                        }
                    }
                    PickerAction::OpenTranscript => {
                        if let Some(thread_id) = picker.open_transcript_pager(locale) {
                            spawn_transcript_load(request_handle.clone(), &load_tx, thread_id);
                        }
                    }
                    PickerAction::Cancel => break None,
                    PickerAction::MoveDown if picker.should_load_more() => {
                        spawn_thread_load(request_handle.clone(), &load_tx, &mut picker);
                    }
                    PickerAction::None | PickerAction::MoveUp | PickerAction::MoveDown => {}
                }
            }
        }
    };
    let restore_result = terminal.restore().context("failed to restore terminal");
    let shutdown_result = session.shutdown().await;
    restore_result?;
    shutdown_result?;
    Ok(selected)
}

#[cfg(test)]
fn render(frame: &mut Frame<'_>, picker: &PickerState) {
    render_with_locale(frame, picker, Locale::default());
}

pub(crate) fn render_with_locale(frame: &mut Frame<'_>, picker: &PickerState, locale: Locale) {
    let area = frame.area();
    if let Some(pager) = picker.transcript_pager.as_ref() {
        let lines = picker.transcript_lines(&pager.thread_id, area.width, locale);
        pager.overlay.render(frame, area, locale, &lines);
        return;
    }
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(picker_vertical_constraints(area.height))
        .split(area);
    let chrome = |rect: ratatui::layout::Rect| {
        ratatui::layout::Rect::new(
            rect.x.saturating_add(1),
            rect.y,
            rect.width.saturating_sub(2),
            rect.height,
        )
    };

    let header = chrome(chunks[0]);
    let title = locale
        .resume_picker_title(matches!(picker.action, SessionPickerAction::Fork))
        .to_string();
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            truncate_display(&title, usize::from(header.width)),
            Style::default().add_modifier(Modifier::BOLD),
        ))),
        header,
    );

    let search = chrome(chunks[2]);
    frame.render_widget(
        Paragraph::new(resume_search_line(picker, locale, search.width))
            .style(Style::default().fg(Color::DarkGray)),
        search,
    );

    let list = ratatui::layout::Rect::new(
        chunks[4].x.saturating_add(2),
        chunks[4].y,
        chunks[4].width.saturating_sub(PICKER_LIST_HORIZONTAL_INSET),
        chunks[4].height,
    );
    render_session_list(frame, list, picker, locale);

    let footer = if picker.loading {
        locale.resume_loading().to_string()
    } else if picker.threads.is_empty() {
        locale.resume_empty().to_string()
    } else if let Some(message) = picker.status_message.as_deref() {
        message.to_string()
    } else {
        format!(
            "{} | {}  {} | {} | {}",
            locale.resume_enter_hint(
                matches!(picker.action, SessionPickerAction::Fork),
                picker.status == SessionStatus::Archived,
            ),
            locale.resume_controls_hint(),
            locale.resume_expand_hint(),
            locale.resume_transcript_hint(),
            locale.resume_density_label(picker.density == SessionListDensity::Dense),
        )
    };
    render_picker_footer(frame, chunks[5], picker, &footer);
}

/// Allocate the picker chrome without allowing the footer to push the list outside the
/// terminal. The picker is commonly rendered in a full-screen area, but tests and embedded
/// hosts also exercise very short terminals, so every constraint is derived from the height.
fn picker_vertical_constraints(height: u16) -> [Constraint; 6] {
    let mut remaining = height;
    let header = remaining.min(1);
    remaining = remaining.saturating_sub(header);
    let header_gap = remaining.min(1);
    remaining = remaining.saturating_sub(header_gap);
    let search = remaining.min(1);
    remaining = remaining.saturating_sub(search);
    let list_gap = remaining.min(1);
    remaining = remaining.saturating_sub(list_gap);
    let footer = remaining.min(4);
    remaining = remaining.saturating_sub(footer);
    [
        Constraint::Length(header),
        Constraint::Length(header_gap),
        Constraint::Length(search),
        Constraint::Length(list_gap),
        Constraint::Length(remaining),
        Constraint::Length(footer),
    ]
}

fn resume_search_line(picker: &PickerState, locale: Locale, width: u16) -> Line<'static> {
    let width = usize::from(width);
    let search = if picker.query.is_empty() {
        locale.resume_search_placeholder().to_string()
    } else {
        format!("{}: {}", locale.resume_search_placeholder(), picker.query)
    };
    let toolbar_wide = format!(
        "{} | {} | {}",
        locale.resume_status_label(picker.status == SessionStatus::Archived),
        locale.resume_filter_label(picker.show_all),
        locale.resume_sort_label(picker.sort_key == ThreadSortKey::CreatedAt),
    );
    let toolbar_compact = format!(
        "{}  {}  {}",
        locale.resume_status_label(picker.status == SessionStatus::Archived),
        locale.resume_filter_label(picker.show_all),
        locale.resume_sort_label(picker.sort_key == ThreadSortKey::CreatedAt),
    );
    let separator = "  ";
    let toolbar = if display_width(&search)
        .saturating_add(display_width(separator))
        .saturating_add(display_width(&toolbar_wide))
        <= width
    {
        toolbar_wide
    } else {
        toolbar_compact
    };
    let available_search = width
        .saturating_sub(display_width(&toolbar))
        .saturating_sub(display_width(separator));
    if available_search == 0 {
        return Line::from(Span::raw(truncate_display(&toolbar, width)));
    }
    let search = truncate_display(&search, available_search);
    let combined = format!("{search}{separator}{toolbar}");
    Line::from(Span::raw(truncate_display(&combined, width)))
}

fn render_picker_footer(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    picker: &PickerState,
    footer: &str,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let progress = format!(
        " {} / {} ",
        if picker.threads.is_empty() {
            0
        } else {
            picker.selected.saturating_add(1)
        },
        picker.threads.len(),
    );
    let progress_width = display_width(&progress);
    let separator_width = usize::from(area.width).saturating_sub(progress_width);
    let separator = if progress_width < usize::from(area.width) {
        format!("{}{}", "─".repeat(separator_width), progress)
    } else {
        "─".repeat(usize::from(area.width))
    };
    frame.render_widget(
        Paragraph::new(separator).style(Style::default().fg(Color::DarkGray)),
        ratatui::layout::Rect::new(area.x, area.y, area.width, 1),
    );
    if area.height == 1 {
        return;
    }
    let hints = ratatui::layout::Rect::new(
        area.x.saturating_add(1),
        area.y.saturating_add(1),
        area.width.saturating_sub(2),
        area.height.saturating_sub(1),
    );
    frame.render_widget(
        Paragraph::new(footer)
            .style(Style::default().fg(Color::DarkGray))
            .wrap(ratatui::widgets::Wrap { trim: true }),
        hints,
    );
}

fn render_session_list(
    frame: &mut Frame<'_>,
    area: ratatui::layout::Rect,
    picker: &PickerState,
    locale: Locale,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    frame.render_widget(Clear, area);
    if picker.threads.is_empty() {
        frame.render_widget(
            Paragraph::new(locale.resume_empty()).style(Style::default().fg(Color::DarkGray)),
            area,
        );
        return;
    }

    let selected = picker.selected.min(picker.threads.len().saturating_sub(1));
    let show_more_above = selected > 0;
    let show_more_below = picker.has_more_pages() || selected + 1 < picker.threads.len();
    let content_height = usize::from(area.height)
        .saturating_sub(usize::from(show_more_above))
        .saturating_sub(usize::from(show_more_below))
        .max(1);
    let mut start = selected;
    let selected_height = session_list_item(
        &picker.threads[selected],
        picker,
        selected,
        locale,
        area.width,
    )
    .len()
    .max(1);
    let mut used = selected_height.min(content_height);
    while start > 0 {
        let previous_height = session_list_item(
            &picker.threads[start - 1],
            picker,
            start - 1,
            locale,
            area.width,
        )
        .len()
        .max(1);
        let separator = usize::from(picker.density == SessionListDensity::Comfortable);
        if used + separator + previous_height > content_height {
            break;
        }
        start -= 1;
        used += separator + previous_height;
    }

    let mut y = area.y;
    if show_more_above {
        frame.render_widget(
            Paragraph::new("↑ more").style(Style::default().fg(Color::DarkGray)),
            ratatui::layout::Rect::new(area.x, y, area.width, 1),
        );
        y = y.saturating_add(1);
    }
    for index in start..picker.threads.len() {
        let content_bottom = area.bottom().saturating_sub(u16::from(show_more_below));
        if y >= content_bottom {
            break;
        }
        for line in session_list_item(&picker.threads[index], picker, index, locale, area.width) {
            if y >= content_bottom {
                break;
            }
            frame.render_widget(
                Paragraph::new(line),
                ratatui::layout::Rect::new(area.x, y, area.width, 1),
            );
            y = y.saturating_add(1);
        }
        if picker.density == SessionListDensity::Comfortable
            && y < content_bottom
            && index + 1 < picker.threads.len()
        {
            y = y.saturating_add(1);
        }
    }
    if show_more_below {
        frame.render_widget(
            Paragraph::new("↓ more").style(Style::default().fg(Color::DarkGray)),
            ratatui::layout::Rect::new(area.x, area.bottom().saturating_sub(1), area.width, 1),
        );
    }
}

fn session_list_item(
    thread: &Thread,
    picker: &PickerState,
    index: usize,
    locale: Locale,
    width: u16,
) -> Vec<Line<'static>> {
    let width = usize::from(width);
    let preview = picker.transcript_preview_text(&thread.id);
    let title =
        thread_line_with_preview(thread, width.saturating_sub(2), locale, preview.as_deref());
    let marker = if picker.selected == index {
        Span::styled(
            "❯ ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
    } else {
        Span::raw("  ")
    };
    let mut title = Line {
        style: title.style,
        alignment: title.alignment,
        spans: std::iter::once(marker).chain(title.spans).collect(),
    };
    if picker.selected == index {
        title = title.style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    }
    let is_expanded = picker.selected == index
        && picker.expanded_thread_id.as_deref() == Some(thread.id.as_str());
    if is_expanded {
        let mut lines = vec![title];
        lines.extend(render_expanded_session_details(
            thread,
            picker,
            width.saturating_sub(2),
            locale,
        ));
        lines
    } else if picker.density == SessionListDensity::Dense {
        vec![title]
    } else {
        let metadata = format!(
            "  {}  {}",
            thread.cwd.display(),
            if thread.updated_at == 0 {
                String::from("-")
            } else {
                thread.updated_at.to_string()
            }
        );
        vec![
            title,
            Line::from(Span::styled(
                truncate_display(&metadata, width),
                Style::default().fg(Color::DarkGray),
            )),
        ]
    }
}

fn render_expanded_session_details(
    thread: &Thread,
    picker: &PickerState,
    width: usize,
    locale: Locale,
) -> Vec<Line<'static>> {
    let indent = "  ";
    let available = width.saturating_sub(display_width(indent));
    let title = thread
        .name
        .as_deref()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or(thread.preview.as_str());
    let cwd = center_truncate_path(&thread.cwd.to_string_lossy(), available.saturating_sub(16));
    let state = match &thread.status {
        ThreadStatus::Active { .. } => "active",
        ThreadStatus::Idle => "idle",
        ThreadStatus::NotLoaded => "not loaded",
        ThreadStatus::SystemError => "error",
    };
    let mut lines = vec![
        metadata_line(
            locale.thread_label(),
            &format!("{title} ({})", thread.id),
            width,
        ),
        metadata_line(locale.cwd_label(), &cwd, width),
        metadata_line(locale.state_label(), &locale.thread_state(state), width),
        metadata_line(
            locale.created_label(),
            &thread.created_at.to_string(),
            width,
        ),
        metadata_line(
            locale.updated_label(),
            &thread.updated_at.to_string(),
            width,
        ),
    ];

    match picker.transcripts.get(&thread.id) {
        Some(SessionTranscriptState::Loading) => {
            lines.push(Line::from(Span::styled(
                truncate_display(
                    &format!("{indent}{}", locale.resume_transcript_loading()),
                    width,
                ),
                Style::default().fg(Color::DarkGray),
            )));
        }
        Some(SessionTranscriptState::Failed) => {
            lines.push(Line::from(Span::styled(
                truncate_display(
                    &format!("{indent}{}", locale.resume_transcript_failed()),
                    width,
                ),
                Style::default().fg(Color::Red),
            )));
        }
        Some(SessionTranscriptState::Loaded(entries)) if entries.is_empty() => {
            lines.push(Line::from(Span::styled(
                truncate_display(
                    &format!("{indent}{}", locale.resume_transcript_empty()),
                    width,
                ),
                Style::default().fg(Color::DarkGray),
            )));
        }
        Some(SessionTranscriptState::Loaded(entries)) => {
            lines.push(Line::from(Span::styled(
                truncate_display(&format!("{indent}{}", locale.transcript_title()), width),
                Style::default().add_modifier(Modifier::BOLD),
            )));
            for entry in entries {
                lines.extend(
                    render_transcript_entry_lines_wrapped(
                        entry,
                        u16::try_from(width).unwrap_or(u16::MAX),
                        locale,
                        &thread.cwd,
                    )
                    .into_iter()
                    .map(|line| line.line),
                );
            }
        }
        None => {}
    }
    lines
}

fn metadata_line(label: &str, value: &str, width: usize) -> Line<'static> {
    let prefix = format!("  {label}: ");
    if width == 0 {
        return Line::default();
    }
    if display_width(&prefix) >= width {
        return Line::from(Span::styled(
            truncate_display(&prefix, width),
            Style::default().fg(Color::DarkGray),
        ));
    }
    let available = width.saturating_sub(display_width(&prefix));
    Line::from(vec![
        Span::styled(prefix, Style::default().fg(Color::DarkGray)),
        Span::raw(truncate_display(value, available)),
    ])
}

fn thread_line_with_preview(
    thread: &Thread,
    width: usize,
    locale: Locale,
    transcript_preview: Option<&str>,
) -> Line<'static> {
    let title = thread
        .name
        .as_deref()
        .filter(|name| !name.trim().is_empty())
        .unwrap_or_else(|| {
            let preview = transcript_preview.unwrap_or(thread.preview.as_str());
            if preview.trim().is_empty() {
                locale.untitled_conversation()
            } else {
                preview
            }
        });
    let state = match &thread.status {
        ThreadStatus::Active { .. } => "active",
        ThreadStatus::Idle => "idle",
        ThreadStatus::NotLoaded => "not loaded",
        ThreadStatus::SystemError => "error",
    };
    let cwd = thread.cwd.to_string_lossy();
    let max_width = width.saturating_sub(2);
    let state_text = format!("[{}]", locale.thread_state(state));
    let reserved = display_width(&format!("{title}  {state_text}  "));
    let cwd = center_truncate_path(&cwd, max_width.saturating_sub(reserved));
    let truncated = truncate_display(&format!("{title}  {state_text}  {cwd}"), max_width);
    Line::from(Span::raw(truncated))
}

fn truncate_display(text: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }
    if display_width(text) <= max_width {
        return text.to_string();
    }
    if max_width <= 1 {
        return "…".to_string();
    }
    let mut output = String::new();
    let mut width = 0;
    for grapheme in unicode_segmentation::UnicodeSegmentation::graphemes(text, true) {
        let grapheme_width = display_width(grapheme);
        if width + grapheme_width + 1 > max_width {
            break;
        }
        output.push_str(grapheme);
        width += grapheme_width;
    }
    output.push('…');
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_server_protocol::protocol::v2::{SessionSource, ThreadActiveFlag, ThreadHistoryMode};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use std::path::PathBuf;

    fn thread(id: &str, preview: &str, ephemeral: bool) -> Thread {
        Thread {
            id: id.to_string(),
            extra: None,
            session_id: format!("session-{id}"),
            forked_from_id: None,
            parent_thread_id: None,
            preview: preview.to_string(),
            ephemeral,
            section: None,
            section_entered_at: None,
            project_id: None,
            history_mode: ThreadHistoryMode::default(),
            model_provider: "fixture".to_string(),
            created_at: 1,
            updated_at: 1,
            recency_at: None,
            status: ThreadStatus::Active {
                active_flags: vec![ThreadActiveFlag::WaitingOnUserInput],
            },
            path: None,
            cwd: PathBuf::from("/workspace"),
            cli_version: "test".to_string(),
            source: SessionSource::Cli,
            can_accept_direct_input: Some(true),
            thread_source: None,
            agent_nickname: None,
            agent_role: None,
            git_info: None,
            name: None,
            turns: Vec::new(),
        }
    }

    fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
        let buffer = terminal.backend().buffer();
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn picker_filters_ephemeral_threads_and_navigates() {
        let mut picker = PickerState::new(
            vec![
                thread("hidden", "temporary", true),
                thread("one", "first", false),
                thread("two", "second", false),
            ],
            SessionPickerAction::Resume,
            SessionStatus::Active,
            None,
            true,
        );
        assert_eq!(picker.threads.len(), 2);
        assert_eq!(picker.selected_thread_id(), Some("one"));
        assert_eq!(
            picker.handle_event(Event::Key(crossterm::event::KeyEvent::new(
                KeyCode::Down,
                KeyModifiers::NONE,
            ))),
            PickerAction::MoveDown
        );
        assert_eq!(picker.selected_thread_id(), Some("two"));
        assert_eq!(
            picker.handle_event(Event::Key(crossterm::event::KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            ))),
            PickerAction::Select
        );
    }

    #[test]
    fn searchable_picker_keeps_plain_vim_letters_for_query_input() {
        let mut picker = PickerState::new(
            vec![
                thread("one", "first", false),
                thread("two", "second", false),
            ],
            SessionPickerAction::Resume,
            SessionStatus::Active,
            None,
            true,
        );

        assert_eq!(
            picker.handle_event(Event::Key(crossterm::event::KeyEvent::new(
                KeyCode::Char('j'),
                KeyModifiers::NONE,
            ))),
            PickerAction::Reload
        );
        assert_eq!(picker.query, "j");
        assert_eq!(picker.selected, 0);
    }

    #[test]
    fn picker_control_navigation_matches_codex_list_bindings() {
        let mut picker = PickerState::new(
            vec![
                thread("one", "first", false),
                thread("two", "second", false),
                thread("three", "third", false),
            ],
            SessionPickerAction::Resume,
            SessionStatus::Active,
            None,
            true,
        );

        assert_eq!(
            picker.handle_event(Event::Key(crossterm::event::KeyEvent::new(
                KeyCode::Char('n'),
                KeyModifiers::CONTROL,
            ))),
            PickerAction::MoveDown
        );
        assert_eq!(picker.selected, 1);
        assert_eq!(
            picker.handle_event(Event::Key(crossterm::event::KeyEvent::new(
                KeyCode::Char('p'),
                KeyModifiers::CONTROL,
            ))),
            PickerAction::MoveUp
        );
        assert_eq!(picker.selected, 0);
        assert_eq!(
            picker.handle_event(Event::Key(crossterm::event::KeyEvent::new(
                KeyCode::PageDown,
                KeyModifiers::NONE,
            ))),
            PickerAction::MoveDown
        );
        assert_eq!(picker.selected, 2);
    }

    #[test]
    fn unhandled_control_keys_do_not_pollute_resume_query() {
        let mut picker = PickerState::new(
            vec![thread("one", "first", false)],
            SessionPickerAction::Resume,
            SessionStatus::Active,
            None,
            true,
        );
        picker.handle_event(Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char('z'),
            KeyModifiers::CONTROL,
        )));
        assert!(picker.query.is_empty());
    }

    #[test]
    fn picker_render_is_bounded_for_narrow_unicode_terminal() {
        let picker = PickerState::new(
            vec![thread("one", "你好，这是一段很长的预览", false)],
            SessionPickerAction::Resume,
            SessionStatus::Active,
            None,
            true,
        );
        let mut terminal = Terminal::new(TestBackend::new(24, 8)).expect("terminal");
        terminal.draw(|frame| render(frame, &picker)).expect("draw");
        assert!(terminal.backend().buffer().content().iter().all(|cell| cell
            .symbol()
            .chars()
            .count()
            <= 1));
    }

    #[test]
    fn resume_picker_render_matrix_stays_bounded_across_locales_and_widths() {
        let mut threads = (0..12)
            .map(|index| {
                let mut value = thread(
                    &format!("thread-{index:02}"),
                    "这是一个很长的会话预览，用于验证窄终端上的重排与截断行为",
                    false,
                );
                value.name = Some(format!(
                    "会话 {index:02} — a deliberately long title for the picker"
                ));
                value.cwd = PathBuf::from(
                    "/Users/example/Documents/projects/a-very-long-working-directory",
                );
                value
            })
            .collect::<Vec<_>>();
        let selected_id = threads[0].id.clone();
        threads.reverse();

        for locale in [
            Locale::ZhCn,
            Locale::ZhTw,
            Locale::EnUs,
            Locale::JaJp,
            Locale::KoKr,
        ] {
            for width in [40u16, 80, 120] {
                let mut picker = PickerState::new(
                    threads.clone(),
                    SessionPickerAction::Resume,
                    SessionStatus::Active,
                    None,
                    true,
                );
                picker.query = "a very long query that must remain clipped safely".to_string();
                picker.selected = picker
                    .threads
                    .iter()
                    .position(|thread| thread.id == selected_id)
                    .unwrap_or_default();
                let selected_lines = session_list_item(
                    &picker.threads[picker.selected],
                    &picker,
                    picker.selected,
                    locale,
                    width,
                );
                assert!(
                    selected_lines
                        .iter()
                        .all(|line| line.width() <= usize::from(width)),
                    "locale={} width={width} selected row exceeded width: {selected_lines:?}",
                    locale.tag()
                );

                let mut terminal = Terminal::new(TestBackend::new(width, 16)).expect("terminal");
                terminal
                    .draw(|frame| render_with_locale(frame, &picker, locale))
                    .expect("draw");
                let rendered = buffer_text(&terminal);
                assert_eq!(rendered.lines().count(), 16);
                assert!(
                    rendered.contains("00"),
                    "locale={} width={width} selected row was not visible: {rendered:?}",
                    locale.tag()
                );
                assert!(
                    rendered.contains("Enter") && rendered.contains("Esc"),
                    "locale={} width={width} footer lost primary actions: {rendered:?}",
                    locale.tag()
                );
                assert!(
                    terminal.backend().buffer().content().iter().all(|cell| cell
                        .symbol()
                        .chars()
                        .count()
                        <= 1),
                    "locale={} width={width} contains a multi-codepoint cell",
                    locale.tag()
                );
            }
        }
    }

    #[test]
    fn resume_picker_short_terminal_heights_remain_renderable() {
        let picker = PickerState::new(
            vec![thread("one", "preview", false)],
            SessionPickerAction::Resume,
            SessionStatus::Active,
            None,
            true,
        );
        for height in [1u16, 2, 4, 7, 8] {
            let mut terminal = Terminal::new(TestBackend::new(40, height)).expect("terminal");
            terminal
                .draw(|frame| render_with_locale(frame, &picker, Locale::EnUs))
                .expect("draw");
            assert_eq!(terminal.backend().buffer().area.height, height);
        }
    }

    #[test]
    fn resume_search_line_preserves_toolbar_and_display_width_on_narrow_terminals() {
        let mut picker = PickerState::new(
            vec![thread("one", "first", false)],
            SessionPickerAction::Resume,
            SessionStatus::Active,
            None,
            true,
        );
        picker.query = "a very long search query that should be shortened".to_string();
        for width in [8, 16, 24, 40] {
            let line = resume_search_line(&picker, Locale::EnUs, width);
            assert!(
                crate::line_truncation::line_width(&line) <= usize::from(width),
                "width={width} line={line:?}"
            );
        }
    }

    #[test]
    fn long_resume_list_keeps_selected_session_visible_after_end_navigation() {
        let mut threads = (0..20)
            .map(|index| {
                let mut value = thread(&format!("thread-{index:02}"), "preview", false);
                value.name = Some(format!("Session {index:02}"));
                value
            })
            .collect::<Vec<_>>();
        threads.reverse();
        let mut picker = PickerState::new(
            threads,
            SessionPickerAction::Resume,
            SessionStatus::Active,
            None,
            true,
        );
        picker.handle_event(Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::End,
            KeyModifiers::NONE,
        )));

        let mut terminal = Terminal::new(TestBackend::new(48, 10)).expect("terminal");
        terminal.draw(|frame| render(frame, &picker)).expect("draw");
        let text = buffer_text(&terminal);
        assert!(
            text.contains("Session 00"),
            "selected session was clipped from the viewport: {text}"
        );
    }

    #[test]
    fn truncation_preserves_display_width() {
        let text = truncate_display("你好abc", 5);
        assert!(display_width(&text) <= 5);
        assert!(text.ends_with('…'));
    }

    #[test]
    fn session_picker_action_keeps_resume_and_fork_selection_distinct() {
        let target = SessionTarget {
            path: Some(PathBuf::from("/workspace")),
            thread_id: String::from("thread-1"),
            history_mode: Some(ThreadHistoryMode::Legacy),
        };
        assert_eq!(SessionPickerAction::Resume.action_label(), "resume");
        assert_eq!(
            SessionPickerAction::Fork.selection(target.clone()),
            SessionSelection::Fork(target)
        );
    }

    #[test]
    fn session_target_uses_thread_id_when_path_is_not_available() {
        let target = SessionTarget {
            path: None,
            thread_id: String::from("thread-1"),
            history_mode: None,
        };
        assert_eq!(target.display_label(), "thread thread-1");
    }

    #[test]
    fn picker_search_filters_preview_name_path_and_thread_id() {
        let mut named = thread("named", "unrelated", false);
        named.name = Some("Release checklist".to_string());
        let mut picker = PickerState::new(
            vec![
                named,
                thread("preview-id", "Need a deployment review", false),
                thread("other", "unrelated", false),
            ],
            SessionPickerAction::Resume,
            SessionStatus::Active,
            Some(PathBuf::from("/workspace")),
            true,
        );

        picker.query = "release".to_string();
        picker.set_threads(picker.threads.clone());
        assert_eq!(picker.threads.len(), 1);
        assert_eq!(picker.selected_thread_id(), Some("named"));

        picker.query = "preview-id".to_string();
        picker.set_threads(vec![thread("preview-id", "unrelated", false)]);
        assert_eq!(picker.selected_thread_id(), Some("preview-id"));
    }

    #[test]
    fn picker_controls_match_codex_resume_shortcut_shapes() {
        let mut picker = PickerState::new(
            vec![thread("one", "first", false)],
            SessionPickerAction::Resume,
            SessionStatus::Active,
            Some(PathBuf::from("/workspace")),
            false,
        );
        assert_eq!(
            picker.handle_event(Event::Key(crossterm::event::KeyEvent::new(
                KeyCode::Char('s'),
                KeyModifiers::CONTROL,
            ))),
            PickerAction::ToggleStatus
        );
        picker.toggle_status();
        assert_eq!(picker.status, SessionStatus::Archived);
        assert_eq!(
            picker.handle_event(Event::Key(crossterm::event::KeyEvent::new(
                KeyCode::Char('o'),
                KeyModifiers::CONTROL,
            ))),
            PickerAction::ToggleDensity
        );
        picker.density = picker.density.toggle();
        assert_eq!(picker.density, SessionListDensity::Dense);
    }

    #[test]
    fn ctrl_t_opens_a_shared_transcript_pager_and_starts_loading_when_needed() {
        let mut picker = PickerState::new(
            vec![thread("one", "first", false)],
            SessionPickerAction::Resume,
            SessionStatus::Active,
            None,
            true,
        );
        let key = Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char('t'),
            KeyModifiers::CONTROL,
        ));
        assert_eq!(picker.handle_event(key), PickerAction::OpenTranscript);
        assert_eq!(
            picker.open_transcript_pager(Locale::EnUs),
            Some("one".to_string())
        );
        assert!(picker.transcript_pager.is_some());
        assert!(matches!(
            picker.transcripts.get("one"),
            Some(SessionTranscriptState::Loading)
        ));

        let mut terminal = Terminal::new(TestBackend::new(40, 8)).expect("terminal");
        terminal
            .draw(|frame| render_with_locale(frame, &picker, Locale::EnUs))
            .expect("draw");
        assert!(buffer_text(&terminal).contains("Loading transcript"));
    }

    #[test]
    fn transcript_pager_renders_canonical_entries_scrolls_and_closes_with_ctrl_t() {
        let mut picker = PickerState::new(
            vec![thread("one", "first", false)],
            SessionPickerAction::Resume,
            SessionStatus::Active,
            None,
            true,
        );
        picker.transcripts.insert(
            "one".to_string(),
            SessionTranscriptState::Loaded(vec![TranscriptEntry {
                id: "entry-1".to_string(),
                kind: EntryKind::Assistant,
                text: (0..16)
                    .map(|index| format!("assistant line {index}"))
                    .collect::<Vec<_>>()
                    .join("\n"),
                streaming: false,
                status: None,
                summary: Vec::new(),
            }]),
        );
        assert_eq!(picker.open_transcript_pager(Locale::EnUs), None);

        let mut terminal = Terminal::new(TestBackend::new(40, 8)).expect("terminal");
        terminal
            .draw(|frame| render_with_locale(frame, &picker, Locale::EnUs))
            .expect("draw");
        let text = buffer_text(&terminal);
        assert!(text.contains("assistant line"));
        let pager = picker.transcript_pager.as_mut().expect("pager");
        assert_eq!(
            pager
                .overlay
                .handle_event(&Event::Key(crossterm::event::KeyEvent::new(
                    KeyCode::Home,
                    KeyModifiers::NONE,
                ))),
            PagerAction::Consumed
        );
        assert_eq!(
            pager
                .overlay
                .handle_event(&Event::Key(crossterm::event::KeyEvent::new(
                    KeyCode::Char('t'),
                    KeyModifiers::CONTROL,
                ))),
            PagerAction::Close
        );
    }

    #[test]
    fn raw_ctrl_t_keycode_matches_codex_resume_shortcut() {
        let mut picker = PickerState::new(
            vec![thread("one", "first", false)],
            SessionPickerAction::Resume,
            SessionStatus::Active,
            None,
            true,
        );
        assert_eq!(
            picker.handle_event(Event::Key(crossterm::event::KeyEvent::new(
                KeyCode::Char('\u{0014}'),
                KeyModifiers::NONE,
            ))),
            PickerAction::OpenTranscript
        );
    }

    #[test]
    fn ctrl_e_toggles_selected_session_expansion_and_transcript_loading() {
        let mut picker = PickerState::new(
            vec![thread("one", "first", false)],
            SessionPickerAction::Resume,
            SessionStatus::Active,
            None,
            true,
        );
        let key = Event::Key(crossterm::event::KeyEvent::new(
            KeyCode::Char('e'),
            KeyModifiers::CONTROL,
        ));
        assert_eq!(
            picker.handle_event(key.clone()),
            PickerAction::ToggleExpanded
        );
        assert_eq!(
            picker.toggle_selected_expansion(),
            Some(String::from("one"))
        );
        assert_eq!(picker.expanded_thread_id.as_deref(), Some("one"));
        assert!(matches!(
            picker.transcripts.get("one"),
            Some(SessionTranscriptState::Loading)
        ));

        assert_eq!(picker.handle_event(key), PickerAction::ToggleExpanded);
        assert_eq!(picker.toggle_selected_expansion(), None);
        assert_eq!(picker.expanded_thread_id, None);
    }

    #[test]
    fn raw_ctrl_e_keycode_matches_codex_resume_shortcut() {
        let mut picker = PickerState::new(
            vec![thread("one", "first", false)],
            SessionPickerAction::Resume,
            SessionStatus::Active,
            None,
            true,
        );
        assert_eq!(
            picker.handle_event(Event::Key(crossterm::event::KeyEvent::new(
                KeyCode::Char('\u{0005}'),
                KeyModifiers::NONE,
            ))),
            PickerAction::ToggleExpanded
        );
    }

    #[test]
    fn expanded_transcript_uses_canonical_entries_and_stays_bounded() {
        let mut picker = PickerState::new(
            vec![thread("one", "first", false)],
            SessionPickerAction::Resume,
            SessionStatus::Active,
            None,
            true,
        );
        picker.expanded_thread_id = Some(String::from("one"));
        picker.transcripts.insert(
            String::from("one"),
            SessionTranscriptState::Loaded(vec![TranscriptEntry {
                id: String::from("entry-1"),
                kind: EntryKind::Assistant,
                text: String::from("a long assistant response that wraps safely"),
                streaming: false,
                status: None,
                summary: Vec::new(),
            }]),
        );
        let lines = render_expanded_session_details(&picker.threads[0], &picker, 24, Locale::EnUs);
        assert!(lines
            .iter()
            .any(|line| line.to_string().contains("assistant")));
        assert!(lines
            .iter()
            .all(|line| display_width(&line.to_string()) <= 24));
    }

    #[test]
    fn transcript_load_event_records_loaded_and_failed_states() {
        let mut picker = PickerState::new(
            vec![thread("one", "first", false)],
            SessionPickerAction::Resume,
            SessionStatus::Active,
            None,
            true,
        );
        picker.set_transcript(
            String::from("one"),
            Ok(vec![TranscriptEntry {
                id: String::from("entry-1"),
                kind: EntryKind::User,
                text: String::from("hello"),
                streaming: false,
                status: None,
                summary: Vec::new(),
            }]),
        );
        assert!(matches!(
            picker.transcripts.get("one"),
            Some(SessionTranscriptState::Loaded(entries)) if entries.len() == 1
        ));

        picker.set_transcript(String::from("one"), Err(std::io::Error::other("offline")));
        assert!(matches!(
            picker.transcripts.get("one"),
            Some(SessionTranscriptState::Failed)
        ));
    }

    #[test]
    fn picker_only_reloads_for_query_mutations() {
        let mut picker = PickerState::new(
            vec![thread("one", "first", false)],
            SessionPickerAction::Resume,
            SessionStatus::Active,
            Some(PathBuf::from("/workspace")),
            false,
        );

        assert_eq!(
            picker.handle_event(Event::Key(crossterm::event::KeyEvent::new(
                KeyCode::F(5),
                KeyModifiers::NONE,
            ))),
            PickerAction::None
        );
        assert_eq!(
            picker.handle_event(Event::Key(crossterm::event::KeyEvent::new(
                KeyCode::Char('x'),
                KeyModifiers::NONE,
            ))),
            PickerAction::Reload
        );
    }

    #[test]
    fn stale_thread_load_results_do_not_clear_newer_loading_state() {
        let mut picker = PickerState::new(
            Vec::new(),
            SessionPickerAction::Resume,
            SessionStatus::Active,
            None,
            true,
        );
        let first = picker.begin_load();
        let second = picker.begin_load();
        picker.apply_threads(first, vec![thread("stale", "stale", false)]);

        assert!(picker.loading);
        assert!(picker.threads.is_empty());

        picker.apply_threads(second, vec![thread("fresh", "fresh", false)]);
        assert!(!picker.loading);
        assert_eq!(picker.selected_thread_id(), Some("fresh"));
    }

    #[test]
    fn thread_pages_append_unique_threads_and_keep_the_next_cursor() {
        let mut picker = PickerState::new(
            Vec::new(),
            SessionPickerAction::Resume,
            SessionStatus::Active,
            None,
            true,
        );
        let first_token = picker.begin_load();
        picker.apply_thread_page(
            first_token,
            ThreadPage {
                threads: vec![
                    thread("one", "first", false),
                    thread("two", "second", false),
                ],
                next_cursor: Some("cursor-1".to_string()),
            },
        );
        assert_eq!(picker.threads.len(), 2);
        assert!(matches!(
            picker.pagination.next_cursor.as_ref(),
            Some(PageCursor::AppServer(cursor)) if cursor == "cursor-1"
        ));
        picker.selected = 1;
        assert!(picker.should_load_more());

        let second_token = picker.begin_load();
        picker.apply_thread_page(
            second_token,
            ThreadPage {
                threads: vec![
                    thread("two", "duplicate", false),
                    thread("three", "third", false),
                ],
                next_cursor: None,
            },
        );
        assert_eq!(
            picker
                .threads
                .iter()
                .map(|thread| thread.id.as_str())
                .collect::<Vec<_>>(),
            vec!["one", "two", "three"]
        );
        assert!(picker.pagination.next_cursor.is_none());
        assert!(!picker.should_load_more());
    }

    #[test]
    fn query_mutation_invalidates_a_previous_page_cursor() {
        let mut picker = PickerState::new(
            vec![thread("one", "first", false)],
            SessionPickerAction::Resume,
            SessionStatus::Active,
            None,
            true,
        );
        picker.pagination.complete_page(
            Some(PageCursor::AppServer("cursor-1".to_string())),
            0,
            false,
        );
        picker.seen_cursors.insert("cursor-1".to_string());
        assert_eq!(
            picker.handle_event(Event::Key(crossterm::event::KeyEvent::new(
                KeyCode::Char('x'),
                KeyModifiers::NONE,
            ))),
            PickerAction::Reload
        );
        assert!(picker.pagination.next_cursor.is_none());
        assert!(picker.seen_cursors.is_empty());
    }

    #[test]
    fn page_navigation_matches_codex_shape_and_reaches_the_next_cursor() {
        let mut picker = PickerState::new(
            (0..12)
                .map(|index| thread(&format!("thread-{index}"), "preview", false))
                .collect(),
            SessionPickerAction::Resume,
            SessionStatus::Active,
            None,
            true,
        );
        picker.pagination.complete_page(
            Some(PageCursor::AppServer("cursor-1".to_string())),
            12,
            false,
        );
        assert_eq!(
            picker.handle_event(Event::Key(crossterm::event::KeyEvent::new(
                KeyCode::PageDown,
                KeyModifiers::NONE,
            ))),
            PickerAction::MoveDown
        );
        assert_eq!(picker.selected, 10);
        assert_eq!(
            picker.handle_event(Event::Key(crossterm::event::KeyEvent::new(
                KeyCode::PageDown,
                KeyModifiers::NONE,
            ))),
            PickerAction::MoveDown
        );
        assert_eq!(picker.selected, 11);
        assert!(picker.should_load_more());
        assert_eq!(
            picker.handle_event(Event::Key(crossterm::event::KeyEvent::new(
                KeyCode::Home,
                KeyModifiers::NONE,
            ))),
            PickerAction::MoveUp
        );
        assert_eq!(picker.selected, 0);
    }
}
