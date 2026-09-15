use super::*;
use crate::bottom_pane::command_popup::CommandPopup;
use app_server_protocol::protocol::v2::{
    CommandExecutionApprovalDecision, CommandExecutionRequestApprovalParams, McpServerStartupState,
    McpServerStatusDetail, McpServerStatusUpdatedNotification, ServerNotification, ServerRequest,
    UserInput,
};
use app_server_protocol::RequestId;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn dispatch_connected_input(app: &mut App, event: Event) -> AppAction {
    app.handle_tui_event(to_tui_event(event), true)
}

fn dispatch_disconnected_input(app: &mut App, event: Event) -> AppAction {
    app.handle_tui_event(to_tui_event(event), false)
}

fn to_tui_event(event: Event) -> TuiEvent {
    match event {
        Event::Key(key) => TuiEvent::Key(key),
        Event::Paste(text) => TuiEvent::Paste(text),
        Event::Resize(width, height) => TuiEvent::Resize(ratatui::layout::Size { width, height }),
        Event::FocusGained => TuiEvent::FocusGained,
        Event::FocusLost => TuiEvent::FocusLost,
        _ => TuiEvent::Draw,
    }
}

#[test]
fn active_bottom_pane_receives_input_before_the_chat_composer() {
    let mut app = App::default();
    app.composer.insert("draft");
    app.bottom_pane
        .enqueue(ServerRequest::ItemCommandExecutionRequestApproval {
            id: RequestId::Integer(7),
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

    let ignored = dispatch_connected_input(
        &mut app,
        Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)),
    );
    assert_eq!(ignored, AppAction::None);
    assert_eq!(app.composer.text(), "draft");

    let response = dispatch_connected_input(
        &mut app,
        Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
    );
    assert!(matches!(
        response,
        AppAction::Respond(AppServerResponse::Command {
            response: app_server_protocol::protocol::v2::CommandExecutionRequestApprovalResponse {
                decision: CommandExecutionApprovalDecision::Accept,
            },
            ..
        })
    ));
    assert!(!app.bottom_pane.is_active());
    assert_eq!(app.composer.text(), "draft");
}

#[test]
fn mcp_startup_status_is_app_scoped_and_clears_when_ready() {
    let mut app = App::default();
    app.apply_notification(ServerNotification::McpServerStatusUpdated(
        McpServerStatusUpdatedNotification {
            thread_id: None,
            name: "docs".to_string(),
            status: McpServerStartupState::Failed,
            error: Some("offline".to_string()),
            failure_reason: None,
        },
    ));
    assert_eq!(app.projection.status(), "");
    assert_eq!(app.status_value(), "MCP startup issue: docs: offline");

    app.apply_notification(ServerNotification::McpServerStatusUpdated(
        McpServerStatusUpdatedNotification {
            thread_id: None,
            name: "docs".to_string(),
            status: McpServerStartupState::Ready,
            error: None,
            failure_reason: None,
        },
    ));
    assert_eq!(app.status_value(), "");
}

#[test]
fn startup_protected_request_keeps_draft_until_the_request_is_resolved() {
    let mut app = App::default();
    app.begin_startup_input_boundary();
    app.composer.insert("startup draft");
    app.bottom_pane
        .enqueue(ServerRequest::ItemCommandExecutionRequestApproval {
            id: RequestId::Integer(8),
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
        .expect("queue startup approval");
    app.note_startup_protected_request();

    let ignored = dispatch_connected_input(
        &mut app,
        Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)),
    );
    assert_eq!(ignored, AppAction::None);
    assert_eq!(app.composer.text(), "startup draft");
    assert!(app.has_queued_startup_protected_request());

    let response = dispatch_connected_input(
        &mut app,
        Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
    );
    assert!(matches!(
        response,
        AppAction::Respond(AppServerResponse::Command { .. })
    ));
    assert!(!app.has_queued_startup_protected_request());
    assert_eq!(app.composer.text(), "startup draft");
}

