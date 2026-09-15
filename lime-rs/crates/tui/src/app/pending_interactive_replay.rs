//! Pending interactive request state used by Codex-shaped thread event replay.
//!
//! A request can outlive the view that first received it. The replay state is therefore kept
//! separately from the bounded event queue and is updated by inbound server lifecycle
//! notifications, outbound responses, and queue eviction.

use app_server_protocol::protocol::v2::{ServerNotification, ServerRequest, ThreadItem};
use app_server_protocol::RequestId;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct ElicitationRequestKey {
    server_name: String,
    request_id: RequestId,
}

impl ElicitationRequestKey {
    fn new(server_name: String, request_id: RequestId) -> Self {
        Self {
            server_name,
            request_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PendingInteractiveRequest {
    ExecApproval {
        turn_id: String,
        item_id: String,
        approval_id: String,
    },
    PatchApproval {
        turn_id: String,
        item_id: String,
    },
    Elicitation(ElicitationRequestKey),
    RequestPermissions {
        turn_id: String,
        item_id: String,
    },
    RequestUserInput {
        turn_id: String,
        item_id: String,
    },
}

#[derive(Debug, Default)]
// Tracks which interactive prompts are still unresolved in the thread-event buffer.
//
// Thread snapshots are replayed when switching threads/agents. Most events replay verbatim, but
// approvals and input prompts only replay while they are still pending. The fast lookup sets are
// paired with turn-indexed queues so turn completion and FIFO user-input answers can clear stale
// prompts without scanning the buffered event list.
pub(super) struct PendingInteractiveReplayState {
    exec_approval_call_ids: HashSet<String>,
    exec_approval_call_ids_by_turn_id: HashMap<String, Vec<String>>,
    patch_approval_call_ids: HashSet<String>,
    patch_approval_call_ids_by_turn_id: HashMap<String, Vec<String>>,
    elicitation_requests: HashSet<ElicitationRequestKey>,
    request_permissions_call_ids: HashSet<String>,
    request_permissions_call_ids_by_turn_id: HashMap<String, Vec<String>>,
    request_user_input_call_ids: HashSet<String>,
    request_user_input_call_ids_by_turn_id: HashMap<String, Vec<String>>,
    pending_requests_by_request_id: HashMap<RequestId, PendingInteractiveRequest>,
}

impl PendingInteractiveReplayState {
    pub(super) fn note_server_request(&mut self, request: &ServerRequest) {
        // Request IDs are the response identity in Lime's protocol. If a server reuses one, make
        // sure the old classification cannot leak into the replacement request.
        self.remove_request(request.id());

        match request {
            ServerRequest::ItemCommandExecutionRequestApproval { id, params } => {
                let approval_id = params
                    .approval_id
                    .clone()
                    .unwrap_or_else(|| params.item_id.clone());
                self.exec_approval_call_ids.insert(approval_id.clone());
                self.exec_approval_call_ids_by_turn_id
                    .entry(params.turn_id.clone())
                    .or_default()
                    .push(approval_id.clone());
                self.pending_requests_by_request_id.insert(
                    id.clone(),
                    PendingInteractiveRequest::ExecApproval {
                        turn_id: params.turn_id.clone(),
                        item_id: params.item_id.clone(),
                        approval_id,
                    },
                );
            }
            ServerRequest::ItemFileChangeRequestApproval { id, params } => {
                self.patch_approval_call_ids.insert(params.item_id.clone());
                self.patch_approval_call_ids_by_turn_id
                    .entry(params.turn_id.clone())
                    .or_default()
                    .push(params.item_id.clone());
                self.pending_requests_by_request_id.insert(
                    id.clone(),
                    PendingInteractiveRequest::PatchApproval {
                        turn_id: params.turn_id.clone(),
                        item_id: params.item_id.clone(),
                    },
                );
            }
            ServerRequest::McpServerElicitationRequest { id, params } => {
                let key = ElicitationRequestKey::new(params.server_name.clone(), id.clone());
                self.elicitation_requests.insert(key.clone());
                self.pending_requests_by_request_id
                    .insert(id.clone(), PendingInteractiveRequest::Elicitation(key));
            }
            ServerRequest::ItemPermissionsRequestApproval { id, params } => {
                self.request_permissions_call_ids
                    .insert(params.item_id.clone());
                self.request_permissions_call_ids_by_turn_id
                    .entry(params.turn_id.clone())
                    .or_default()
                    .push(params.item_id.clone());
                self.pending_requests_by_request_id.insert(
                    id.clone(),
                    PendingInteractiveRequest::RequestPermissions {
                        turn_id: params.turn_id.clone(),
                        item_id: params.item_id.clone(),
                    },
                );
            }
            ServerRequest::ItemToolRequestUserInput { id, params } => {
                self.request_user_input_call_ids
                    .insert(params.item_id.clone());
                self.request_user_input_call_ids_by_turn_id
                    .entry(params.turn_id.clone())
                    .or_default()
                    .push(params.item_id.clone());
                self.pending_requests_by_request_id.insert(
                    id.clone(),
                    PendingInteractiveRequest::RequestUserInput {
                        turn_id: params.turn_id.clone(),
                        item_id: params.item_id.clone(),
                    },
                );
            }
            ServerRequest::CurrentTimeRead { .. } | ServerRequest::DynamicToolCall { .. } => {}
        }
    }

    pub(super) fn note_server_notification(&mut self, notification: &ServerNotification) {
        match notification {
            ServerNotification::ItemStarted(notification) => match &notification.item {
                ThreadItem::CommandExecution { id, .. } => {
                    self.clear_exec_approval_item(id);
                }
                ThreadItem::FileChange { id, .. } => {
                    self.clear_patch_approval_item(id);
                }
                _ => {}
            },
            ServerNotification::TurnCompleted(params) => {
                self.clear_exec_approval_turn(&params.turn.id);
                self.clear_patch_approval_turn(&params.turn.id);
                self.clear_request_permissions_turn(&params.turn.id);
                self.clear_request_user_input_turn(&params.turn.id);
            }
            ServerNotification::ServerRequestResolved(params) => {
                self.remove_request(&params.request_id);
            }
            ServerNotification::ThreadClosed(_) => self.clear(),
            _ => {}
        }
    }

    pub(super) fn note_evicted_server_request(&mut self, request: &ServerRequest) {
        if self
            .pending_requests_by_request_id
            .contains_key(request.id())
        {
            self.remove_request(request.id());
            return;
        }

        // The request may have been replaced or filtered before eviction. Remove its category
        // identity as a defensive fallback so an evicted prompt can never replay.
        match request {
            ServerRequest::ItemCommandExecutionRequestApproval { params, .. } => {
                let approval_id = params.approval_id.as_ref().unwrap_or(&params.item_id);
                self.exec_approval_call_ids.remove(approval_id);
                Self::remove_call_id_from_turn_map_entry(
                    &mut self.exec_approval_call_ids_by_turn_id,
                    &params.turn_id,
                    approval_id,
                );
            }
            ServerRequest::ItemFileChangeRequestApproval { params, .. } => {
                self.patch_approval_call_ids.remove(&params.item_id);
                Self::remove_call_id_from_turn_map_entry(
                    &mut self.patch_approval_call_ids_by_turn_id,
                    &params.turn_id,
                    &params.item_id,
                );
            }
            ServerRequest::McpServerElicitationRequest { id, params } => {
                self.elicitation_requests
                    .remove(&ElicitationRequestKey::new(
                        params.server_name.clone(),
                        id.clone(),
                    ));
            }
            ServerRequest::ItemPermissionsRequestApproval { params, .. } => {
                self.request_permissions_call_ids.remove(&params.item_id);
                Self::remove_call_id_from_turn_map_entry(
                    &mut self.request_permissions_call_ids_by_turn_id,
                    &params.turn_id,
                    &params.item_id,
                );
            }
            ServerRequest::ItemToolRequestUserInput { params, .. } => {
                self.request_user_input_call_ids.remove(&params.item_id);
                Self::remove_call_id_from_turn_map_entry(
                    &mut self.request_user_input_call_ids_by_turn_id,
                    &params.turn_id,
                    &params.item_id,
                );
            }
            ServerRequest::CurrentTimeRead { .. } | ServerRequest::DynamicToolCall { .. } => {}
        }
    }

    pub(super) fn note_outbound_response(&mut self, request_id: &RequestId) {
        self.remove_request(request_id);
    }

    pub(super) fn should_replay_snapshot_request(&self, request: &ServerRequest) -> bool {
        match request {
            ServerRequest::ItemCommandExecutionRequestApproval { params, .. } => self
                .exec_approval_call_ids
                .contains(params.approval_id.as_ref().unwrap_or(&params.item_id)),
            ServerRequest::ItemFileChangeRequestApproval { params, .. } => {
                self.patch_approval_call_ids.contains(&params.item_id)
            }
            ServerRequest::McpServerElicitationRequest { id, params } => self
                .elicitation_requests
                .contains(&ElicitationRequestKey::new(
                    params.server_name.clone(),
                    id.clone(),
                )),
            ServerRequest::ItemPermissionsRequestApproval { params, .. } => {
                self.request_permissions_call_ids.contains(&params.item_id)
            }
            ServerRequest::ItemToolRequestUserInput { params, .. } => {
                self.request_user_input_call_ids.contains(&params.item_id)
            }
            ServerRequest::CurrentTimeRead { .. } | ServerRequest::DynamicToolCall { .. } => true,
        }
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn has_pending_thread_approvals(&self) -> bool {
        !self.exec_approval_call_ids.is_empty()
            || !self.patch_approval_call_ids.is_empty()
            || !self.elicitation_requests.is_empty()
            || !self.request_permissions_call_ids.is_empty()
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn has_pending_thread_user_input(&self) -> bool {
        !self.request_user_input_call_ids.is_empty()
    }

    fn clear_exec_approval_turn(&mut self, turn_id: &str) {
        if let Some(call_ids) = self.exec_approval_call_ids_by_turn_id.remove(turn_id) {
            for call_id in call_ids {
                self.exec_approval_call_ids.remove(&call_id);
            }
        }
        self.pending_requests_by_request_id.retain(|_, pending| {
            !matches!(
                pending,
                PendingInteractiveRequest::ExecApproval {
                    turn_id: pending_turn_id,
                    ..
                } if pending_turn_id == turn_id
            )
        });
    }

    fn clear_exec_approval_item(&mut self, item_id: &str) {
        let matching = self
            .pending_requests_by_request_id
            .values()
            .filter_map(|pending| match pending {
                PendingInteractiveRequest::ExecApproval {
                    turn_id,
                    item_id: pending_item_id,
                    approval_id,
                } if pending_item_id == item_id => Some((turn_id.clone(), approval_id.clone())),
                _ => None,
            })
            .collect::<Vec<_>>();
        self.exec_approval_call_ids.remove(item_id);
        for (turn_id, approval_id) in matching {
            self.exec_approval_call_ids.remove(&approval_id);
            Self::remove_call_id_from_turn_map_entry(
                &mut self.exec_approval_call_ids_by_turn_id,
                &turn_id,
                &approval_id,
            );
        }
        self.pending_requests_by_request_id.retain(|_, pending| {
            !matches!(
                pending,
                PendingInteractiveRequest::ExecApproval { item_id: pending_item_id, .. }
                    if pending_item_id == item_id
            )
        });
    }

    fn clear_patch_approval_turn(&mut self, turn_id: &str) {
        if let Some(call_ids) = self.patch_approval_call_ids_by_turn_id.remove(turn_id) {
            for call_id in call_ids {
                self.patch_approval_call_ids.remove(&call_id);
            }
        }
        self.pending_requests_by_request_id.retain(|_, pending| {
            !matches!(
                pending,
                PendingInteractiveRequest::PatchApproval {
                    turn_id: pending_turn_id,
                    ..
                } if pending_turn_id == turn_id
            )
        });
    }

    fn clear_patch_approval_item(&mut self, item_id: &str) {
        self.patch_approval_call_ids.remove(item_id);
        Self::remove_call_id_from_turn_map(&mut self.patch_approval_call_ids_by_turn_id, item_id);
        self.pending_requests_by_request_id.retain(|_, pending| {
            !matches!(
                pending,
                PendingInteractiveRequest::PatchApproval { item_id: pending_item_id, .. }
                    if pending_item_id == item_id
            )
        });
    }

    fn clear_request_permissions_turn(&mut self, turn_id: &str) {
        if let Some(call_ids) = self.request_permissions_call_ids_by_turn_id.remove(turn_id) {
            for call_id in call_ids {
                self.request_permissions_call_ids.remove(&call_id);
            }
        }
        self.pending_requests_by_request_id.retain(|_, pending| {
            !matches!(
                pending,
                PendingInteractiveRequest::RequestPermissions {
                    turn_id: pending_turn_id,
                    ..
                } if pending_turn_id == turn_id
            )
        });
    }

    fn clear_request_user_input_turn(&mut self, turn_id: &str) {
        if let Some(call_ids) = self.request_user_input_call_ids_by_turn_id.remove(turn_id) {
            for call_id in call_ids {
                self.request_user_input_call_ids.remove(&call_id);
            }
        }
        self.pending_requests_by_request_id.retain(|_, pending| {
            !matches!(
                pending,
                PendingInteractiveRequest::RequestUserInput {
                    turn_id: pending_turn_id,
                    ..
                } if pending_turn_id == turn_id
            )
        });
    }

    fn remove_call_id_from_turn_map(
        call_ids_by_turn_id: &mut HashMap<String, Vec<String>>,
        call_id: &str,
    ) {
        call_ids_by_turn_id.retain(|_, call_ids| {
            call_ids.retain(|queued_call_id| queued_call_id != call_id);
            !call_ids.is_empty()
        });
    }

    fn remove_call_id_from_turn_map_entry(
        call_ids_by_turn_id: &mut HashMap<String, Vec<String>>,
        turn_id: &str,
        call_id: &str,
    ) {
        let mut remove_turn_entry = false;
        if let Some(call_ids) = call_ids_by_turn_id.get_mut(turn_id) {
            call_ids.retain(|queued_call_id| queued_call_id != call_id);
            remove_turn_entry = call_ids.is_empty();
        }
        if remove_turn_entry {
            call_ids_by_turn_id.remove(turn_id);
        }
    }

    fn clear(&mut self) {
        self.exec_approval_call_ids.clear();
        self.exec_approval_call_ids_by_turn_id.clear();
        self.patch_approval_call_ids.clear();
        self.patch_approval_call_ids_by_turn_id.clear();
        self.elicitation_requests.clear();
        self.request_permissions_call_ids.clear();
        self.request_permissions_call_ids_by_turn_id.clear();
        self.request_user_input_call_ids.clear();
        self.request_user_input_call_ids_by_turn_id.clear();
        self.pending_requests_by_request_id.clear();
    }

    fn remove_request(&mut self, request_id: &RequestId) {
        let Some(pending) = self.pending_requests_by_request_id.remove(request_id) else {
            return;
        };
        match pending {
            PendingInteractiveRequest::ExecApproval {
                turn_id,
                approval_id,
                ..
            } => {
                self.exec_approval_call_ids.remove(&approval_id);
                Self::remove_call_id_from_turn_map_entry(
                    &mut self.exec_approval_call_ids_by_turn_id,
                    &turn_id,
                    &approval_id,
                );
            }
            PendingInteractiveRequest::PatchApproval { turn_id, item_id } => {
                self.patch_approval_call_ids.remove(&item_id);
                Self::remove_call_id_from_turn_map_entry(
                    &mut self.patch_approval_call_ids_by_turn_id,
                    &turn_id,
                    &item_id,
                );
            }
            PendingInteractiveRequest::Elicitation(key) => {
                self.elicitation_requests.remove(&key);
            }
            PendingInteractiveRequest::RequestPermissions { turn_id, item_id } => {
                self.request_permissions_call_ids.remove(&item_id);
                Self::remove_call_id_from_turn_map_entry(
                    &mut self.request_permissions_call_ids_by_turn_id,
                    &turn_id,
                    &item_id,
                );
            }
            PendingInteractiveRequest::RequestUserInput { turn_id, item_id } => {
                self.request_user_input_call_ids.remove(&item_id);
                Self::remove_call_id_from_turn_map_entry(
                    &mut self.request_user_input_call_ids_by_turn_id,
                    &turn_id,
                    &item_id,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_server_protocol::protocol::v2::{
        CommandExecutionRequestApprovalParams, FileChangeRequestApprovalParams,
        ItemStartedNotification, McpServerElicitationRequest, McpServerElicitationRequestParams,
        PermissionsRequestApprovalParams, RequestPermissionProfile,
        ServerRequestResolvedNotification, ThreadClosedNotification, ToolRequestUserInputParams,
        Turn, TurnCompletedNotification, TurnItemsView, TurnStatus,
    };

    fn request_user_input(id: i64, item_id: &str, turn_id: &str) -> ServerRequest {
        ServerRequest::ItemToolRequestUserInput {
            id: RequestId::Integer(id),
            params: ToolRequestUserInputParams {
                thread_id: "thread".to_string(),
                turn_id: turn_id.to_string(),
                item_id: item_id.to_string(),
                questions: Vec::new(),
                is_blocking: true,
                auto_resolution_ms: None,
            },
        }
    }

    fn exec_approval(
        id: i64,
        item_id: &str,
        approval_id: Option<&str>,
        turn_id: &str,
    ) -> ServerRequest {
        ServerRequest::ItemCommandExecutionRequestApproval {
            id: RequestId::Integer(id),
            params: CommandExecutionRequestApprovalParams {
                thread_id: "thread".to_string(),
                turn_id: turn_id.to_string(),
                item_id: item_id.to_string(),
                started_at_ms: 0,
                approval_id: approval_id.map(str::to_string),
                reason: None,
                network_approval_context: None,
                command: Some("echo ok".to_string()),
                cwd: None,
                available_decisions: None,
            },
        }
    }

    fn patch_approval(id: i64, item_id: &str, turn_id: &str) -> ServerRequest {
        ServerRequest::ItemFileChangeRequestApproval {
            id: RequestId::Integer(id),
            params: FileChangeRequestApprovalParams {
                thread_id: "thread".to_string(),
                turn_id: turn_id.to_string(),
                item_id: item_id.to_string(),
                started_at_ms: 0,
                reason: None,
                grant_root: None,
            },
        }
    }

    fn permissions_approval(id: i64, item_id: &str, turn_id: &str) -> ServerRequest {
        ServerRequest::ItemPermissionsRequestApproval {
            id: RequestId::Integer(id),
            params: PermissionsRequestApprovalParams {
                thread_id: "thread".to_string(),
                turn_id: turn_id.to_string(),
                item_id: item_id.to_string(),
                environment_id: None,
                started_at_ms: 0,
                cwd: "/tmp".to_string(),
                reason: None,
                permissions: RequestPermissionProfile {
                    network: None,
                    file_system: None,
                },
            },
        }
    }

    fn elicitation(id: &str) -> ServerRequest {
        ServerRequest::McpServerElicitationRequest {
            id: RequestId::String(id.to_string()),
            params: McpServerElicitationRequestParams {
                thread_id: "thread".to_string(),
                turn_id: Some("turn".to_string()),
                server_name: "server".to_string(),
                request: McpServerElicitationRequest::Form {
                    meta: None,
                    message: "Confirm".to_string(),
                    requested_schema: serde_json::Map::new(),
                },
            },
        }
    }

    fn turn_completed(turn_id: &str) -> ServerNotification {
        ServerNotification::TurnCompleted(TurnCompletedNotification {
            thread_id: "thread".to_string(),
            turn: Turn {
                id: turn_id.to_string(),
                items: Vec::new(),
                items_view: TurnItemsView::Full,
                status: TurnStatus::Completed,
                error: None,
                started_at: None,
                completed_at: None,
                duration_ms: None,
            },
        })
    }

    fn resolved(id: RequestId) -> ServerNotification {
        ServerNotification::ServerRequestResolved(ServerRequestResolvedNotification {
            thread_id: "thread".to_string(),
            request_id: id,
        })
    }

    #[test]
    fn each_interactive_request_is_pending_until_resolved() {
        let mut state = PendingInteractiveReplayState::default();
        let requests = [
            request_user_input(1, "input", "turn"),
            exec_approval(2, "exec", Some("approval"), "turn"),
            patch_approval(3, "patch", "turn"),
            permissions_approval(4, "permissions", "turn"),
            elicitation("elicitation"),
        ];

        for request in &requests {
            state.note_server_request(request);
            assert!(state.should_replay_snapshot_request(request));
        }
        assert!(state.has_pending_thread_approvals());
        assert!(state.has_pending_thread_user_input());
    }

    #[test]
    fn outbound_response_clears_only_the_matching_request() {
        let mut state = PendingInteractiveReplayState::default();
        let first = request_user_input(1, "input-1", "turn");
        let second = request_user_input(2, "input-2", "turn");
        state.note_server_request(&first);
        state.note_server_request(&second);

        state.note_outbound_response(&RequestId::Integer(1));

        assert!(!state.should_replay_snapshot_request(&first));
        assert!(state.should_replay_snapshot_request(&second));
        assert!(state.has_pending_thread_user_input());
    }

    #[test]
    fn server_resolution_clears_request_identity_and_category() {
        let mut state = PendingInteractiveReplayState::default();
        let request = exec_approval(1, "item", Some("approval"), "turn");
        state.note_server_request(&request);
        state.note_server_notification(&resolved(RequestId::Integer(1)));

        assert!(!state.should_replay_snapshot_request(&request));
        assert!(!state.has_pending_thread_approvals());
    }

    #[test]
    fn item_started_clears_command_and_file_approval_by_item_id() {
        let mut state = PendingInteractiveReplayState::default();
        let exec = exec_approval(1, "exec-item", Some("approval-id"), "turn");
        let patch = patch_approval(2, "patch-item", "turn");
        state.note_server_request(&exec);
        state.note_server_request(&patch);

        state.note_server_notification(&ServerNotification::ItemStarted(ItemStartedNotification {
            thread_id: "thread".to_string(),
            turn_id: "turn".to_string(),
            started_at_ms: 0,
            item: ThreadItem::CommandExecution {
                id: "exec-item".to_string(),
                metadata: None,
                plugin_id: None,
                script_path: None,
                command: "echo ok".to_string(),
                cwd: "/tmp".to_string(),
                process_id: None,
                source: Default::default(),
                status: app_server_protocol::protocol::v2::CommandExecutionStatus::InProgress,
                command_actions: Vec::new(),
                aggregated_output: None,
                exit_code: None,
                duration_ms: None,
                terminal_interactions: Vec::new(),
            },
        }));
        assert!(!state.should_replay_snapshot_request(&exec));
        assert!(state.should_replay_snapshot_request(&patch));

        state.note_server_notification(&ServerNotification::ItemStarted(ItemStartedNotification {
            thread_id: "thread".to_string(),
            turn_id: "turn".to_string(),
            started_at_ms: 0,
            item: ThreadItem::FileChange {
                id: "patch-item".to_string(),
                metadata: None,
                changes: Vec::new(),
                status: app_server_protocol::protocol::v2::PatchApplyStatus::InProgress,
            },
        }));
        assert!(!state.should_replay_snapshot_request(&patch));
    }

    #[test]
    fn turn_completion_clears_all_turn_indexed_prompts() {
        let mut state = PendingInteractiveReplayState::default();
        let requests = [
            request_user_input(1, "input", "turn"),
            exec_approval(2, "exec", None, "turn"),
            patch_approval(3, "patch", "turn"),
            permissions_approval(4, "permissions", "turn"),
        ];
        for request in &requests {
            state.note_server_request(request);
        }

        state.note_server_notification(&turn_completed("turn"));

        assert!(requests
            .iter()
            .all(|request| !state.should_replay_snapshot_request(request)));
        assert!(!state.has_pending_thread_approvals());
        assert!(!state.has_pending_thread_user_input());
    }

    #[test]
    fn user_input_requests_are_removed_in_fifo_order_per_turn() {
        let mut state = PendingInteractiveReplayState::default();
        let first = request_user_input(1, "first", "turn");
        let second = request_user_input(2, "second", "turn");
        state.note_server_request(&first);
        state.note_server_request(&second);

        state.note_outbound_response(&RequestId::Integer(1));

        assert!(!state.should_replay_snapshot_request(&first));
        assert!(state.should_replay_snapshot_request(&second));
    }

    #[test]
    fn evicted_request_does_not_replay() {
        let mut state = PendingInteractiveReplayState::default();
        let request = permissions_approval(1, "permissions", "turn");
        state.note_server_request(&request);
        state.note_evicted_server_request(&request);

        assert!(!state.should_replay_snapshot_request(&request));
        assert!(!state.has_pending_thread_approvals());
    }

    #[test]
    fn closing_thread_clears_all_prompt_categories() {
        let mut state = PendingInteractiveReplayState::default();
        let requests = [
            request_user_input(1, "input", "turn"),
            exec_approval(2, "exec", None, "turn"),
            patch_approval(3, "patch", "turn"),
            permissions_approval(4, "permissions", "turn"),
            elicitation("elicitation"),
        ];
        for request in &requests {
            state.note_server_request(request);
        }

        state.note_server_notification(&ServerNotification::ThreadClosed(
            ThreadClosedNotification {
                thread_id: "thread".to_string(),
            },
        ));

        assert!(requests
            .iter()
            .all(|request| !state.should_replay_snapshot_request(request)));
        assert!(!state.has_pending_thread_approvals());
        assert!(!state.has_pending_thread_user_input());
    }
}
