use super::*;
use crate::execution_process::ExecutionProcessServer;
use app_server_protocol::protocol::v2::ThreadShellCommandParams;
use async_trait::async_trait;
use serde_json::json;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::sync::Notify;
use tokio::time::{sleep, timeout, Duration};

struct BlockingShellHost {
    started: Notify,
    release: Notify,
}

#[async_trait]
impl ExecutionBackend for BlockingShellHost {
    async fn start_turn(
        &self,
        _request: ExecutionRequest,
        sink: &mut dyn RuntimeEventSink,
    ) -> Result<(), RuntimeCoreError> {
        sink.emit(RuntimeEvent::new("turn.started", json!({})))?;
        self.started.notify_one();
        self.release.notified().await;
        sink.emit(RuntimeEvent::new("turn.completed", json!({})))
    }

    async fn cancel_turn(
        &self,
        _request: CancelExecutionRequest,
        _sink: &mut dyn RuntimeEventSink,
    ) -> Result<(), RuntimeCoreError> {
        Ok(())
    }

    async fn respond_action(
        &self,
        _request: ActionRespondRequest,
        _sink: &mut dyn RuntimeEventSink,
    ) -> Result<(), RuntimeCoreError> {
        Ok(())
    }
}

#[tokio::test]
async fn idle_shell_command_persists_a_standalone_turn_and_output() {
    let temp = TempDir::new().expect("shell temp dir");
    let store = Arc::new(
        ProjectionStore::initialize(temp.path().join("projection.sqlite"))
            .expect("shell projection store"),
    );
    let core = RuntimeCore::with_backend(Arc::new(MockBackend))
        .with_execution_process_server(ExecutionProcessServer::default())
        .with_projection_store(store.clone());
    start_shell_session(&core, &temp, "session-shell-idle", "thread-shell-idle");

    core.run_thread_shell_command(ThreadShellCommandParams {
        thread_id: "thread-shell-idle".to_string(),
        command: "printf shell-loop-ready".to_string(),
    })
    .await
    .expect("run idle shell command");

    wait_for_event(&core, "session-shell-idle", "turn.completed").await;
    let events = core
        .events_for_session("session-shell-idle")
        .expect("shell events");
    let event_types = events
        .iter()
        .map(|event| event.event_type.as_str())
        .collect::<Vec<_>>();
    assert_lifecycle_order(
        &event_types,
        &[
            "turn.accepted",
            "turn.started",
            "command.started",
            "command.output",
            "command.exited",
            "turn.completed",
        ],
    );

    let thread = read_full_thread(&core, "thread-shell-idle").await;
    assert_eq!(thread.turns.len(), 1);
    assert_eq!(
        thread.turns[0].status,
        agent_protocol::TurnStatus::Completed
    );
    let item = thread.turns[0]
        .items
        .iter()
        .find(|item| {
            matches!(
                item.payload,
                agent_protocol::ThreadItemPayload::Command { .. }
            )
        })
        .expect("shell command item");
    let agent_protocol::ThreadItemPayload::Command {
        command,
        cwd,
        output,
        exit_code,
    } = &item.payload
    else {
        unreachable!();
    };
    let expected_cwd = std::fs::canonicalize(temp.path()).expect("canonical shell cwd");
    assert_eq!(command, "printf shell-loop-ready");
    assert_eq!(cwd.as_deref(), expected_cwd.to_str());
    assert_eq!(output.as_deref(), Some("shell-loop-ready"));
    assert_eq!(*exit_code, Some(0));
    assert_eq!(item.metadata["commandExecutionSource"], json!("userShell"));
    assert!(item.metadata["processId"].as_str().is_some());
    let output_event = events
        .iter()
        .find(|event| event.event_type == "command.output")
        .expect("shell output delta");
    assert_eq!(output_event.payload["commandId"], item.item_id.as_str());
    assert_eq!(output_event.payload["delta"], "shell-loop-ready");
    assert_eq!(
        output_event.payload["metadata"]["executionProcessControlStatus"],
        "registered"
    );

    let restarted = RuntimeCore::with_backend(Arc::new(MockBackend))
        .with_execution_process_server(ExecutionProcessServer::default())
        .with_projection_store(store);
    let restored = read_full_thread(&restarted, "thread-shell-idle").await;
    assert_eq!(restored.turns.len(), 1);
    assert_eq!(restored.turns[0].items.len(), 1);
    assert_eq!(
        restored.turns[0].items[0].metadata["commandExecutionSource"],
        json!("userShell")
    );
}