#[test]
fn startup_boundary_ignores_protected_requests_for_background_threads() {
    let mut app = App::default();
    app.begin_startup_input_boundary();
    app.set_thread_id("main".to_string());
    app.ensure_thread_channel("background").store.push_request(
        ServerRequest::ItemCommandExecutionRequestApproval {
            id: RequestId::Integer(9),
            params: CommandExecutionRequestApprovalParams {
                thread_id: "background".to_string(),
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
        },
    );

    assert!(!app.has_queued_startup_protected_request());
}

#[test]
fn startup_boundary_ends_on_the_first_safe_user_input() {
    let mut app = App::default();
    app.begin_startup_input_boundary();

    assert!(!app.release_startup_input_boundary_if_ready(false));
    assert!(app.startup_protected_input_boundary);
    assert!(app.release_startup_input_boundary_if_ready(true));
    assert!(!app.startup_protected_input_boundary);
    assert!(!app.startup_pending_protected_request);
}

#[test]
fn first_safe_user_input_releases_boundary_before_reaching_composer() {
    let mut app = App::default();
    app.begin_startup_input_boundary();

    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)),
        ),
        AppAction::None
    );
    assert!(!app.startup_protected_input_boundary);
    assert_eq!(app.composer.text(), "x");
}

#[test]
fn startup_boundary_waits_for_visible_request_before_releasing() {
    let mut app = App::default();
    app.begin_startup_input_boundary();
    app.bottom_pane
        .enqueue(ServerRequest::ItemCommandExecutionRequestApproval {
            id: RequestId::Integer(10),
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
        .expect("queue startup approval");
    app.note_startup_protected_request();

    assert!(!app.release_startup_input_boundary_if_ready(true));
    assert!(app.startup_protected_input_boundary);
    assert!(app.startup_pending_protected_request);
}

#[test]
fn ctrl_g_requests_external_editor_after_the_current_draw() {
    let mut app = App::default();

    let action = dispatch_connected_input(
        &mut app,
        Event::Key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL)),
    );

    assert_eq!(action, AppAction::None);
    assert_eq!(app.external_editor_state(), ExternalEditorState::Requested);
}

#[test]
fn slash_pwd_and_cwd_alias_display_current_working_directory_from_composer() {
    let mut output = Vec::new();
    for command in ["/pwd", "/cwd", "/pwd x"] {
        let mut app = App::default();
        app.set_cwd(std::path::PathBuf::from("/tmp/project"));
        app.set_locale(Locale::EnUs);
        app.replace_composer(command.to_string());

        let action = dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        );

        assert_eq!(action, AppAction::None);
        output.push(app.projection.status().to_string());
    }

    assert_eq!(
        output,
        vec![
            "Current working directory: /tmp/project",
            "Current working directory: /tmp/project",
            "Usage: /pwd",
        ]
    );
}

#[test]
fn mcp_slash_commands_request_the_matching_inventory_detail() {
    for (command, detail) in [
        ("/mcp", McpServerStatusDetail::ToolsAndAuthOnly),
        ("/mcp verbose", McpServerStatusDetail::Full),
    ] {
        let mut app = App::default();
        app.replace_composer(command.to_string());
        assert_eq!(
            dispatch_connected_input(
                &mut app,
                Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
            ),
            AppAction::FetchMcpInventory { detail }
        );
        assert!(app.composer.is_empty());
        assert!(app.composer.command_popup().is_none());
    }
}

#[test]
fn mcp_slash_command_rejects_unknown_arguments_with_localized_usage() {
    let mut app = App::default();
    app.set_locale(Locale::ZhCn);
    app.replace_composer("/mcp compact".to_string());

    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        ),
        AppAction::None
    );
    assert_eq!(app.projection.status(), "用法：/mcp [verbose]");
    assert!(app.composer.is_empty());
}

#[test]
fn tab_queues_a_follow_up_without_submitting_the_active_turn() {
    let mut app = App::default();
    app.composer.insert("follow up");
    app.projection.apply(
        app_server_protocol::protocol::v2::ServerNotification::TurnStarted(
            app_server_protocol::protocol::v2::TurnStartedNotification {
                thread_id: "thread-1".to_string(),
                turn: app_server_protocol::protocol::v2::Turn {
                    id: "turn-1".to_string(),
                    items: Vec::new(),
                    items_view: Default::default(),
                    status: app_server_protocol::protocol::v2::TurnStatus::InProgress,
                    error: None,
                    started_at: None,
                    completed_at: None,
                    duration_ms: None,
                },
            },
        ),
    );

    let action = dispatch_connected_input(
        &mut app,
        Event::Key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)),
    );

    assert_eq!(action, AppAction::Queue("follow up".to_string()));
    assert!(app.composer.is_empty());
}

