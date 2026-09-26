use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{ensure, Context, Result};
use app_server_protocol::{JsonRpcError, JsonRpcMessage, JsonRpcResponse};
use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{accept_async, WebSocketStream};

use super::focus_palette::PtyLime;

const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
const TRANSCRIPT_TIMEOUT: Duration = Duration::from_secs(30);
const HISTORY_FAILURE_THREAD_ID: &str = "thread-history-failure-1";
const HISTORY_FAILURE_TURN_ID: &str = "turn-history-failure-1";
const HISTORY_FAILURE_CURSOR: &str = "history-cursor";

struct ResumeFixture {
    cli_bin: PathBuf,
    app_server_bin: PathBuf,
    backend_path: PathBuf,
    ledger_path: PathBuf,
    cwd: PathBuf,
    node_bin: PathBuf,
}

impl ResumeFixture {
    fn from_env() -> Self {
        Self {
            cli_bin: required_path("LIME_TEST_CLI_BIN"),
            app_server_bin: required_path("LIME_TEST_APP_SERVER_BIN"),
            backend_path: required_path("LIME_TEST_TERMINAL_BACKEND"),
            ledger_path: required_path("LIME_TEST_TERMINAL_LEDGER"),
            cwd: required_path("LIME_TEST_TERMINAL_CWD"),
            node_bin: required_path("LIME_TEST_NODE_BIN"),
        }
    }
}

struct TranscriptExpectation<'a> {
    thread_id: &'a str,
    oldest: &'a str,
    newest: &'a str,
    hidden_prompt: Option<&'a str>,
}

/// Exercise complete paginated and legacy transcripts through real `lime resume` PTYs.
///
/// The paginated fixture spans at least three item pages, so reaching its oldest item also covers
/// Codex's `transcript_home_loads_every_older_history_page` contract. Lime additionally keeps its
/// bounded resume preview unit-tested, while this Gate B binds the complete transcript loader to
/// public App Server JSON-RPC, canonical identities, alternate-screen input, and terminal restore.
#[test]
fn resume_picker_loads_complete_paginated_and_legacy_transcripts() -> Result<()> {
    if std::env::var_os("LIME_TEST_TUI_HISTORY_PAGINATION").is_none() {
        return Ok(());
    }

    let fixture = ResumeFixture::from_env();
    let paginated_thread_id = required_path("LIME_TEST_TUI_HISTORY_THREAD_ID")
        .to_string_lossy()
        .into_owned();
    let legacy_thread_id = required_path("LIME_TEST_TUI_LEGACY_THREAD_ID")
        .to_string_lossy()
        .into_owned();
    assert_complete_transcript(
        &fixture,
        TranscriptExpectation {
            thread_id: &paginated_thread_id,
            oldest: "SEED_000",
            newest: "SEED_100",
            hidden_prompt: Some("NESTED_REVIEW_PROMPT"),
        },
    )?;
    assert_complete_transcript(
        &fixture,
        TranscriptExpectation {
            thread_id: &legacy_thread_id,
            oldest: "LEGACY_000",
            newest: "LEGACY_050",
            hidden_prompt: None,
        },
    )?;
    Ok(())
}

/// Match Codex's main-scrollback contract without relying on transcript-overlay navigation.
#[test]
fn underfilled_scrollback_fetches_older_pages_without_opening_the_transcript() -> Result<()> {
    if std::env::var_os("LIME_TEST_TUI_HISTORY_PAGINATION").is_none() {
        return Ok(());
    }

    let fixture = ResumeFixture::from_env();
    let thread_id = required_path("LIME_TEST_TUI_HISTORY_THREAD_ID")
        .to_string_lossy()
        .into_owned();
    let mut terminal = PtyLime::start_resume(
        &fixture.cli_bin,
        &fixture.app_server_bin,
        &fixture.backend_path,
        &fixture.ledger_path,
        &fixture.cwd,
        &fixture.node_bin,
        &thread_id,
    )?;
    terminal.resize(400, 120)?;
    terminal.wait_for_startup_with_timeout(STARTUP_TIMEOUT)?;
    terminal.wait_for_screen_compact_contains("SEED_000", TRANSCRIPT_TIMEOUT)?;
    terminal.wait_for_screen_compact_contains("SEED_100", TRANSCRIPT_TIMEOUT)?;
    ensure!(
        !terminal.screen_contents().contains("TRANSCRIPT"),
        "main scrollback top-up unexpectedly opened the transcript overlay"
    );

    exit_and_assert_terminal_restored(terminal, &thread_id)
}

