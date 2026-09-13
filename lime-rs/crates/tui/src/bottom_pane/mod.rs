mod approval_overlay;
mod chat_composer;
mod footer;
mod mcp_server_elicitation;
mod render;
mod request_user_input;
mod textarea;

use std::collections::VecDeque;

use app_server_protocol::protocol::v2::{
    CommandExecutionApprovalDecision, CommandExecutionRequestApprovalResponse,
    FileChangeApprovalDecision, FileChangeRequestApprovalResponse, GrantedPermissionProfile,
    McpServerElicitationRequestResponse, PermissionGrantScope, PermissionsRequestApprovalResponse,
    ServerRequest, ToolRequestUserInputResponse,
};
use app_server_protocol::RequestId;
use crossterm::event::{Event, KeyEvent};

use approval_overlay::ApprovalOverlay;
pub(crate) use chat_composer::{
    ChatComposer, FileSearchPopupAction, FileSearchRequest, InputResult, SkillPopupAction,
};
use mcp_server_elicitation::McpServerElicitationOverlay;
use request_user_input::RequestUserInputOverlay;
pub(crate) use textarea::{TextArea, TextAreaState};

pub(crate) use footer::render_footer;
pub(crate) use render::{desired_height_with_locale, render_with_locale};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AppServerResponse {
    Command {
        id: RequestId,
        response: CommandExecutionRequestApprovalResponse,
    },
    FileChange {
        id: RequestId,
        response: FileChangeRequestApprovalResponse,
    },
    Permissions {
        id: RequestId,
        response: PermissionsRequestApprovalResponse,
    },
    UserInput {
        id: RequestId,
        response: ToolRequestUserInputResponse,
    },
    McpElicitation {
        id: RequestId,
        response: McpServerElicitationRequestResponse,
    },
}

impl AppServerResponse {
    pub(crate) fn fail_closed(request: ServerRequest) -> Result<Self, Box<ServerRequest>> {
        match request {
            ServerRequest::ItemCommandExecutionRequestApproval { id, .. } => Ok(Self::Command {
                id,
                response: CommandExecutionRequestApprovalResponse {
                    decision: CommandExecutionApprovalDecision::Cancel,
                },
            }),
            ServerRequest::ItemFileChangeRequestApproval { id, .. } => Ok(Self::FileChange {
                id,
                response: FileChangeRequestApprovalResponse {
                    decision: FileChangeApprovalDecision::Cancel,
                },
            }),
            ServerRequest::ItemPermissionsRequestApproval { id, .. } => Ok(Self::Permissions {
                id,
                response: PermissionsRequestApprovalResponse {
                    permissions: GrantedPermissionProfile::default(),
                    scope: PermissionGrantScope::Turn,
                    strict_auto_review: None,
                },
            }),
            ServerRequest::ItemToolRequestUserInput { id, .. } => Ok(Self::UserInput {
                id,
                response: ToolRequestUserInputResponse {
                    answers: Default::default(),
                },
            }),
            request => Err(Box::new(request)),
        }
    }
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
enum PendingInteraction {
    Approval(ApprovalOverlay),
    UserInput(RequestUserInputOverlay),
    McpElicitation(McpServerElicitationOverlay),
}

impl PendingInteraction {
    fn from_server_request(request: ServerRequest) -> Result<Self, Box<ServerRequest>> {
        match request {
            request @ (ServerRequest::ItemCommandExecutionRequestApproval { .. }
            | ServerRequest::ItemFileChangeRequestApproval { .. }
            | ServerRequest::ItemPermissionsRequestApproval { .. }) => Ok(Self::Approval(
                ApprovalOverlay::from_server_request(request),
            )),
            ServerRequest::ItemToolRequestUserInput { id, params } => {
                Ok(Self::UserInput(RequestUserInputOverlay::new(id, params)))
            }
            ServerRequest::McpServerElicitationRequest { id, params } => {
                McpServerElicitationOverlay::from_server_request(id.clone(), &params)
                    .map(Self::McpElicitation)
                    .ok_or_else(|| {
                        Box::new(ServerRequest::McpServerElicitationRequest { id, params })
                    })
            }
            request => Err(Box::new(request)),
        }
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Option<AppServerResponse> {
        match self {
            Self::Approval(approval) => approval.handle_key_event(key),
            Self::UserInput(request) => request.handle_key_event(key),
            Self::McpElicitation(request) => request.handle_key_event(key),
        }
    }
}

#[derive(Debug, Default)]
pub(crate) struct BottomPane {
    queue: VecDeque<PendingInteraction>,
}

impl BottomPane {
    pub(crate) fn enqueue(&mut self, request: ServerRequest) -> Result<(), Box<ServerRequest>> {
        self.queue
            .push_back(PendingInteraction::from_server_request(request)?);
        Ok(())
    }