#[test]
fn escape_interrupts_only_an_active_turn_and_preserves_the_draft() {
    let mut app = App::default();
    app.composer.insert("keep this draft");
    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE,))
        ),
        AppAction::None
    );

    app.start_turn("turn-1".to_string());
    assert!(app.active_turn_elapsed(Instant::now()).is_some());
    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE,))
        ),
        AppAction::Interrupt
    );
    assert_eq!(app.composer.text(), "keep this draft");
}

#[test]
fn ctrl_c_clears_idle_plain_text_draft_and_keeps_it_recallable() {
    let mut app = App::default();
    app.composer.insert("draft");

    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL,)),
        ),
        AppAction::None
    );
    assert!(app.composer.is_empty());

    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE,)),
        ),
        AppAction::None
    );
    assert_eq!(app.composer.text(), "draft");
}

#[test]
fn ctrl_c_clears_active_turn_draft_without_interrupting() {
    let mut app = App::default();
    app.composer.insert("draft");
    app.start_turn("turn-1".to_string());

    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL,)),
        ),
        AppAction::None
    );
    assert!(app.composer.is_empty());
}

#[test]
fn ctrl_c_preserves_attachment_draft_until_image_history_is_supported() {
    let mut app = App::default();
    app.composer.insert("draft");
    app.attach_image(std::path::PathBuf::from("/tmp/draft.png"));

    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL,)),
        ),
        AppAction::Interrupt
    );
    assert_eq!(app.composer.text(), "draft");
    assert!(app.composer.has_pending_images());
}

#[test]
fn turn_completion_clears_the_active_status_timer() {
    use app_server_protocol::protocol::v2::{
        Turn, TurnCompletedNotification, TurnItemsView, TurnStatus,
    };

    let mut app = App::default();
    app.set_thread_id("thread-1".to_string());
    app.start_turn("turn-1".to_string());
    app.apply_notification(ServerNotification::TurnCompleted(
        TurnCompletedNotification {
            thread_id: "thread-1".to_string(),
            turn: Turn {
                id: "turn-1".to_string(),
                items: Vec::new(),
                items_view: TurnItemsView::Full,
                status: TurnStatus::Completed,
                error: None,
                started_at: Some(1),
                completed_at: Some(2),
                duration_ms: Some(1),
            },
        },
    ));

    assert!(app.projection.active_turn_id().is_none());
    assert!(app.active_turn_elapsed(Instant::now()).is_none());
}

#[test]
fn permission_profile_catalog_is_trimmed_deduplicated_and_used_for_cycles() {
    let mut app = App::default();
    app.set_permission_profiles([
        " custom-read ".to_string(),
        "custom-write".to_string(),
        "custom-read".to_string(),
        "".to_string(),
    ]);

    assert_eq!(
        app.permission_profiles,
        vec!["custom-read".to_string(), "custom-write".to_string()]
    );
    assert_eq!(
        app.cycle_permission_profile(Some("custom-read"), 1),
        "custom-write"
    );
    assert_eq!(
        app.cycle_permission_profile(Some("custom-write"), 1),
        "custom-read"
    );

    app.set_permission_profiles([" ".to_string(), "custom-read".to_string()]);
    assert_eq!(app.permission_profiles, vec!["custom-read".to_string()]);
    app.set_permission_profiles(std::iter::empty::<String>());
    assert!(app.permission_profiles.is_empty());
    assert_eq!(
        app.cycle_permission_profile(Some(":read-only"), 1),
        ":workspace"
    );
}

#[test]
fn backtab_cycles_server_collaboration_modes_only_when_idle() {
    let mut app = App::default();
    app.set_settings(
        Some("fixture-model".to_string()),
        Some("fixture-provider".to_string()),
        Some("medium".to_string()),
        None,
    );
    app.set_collaboration_modes(vec![
        app_server_protocol::protocol::v2::CollaborationModeMask {
            name: "Plan".to_string(),
            mode: Some(agent_protocol::ModeKind::Plan),
            model: None,
            reasoning_effort: Some(Some("high".to_string())),
        },
        app_server_protocol::protocol::v2::CollaborationModeMask {
            name: "Default".to_string(),
            mode: Some(agent_protocol::ModeKind::Default),
            model: None,
            reasoning_effort: Some(None),
        },
    ]);

    assert_eq!(
        app.collaboration_mode.as_ref().map(|mode| mode.mode),
        Some(agent_protocol::ModeKind::Default)
    );
    let plan_mode = agent_protocol::CollaborationMode {
        mode: agent_protocol::ModeKind::Plan,
        settings: agent_protocol::CollaborationModeSettings {
            model: "fixture-model".to_string(),
            reasoning_effort: Some("high".to_string()),
            developer_instructions: None,
        },
    };
    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE,))
        ),
        AppAction::ChangeCollaborationMode(plan_mode.clone())
    );
    app.collaboration_mode = Some(plan_mode);
    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE,))
        ),
        AppAction::ChangeCollaborationMode(agent_protocol::CollaborationMode {
            mode: agent_protocol::ModeKind::Default,
            settings: agent_protocol::CollaborationModeSettings {
                model: "fixture-model".to_string(),
                reasoning_effort: None,
                developer_instructions: None,
            },
        })
    );

    app.start_turn("turn-1".to_string());
    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE,))
        ),
        AppAction::None
    );
}