/// Exercise the transcript pager's failed-page footer and retry through a public remote transport.
///
/// The first request for the existing older-history cursor returns a JSON-RPC error. The same
/// cursor succeeds on Home retry, proving that the pager preserves the canonical pagination state
/// and that the user-visible failure/retry surface is wired through the real PTY.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn transcript_history_failure_keeps_anchor_and_home_retry_recovers() -> Result<()> {
    if std::env::var_os("LIME_TEST_TUI_HISTORY_PAGINATION").is_none() {
        return Ok(());
    }

    let cli_bin = required_path("LIME_TEST_CLI_BIN");
    let cwd = required_path("LIME_TEST_TERMINAL_CWD");
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let address = listener.local_addr()?;
    let failed_once = Arc::new(AtomicBool::new(false));
    let server_failed_once = failed_once.clone();
    let server =
        tokio::spawn(async move { run_history_failure_server(listener, server_failed_once).await });

    let remote_url = format!("ws://{address}/rpc");
    let mut terminal = super::focus_palette::PtyLime::start_remote_resume(
        &cli_bin,
        &remote_url,
        &cwd,
        HISTORY_FAILURE_THREAD_ID,
    )?;
    terminal.wait_for_startup_with_timeout(STARTUP_TIMEOUT)?;
    terminal.write_input(&[0x14])?;
    terminal.wait_for_screen_compact_contains("TRANSCRIPT", TRANSCRIPT_TIMEOUT)?;

    terminal.write_input(b"\x1b[H")?;
    terminal.wait_for_screen_compact_contains("History load failed", TRANSCRIPT_TIMEOUT)?;
    ensure!(
        terminal.screen_contains("Home retry"),
        "failed transcript page did not expose Home retry footer:\n{}",
        terminal.screen_contents()
    );

    terminal.write_input(b"\x1b[H")?;
    terminal.wait_for_screen_compact_contains("older-history", TRANSCRIPT_TIMEOUT)?;
    ensure!(
        !terminal.screen_contains("History load failed"),
        "successful history retry kept the failed footer:\n{}",
        terminal.screen_contents()
    );

    terminal.write_input(&[0x14])?;
    terminal.wait_for_screen_without("TRANSCRIPT", TRANSCRIPT_TIMEOUT)?;
    terminal.write_input(&[0x04])?;
    terminal.wait_for_exit()?;
    ensure!(
        terminal.output_contains(b"\x1b[?1049l"),
        "alternate screen was not restored after history failure/retry"
    );
    ensure!(
        failed_once.load(Ordering::SeqCst),
        "remote history fixture never observed the injected failure"
    );

    let server_result = tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .context("history failure fixture did not finish")??;
    server_result?;
    Ok(())
}

