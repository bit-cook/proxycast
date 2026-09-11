//! Drives automatic reconnect through the real TUI, remote App Server transport, and PTY.
//!
//! The fixture speaks the public App Server JSON-RPC WebSocket contract. It is intentionally
//! transport-only: the TUI still uses `RemoteTransport`, `ClientSession`, and the canonical
//! `thread/start`/`thread/resume` shapes used by a Cloud transport.

use std::ffi::OsString;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use anyhow::{bail, ensure, Context, Result};
use app_server_protocol::{JsonRpcError, JsonRpcMessage, JsonRpcResponse, RequestId};
use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{oneshot, Mutex};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{accept_async, WebSocketStream};

const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);
const RECONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const THREAD_ID: &str = "thread-reconnect-1";
const TURN_ID: &str = "turn-reconnect-1";
const INITIAL_CWD: &str = "/tmp/reconnect-initial";
const RESTORED_CWD: &str = "/tmp/reconnect-restored";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn automatic_reconnect_restores_draft_and_routes_new_notifications() -> Result<()> {
    if std::env::var_os("LIME_TEST_TUI_GATE_B").is_none() {
        return Ok(());
    }

    let cli_bin = required_test_path("LIME_TEST_CLI_BIN");
    let cwd = required_test_path("LIME_TEST_TUI_REMOTE_CWD");
    let listener = TcpListener::bind(("127.0.0.1", 0)).await?;
    let address = listener.local_addr()?;
    let (disconnect_tx, disconnect_rx) = oneshot::channel();
    let (restore_tx, restore_rx) = oneshot::channel();
    let (restore_ready_tx, restore_ready_rx) = oneshot::channel();
    let methods = std::sync::Arc::new(Mutex::new(Vec::<String>::new()));
    let server_methods = methods.clone();
    let server = tokio::spawn(async move {
        run_fixture_server(
            listener,
            server_methods,
            disconnect_rx,
            restore_rx,
            restore_ready_tx,
        )
        .await
    });

    let mut terminal = PtyReconnect::start(&cli_bin, &format!("ws://{address}/rpc"), &cwd)?;
    terminal.wait_for_startup()?;
    terminal.wait_for_screen("fixture-model", STARTUP_TIMEOUT)?;
    terminal.write_input(b"/pwd\r")?;
    terminal.wait_for_screen(INITIAL_CWD, RECONNECT_TIMEOUT)?;

    terminal.write_input(b"preserved-draft")?;
    terminal.wait_for_screen("preserved-draft", RECONNECT_TIMEOUT)?;
    disconnect_tx
        .send(())
        .map_err(|_| anyhow::anyhow!("fixture disconnected before TUI input"))?;
    terminal.wait_for_screen("reconnecting", RECONNECT_TIMEOUT)?;

    terminal.write_input(b"!")?;
    terminal.wait_for_screen("preserved-draft!", RECONNECT_TIMEOUT)?;
    restore_ready_rx
        .await
        .map_err(|_| anyhow::anyhow!("TUI did not open a reconnect transport"))?;
    restore_tx
        .send(())
        .map_err(|_| anyhow::anyhow!("fixture reconnect gate was not waiting"))?;
    terminal.wait_for_screen("fresh-notification-after-reconnect", RECONNECT_TIMEOUT)?;
    ensure!(
        terminal.screen_contains("preserved-draft!"),
        "draft was lost after recovery; screen:\n{}",
        terminal.screen_contents()
    );
    terminal.write_input(&[21])?;
    terminal.write_input(b"/pwd\r")?;
    terminal.wait_for_screen(RESTORED_CWD, RECONNECT_TIMEOUT)?;

    terminal.write_input(&[4])?;
    terminal.wait_for_exit()?;
    ensure!(
        terminal.output_contains(b"\x1b[?1049l"),
        "alternate screen was not restored after reconnect test"
    );

    let observed = tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .context("reconnect fixture did not finish")??;
    let observed = observed?;
    let methods = methods.lock().await.clone();
    ensure!(
        methods
            .iter()
            .filter(|method| method.as_str() == "thread/resume")
            .count()
            == 2,
        "reconnect did not issue exactly two thread/resume requests: {methods:?}"
    );
    ensure!(
        !methods.iter().any(|method| method == "turn/start"),
        "reconnect started a new turn instead of resuming: {methods:?}"
    );
    ensure!(
        observed,
        "reconnect fixture did not observe the final client close"
    );
    Ok(())
}

