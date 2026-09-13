use agent_protocol::response_item::MessagePhase;
use app_server_protocol::protocol::v2::{
    CollabAgentToolCallStatus, CommandExecutionStatus, DynamicToolCallStatus, HookRunStatus,
    McpToolCallStatus, PatchApplyStatus, PatchChangeKind, ServerNotification, Thread, ThreadItem,
    TurnStatus, UserInput,
};
use std::collections::{HashMap, HashSet};

use crate::history_cell::{
    compact_text, computer_activity_summary, invocation_text as mcp_invocation_text,
    is_computer_activity, summary as mcp_summary, web_search_detail,
};
use crate::history_filter::{
    filter_review_mode_items, filter_review_mode_items_with_state, filter_user_message_ids,
    hidden_user_message_ids, user_message_id,
};
use crate::multi_agents;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EntryKind {
    User,
    Assistant,
    Reasoning,
    Command,
    Patch,
    Mcp,
    Plan,
    MultiAgent,
    Tool,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EntryStatus {
    Running,
    Completed,
    Failed,
    Declined,
    Interrupted,
}

impl EntryStatus {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Declined => "declined",
            Self::Interrupted => "interrupted",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TranscriptEntry {
    pub(crate) id: String,
    pub(crate) kind: EntryKind,
    pub(crate) text: String,
    pub(crate) streaming: bool,
    pub(crate) status: Option<EntryStatus>,
    /// Stable, display-ready facts derived from the canonical item payload.
    pub(crate) summary: Vec<String>,
}

/// Completion metadata is attached to the last visible item of a completed turn.
///
/// It deliberately lives beside the item projection rather than inside `TranscriptEntry`: a
/// separator is presentation metadata and must not become a fake canonical ThreadItem or leak
/// into transcript export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CompletionBoundary {
    pub(crate) after_entry_id: String,
    pub(crate) elapsed_seconds: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CompletionMetadata {
    pub(crate) elapsed_seconds: Option<u64>,
}

#[derive(Debug, Default)]
pub(crate) struct ConversationProjection {
    entries: Vec<TranscriptEntry>,
    completion_boundaries: Vec<CompletionBoundary>,
    active_turn_id: Option<String>,
    status: String,
    review_mode: bool,
    active_hooks: Vec<ActiveHook>,
    /// Explicit assistant phases keyed by canonical item id. Legacy items
    /// omit this field and retain the historical final-answer behavior.
    assistant_phases: HashMap<String, MessagePhase>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ActiveHook {
    id: String,
    turn_id: Option<String>,
    status_message: Option<String>,
}

impl ConversationProjection {
    pub(crate) fn entries(&self) -> &[TranscriptEntry] {
        &self.entries
    }

    pub(crate) fn completion_after(&self, entry_id: &str) -> Option<&CompletionBoundary> {
        self.completion_boundaries
            .iter()
            .find(|boundary| boundary.after_entry_id == entry_id)
    }

    pub(crate) fn add_completion_boundary(
        &mut self,
        after_entry_id: impl Into<String>,
        elapsed_seconds: Option<u64>,
    ) {
        let after_entry_id = after_entry_id.into();
        if self
            .completion_boundaries
            .iter()
            .any(|boundary| boundary.after_entry_id == after_entry_id)
        {
            return;
        }
        self.completion_boundaries.push(CompletionBoundary {
            after_entry_id,
            elapsed_seconds,
        });
    }

    /// Remove entries previously projected from a page whose canonical review classification was
    /// completed only after loading adjacent Turn metadata.
    pub(crate) fn remove_hidden_entries(&mut self, hidden_ids: &HashSet<String>) {
        if hidden_ids.is_empty() {
            return;
        }
        self.entries.retain(|entry| !hidden_ids.contains(&entry.id));
        self.completion_boundaries
            .retain(|boundary| !hidden_ids.contains(&boundary.after_entry_id));
    }

    pub(crate) fn active_turn_id(&self) -> Option<&str> {
        self.active_turn_id.as_deref()
    }

    pub(crate) fn status(&self) -> &str {
        self.hook_status().unwrap_or(&self.status)
    }

    pub(crate) fn set_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
    }

    pub(crate) fn add_system_message(&mut self, message: impl Into<String>) {
        self.push_system(message.into());
    }

    pub(crate) fn start_turn(&mut self, turn_id: String) {
        self.active_turn_id = Some(turn_id);
        self.status = "running".to_string();
    }

    pub(crate) fn final_answer(&self) -> String {
        self.entries
            .iter()
            .rev()
            .find(|entry| {
                entry.kind == EntryKind::Assistant
                    && !entry.text.is_empty()
                    && self.assistant_phases.get(&entry.id) != Some(&MessagePhase::Commentary)
            })
            .map(|entry| entry.text.clone())
            .unwrap_or_default()
    }

    /// Prepend older canonical items while preserving transcript order.
    pub(crate) fn prepend_items(&mut self, items: impl IntoIterator<Item = ThreadItem>) {
        let items = filter_review_mode_items(&items.into_iter().collect::<Vec<_>>());
        let mut older = Vec::new();
        for item in items {
            self.record_assistant_phase(&item);
            if let Some(entry) = project_item(&item, false) {
                if !self.entries.iter().any(|current| current.id == entry.id) {
                    older.push(entry);
                }
            }
        }
        if older.is_empty() {
            return;
        }
        older.append(&mut self.entries);
        self.entries = older;
    }

    /// Prepend grouped history while applying canonical-id review filtering from Turn metadata.
    ///
    /// The completion boundary is resolved against the original group tail before filtering, so
    /// hiding a canonical user message cannot move a separator onto a previous visible item.
    pub(crate) fn prepend_grouped_items_with_hidden_ids(
        &mut self,
        groups: impl IntoIterator<Item = (Vec<ThreadItem>, Option<CompletionMetadata>)>,
        hidden_ids: &HashSet<String>,
    ) {
        let mut older = Vec::new();
        let mut boundaries = Vec::new();
        for (items, completion) in groups {
            let final_entry_id = items
                .last()
                .and_then(|item| project_item(item, false).map(|entry| entry.id));
            let filtered = filter_user_message_ids(&filter_review_mode_items(&items), hidden_ids);
            for item in filtered {
                self.record_assistant_phase(&item);
                if let Some(entry) = project_item(&item, false) {
                    if !self.entries.iter().any(|current| current.id == entry.id)
                        && !older
                            .iter()
                            .any(|current: &TranscriptEntry| current.id == entry.id)
                    {
                        older.push(entry);
                    }
                }
            }
            if let (Some(final_entry_id), Some(completion)) = (final_entry_id, completion) {
                if self.entries.iter().any(|entry| entry.id == final_entry_id)
                    || older.iter().any(|entry| entry.id == final_entry_id)
                {
                    boundaries.push(CompletionBoundary {
                        after_entry_id: final_entry_id,
                        elapsed_seconds: completion.elapsed_seconds,
                    });
                }
            }
        }
        if !older.is_empty() {
            older.append(&mut self.entries);
            self.entries = older;
        }
        boundaries.retain(|boundary| {
            !self
                .completion_boundaries
                .iter()
                .any(|known| known.after_entry_id == boundary.after_entry_id)
        });
        boundaries.append(&mut self.completion_boundaries);
        self.completion_boundaries = boundaries;
    }

    pub(crate) fn hydrate_thread(&mut self, thread: Thread) {
        self.entries.clear();
        self.completion_boundaries.clear();
        self.active_turn_id = None;
        self.status = "ready".to_string();
        self.review_mode = false;
        self.active_hooks.clear();
        self.assistant_phases.clear();

        let hidden_user_messages = hidden_user_message_ids(&thread.turns);
        for turn in thread.turns {
            if turn.status == TurnStatus::InProgress {
                self.active_turn_id = Some(turn.id.clone());
                self.status = "running".to_string();
            }
            let final_entry_id = (turn.status == TurnStatus::Completed)
                .then(|| turn.items.last())
                .flatten()
                .and_then(|item| project_item(item, false).map(|entry| entry.id));
            for item in turn.items {
                self.update_review_mode(&item);
                self.record_assistant_phase(&item);
                if user_message_id(&item).is_some_and(|id| hidden_user_messages.contains(id)) {
                    continue;
                }
                if let Some(entry) = project_item(&item, false) {
                    self.replace_entry(entry);
                }
            }
            if turn.status == TurnStatus::Completed {
                if let Some(entry_id) =
                    final_entry_id.filter(|id| self.entries.iter().any(|current| current.id == *id))
                {
                    self.add_completion_boundary(
                        entry_id,
                        turn.duration_ms
                            .and_then(|duration| u64::try_from(duration).ok())
                            .map(|duration| duration / 1_000),
                    );
                }
            }
            if self.active_turn_id.is_none() {
                self.status = turn_status(turn.status).to_string();
            }
        }
    }

    pub(crate) fn apply(&mut self, notification: ServerNotification) {
        match notification {
            ServerNotification::TurnStarted(params) => {
                self.active_turn_id = Some(params.turn.id);
                self.status = "running".to_string();
            }
            ServerNotification::TurnCompleted(params) => {
                self.active_turn_id = None;
                self.active_hooks
                    .retain(|hook| hook.turn_id.as_deref() != Some(params.turn.id.as_str()));
                // The terminal client may miss an item delta or item/completed
                // notification while the transport is reconnecting. The
                // completed turn is the canonical repair point for those
                // transcript entries.
                let web_search_lifecycle = match params.turn.status {
                    TurnStatus::Completed => WebSearchLifecycle::Completed,
                    TurnStatus::Interrupted | TurnStatus::Failed | TurnStatus::InProgress => {
                        WebSearchLifecycle::Historical
                    }
                };
                self.merge_canonical_items(&params.turn.items, web_search_lifecycle);
                if params.turn.status == TurnStatus::Completed {
                    let last_entry_id = params.turn.items.last().and_then(|item| {
                        project_item(item, false)
                            .map(|entry| entry.id)
                            .filter(|id| self.entries.iter().any(|current| current.id == *id))
                    });
                    if let Some(entry_id) = last_entry_id {
                        self.add_completion_boundary(
                            entry_id,
                            params
                                .turn
                                .duration_ms
                                .and_then(|duration| u64::try_from(duration).ok())
                                .map(|duration| duration / 1_000),
                        );
                    }
                }
                self.settle_running_entries(params.turn.status);
                self.status = turn_status(params.turn.status).to_string();
            }
            ServerNotification::ItemStarted(params) => {
                if self.should_hide_realtime_item(&params.item) {
                    return;
                }
                if let Some(entry) =
                    project_item_with_lifecycle(&params.item, true, WebSearchLifecycle::Started)
                {
                    self.record_assistant_phase(&params.item);
                    self.replace_entry(entry);
                }
            }
            ServerNotification::ItemCompleted(params) => {
                if self.should_hide_realtime_item(&params.item) {
                    return;
                }
                if let Some(entry) =
                    project_item_with_lifecycle(&params.item, false, WebSearchLifecycle::Completed)
                {
                    self.record_assistant_phase(&params.item);
                    self.replace_entry(entry);
                }
            }
            ServerNotification::AgentMessageDelta(params) => {
                self.append_delta(params.item_id, EntryKind::Assistant, params.delta);
            }
            ServerNotification::ReasoningSummaryTextDelta(params) => {
                self.append_delta(params.item_id, EntryKind::Reasoning, params.delta);
            }
            ServerNotification::ReasoningTextDelta(params) => {
                self.append_delta(params.item_id, EntryKind::Reasoning, params.delta);
            }
            ServerNotification::PlanDelta(params) => {
                self.append_delta(params.item_id, EntryKind::Plan, params.delta);
            }
            ServerNotification::CommandExecutionOutputDelta(params) => {
                self.append_delta(params.item_id, EntryKind::Command, params.delta);
            }
            ServerNotification::FileChangePatchUpdated(params) => {
                self.replace_entry(TranscriptEntry {
                    id: params.item_id,
                    kind: EntryKind::Patch,
                    text: format_patch(&params.changes),
                    streaming: true,
                    status: Some(EntryStatus::Running),
                    summary: Vec::new(),
                });
            }
            ServerNotification::TurnDiffUpdated(params) => {
                self.replace_entry(TranscriptEntry {
                    id: format!("turn-{}-diff", params.turn_id),
                    kind: EntryKind::Patch,
                    text: params.diff,
                    streaming: true,
                    status: Some(EntryStatus::Running),
                    summary: Vec::new(),
                });
            }
            ServerNotification::TurnPlanUpdated(params) => {
                let text = params
                    .plan
                    .iter()
                    .map(|step| format!("{} {}", plan_marker(step.status), step.step))
                    .collect::<Vec<_>>()
                    .join("\n");
                self.replace_entry(TranscriptEntry {
                    id: format!("turn-{}-plan", params.turn_id),
                    kind: EntryKind::Plan,
                    text,
                    streaming: true,
                    status: Some(EntryStatus::Running),
                    summary: Vec::new(),
                });
            }
            ServerNotification::Warning(params) => {
                self.push_system(format!("warning: {}", params.message));
            }
            ServerNotification::Error(params) => {
                self.status = if params.will_retry {
                    "retrying".to_string()
                } else {
                    "failed".to_string()
                };
                self.push_system(params.error.message);
            }
            ServerNotification::HookStarted(params) => {
                self.start_hook(params.turn_id, params.run);
            }
            ServerNotification::HookCompleted(params) => {
                self.complete_hook(params.run);
            }
            _ => {}
        }
    }

    fn start_hook(
        &mut self,
        turn_id: Option<String>,
        run: app_server_protocol::protocol::v2::HookRunSummary,
    ) {
        if run.status != HookRunStatus::Running {
            return;
        }
        if let Some(existing) = self.active_hooks.iter_mut().find(|hook| hook.id == run.id) {
            existing.turn_id = turn_id;
            existing.status_message = run.status_message;
            return;
        }
        self.active_hooks.push(ActiveHook {
            id: run.id,
            turn_id,
            status_message: run.status_message,
        });
    }

    fn complete_hook(&mut self, run: app_server_protocol::protocol::v2::HookRunSummary) {
        self.active_hooks.retain(|hook| hook.id != run.id);
        let Some(text) = crate::history_cell::status_text(run.status) else {
            return;
        };
        if crate::history_cell::is_quiet_success(&run) {
            return;
        }
        let status = match run.status {
            HookRunStatus::Completed => EntryStatus::Completed,
            HookRunStatus::Running => EntryStatus::Running,
            HookRunStatus::Failed | HookRunStatus::Blocked | HookRunStatus::Stopped => {
                EntryStatus::Failed
            }
        };
        self.replace_entry(TranscriptEntry {
            id: format!("hook-{}", run.id),
            kind: EntryKind::System,
            text: text.to_string(),
            streaming: false,
            status: Some(status),
            summary: crate::history_cell::output_details(&run),
        });
    }

    fn hook_status(&self) -> Option<&str> {
        if self.active_hooks.is_empty() {
            return None;
        }
        let first = self.active_hooks[0]
            .status_message
            .as_deref()
            .map(str::trim)
            .filter(|message| !message.is_empty());
        if self.active_hooks.len() == 1 {
            return first.or(Some("running hook"));
        }
        if first.is_some()
            && self
                .active_hooks
                .iter()
                .all(|hook| hook.status_message.as_deref().map(str::trim) == first)
        {
            return first;
        }
        Some("running hooks")
    }

    fn append_delta(&mut self, id: String, kind: EntryKind, delta: String) {
        if let Some(entry) = self.entries.iter_mut().find(|entry| entry.id == id) {
            entry.text.push_str(&delta);
            entry.streaming = true;
            return;
        }
        self.entries.push(TranscriptEntry {
            id,
            kind,
            text: delta,
            streaming: true,
            status: (kind == EntryKind::Command || kind == EntryKind::Plan)
                .then_some(EntryStatus::Running),
            summary: Vec::new(),
        });
    }

    fn update_review_mode(&mut self, item: &ThreadItem) {
        match item {
            ThreadItem::EnteredReviewMode { .. } => self.review_mode = true,
            ThreadItem::ExitedReviewMode { .. } => self.review_mode = false,
            _ => {}
        }
    }

    fn should_hide_realtime_item(&mut self, item: &ThreadItem) -> bool {
        let hidden = matches!(item, ThreadItem::UserMessage { .. }) && self.review_mode;
        self.update_review_mode(item);
        hidden
    }

    fn replace_entry(&mut self, entry: TranscriptEntry) {
        if let Some(current) = self
            .entries
            .iter_mut()
            .find(|current| current.id == entry.id)
        {
            *current = entry;
        } else {
            self.entries.push(entry);
        }
    }

    fn merge_canonical_items(
        &mut self,
        items: &[ThreadItem],
        web_search_lifecycle: WebSearchLifecycle,
    ) {
        let initial_review_mode = self.review_mode;
        let filtered = filter_review_mode_items_with_state(items, initial_review_mode);
        for item in items {
            self.update_review_mode(item);
            self.record_assistant_phase(item);
        }
        let projected = filtered
            .iter()
            .filter_map(|item| project_item_with_lifecycle(item, false, web_search_lifecycle))
            .collect::<Vec<_>>();

        for (index, entry) in projected.into_iter().enumerate() {
            if let Some(current) = self
                .entries
                .iter_mut()
                .find(|current| current.id == entry.id)
            {
                *current = entry;
                continue;
            }

            // Place a repaired item next to the nearest canonical neighbor so
            // a missing user message cannot appear after its assistant reply.
            let next_index = filtered
                .iter()
                .skip(index + 1)
                .filter_map(projected_item_id)
                .find_map(|id| self.entries.iter().position(|current| current.id == id));
            let previous_index = filtered
                .iter()
                .take(index)
                .filter_map(projected_item_id)
                .rev()
                .find_map(|id| self.entries.iter().position(|current| current.id == id));
            let insert_at = next_index
                .or_else(|| previous_index.map(|position| position + 1))
                .unwrap_or(self.entries.len());
            self.entries.insert(insert_at, entry);
        }
    }

    fn push_system(&mut self, text: String) {
        self.entries.push(TranscriptEntry {
            id: format!("system-{}", self.entries.len()),
            kind: EntryKind::System,
            text,
            streaming: false,
            status: None,
            summary: Vec::new(),
        });
    }

    fn record_assistant_phase(&mut self, item: &ThreadItem) {
        if let ThreadItem::AgentMessage { id, phase, .. } = item {
            match phase {
                Some(phase) => {
                    self.assistant_phases.insert(id.clone(), phase.clone());
                }
                None => {
                    self.assistant_phases.remove(id);
                }
            }
        }
    }

    fn settle_running_entries(&mut self, status: TurnStatus) {
        let Some(entry_status) = (match status {
            TurnStatus::Completed => Some(EntryStatus::Completed),
            TurnStatus::Failed => Some(EntryStatus::Failed),
            TurnStatus::Interrupted => Some(EntryStatus::Interrupted),
            TurnStatus::InProgress => None,
        }) else {
            return;
        };

        for entry in &mut self.entries {
            if entry.status == Some(EntryStatus::Running) {
                entry.status = Some(entry_status);
            }
        }
    }
}

fn projected_item_id(item: &ThreadItem) -> Option<String> {
    project_item(item, false).map(|entry| entry.id)
}

fn turn_status(status: TurnStatus) -> &'static str {
    match status {
        TurnStatus::Completed => "ready",
        TurnStatus::Interrupted => "interrupted",
        TurnStatus::Failed => "failed",
        TurnStatus::InProgress => "running",
    }
}

fn plan_marker(status: app_server_protocol::protocol::v2::TurnPlanStepStatus) -> &'static str {
    match status {
        app_server_protocol::protocol::v2::TurnPlanStepStatus::Pending => "[ ]",
        app_server_protocol::protocol::v2::TurnPlanStepStatus::InProgress => "[~]",
        app_server_protocol::protocol::v2::TurnPlanStepStatus::Completed => "[x]",
    }
}