    pub(crate) fn supports_request(request: &ServerRequest) -> bool {
        match request {
            ServerRequest::McpServerElicitationRequest { id, params } => {
                McpServerElicitationOverlay::from_server_request(id.clone(), params).is_some()
            }
            ServerRequest::DynamicToolCall { .. } => false,
            ServerRequest::CurrentTimeRead { .. }
            | ServerRequest::ItemCommandExecutionRequestApproval { .. }
            | ServerRequest::ItemFileChangeRequestApproval { .. }
            | ServerRequest::ItemPermissionsRequestApproval { .. }
            | ServerRequest::ItemToolRequestUserInput { .. } => true,
        }
    }

    pub(crate) fn is_active(&self) -> bool {
        !self.queue.is_empty()
    }

    pub(crate) fn clear(&mut self) {
        self.queue.clear();
    }

    fn current(&self) -> Option<&PendingInteraction> {
        self.queue.front()
    }

    pub(crate) fn handle_event(&mut self, event: Event) -> Option<AppServerResponse> {
        match event {
            Event::Key(key) => self.handle_key_event(key),
            Event::Paste(text) => {
                match self.queue.front_mut() {
                    Some(PendingInteraction::UserInput(request)) => {
                        request.editing = true;
                        request.composer.insert(&text);
                    }
                    Some(PendingInteraction::McpElicitation(request)) => {
                        request.handle_paste(&text);
                    }
                    _ => return None,
                }
                None
            }
            _ => None,
        }
    }