fn assert_complete_transcript(
    fixture: &ResumeFixture,
    expectation: TranscriptExpectation<'_>,
) -> Result<()> {
    let mut terminal = PtyLime::start_resume(
        &fixture.cli_bin,
        &fixture.app_server_bin,
        &fixture.backend_path,
        &fixture.ledger_path,
        &fixture.cwd,
        &fixture.node_bin,
        expectation.thread_id,
    )?;
    terminal.wait_for_startup_with_timeout(STARTUP_TIMEOUT)?;

    // Ctrl-T opens the complete transcript. Home is sent as the terminal's canonical ESC [ H
    // sequence; the pager requests an older page only when the scroll reaches its top.
    terminal.write_input(&[0x14])?;
    terminal.wait_for_screen_compact_contains("TRANSCRIPT", TRANSCRIPT_TIMEOUT)?;
    terminal.write_input(b"\x1b[H")?;
    terminal.wait_for_screen_compact_contains(expectation.oldest, TRANSCRIPT_TIMEOUT)?;
    let top = terminal.screen_contents();
    if let Some(hidden_prompt) = expectation.hidden_prompt {
        ensure!(
            !top.contains(hidden_prompt),
            "hidden prompt {hidden_prompt:?} leaked into top transcript page:\n{top}"
        );
    }

    terminal.write_input(b"\x1b[F")?;
    terminal.wait_for_screen_compact_contains(expectation.newest, TRANSCRIPT_TIMEOUT)?;
    let bottom = terminal.screen_contents();
    if let Some(hidden_prompt) = expectation.hidden_prompt {
        ensure!(
            !bottom.contains(hidden_prompt),
            "hidden prompt {hidden_prompt:?} leaked into newest transcript page:\n{bottom}"
        );
    }
    let mut previous_separator = false;
    for line in bottom.lines().filter(|line| !line.trim().is_empty()) {
        let separator = line.trim().starts_with("---");
        ensure!(
            !(separator && previous_separator),
            "completion separators duplicated after cross-page reconciliation:\n{bottom}"
        );
        previous_separator = separator;
    }

    terminal.write_input(&[0x14])?;
    terminal.wait_for_screen_without("TRANSCRIPT", TRANSCRIPT_TIMEOUT)?;
    exit_and_assert_terminal_restored(terminal, expectation.thread_id)
}

fn exit_and_assert_terminal_restored(mut terminal: PtyLime, thread_id: &str) -> Result<()> {
    terminal.write_input(&[0x04])?;
    terminal.wait_for_exit()?;
    ensure!(
        terminal.output_contains(b"\x1b[?1049l"),
        "alternate screen was not restored after transcript test for {}",
        thread_id
    );
    Ok(())
}

async fn run_history_failure_server(
    listener: TcpListener,
    failed_once: Arc<AtomicBool>,
) -> Result<()> {
    let (stream, _) = listener.accept().await?;
    let mut socket = accept_async(stream).await?;
    while let Some(message) = socket.next().await {
        let Message::Text(text) = message? else {
            continue;
        };
        let JsonRpcMessage::Request(request) = app_server_transport::decode_message(&text)? else {
            continue;
        };
        if request.method == "thread/items/list"
            && request
                .params
                .as_ref()
                .and_then(|params| params.get("cursor"))
                .and_then(Value::as_str)
                == Some(HISTORY_FAILURE_CURSOR)
            && !failed_once.swap(true, Ordering::SeqCst)
        {
            send_history_failure(&mut socket, request.id, "fixture history page failed once")
                .await?;
            continue;
        }
        let result = history_failure_response(&request.method, request.params.as_ref());
        socket
            .send(Message::Text(app_server_transport::encode_message(
                &JsonRpcMessage::Response(JsonRpcResponse::new(request.id, result)?),
            )?))
            .await?;
    }
    Ok(())
}

async fn send_history_failure(
    socket: &mut WebSocketStream<TcpStream>,
    id: app_server_protocol::RequestId,
    message: &str,
) -> Result<()> {
    socket
        .send(Message::Text(app_server_transport::encode_message(
            &JsonRpcMessage::Error(app_server_protocol::JsonRpcErrorResponse {
                id,
                error: JsonRpcError::new(-32000, message),
            }),
        )?))
        .await?;
    Ok(())
}