#[test]
fn settings_updates_keep_the_active_collaboration_mode_in_sync() {
    let mut app = App {
        collaboration_mode: Some(agent_protocol::CollaborationMode {
            mode: agent_protocol::ModeKind::Plan,
            settings: agent_protocol::CollaborationModeSettings {
                model: "old-model".to_string(),
                reasoning_effort: Some("high".to_string()),
                developer_instructions: None,
            },
        }),
        ..App::default()
    };

    app.set_settings(
        Some("new-model".to_string()),
        Some("fixture-provider".to_string()),
        Some("low".to_string()),
        None,
    );

    let mode = app.collaboration_mode.as_ref().expect("active mode");
    assert_eq!(mode.settings.model, "new-model");
    assert_eq!(mode.settings.reasoning_effort.as_deref(), Some("low"));

    app.set_settings(
        Some("newer-model".to_string()),
        Some("fixture-provider".to_string()),
        None,
        None,
    );
    let mode = app.collaboration_mode.as_ref().expect("active mode");
    assert_eq!(mode.settings.model, "newer-model");
    assert_eq!(mode.settings.reasoning_effort.as_deref(), Some("low"));
}

#[test]
fn an_open_popup_owns_escape_before_active_turn_interruption() {
    let mut app = App::default();
    app.start_turn("turn-1".to_string());
    dispatch_connected_input(
        &mut app,
        Event::Key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE)),
    );
    assert!(app.composer.command_popup().is_some());

    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE,))
        ),
        AppAction::None
    );
    assert!(app.composer.command_popup().is_none());
    assert!(app.projection.active_turn_id().is_some());
}

#[test]
fn history_search_owns_escape_before_active_turn_interruption() {
    let mut app = App::default();
    app.composer.load_history(["previous prompt".to_string()]);
    app.composer.insert("previous");
    app.start_turn("turn-1".to_string());

    dispatch_connected_input(
        &mut app,
        Event::Key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL)),
    );
    assert!(app.composer.history_search_active());
    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE,))
        ),
        AppAction::None
    );
    assert!(!app.composer.history_search_active());
    assert_eq!(app.composer.text(), "previous");
    assert!(app.projection.active_turn_id().is_some());
}

#[test]
fn vim_slash_command_toggles_composer_mode_and_projects_localized_status() {
    let mut app = App::default();
    app.set_locale(Locale::ZhCn);
    app.composer.insert("/vim");

    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        ),
        AppAction::None
    );
    assert!(app.composer.is_vim_normal_mode());
    assert!(app.composer.is_empty());
    assert_eq!(app.projection.status(), "已启用 Vim 编辑模式");

    app.composer.insert("/vim");
    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        ),
        AppAction::None
    );
    assert!(!app.composer.is_vim_normal_mode());
    assert!(app.composer.is_empty());
    assert_eq!(app.projection.status(), "已关闭 Vim 编辑模式");
}

#[test]
fn vim_insert_escape_returns_to_normal_before_interrupting_an_active_turn() {
    let mut app = App::default();
    app.composer.set_vim_enabled(true);
    app.start_turn("turn-1".to_string());

    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE)),
        ),
        AppAction::None
    );
    assert!(!app.composer.is_vim_normal_mode());
    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        ),
        AppAction::None
    );
    assert!(app.composer.is_vim_normal_mode());
    assert!(app.projection.active_turn_id().is_some());

    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        ),
        AppAction::Interrupt
    );
}