    pub(crate) fn handle_key_event(&mut self, key: KeyEvent) -> Option<AppServerResponse> {
        let response = self.queue.front_mut()?.handle_key_event(key)?;
        self.queue.pop_front();
        Some(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_server_protocol::protocol::v2::{
        CommandExecutionApprovalDecision, CommandExecutionRequestApprovalParams,
        DynamicToolCallParams, DynamicToolCallPhase, FileChangeApprovalDecision,
        FileChangeRequestApprovalParams, PermissionsRequestApprovalParams,
        RequestPermissionProfile, ToolRequestUserInputOption, ToolRequestUserInputParams,
        ToolRequestUserInputQuestion,
    };
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use serde_json::json;

    fn key(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    #[test]
    fn queues_requests_and_resolves_them_in_arrival_order() {
        let mut pane = BottomPane::default();
        pane.enqueue(ServerRequest::ItemCommandExecutionRequestApproval {
            id: RequestId::Integer(1),
            params: CommandExecutionRequestApprovalParams {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                item_id: "command-1".to_string(),
                started_at_ms: 1,
                approval_id: None,
                reason: None,
                network_approval_context: None,
                command: Some("cargo test".to_string()),
                cwd: Some("/workspace".to_string()),
                available_decisions: None,
            },
        })
        .expect("queue approval");
        pane.enqueue(ServerRequest::ItemToolRequestUserInput {
            id: RequestId::Integer(2),
            params: ToolRequestUserInputParams {
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
                auto_resolution_ms: None,
            },
        })
        .expect("queue user input");

        let first = pane.handle_event(key(KeyCode::Enter));
        assert!(matches!(
            first,
            Some(AppServerResponse::Command {
                id: RequestId::Integer(1),
                ..
            })
        ));
        assert!(pane.is_active());

        let second = pane.handle_event(key(KeyCode::Enter));
        assert!(matches!(
            second,
            Some(AppServerResponse::UserInput {
                id: RequestId::Integer(2),
                ..
            })
        ));
        assert!(!pane.is_active());
    }

    #[test]
    fn unsupported_requests_return_for_rejection() {
        let mut pane = BottomPane::default();
        let dynamic_tool = ServerRequest::DynamicToolCall {
            id: RequestId::Integer(3),
            params: DynamicToolCallParams {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                call_id: "call-1".to_string(),
                namespace: None,
                tool: "browser.open".to_string(),
                arguments: json!({}),
                phase: DynamicToolCallPhase::Preflight,
                approval_token: None,
            },
        };
        assert!(matches!(
            pane.enqueue(dynamic_tool),
            Err(request) if matches!(*request, ServerRequest::DynamicToolCall { .. })
        ));

        let mcp_elicitation = serde_json::from_value::<ServerRequest>(json!({
            "method": "mcpServer/elicitation/request",
            "id": 4,
            "params": {
                "threadId": "thread-1",
                "turnId": "turn-1",
                "serverName": "form-server",
                "mode": "form",
                "message": "Choose a value",
                "requestedSchema": { "type": "object", "properties": {} }
            }
        }))
        .expect("MCP elicitation request");
        assert!(pane.enqueue(mcp_elicitation).is_ok());
        assert!(pane.is_active());
        assert!(matches!(
            pane.handle_event(key(KeyCode::Enter)),
            Some(AppServerResponse::McpElicitation {
                response: McpServerElicitationRequestResponse {
                    action: app_server_protocol::protocol::v2::McpServerElicitationAction::Accept,
                    content: Some(_),
                    ..
                },
                ..
            })
        ));
        assert!(!pane.is_active());

        let unsupported_tool_suggestion = serde_json::from_value::<ServerRequest>(json!({
            "method": "mcpServer/elicitation/request",
            "id": 6,
            "params": {
                "threadId": "thread-1",
                "turnId": "turn-1",
                "serverName": "form-server",
                "mode": "form",
                "_meta": { "codex_approval_kind": "tool_suggestion" },
                "message": "Install a tool",
                "requestedSchema": { "type": "object", "properties": {} }
            }
        }))
        .expect("tool suggestion MCP elicitation request");
        assert!(matches!(
            pane.enqueue(unsupported_tool_suggestion),
            Err(request) if matches!(*request, ServerRequest::McpServerElicitationRequest { .. })
        ));
        assert!(!pane.is_active());

        let supported_mcp_elicitation = serde_json::from_value::<ServerRequest>(json!({
            "method": "mcpServer/elicitation/request",
            "id": 5,
            "params": {
                "threadId": "thread-1",
                "turnId": "turn-1",
                "serverName": "form-server",
                "mode": "form",
                "message": "Choose a value",
                "requestedSchema": {
                    "type": "object",
                    "properties": {
                        "confirmed": { "type": "boolean" }
                    },
                    "required": ["confirmed"]
                }
            }
        }))
        .expect("supported MCP elicitation request");
        assert!(pane.enqueue(supported_mcp_elicitation).is_ok());
        assert!(pane.is_active());
    }

    #[test]
    fn non_interactive_responses_fail_closed_for_every_supported_interaction() {
        let command =
            AppServerResponse::fail_closed(ServerRequest::ItemCommandExecutionRequestApproval {
                id: RequestId::Integer(1),
                params: CommandExecutionRequestApprovalParams {
                    thread_id: "thread-1".to_string(),
                    turn_id: "turn-1".to_string(),
                    item_id: "command-1".to_string(),
                    started_at_ms: 1,
                    approval_id: None,
                    reason: None,
                    network_approval_context: None,
                    command: None,
                    cwd: None,
                    available_decisions: None,
                },
            })
            .expect("command response");
        assert!(matches!(
            command,
            AppServerResponse::Command {
                response: CommandExecutionRequestApprovalResponse {
                    decision: CommandExecutionApprovalDecision::Cancel,
                },
                ..
            }
        ));

        let file_change =
            AppServerResponse::fail_closed(ServerRequest::ItemFileChangeRequestApproval {
                id: RequestId::Integer(2),
                params: FileChangeRequestApprovalParams {
                    thread_id: "thread-1".to_string(),
                    turn_id: "turn-1".to_string(),
                    item_id: "patch-1".to_string(),
                    started_at_ms: 1,
                    reason: None,
                    grant_root: None,
                },
            })
            .expect("file change response");
        assert!(matches!(
            file_change,
            AppServerResponse::FileChange {
                response: FileChangeRequestApprovalResponse {
                    decision: FileChangeApprovalDecision::Cancel,
                },
                ..
            }
        ));

        let permissions =
            AppServerResponse::fail_closed(ServerRequest::ItemPermissionsRequestApproval {
                id: RequestId::Integer(3),
                params: PermissionsRequestApprovalParams {
                    thread_id: "thread-1".to_string(),
                    turn_id: "turn-1".to_string(),
                    item_id: "permissions-1".to_string(),
                    environment_id: None,
                    started_at_ms: 1,
                    cwd: "/workspace".to_string(),
                    reason: None,
                    permissions: RequestPermissionProfile {
                        network: None,
                        file_system: None,
                    },
                },
            })
            .expect("permissions response");
        assert!(matches!(
            permissions,
            AppServerResponse::Permissions {
                response: PermissionsRequestApprovalResponse {
                    permissions: GrantedPermissionProfile {
                        network: None,
                        file_system: None,
                    },
                    scope: PermissionGrantScope::Turn,
                    strict_auto_review: None,
                },
                ..
            }
        ));

        let user_input = AppServerResponse::fail_closed(ServerRequest::ItemToolRequestUserInput {
            id: RequestId::Integer(4),
            params: ToolRequestUserInputParams {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                item_id: "question-1".to_string(),
                questions: Vec::new(),
                auto_resolution_ms: None,
            },
        })
        .expect("user input response");
        assert!(matches!(
            user_input,
            AppServerResponse::UserInput {
                response: ToolRequestUserInputResponse { answers },
                ..
            } if answers.is_empty()
        ));
    }
}
