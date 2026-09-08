//! Codex-shaped Agents Overview state and canonical thread projection.

use super::agents_overview_view::{AgentsOverviewGroup, AgentsOverviewRow, AgentsOverviewView};
use crate::app_server_session::AppServerSession;
use anyhow::{anyhow, Result};
use app_server_protocol::protocol::v2::ServerNotification;
use app_server_protocol::protocol::v2::{Thread, TurnStatus, UserInput};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::task::AbortHandle;

#[allow(dead_code)]
pub(crate) const AGENTS_OVERVIEW_VIEW_ID: &str = "agents-overview";

#[derive(Debug, Default)]
pub(crate) struct AgentsOverviewState {
    pub(crate) threads: Vec<Thread>,
    /// True after the first server-backed seed has completed.
    pub(crate) initialized: bool,
    /// Monotonic refresh identity. A response from an older request is ignored.
    pub(crate) request_id: Option<u64>,
    /// A refresh requested while another refresh is in flight is coalesced.
    pub(crate) refresh_pending: bool,
    /// Thread ids with metadata changes that should be refreshed on the next pass.
    pub(crate) refresh_thread_ids: HashSet<String>,
    /// Abort handle for a background refresh task, when one is installed by a host.
    pub(crate) refresh_task: Option<AbortHandle>,
    /// Notifications received during a refresh, keyed by thread and replayed afterward.
    pub(crate) refresh_notifications: HashMap<String, Vec<ServerNotification>>,
    /// Whether the overview is currently rendered as the primary full-screen surface.
    pub(crate) rendered_full_screen: bool,
    /// Canonical order of rows currently visible after filtering/grouping.
    pub(crate) visible_thread_ids: Vec<String>,
    /// Codex-compatible shared view state for hosts that render through a selection surface.
    /// `view` remains the Lime terminal owner; this mirror lets adapters share the same shape
    /// without introducing a second interaction model.
    pub(crate) view_state: Arc<Mutex<super::agents_overview_view::AgentsOverviewView>>,
    /// Drafts are retained per thread while the command center is open or a root is resumed.
    pub(crate) input_states: HashMap<String, String>,
    pub(crate) view: AgentsOverviewView,
    pub(crate) refreshing: bool,
    pub(crate) refresh_generation: u64,
}

impl AgentsOverviewState {
    pub(crate) fn new(primary_thread_id: Option<&str>) -> Self {
        let view = AgentsOverviewView::new(Vec::new(), primary_thread_id);
        Self {
            threads: Vec::new(),
            initialized: false,
            request_id: None,
            refresh_pending: false,
            refresh_thread_ids: HashSet::new(),
            refresh_task: None,
            refresh_notifications: HashMap::new(),
            rendered_full_screen: false,
            visible_thread_ids: Vec::new(),
            view_state: Arc::new(Mutex::new(view.clone())),
            input_states: HashMap::new(),
            view,
            refreshing: false,
            refresh_generation: 0,
        }
    }

    pub(crate) fn begin_refresh(&mut self) -> u64 {
        if self.refreshing {
            self.refresh_pending = true;
            return self.refresh_generation;
        }
        self.refresh_generation = self.refresh_generation.wrapping_add(1);
        self.refreshing = true;
        self.request_id = Some(self.refresh_generation);
        self.refresh_generation
    }

    pub(crate) fn replace_threads(
        &mut self,
        threads: Vec<Thread>,
        primary_thread_id: Option<&str>,
    ) {
        // App Server's recent index is a seed, not an eviction list. Keep locally observed
        // sessions until an explicit archive/delete notification removes them.
        let mut by_id = self
            .threads
            .drain(..)
            .map(|thread| (thread.id.clone(), thread))
            .collect::<HashMap<_, _>>();
        for thread in threads {
            by_id.insert(thread.id.clone(), thread);
        }
        self.threads = by_id.into_values().collect();
        let rows = build_rows(&self.threads, primary_thread_id);
        self.view.update_rows(rows);
        self.visible_thread_ids = self
            .view
            .visible_rows()
            .into_iter()
            .map(|row| row.thread.id.clone())
            .collect();
        self.sync_view_state();
        self.refreshing = false;
        self.initialized = true;
        self.request_id = None;
        self.refresh_thread_ids.clear();
    }

    pub(crate) fn apply_refresh(
        &mut self,
        generation: u64,
        threads: Vec<Thread>,
        primary_thread_id: Option<&str>,
    ) -> bool {
        if generation != self.refresh_generation {
            return false;
        }
        self.replace_threads(threads, primary_thread_id);
        true
    }

    pub(crate) fn sync_view_state(&self) {
        if let Ok(mut state) = self.view_state.lock() {
            *state = self.view.clone();
        }
    }

    pub(crate) fn take_refresh_pending(&mut self) -> bool {
        std::mem::take(&mut self.refresh_pending)
    }
}

impl Drop for AgentsOverviewState {
    fn drop(&mut self) {
        if let Some(handle) = self.refresh_task.take() {
            handle.abort();
        }
    }
}