fn project_item(item: &ThreadItem, streaming: bool) -> Option<TranscriptEntry> {
    project_item_with_lifecycle(item, streaming, WebSearchLifecycle::Historical)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WebSearchLifecycle {
    Historical,
    Started,
    Completed,
}

fn project_item_with_lifecycle(
    item: &ThreadItem,
    streaming: bool,
    web_search_lifecycle: WebSearchLifecycle,
) -> Option<TranscriptEntry> {
    let (id, kind, text, status, summary) = match item {
        ThreadItem::UserMessage {
            id,
            client_id,
            content,
            ..
        } => {
            let (text, summary) = user_input_projection(content);
            (
                client_id.as_ref().unwrap_or(id).clone(),
                EntryKind::User,
                text,
                None,
                summary,
            )
        }
        ThreadItem::HookPrompt { id, fragments, .. } => (
            id.clone(),
            EntryKind::System,
            fragments
                .iter()
                .map(|fragment| fragment.text.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
            None,
            Vec::new(),
        ),
        ThreadItem::AgentMessage { id, text, .. } => (
            id.clone(),
            EntryKind::Assistant,
            text.clone(),
            None,
            Vec::new(),
        ),
        ThreadItem::Plan { id, text, .. } => (
            id.clone(),
            EntryKind::Plan,
            text.clone(),
            Some(EntryStatus::Completed),
            Vec::new(),
        ),
        ThreadItem::Reasoning {
            id,
            summary,
            content,
            ..
        } => {
            let text = if summary.is_empty() { content } else { summary }.join("\n");
            (id.clone(), EntryKind::Reasoning, text, None, Vec::new())
        }
        ThreadItem::CommandExecution {
            id,
            command,
            status,
            aggregated_output,
            exit_code,
            duration_ms,
            ..
        } => {
            let mut summary = Vec::new();
            if let Some(exit_code) = exit_code {
                summary.push(format!("exit {exit_code}"));
            }
            if let Some(duration_ms) = duration_ms {
                summary.push(format!("duration {duration_ms}ms"));
            }
            (
                id.clone(),
                EntryKind::Command,
                command_text(command, aggregated_output.as_deref(), streaming),
                Some(command_entry_status(*status)),
                summary,
            )
        }
        ThreadItem::FileChange {
            id,
            changes,
            status,
            ..
        } => (
            id.clone(),
            EntryKind::Patch,
            format_patch(changes),
            Some(patch_entry_status(*status)),
            patch_summary(changes),
        ),
        ThreadItem::McpToolCall {
            id,
            server,
            tool,
            arguments,
            status,
            result,
            error,
            duration_ms,
            ..
        } => {
            let mut summary = mcp_summary(result.as_deref(), error.as_ref(), *duration_ms);
            if is_computer_activity(server) {
                summary.extend(computer_activity_summary(
                    arguments,
                    result.as_deref(),
                    error.as_ref(),
                ));
            }
            (
                id.clone(),
                EntryKind::Mcp,
                mcp_invocation_text(server, tool, arguments),
                Some(mcp_entry_status(*status)),
                summary,
            )
        }
        ThreadItem::DynamicToolCall {
            id,
            tool,
            status,
            content_items,
            success,
            duration_ms,
            ..
        } => (
            id.clone(),
            EntryKind::Tool,
            tool.clone(),
            Some(dynamic_entry_status(*status)),
            dynamic_summary(content_items.as_deref(), *success, *duration_ms),
        ),
        item @ ThreadItem::CollabAgentToolCall {
            id, tool, status, ..
        } => {
            let text = multi_agents::tool_call_history_cell(item)
                .map(|cell| cell.title)
                .unwrap_or_else(|| format!("{tool:?}"));
            (
                id.clone(),
                EntryKind::MultiAgent,
                text,
                Some(collab_entry_status(*status)),
                multi_agents::collab_summary_for_item(item),
            )
        }
        item @ ThreadItem::SubAgentActivity {
            id,
            kind,
            agent_path,
            ..
        } => {
            let status = multi_agents::sub_agent_activity_display(item)
                .map(|activity| {
                    if activity.is_running_hint {
                        EntryStatus::Running
                    } else {
                        EntryStatus::Interrupted
                    }
                })
                .or(Some(EntryStatus::Running));
            (
                id.clone(),
                EntryKind::MultiAgent,
                multi_agents::sub_agent_activity_summary(*kind, agent_path),
                status,
                Vec::new(),
            )
        }
        ThreadItem::WebSearch(item) => {
            let detail =
                web_search_detail(item.query.as_deref().unwrap_or(""), item.action.as_ref());
            let text = match web_search_lifecycle {
                WebSearchLifecycle::Historical => {
                    if detail.is_empty() {
                        "web search".to_string()
                    } else {
                        format!("web search: {detail}")
                    }
                }
                WebSearchLifecycle::Started => {
                    if detail.is_empty() {
                        "searching the web".to_string()
                    } else {
                        format!("searching the web {detail}")
                    }
                }
                WebSearchLifecycle::Completed => {
                    if detail.is_empty() {
                        "searched the web".to_string()
                    } else {
                        format!("searched the web for {detail}")
                    }
                }
            };
            (item.id.clone(), EntryKind::Tool, text, None, Vec::new())
        }
        ThreadItem::ImageView { id, path, .. } => (
            id.clone(),
            EntryKind::Tool,
            format!("view image: {path}"),
            None,
            Vec::new(),
        ),
        // Sleep is an internal runtime control item in Codex and is not part of the
        // user-visible transcript or agent status feed.
        ThreadItem::Sleep(_) => return None,
        ThreadItem::ImageGeneration(item) => {
            let mut summary = Vec::new();
            if let Some(path) = item.saved_path.as_deref() {
                summary.push(format!("saved: {path}"));
            }
            if let Some(prompt) = item.revised_prompt.as_deref() {
                summary.push(format!("revised prompt: {}", compact_text(prompt)));
            }
            (
                item.id.clone(),
                EntryKind::Tool,
                "image generation".to_string(),
                image_generation_status(&item.status),
                summary,
            )
        }
        ThreadItem::EnteredReviewMode { id, review, .. } => (
            id.clone(),
            EntryKind::System,
            format!("review started: {review}"),
            Some(EntryStatus::Running),
            Vec::new(),
        ),
        ThreadItem::ExitedReviewMode { id, review, .. } => (
            id.clone(),
            EntryKind::System,
            format!("review completed: {review}"),
            Some(EntryStatus::Completed),
            Vec::new(),
        ),
        ThreadItem::ContextCompaction { id, .. } => (
            id.clone(),
            EntryKind::System,
            "context compacted".to_string(),
            None,
            Vec::new(),
        ),
        ThreadItem::UnknownItem {
            id, upstream_type, ..
        } => (
            id.clone(),
            EntryKind::System,
            format!("unsupported item: {upstream_type}"),
            Some(EntryStatus::Failed),
            Vec::new(),
        ),
    };

    Some(TranscriptEntry {
        id,
        kind,
        text,
        streaming,
        status,
        summary,
    })
}

fn command_text(command: &str, output: Option<&str>, streaming: bool) -> String {
    let mut text = command.to_string();
    if let Some(output) = output.filter(|output| !output.is_empty()) {
        if !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(output);
    } else if streaming && !text.ends_with('\n') {
        text.push('\n');
    }
    text
}

fn patch_summary(changes: &[app_server_protocol::protocol::v2::FileUpdateChange]) -> Vec<String> {
    let mut added = 0;
    let mut deleted = 0;
    let mut updated = 0;
    for change in changes {
        match &change.kind {
            PatchChangeKind::Add => added += 1,
            PatchChangeKind::Delete => deleted += 1,
            PatchChangeKind::Update { .. } => updated += 1,
        }
    }
    let mut details = vec![format!("files: {}", changes.len())];
    if added > 0 {
        details.push(format!("added: {added}"));
    }
    if deleted > 0 {
        details.push(format!("deleted: {deleted}"));
    }
    if updated > 0 {
        details.push(format!("updated: {updated}"));
    }
    details
}

fn dynamic_summary(
    content_items: Option<&[app_server_protocol::protocol::v2::DynamicToolCallOutputContentItem]>,
    success: Option<bool>,
    duration_ms: Option<i64>,
) -> Vec<String> {
    let mut details = Vec::new();
    if let Some(success) = success {
        details.push(format!("success: {success}"));
    }
    if let Some(content_items) = content_items {
        details.push(format!("content items: {}", content_items.len()));
        details.extend(dynamic_content_previews(content_items));
    }
    if let Some(duration_ms) = duration_ms {
        details.push(format!("duration {duration_ms}ms"));
    }
    details
}

fn dynamic_content_previews(
    content_items: &[app_server_protocol::protocol::v2::DynamicToolCallOutputContentItem],
) -> Vec<String> {
    content_items
        .iter()
        .filter_map(|item| match item {
            app_server_protocol::protocol::v2::DynamicToolCallOutputContentItem::InputText {
                text,
            } if !text.trim().is_empty() => Some(format!("output: {}", compact_text(text))),
            _ => None,
        })
        .take(4)
        .collect()
}

fn command_entry_status(status: CommandExecutionStatus) -> EntryStatus {
    match status {
        CommandExecutionStatus::InProgress => EntryStatus::Running,
        CommandExecutionStatus::Completed => EntryStatus::Completed,
        CommandExecutionStatus::Failed => EntryStatus::Failed,
        CommandExecutionStatus::Declined => EntryStatus::Declined,
    }
}

fn patch_entry_status(status: PatchApplyStatus) -> EntryStatus {
    match status {
        PatchApplyStatus::InProgress => EntryStatus::Running,
        PatchApplyStatus::Completed => EntryStatus::Completed,
        PatchApplyStatus::Failed => EntryStatus::Failed,
        PatchApplyStatus::Declined => EntryStatus::Declined,
    }
}

fn mcp_entry_status(status: McpToolCallStatus) -> EntryStatus {
    match status {
        McpToolCallStatus::InProgress => EntryStatus::Running,
        McpToolCallStatus::Completed => EntryStatus::Completed,
        McpToolCallStatus::Failed => EntryStatus::Failed,
    }
}

fn dynamic_entry_status(status: DynamicToolCallStatus) -> EntryStatus {
    match status {
        DynamicToolCallStatus::InProgress => EntryStatus::Running,
        DynamicToolCallStatus::Completed => EntryStatus::Completed,
        DynamicToolCallStatus::Failed => EntryStatus::Failed,
    }
}

fn collab_entry_status(status: CollabAgentToolCallStatus) -> EntryStatus {
    match status {
        CollabAgentToolCallStatus::InProgress => EntryStatus::Running,
        CollabAgentToolCallStatus::Completed => EntryStatus::Completed,
        CollabAgentToolCallStatus::Failed => EntryStatus::Failed,
    }
}

fn image_generation_status(status: &str) -> Option<EntryStatus> {
    match status.to_ascii_lowercase().as_str() {
        "in_progress" | "in-progress" | "running" => Some(EntryStatus::Running),
        "completed" | "succeeded" | "success" => Some(EntryStatus::Completed),
        "failed" | "error" => Some(EntryStatus::Failed),
        "declined" => Some(EntryStatus::Declined),
        _ => None,
    }
}

fn format_patch(changes: &[app_server_protocol::protocol::v2::FileUpdateChange]) -> String {
    changes
        .iter()
        .map(|change| {
            let (kind, move_path) = match &change.kind {
                app_server_protocol::protocol::v2::PatchChangeKind::Add => ("added", None),
                app_server_protocol::protocol::v2::PatchChangeKind::Delete => ("deleted", None),
                app_server_protocol::protocol::v2::PatchChangeKind::Update { move_path } => {
                    ("updated", move_path.as_deref())
                }
            };
            let path = move_path
                .filter(|path| !path.trim().is_empty())
                .map(|path| format!("{} → {path}", change.path))
                .unwrap_or_else(|| change.path.clone());
            if change.diff.trim().is_empty() {
                format!("{kind} {path}")
            } else {
                format!("{kind} {path}\n{}", change.diff)
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn user_input_projection(content: &[UserInput]) -> (String, Vec<String>) {
    let mut text = Vec::new();
    let mut image_count = 0usize;

    for input in content {
        match input {
            UserInput::Text { text: value, .. } => text.push(value.clone()),
            UserInput::Image { .. } | UserInput::LocalImage { .. } => {
                image_count += 1;
            }
            UserInput::Skill { name, .. } => text.push(format!("[skill: {name}]")),
            UserInput::Mention { name, .. } => text.push(format!("[@{name}]")),
        }
    }

    let summary = (1..=image_count)
        .map(|index| format!("image: {index}"))
        .collect();
    (text.join("\n"), summary)
}

#[cfg(test)]
fn mcp_content_previews(content: &[serde_json::Value]) -> Vec<String> {
    crate::history_cell::content_previews(content)
}

#[cfg(test)]
#[path = "projection_tests.rs"]
mod tests;