#[test]
fn vim_search_owns_escape_and_paste_before_popup_or_active_turn() {
    let mut app = App::default();
    app.composer.set_vim_enabled(true);
    app.composer.insert("alpha beta");
    app.start_turn("turn-1".to_string());

    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE)),
        ),
        AppAction::None
    );
    assert!(app.composer.vim_search_active());

    assert_eq!(
        dispatch_connected_input(&mut app, Event::Paste("beta".to_string())),
        AppAction::None
    );
    assert_eq!(app.composer.text(), "alpha beta");
    assert_eq!(
        app.composer.vim_search_query().map(|(query, _)| query),
        Some("beta")
    );

    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
        ),
        AppAction::None
    );
    assert!(!app.composer.vim_search_active());
    assert!(app.projection.active_turn_id().is_some());
}

#[test]
fn vim_normal_up_and_down_do_not_replace_the_draft_with_history() {
    let mut app = App::default();
    app.composer.load_history(["previous prompt".to_string()]);
    app.composer.insert("current draft");
    app.composer.set_vim_enabled(true);

    for code in [KeyCode::Up, KeyCode::Down] {
        assert_eq!(
            dispatch_connected_input(
                &mut app,
                Event::Key(KeyEvent::new(code, KeyModifiers::NONE)),
            ),
            AppAction::None
        );
        assert_eq!(app.composer.text(), "current draft");
        assert!(app.composer.is_vim_normal_mode());
    }
}

#[test]
fn codex_style_effort_and_permission_shortcuts_are_not_inserted_into_draft() {
    let mut app = App::default();
    app.composer.insert("draft");
    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Char('.'), KeyModifiers::ALT,))
        ),
        AppAction::IncreaseEffort
    );
    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::F(8), KeyModifiers::NONE,))
        ),
        AppAction::NextPermissions
    );
    assert_eq!(app.composer.text(), "draft");
}

#[test]
fn copy_shortcut_and_slash_command_do_not_become_turn_input() {
    let mut app = App::default();
    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL,))
        ),
        AppAction::CopyLastResponse
    );
    app.composer.insert("/copy");
    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE,))
        ),
        AppAction::CopyLastResponse
    );
    assert!(app.composer.is_empty());
}

#[test]
fn export_slash_command_targets_the_canonical_transcript() {
    let mut app = App::default();
    app.composer.insert("/export");
    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE,))
        ),
        AppAction::None
    );
    assert!(app.export_picker.is_some());
    assert!(app.composer.is_empty());

    let mut app = App::default();
    app.composer.insert("/export transcript.md");
    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE,))
        ),
        AppAction::ExportTranscript {
            path: Some(std::path::PathBuf::from("transcript.md")),
        }
    );
    assert!(app.composer.is_empty());
}

#[test]
fn slash_popup_filters_and_executes_immediate_commands() {
    let mut app = App::default();
    for character in ['/', 'm'] {
        assert_eq!(
            dispatch_connected_input(
                &mut app,
                Event::Key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE,))
            ),
            AppAction::None
        );
    }
    assert_eq!(
        app.composer
            .command_popup()
            .and_then(CommandPopup::selected),
        Some(SlashCommand::Model)
    );

    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE,))
        ),
        AppAction::Submit("/model".to_string())
    );
    assert!(app.composer.command_popup().is_none());
    assert!(app.composer.is_empty());
}

#[test]
fn slash_popup_completes_argument_commands_and_reopens_after_cancelled_input_changes() {
    let mut app = App::default();
    dispatch_connected_input(
        &mut app,
        Event::Key(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE)),
    );
    assert!(app.composer.command_popup().is_some());
    dispatch_connected_input(
        &mut app,
        Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
    );
    assert!(app.composer.command_popup().is_none());

    dispatch_connected_input(
        &mut app,
        Event::Key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE)),
    );
    assert_eq!(
        app.composer
            .command_popup()
            .and_then(CommandPopup::selected),
        Some(SlashCommand::Effort)
    );
    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE,))
        ),
        AppAction::None
    );
    assert_eq!(app.composer.text(), "/effort ");
    assert!(app.composer.command_popup().is_none());
}

#[test]
fn status_command_opens_an_ephemeral_pager_and_consumes_input_until_closed() {
    let mut app = App::default();
    app.composer.insert("real prompt");
    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE,))
        ),
        AppAction::Submit("real prompt".to_string())
    );
    app.set_thread_id("thread-1".to_string());
    app.set_settings(
        Some("gpt-5".to_string()),
        Some("openai".to_string()),
        Some("high".to_string()),
        Some(":workspace".to_string()),
    );
    app.composer.insert("/status");

    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE,))
        ),
        AppAction::None
    );
    assert!(app.pager_overlay.is_some());
    assert!(app.composer.is_empty());

    dispatch_connected_input(
        &mut app,
        Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)),
    );
    assert!(app.composer.is_empty());
    assert!(app.pager_overlay.is_some());
    dispatch_connected_input(
        &mut app,
        Event::Key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE)),
    );
    assert!(app.pager_overlay.is_none());

    dispatch_connected_input(
        &mut app,
        Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)),
    );
    assert_eq!(app.composer.text(), "real prompt");
    assert!(app.projection.entries().is_empty());
}