pub(crate) fn build_rows(
    threads: &[Thread],
    primary_thread_id: Option<&str>,
) -> Vec<AgentsOverviewRow> {
    let mut by_id = HashMap::new();
    for thread in threads.iter().filter(|thread| !thread.ephemeral) {
        by_id.insert(thread.id.clone(), thread);
    }
    let mut children: HashMap<&str, Vec<&Thread>> = HashMap::new();
    for thread in by_id.values() {
        if let Some(parent) = thread.parent_thread_id.as_deref() {
            children.entry(parent).or_default().push(thread);
        }
    }
    let mut rows = by_id
        .values()
        .filter(|thread| thread.parent_thread_id.is_none())
        .map(|thread| AgentsOverviewRow {
            thread: (*thread).clone(),
            group: agents_overview_group(thread, &children),
            is_current: primary_thread_id == Some(thread.id.as_str()),
        })
        .collect::<Vec<_>>();
    if rows.is_empty() {
        rows = by_id
            .values()
            .map(|thread| AgentsOverviewRow {
                thread: (*thread).clone(),
                group: AgentsOverviewGroup::for_status(&thread.status),
                is_current: primary_thread_id == Some(thread.id.as_str()),
            })
            .collect();
    }
    rows.sort_by(|left, right| {
        left.group
            .cmp(&right.group)
            .then_with(|| right.thread.updated_at.cmp(&left.thread.updated_at))
            .then_with(|| left.thread.id.cmp(&right.thread.id))
    });
    rows
}

pub(crate) fn agents_overview_group(
    thread: &Thread,
    children: &HashMap<&str, Vec<&Thread>>,
) -> AgentsOverviewGroup {
    children.get(thread.id.as_str()).into_iter().flatten().fold(
        AgentsOverviewGroup::for_status(&thread.status),
        |group, child| group.min(agents_overview_group(child, children)),
    )
}

impl super::App {
    pub(crate) fn open_agents_overview(&mut self) {
        let primary = self.primary_thread_id.as_deref();
        let mut overview = AgentsOverviewState::new(primary);
        overview.rendered_full_screen = true;
        self.agents_overview = Some(overview);
        if !self.composer.text().trim_start().starts_with("/subagents") {
            self.capture_current_thread_input();
        }
    }

    #[allow(dead_code)]
    pub(crate) fn apply_agents_overview_thread_refresh(
        &mut self,
        generation: u64,
        result: Result<Vec<Thread>, String>,
    ) -> bool {
        let primary = self.primary_thread_id.clone();
        let Some(overview) = self.agents_overview.as_mut() else {
            return false;
        };
        match result {
            Ok(threads) => overview.apply_refresh(generation, threads, primary.as_deref()),
            Err(error) => {
                overview.refreshing = false;
                overview.request_id = None;
                self.projection
                    .set_status(format!("agents overview unavailable: {error}"));
                false
            }
        }
    }

    #[allow(dead_code)]
    pub(crate) fn select_agents_overview_thread(&self) -> Option<String> {
        self.agents_overview
            .as_ref()
            .and_then(|overview| overview.view.selected_thread_id().map(str::to_owned))
    }

    #[allow(dead_code)]
    pub(crate) fn agents_overview_view(
        &self,
        threads: Vec<Thread>,
        selected_thread_id: Option<&str>,
    ) -> AgentsOverviewView {
        let rows = build_rows(&threads, self.primary_thread_id.as_deref());
        AgentsOverviewView::new(rows, selected_thread_id)
    }

    #[allow(dead_code)]
    pub(crate) fn repaint_agents_overview(&mut self) {
        let primary = self.primary_thread_id.clone();
        if let Some(overview) = self.agents_overview.as_mut() {
            overview
                .view
                .update_rows(build_rows(&overview.threads, primary.as_deref()));
            overview.visible_thread_ids = overview
                .view
                .visible_rows()
                .into_iter()
                .map(|row| row.thread.id.clone())
                .collect();
            overview.sync_view_state();
        }
    }

    /// Start a background task through the current App Server session.
    pub(crate) async fn dispatch_agents_overview_task(
        &self,
        app_server: &AppServerSession,
        prompt: String,
        cwd: Option<PathBuf>,
    ) -> Result<(String, String)> {
        let thread = app_server
            .start_thread_with_session_start_source(
                cwd.unwrap_or_else(|| self.cwd.clone()),
                self.model.clone(),
                self.model_provider.clone(),
                None,
            )
            .await?;
        let thread_id = thread.thread.id.clone();
        let turn_id = self
            .submit_agents_overview_prompt(app_server, thread_id.clone(), prompt)
            .await?;
        Ok((thread_id, turn_id))
    }

    pub(crate) async fn submit_agents_overview_prompt(
        &self,
        app_server: &AppServerSession,
        thread_id: String,
        prompt: String,
    ) -> Result<String> {
        if prompt.trim().is_empty() {
            return Err(anyhow!("background task prompt must not be empty"));
        }
        app_server
            .turn_start(
                thread_id,
                vec![UserInput::Text {
                    text: prompt,
                    text_elements: Vec::new(),
                }],
            )
            .await
    }

    pub(crate) async fn stop_agents_overview_thread(
        &self,
        app_server: &AppServerSession,
        thread_id: String,
    ) -> Result<Option<String>> {
        let thread = app_server.thread_read(thread_id.clone(), true).await?;
        let turn_id = thread
            .thread
            .turns
            .into_iter()
            .rev()
            .find(|turn| turn.status == TurnStatus::InProgress)
            .map(|turn| turn.id);
        let Some(turn_id) = turn_id else {
            return Ok(None);
        };
        app_server
            .turn_interrupt(thread_id, turn_id.clone())
            .await?;
        Ok(Some(turn_id))
    }
}

#[cfg(test)]
#[path = "agents_overview_tests.rs"]
mod tests;