#[tokio::test]
async fn active_shell_command_reuses_the_current_turn() {
    let temp = TempDir::new().expect("active shell temp dir");
    let store = Arc::new(
        ProjectionStore::initialize(temp.path().join("projection.sqlite"))
            .expect("active shell projection store"),
    );
    let backend = Arc::new(BlockingShellHost {
        started: Notify::new(),
        release: Notify::new(),
    });
    let core = RuntimeCore::with_backend(backend.clone())
        .with_execution_process_server(ExecutionProcessServer::default())
        .with_projection_store(store);
    start_shell_session(&core, &temp, "session-shell-active", "thread-shell-active");

    let turn_core = core.clone();
    let turn = tokio::spawn(async move {
        turn_core
            .start_turn(
                AgentSessionTurnStartParams {
                    session_id: "session-shell-active".to_string(),
                    turn_id: Some("turn-shell-active".to_string()),
                    input: AgentInput {
                        text: "hold the turn".to_string(),
                        attachments: Vec::new(),
                    },
                    runtime_options: None,
                    queue_if_busy: false,
                    skip_pre_submit_resume: false,
                },
                RuntimeHostContext::default(),
            )
            .await
    });
    timeout(Duration::from_secs(2), backend.started.notified())
        .await
        .expect("active turn started");

    core.run_thread_shell_command(ThreadShellCommandParams {
        thread_id: "thread-shell-active".to_string(),
        command: "printf active-shell-ready".to_string(),
    })
    .await
    .expect("run active shell command");
    wait_for_event(&core, "session-shell-active", "command.exited").await;

    let events = core
        .events_for_session("session-shell-active")
        .expect("active shell events");
    let command_events = events
        .iter()
        .filter(|event| event.event_type.starts_with("command."))
        .collect::<Vec<_>>();
    assert_eq!(command_events.len(), 3);
    assert!(command_events
        .iter()
        .all(|event| event.turn_id.as_deref() == Some("turn-shell-active")));
    assert_eq!(
        command_events
            .iter()
            .map(|event| event.event_type.as_str())
            .collect::<Vec<_>>(),
        vec!["command.started", "command.output", "command.exited"]
    );
    let thread = read_full_thread(&core, "thread-shell-active").await;
    assert_eq!(thread.turns.len(), 1);
    assert_eq!(thread.turns[0].turn_id.as_str(), "turn-shell-active");

    backend.release.notify_one();
    timeout(Duration::from_secs(2), turn)
        .await
        .expect("active turn completion timeout")
        .expect("active turn task")
        .expect("active turn result");
}

#[tokio::test]
async fn shell_command_rejects_empty_input_and_missing_execution_environment() {
    let temp = TempDir::new().expect("shell validation temp dir");
    let store = Arc::new(
        ProjectionStore::initialize(temp.path().join("projection.sqlite"))
            .expect("shell validation projection store"),
    );
    let core = RuntimeCore::with_backend(Arc::new(MockBackend)).with_projection_store(store);
    start_shell_session(
        &core,
        &temp,
        "session-shell-validation",
        "thread-shell-validation",
    );

    let empty = core
        .run_thread_shell_command(ThreadShellCommandParams {
            thread_id: "thread-shell-validation".to_string(),
            command: "   ".to_string(),
        })
        .await
        .expect_err("empty shell command must fail");
    assert!(matches!(empty, RuntimeCoreError::InvalidRequest(_)));

    let unavailable = core
        .run_thread_shell_command(ThreadShellCommandParams {
            thread_id: "thread-shell-validation".to_string(),
            command: "printf unreachable".to_string(),
        })
        .await
        .expect_err("missing execution environment must fail");
    assert!(matches!(unavailable, RuntimeCoreError::Backend(_)));
}

fn start_shell_session(core: &RuntimeCore, temp: &TempDir, session_id: &str, thread_id: &str) {
    core.start_session(AgentSessionStartParams {
        session_id: Some(session_id.to_string()),
        thread_id: Some(thread_id.to_string()),
        app_id: "agent-chat".to_string(),
        workspace_id: None,
        business_object_ref: Some(BusinessObjectRef {
            kind: "agent.thread".to_string(),
            id: thread_id.to_string(),
            title: None,
            uri: None,
            metadata: Some(json!({
                "providerSelector": "provider-test",
                "providerName": "provider-test",
                "modelName": "model-test",
                "workingDir": temp.path().to_string_lossy(),
            })),
        }),
        locale: None,
    })
    .expect("start shell session");
}

async fn wait_for_event(core: &RuntimeCore, session_id: &str, event_type: &str) {
    timeout(Duration::from_secs(3), async {
        loop {
            if core
                .events_for_session(session_id)
                .expect("session events")
                .iter()
                .any(|event| event.event_type == event_type)
            {
                break;
            }
            sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("timed out waiting for {event_type}"));
}

async fn read_full_thread(core: &RuntimeCore, thread_id: &str) -> agent_protocol::Thread {
    core.read_thread(agent_protocol::thread::ThreadReadParams {
        thread_id: agent_protocol::ThreadId::new(thread_id),
        turns_view: agent_protocol::ThreadTurnsView::Full,
    })
    .await
    .expect("read shell thread")
    .thread
}

fn assert_lifecycle_order(actual: &[&str], expected: &[&str]) {
    let mut cursor = 0;
    for event_type in actual {
        if expected.get(cursor) == Some(event_type) {
            cursor += 1;
        }
    }
    assert_eq!(cursor, expected.len(), "event order: {actual:?}");
}