async fn run_fixture_server(
    listener: TcpListener,
    methods: std::sync::Arc<Mutex<Vec<String>>>,
    mut disconnect_rx: oneshot::Receiver<()>,
    restore_rx: oneshot::Receiver<()>,
    restore_ready_tx: oneshot::Sender<()>,
) -> Result<bool> {
    let mut restore_ready_tx = Some(restore_ready_tx);
    let mut restore_rx = Some(restore_rx);
    let mut connection_index = 0_u8;
    loop {
        let (stream, _) = listener.accept().await?;
        let mut socket = accept_async(stream).await?;
        if connection_index == 1 {
            if let Some(sender) = restore_ready_tx.take() {
                let _ = sender.send(());
            }
            if let Some(receiver) = restore_rx.take() {
                let _ = receiver.await;
            }
        }
        let closed = serve_connection(
            &mut socket,
            connection_index,
            methods.clone(),
            &mut disconnect_rx,
        )
        .await?;
        connection_index = connection_index.saturating_add(1);
        if closed && connection_index >= 3 {
            return Ok(true);
        }
    }
}

async fn serve_connection(
    socket: &mut WebSocketStream<TcpStream>,
    connection_index: u8,
    methods: std::sync::Arc<Mutex<Vec<String>>>,
    disconnect_rx: &mut oneshot::Receiver<()>,
) -> Result<bool> {
    loop {
        let message = if connection_index == 0 {
            tokio::select! {
                _ = &mut *disconnect_rx => {
                    socket.close(None).await?;
                    return Ok(true);
                }
                message = socket.next() => message,
            }
        } else {
            socket.next().await
        };
        let Some(message) = message else {
            return Ok(true);
        };
        let Message::Text(text) = message? else {
            continue;
        };
        let rpc = app_server_transport::decode_message(&text)?;
        let JsonRpcMessage::Request(request) = rpc else {
            continue;
        };
        methods.lock().await.push(request.method.clone());

        if connection_index == 1 && request.method == "thread/resume" {
            send_error(
                socket,
                request.id,
                "retry thread resume after transport recovery",
            )
            .await?;
            continue;
        }

        let response = fixture_response(&request.method, connection_index);
        socket
            .send(Message::Text(app_server_transport::encode_message(
                &JsonRpcMessage::Response(JsonRpcResponse::new(request.id, response)?),
            )?))
            .await?;

        if request.method == "thread/resume" && connection_index >= 2 {
            socket
                .send(Message::Text(app_server_transport::encode_message(
                    &JsonRpcMessage::Notification(app_server_protocol::JsonRpcNotification::new(
                        "item/agentMessage/delta",
                        Some(json!({
                            "threadId": THREAD_ID,
                            "turnId": TURN_ID,
                            "itemId": "live-item",
                            "delta": "fresh-notification-after-reconnect",
                        })),
                    )),
                )?))
                .await?;
        }
    }
}

async fn send_error(
    socket: &mut WebSocketStream<TcpStream>,
    id: RequestId,
    message: &str,
) -> Result<()> {
    socket
        .send(Message::Text(app_server_transport::encode_message(
            &JsonRpcMessage::Error(app_server_protocol::JsonRpcErrorResponse {
                id,
                error: JsonRpcError::new(-32600, message),
            }),
        )?))
        .await?;
    Ok(())
}

fn fixture_response(method: &str, connection_index: u8) -> Value {
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
        "thread/start" | "thread/resume" => thread_response(connection_index),
        "permissionProfile/list" => json!({
            "data": [{"id": ":workspace", "description": "fixture", "allowed": true}],
            "nextCursor": null
        }),
        "collaborationMode/list" => json!({"data": []}),
        "thread/settings/update" => json!({}),
        "promptHistory/read" => json!({
            "logId": "fixture",
            "entryCount": 0,
            "data": [],
            "nextCursor": null
        }),
        "thread/queue/list" => json!({"data": [], "nextCursor": null}),
        "skills/list" => json!({"data": [], "nextCursor": null}),
        _ => json!({}),
    }
}

fn thread_response(connection_index: u8) -> Value {
    let cwd = if connection_index == 0 {
        INITIAL_CWD
    } else {
        RESTORED_CWD
    };
    json!({
        "thread": {
            "id": THREAD_ID,
            "sessionId": THREAD_ID,
            "preview": "",
            "ephemeral": false,
            "projectId": null,
            "historyMode": "legacy",
            "modelProvider": "fixture-provider",
            "createdAt": 1,
            "updatedAt": 2,
            "status": {"type": "active", "activeFlags": []},
            "cwd": cwd,
            "cliVersion": "fixture",
            "source": "appServer",
            "turns": [{
                "id": TURN_ID,
                "items": [],
                "itemsView": "full",
                "status": "inProgress"
            }]
        },
        "model": "fixture-model",
        "modelProvider": "fixture-provider",
        "cwd": cwd,
        "runtimeWorkspaceRoots": [cwd],
        "instructionSources": [],
        "approvalPolicy": "never",
        "approvalsReviewer": "user",
        "sandbox": {"type": "readOnly"},
        "activePermissionProfile": {"id": ":workspace"},
        "reasoningEffort": null,
        "multiAgentMode": "explicitRequestOnly"
    })
}