#[test]
fn ctrl_t_opens_transcript_without_copying_or_mutating_conversation_state() {
    let mut app = App::default();
    app.composer.insert("draft");

    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL,))
        ),
        AppAction::None
    );
    assert!(app
        .pager_overlay
        .as_ref()
        .is_some_and(PagerOverlay::is_transcript));
    assert_eq!(app.composer.text(), "draft");
    assert!(app.projection.entries().is_empty());

    dispatch_connected_input(
        &mut app,
        Event::Key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL)),
    );
    assert!(app.pager_overlay.is_none());
    assert_eq!(app.composer.text(), "draft");
}

#[test]
fn transcript_overlay_requests_older_history_only_when_session_has_more_pages() {
    let mut app = App {
        scrollback_has_older_history: true,
        ..App::default()
    };
    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL,))
        ),
        AppAction::None
    );

    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE,))
        ),
        AppAction::LoadOlderHistory
    );
}

#[test]
fn switching_threads_resets_the_main_transcript_to_the_tail() {
    let mut app = App::default();
    app.set_thread_id("thread-1".to_string());
    app.transcript_scroll = 12;

    app.set_thread_id("thread-2".to_string());

    assert_eq!(app.transcript_scroll, 0);
}

#[test]
fn image_shortcut_attaches_and_allows_image_only_submission() {
    let mut app = App::default();
    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL,))
        ),
        AppAction::PasteImage
    );
    app.attach_image(PathBuf::from("/tmp/input.png"));
    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE,))
        ),
        AppAction::Submit(String::new())
    );
    assert_eq!(
        app.take_pending_images(),
        vec![PathBuf::from("/tmp/input.png")]
    );
}

#[test]
fn failed_image_submission_can_restore_pending_attachments() {
    let mut app = App::default();
    app.attach_image(PathBuf::from("/tmp/one.png"));
    app.attach_image(PathBuf::from("/tmp/two.png"));

    let images = app.take_pending_images();
    assert!(!app.composer.has_pending_images());
    app.restore_pending_images(images);

    assert_eq!(
        app.composer.pending_images(),
        &[PathBuf::from("/tmp/one.png"), PathBuf::from("/tmp/two.png")]
    );
}

#[test]
fn queued_submission_projection_updates_by_id_and_clears_on_thread_change() {
    let queued = |id: &str, text: &str| QueuedSubmission {
        id: id.to_string(),
        input: vec![UserInput::Text {
            text: text.to_string(),
            text_elements: Vec::new(),
        }],
        client_user_message_id: format!("client-{id}"),
    };
    let mut app = App::default();
    app.set_thread_id("thread-1".to_string());
    app.upsert_queued_submission(queued("queue-1", "first"));
    app.upsert_queued_submission(queued("queue-1", "revised"));
    app.upsert_queued_submission(queued("queue-2", "second"));

    assert_eq!(app.queued_submissions.len(), 2);
    assert!(matches!(
        app.queued_submissions[0].input.as_slice(),
        [UserInput::Text { text, .. }] if text == "revised"
    ));

    app.set_thread_id("thread-2".to_string());
    assert!(app.queued_submissions.is_empty());
}

#[test]
fn alt_up_requests_server_delete_before_restoring_the_last_queued_input() {
    let submission = QueuedSubmission {
        id: "queue-1".to_string(),
        input: vec![
            UserInput::LocalImage {
                detail: None,
                path: "/tmp/queued.png".to_string(),
            },
            UserInput::Text {
                text: "revise this follow-up".to_string(),
                text_elements: Vec::new(),
            },
        ],
        client_user_message_id: "client-queue-1".to_string(),
    };
    let mut app = App::default();
    app.set_queued_submissions(vec![submission.clone()]);

    let action = dispatch_connected_input(
        &mut app,
        Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::ALT)),
    );

    assert_eq!(action, AppAction::EditQueuedSubmission(submission.clone()));
    assert_eq!(app.queued_submissions, vec![submission.clone()]);
    assert!(app.composer.is_empty());
    assert!(!app.composer.has_pending_images());

    assert!(app.restore_queued_submission_for_edit(submission));
    assert!(app.queued_submissions.is_empty());
    assert_eq!(app.composer.text(), "revise this follow-up");
    assert_eq!(
        app.composer.pending_images(),
        &[PathBuf::from("/tmp/queued.png")]
    );
}