fn history_failure_response(method: &str, params: Option<&Value>) -> Value {
    match method {
        "initialize" => json!({
            "serverInfo": {
                "name": "app-server",
                "version": "fixture",
                "protocolVersion": app_server_protocol::PROTOCOL_VERSION
            },
            "platform": {"family": "unix", "os": "test"},
            "capabilities": {
                "agentSession": true,
                "capabilityDiscovery": true,
                "artifact": false,
                "workspace": false
            }
        }),
        "thread/resume" => history_failure_thread_response(),
        "thread/items/list" => {
            let cursor = params
                .and_then(|value| value.get("cursor"))
                .and_then(Value::as_str);
            if cursor == Some(HISTORY_FAILURE_CURSOR) {
                json!({
                    "data": [
                        history_failure_item("turn-history-failure-older", "older-history-user", true),
                        history_failure_item("turn-history-failure-older", "older-history", false)
                    ],
                    "nextCursor": null,
                    "backwardsCursor": null
                })
            } else {
                let data = (0..24)
                    .flat_map(|index| {
                        let turn_id = format!("{HISTORY_FAILURE_TURN_ID}-{index}");
                        [
                            history_failure_item(
                                &turn_id,
                                &format!("recent-history-user-{index}"),
                                true,
                            ),
                            history_failure_item(
                                &turn_id,
                                &format!("recent-history-{index}"),
                                false,
                            ),
                        ]
                    })
                    .collect::<Vec<_>>();
                json!({
                    "data": data,
                    "nextCursor": HISTORY_FAILURE_CURSOR,
                    "backwardsCursor": null
                })
            }
        }
        "thread/turns/list" => json!({
            "data": [],
            "nextCursor": null,
            "backwardsCursor": null
        }),
        "permissionProfile/list" => json!({
            "data": [{"id": ":workspace", "description": "fixture", "allowed": true}],
            "nextCursor": null
        }),
        "model/list" => json!({"data": [], "nextCursor": null}),
        "skills/list" => json!({"data": [], "nextCursor": null}),
        "collaborationMode/list" => json!({"data": []}),
        "thread/settings/update" => json!({}),
        "promptHistory/read" => {
            json!({"logId": "fixture", "entryCount": 0, "data": [], "nextCursor": null})
        }
        "thread/queue/list" => json!({"data": [], "nextCursor": null}),
        _ => json!({}),
    }
}

fn history_failure_thread_response() -> Value {
    json!({
        "thread": {
            "id": HISTORY_FAILURE_THREAD_ID,
            "sessionId": HISTORY_FAILURE_THREAD_ID,
            "preview": "",
            "ephemeral": false,
            "projectId": null,
            "historyMode": "paginated",
            "modelProvider": "fixture-provider",
            "createdAt": 1,
            "updatedAt": 2,
            "status": {"type": "idle", "activeFlags": []},
            "cwd": "/tmp/history-failure",
            "cliVersion": "fixture",
            "source": "appServer",
            "turns": []
        },
        "model": "fixture-model",
        "modelProvider": "fixture-provider",
        "cwd": "/tmp/history-failure",
        "runtimeWorkspaceRoots": ["/tmp/history-failure"],
        "instructionSources": [],
        "approvalPolicy": "never",
        "approvalsReviewer": "user",
        "sandbox": {"type": "readOnly"},
        "activePermissionProfile": {"id": ":workspace"},
        "reasoningEffort": null,
        "multiAgentMode": "explicitRequestOnly"
    })
}

fn history_failure_item(turn_id: &str, text: &str, user: bool) -> Value {
    let item = if user {
        json!({
            "type": "userMessage",
            "id": format!("{text}-id"),
            "content": [{"type": "text", "text": text}]
        })
    } else {
        json!({
            "type": "agentMessage",
            "id": format!("{text}-id"),
            "text": text,
            "phase": "final_answer"
        })
    };
    json!({"turnId": turn_id, "item": item})
}

fn required_path(name: &str) -> PathBuf {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("missing {name}"))
}