struct PtyReconnect {
    master: Box<dyn portable_pty::MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    reader: mpsc::Receiver<Vec<u8>>,
    output: Vec<u8>,
    parser: vt100::Parser,
    cursor_answered: bool,
    palette_answered: bool,
}

impl PtyReconnect {
    fn start(cli_bin: &Path, remote_url: &str, cwd: &Path) -> Result<Self> {
        let pair = portable_pty::native_pty_system().openpty(portable_pty::PtySize {
            rows: 32,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        let mut command = portable_pty::CommandBuilder::new(cli_bin);
        for argument in [
            OsString::from("tui"),
            OsString::from("--cd"),
            cwd.as_os_str().to_os_string(),
            OsString::from("--model"),
            OsString::from("fixture-model"),
            OsString::from("--provider"),
            OsString::from("fixture-provider"),
            OsString::from("--remote"),
            OsString::from(remote_url),
        ] {
            command.arg(argument);
        }
        command.cwd(cwd);
        command.env("TERM", "xterm-256color");
        command.env("LIME_LOCALE", "en-US");
        let child = pair.slave.spawn_command(command)?;
        let mut reader = pair.master.try_clone_reader()?;
        let writer = pair.master.take_writer()?;
        let (output_tx, output_rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut buffer = [0_u8; 8192];
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(read) => {
                        if output_tx.send(buffer[..read].to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(error) => {
                        let _ = output_tx.send(format!("PTY_READER_ERROR: {error}\n").into_bytes());
                        break;
                    }
                }
            }
        });
        Ok(Self {
            master: pair.master,
            writer,
            child,
            reader: output_rx,
            output: Vec::new(),
            parser: vt100::Parser::new(32, 120, 0),
            cursor_answered: false,
            palette_answered: false,
        })
    }

    fn wait_for_startup(&mut self) -> Result<()> {
        let deadline = Instant::now() + STARTUP_TIMEOUT;
        while Instant::now() < deadline {
            self.read_output(Duration::from_millis(50))?;
            self.answer_startup_queries()?;
            if self.palette_answered {
                return Ok(());
            }
            if let Some(status) = self.child.try_wait()? {
                bail!(
                    "Lime exited before reconnect test started ({status:?}); output:\n{}",
                    String::from_utf8_lossy(&self.output)
                );
            }
        }
        bail!(
            "Lime did not initialize within {STARTUP_TIMEOUT:?}; screen:\n{}",
            self.screen_contents()
        )
    }

    fn wait_for_screen(&mut self, text: &str, timeout: Duration) -> Result<()> {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if self.screen_contains(text) {
                return Ok(());
            }
            self.read_output(Duration::from_millis(20))?;
        }
        bail!(
            "terminal did not render {text:?}; screen:\n{}",
            self.screen_contents()
        )
    }

    fn answer_startup_queries(&mut self) -> Result<()> {
        if !self.cursor_answered && contains_bytes(&self.output, b"\x1b[6n") {
            self.write_input(b"\x1b[1;1R")?;
            self.cursor_answered = true;
        }
        if !self.palette_answered
            && contains_bytes(&self.output, b"\x1b]10;?")
            && contains_bytes(&self.output, b"\x1b]11;?")
        {
            self.write_input(b"\x1b]10;rgb:ffff/ffff/ffff\x1b\\\x1b]11;rgb:0000/0000/0000\x1b\\")?;
            self.palette_answered = true;
        }
        Ok(())
    }

    fn read_output(&mut self, timeout: Duration) -> Result<()> {
        if let Ok(chunk) = self.reader.recv_timeout(timeout) {
            self.output.extend_from_slice(&chunk);
            self.parser.process(&chunk);
        }
        Ok(())
    }

    fn write_input(&mut self, bytes: &[u8]) -> Result<()> {
        self.writer.write_all(bytes)?;
        self.writer.flush()?;
        Ok(())
    }

    fn wait_for_exit(&mut self) -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            if self.child.try_wait()?.is_some() {
                return Ok(());
            }
            self.read_output(Duration::from_millis(20))?;
        }
        bail!("Lime did not exit after reconnect test")
    }

    fn screen_contains(&self, text: &str) -> bool {
        self.parser.screen().contents().contains(text)
    }

    fn screen_contents(&self) -> String {
        self.parser.screen().contents()
    }

    fn output_contains(&self, needle: &[u8]) -> bool {
        contains_bytes(&self.output, needle)
    }
}

impl Drop for PtyReconnect {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = &self.master;
    }
}

fn required_test_path(name: &str) -> PathBuf {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| panic!("missing {name}"))
}

fn contains_bytes(buffer: &[u8], needle: &[u8]) -> bool {
    buffer.windows(needle.len()).any(|window| window == needle)
}