#[test]
fn queued_skill_input_restores_as_an_editable_dollar_mention() {
    let submission = QueuedSubmission {
        id: "queue-skill".to_string(),
        input: vec![
            UserInput::Skill {
                name: "review".to_string(),
                path: "/skills/review/SKILL.md".to_string(),
            },
            UserInput::Text {
                text: "please check".to_string(),
                text_elements: Vec::new(),
            },
        ],
        client_user_message_id: "client-queue-skill".to_string(),
    };
    let mut app = App::default();
    app.set_queued_submissions(vec![submission.clone()]);

    assert!(app.restore_queued_submission_for_edit(submission));
    assert_eq!(app.composer.text(), "$review please check");
    assert!(app.queued_submissions.is_empty());
}

#[test]
fn alt_up_offers_lossless_remote_image_queue_edit() {
    let remote_image = QueuedSubmission {
        id: "queue-remote".to_string(),
        input: vec![UserInput::Image {
            detail: None,
            url: "https://example.test/input.png".to_string(),
        }],
        client_user_message_id: "client-remote".to_string(),
    };
    let mut app = App::default();
    app.set_queued_submissions(vec![remote_image]);
    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::ALT,))
        ),
        AppAction::EditQueuedSubmission(QueuedSubmission {
            id: "queue-remote".to_string(),
            input: vec![UserInput::Image {
                detail: None,
                url: "https://example.test/input.png".to_string(),
            }],
            client_user_message_id: "client-remote".to_string(),
        })
    );

    let remote_image = app.queued_submissions[0].clone();
    assert!(app.restore_queued_submission_for_edit(remote_image));
    assert_eq!(
        app.composer.remote_image_urls(),
        &["https://example.test/input.png"]
    );
    assert!(app.composer.is_empty());

    app.set_queued_submissions(vec![QueuedSubmission {
        id: "queue-text".to_string(),
        input: vec![UserInput::Text {
            text: "queued".to_string(),
            text_elements: Vec::new(),
        }],
        client_user_message_id: "client-text".to_string(),
    }]);
    app.composer.insert("unsent draft");
    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::ALT,))
        ),
        AppAction::None
    );
    assert_eq!(app.composer.text(), "unsent draft");
    assert_eq!(app.queued_submissions.len(), 1);
}

#[test]
fn remote_image_rows_are_selectable_and_deletable_from_the_composer() {
    let mut app = App::default();
    app.set_remote_image_urls(vec![
        "https://example.test/one.png".to_string(),
        "https://example.test/two.png".to_string(),
    ]);

    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE))
        ),
        AppAction::None
    );
    assert!(app.composer.has_selected_remote_image());

    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE))
        ),
        AppAction::None
    );
    assert_eq!(
        app.composer.remote_image_urls(),
        &["https://example.test/one.png"]
    );
}

#[test]
fn disconnected_input_edits_locally_without_submit_or_queue() {
    let mut app = App::default();
    app.composer.insert("draft");

    assert_eq!(
        dispatch_disconnected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Char('!'), KeyModifiers::NONE,))
        ),
        AppAction::None
    );
    assert_eq!(
        dispatch_disconnected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE,))
        ),
        AppAction::None
    );
    assert_eq!(app.composer.text(), "draft!");
    assert!(app.queued_submissions.is_empty());
}

#[test]
fn disconnected_paste_and_ctrl_c_are_handled_at_the_app_boundary() {
    let mut app = App::default();
    assert_eq!(
        dispatch_disconnected_input(&mut app, Event::Paste("离线草稿".to_string())),
        AppAction::None
    );
    assert_eq!(app.composer.text(), "离线草稿");
    assert_eq!(
        dispatch_disconnected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL,))
        ),
        AppAction::Quit
    );
}

#[test]
fn resume_picker_owns_input_until_cancelled() {
    let mut app = App {
        resume_picker: Some(PickerState::new(
            Vec::new(),
            crate::resume_picker::SessionPickerAction::Resume,
            crate::resume_picker::SessionStatus::Active,
            None,
            true,
        )),
        ..App::default()
    };

    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE,))
        ),
        AppAction::ResumePicker(PickerAction::Reload)
    );
    assert!(app.resume_picker.is_some());
    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE,))
        ),
        AppAction::ResumePicker(PickerAction::Reload)
    );
    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE,))
        ),
        AppAction::None
    );
    assert!(app.resume_picker.is_none());
}

#[test]
fn alt_right_switches_to_the_next_agent_in_spawn_order() {
    let mut app = App::default();
    app.set_thread_id("main".to_string());
    app.agent_navigation
        .upsert("agent-1", Some("Robie".to_string()), None, false);

    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Right, KeyModifiers::ALT,))
        ),
        AppAction::SwitchThread("agent-1".to_string())
    );
}

#[test]
fn subagents_slash_command_opens_the_codex_named_picker() {
    let mut app = App::default();
    app.set_thread_id("main".to_string());
    app.agent_navigation.upsert(
        "agent-1",
        Some("Robie".to_string()),
        Some("worker".to_string()),
        false,
    );
    app.composer.insert("/subagents");

    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE,)),
        ),
        AppAction::None
    );
    assert!(app.agents_overview.is_none());
    assert!(app.agent_picker.is_some());
    assert!(app.composer.is_empty());
}

#[test]
fn agents_slash_command_opens_the_agents_overview() {
    let mut app = App::default();
    app.composer.insert("/agents");

    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        ),
        AppAction::RefreshAgentsOverview
    );
    assert!(app.agents_overview.is_some());
    assert!(app.agent_picker.is_none());
    assert!(app.composer.is_empty());
}

#[test]
fn empty_composer_left_opens_agents_overview_when_local_navigation_is_enabled() {
    let mut app = App::default();
    app.composer.set_agents_navigation_enabled(true);

    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
        ),
        AppAction::RefreshAgentsOverview
    );
    assert!(app.agents_overview.is_some());
    assert!(app.composer.is_empty());
}

#[test]
fn empty_composer_left_stays_in_editor_when_navigation_is_disabled() {
    let mut app = App::default();
    app.composer.set_agents_navigation_enabled(false);

    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)),
        ),
        AppAction::None
    );
    assert!(app.agents_overview.is_none());
    assert!(app.composer.is_empty());
}

#[test]
fn resume_slash_command_opens_the_shared_picker_action() {
    let mut app = App::default();
    app.composer.insert("/resume");

    assert_eq!(
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE,))
        ),
        AppAction::OpenResumePicker
    );
    assert!(app.composer.is_empty());
}

#[test]
fn subagent_activity_marks_the_thread_parent_owned_before_resume() {
    let mut app = App::default();
    app.set_thread_id("agent-1".to_string());
    app.apply_notification(ServerNotification::ItemStarted(
        app_server_protocol::protocol::v2::ItemStartedNotification {
            item: app_server_protocol::protocol::v2::ThreadItem::SubAgentActivity {
                id: "activity-1".to_string(),
                metadata: None,
                kind: app_server_protocol::protocol::v2::SubAgentActivityKind::Started,
                agent_thread_id: "agent-1".to_string(),
                agent_path: "/root/worker".to_string(),
            },
            thread_id: "main".to_string(),
            turn_id: "turn-1".to_string(),
            started_at_ms: 1,
        },
    ));

    assert!(!app.can_accept_direct_input());
    assert_eq!(app.projection.status(), "sub-agent thread is parent-owned");
}

#[test]
fn sub_agent_activity_updates_navigation_liveness_and_label() {
    let mut app = App::default();
    app.set_thread_id("main".to_string());
    app.apply_notification(ServerNotification::ItemStarted(
        app_server_protocol::protocol::v2::ItemStartedNotification {
            item: app_server_protocol::protocol::v2::ThreadItem::SubAgentActivity {
                id: "activity-1".to_string(),
                metadata: None,
                kind: app_server_protocol::protocol::v2::SubAgentActivityKind::Started,
                agent_thread_id: "agent-1".to_string(),
                agent_path: "/root/worker".to_string(),
            },
            thread_id: "main".to_string(),
            turn_id: "turn-1".to_string(),
            started_at_ms: 1,
        },
    ));

    assert!(
        app.agent_navigation
            .get("agent-1")
            .expect("activity creates picker row")
            .is_running
    );
    assert_eq!(
        app.agent_navigation
            .active_agent_label(Some("agent-1"), Some("main")),
        Some("`/root/worker`".to_string())
    );
}
