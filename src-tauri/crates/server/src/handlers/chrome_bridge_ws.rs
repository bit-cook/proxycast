//! ChromeBridge WebSocket 路由处理器

use crate::chrome_bridge::{
    chrome_bridge_hub, ControlCommandPayload, ObserverCommandResultPayload,
};
use crate::AppState;
use axum::extract::{
    ws::{Message, WebSocket},
    Path, Query, State, WebSocketUpgrade,
};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Debug, Deserialize, Default)]
pub struct ObserverQuery {
    #[serde(default, alias = "profileKey")]
    pub profile_key: Option<String>,
}

pub async fn chrome_observer_ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(proxycast_key): Path<String>,
    Query(query): Query<ObserverQuery>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if proxycast_key != state.api_key {
        return axum::http::Response::builder()
            .status(401)
            .body(axum::body::Body::from("Invalid Proxycast_Key"))
            .unwrap()
            .into_response();
    }

    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    ws.on_upgrade(move |socket| {
        handle_observer_socket(socket, query.profile_key.clone(), user_agent)
    })
}

pub async fn chrome_control_ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Path(proxycast_key): Path<String>,
    headers: HeaderMap,
) -> impl IntoResponse {
    if proxycast_key != state.api_key {
        return axum::http::Response::builder()
            .status(401)
            .body(axum::body::Body::from("Invalid Proxycast_Key"))
            .unwrap()
            .into_response();
    }

    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    ws.on_upgrade(move |socket| handle_control_socket(socket, user_agent))
}

async fn handle_observer_socket(
    socket: WebSocket,
    profile_key: Option<String>,
    user_agent: Option<String>,
) {
    let hub = chrome_bridge_hub();
    let client_id = format!("observer-{}", Uuid::new_v4());

    let (mut ws_sender, mut ws_receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    hub.register_observer(
        client_id.clone(),
        profile_key.clone(),
        user_agent.clone(),
        tx.clone(),
    )
    .await;

    let _ = tx.send(
        serde_json::json!({
            "type": "connection_ack",
            "message": "Chrome observer connected",
            "data": {
                "clientId": client_id,
                "profileKey": profile_key,
            }
        })
        .to_string(),
    );

    let send_task = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if ws_sender.send(Message::Text(message)).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(message)) = ws_receiver.next().await {
        match message {
            Message::Text(text) => {
                if let Ok(value) = serde_json::from_str::<Value>(&text) {
                    handle_observer_message(&hub, &client_id, value).await;
                }
            }
            Message::Ping(payload) => {
                let _ = tx.send(
                    serde_json::json!({
                        "type": "pong",
                        "timestamp": chrono::Utc::now().timestamp_millis(),
                        "payload_len": payload.len(),
                    })
                    .to_string(),
                );
            }
            Message::Pong(_) => {}
            Message::Binary(_) => {}
            Message::Close(_) => break,
        }
    }

    send_task.abort();
    hub.unregister_observer(&client_id).await;
}

async fn handle_control_socket(socket: WebSocket, user_agent: Option<String>) {
    let hub = chrome_bridge_hub();
    let client_id = format!("control-{}", Uuid::new_v4());

    let (mut ws_sender, mut ws_receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<String>();

    hub.register_control(client_id.clone(), user_agent.clone(), tx.clone())
        .await;

    let _ = tx.send(
        serde_json::json!({
            "type": "connection_ack",
            "message": "Chrome control connected",
            "data": {
                "clientId": client_id,
            }
        })
        .to_string(),
    );

    let send_task = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if ws_sender.send(Message::Text(message)).await.is_err() {
                break;
            }
        }
    });

    while let Some(Ok(message)) = ws_receiver.next().await {
        match message {
            Message::Text(text) => {
                if let Ok(value) = serde_json::from_str::<Value>(&text) {
                    handle_control_message(&hub, &client_id, value).await;
                }
            }
            Message::Ping(payload) => {
                let _ = tx.send(
                    serde_json::json!({
                        "type": "pong",
                        "timestamp": chrono::Utc::now().timestamp_millis(),
                        "payload_len": payload.len(),
                    })
                    .to_string(),
                );
            }
            Message::Pong(_) => {}
            Message::Binary(_) => {}
            Message::Close(_) => break,
        }
    }

    send_task.abort();
    hub.unregister_control(&client_id).await;
}

async fn handle_observer_message(
    hub: &std::sync::Arc<crate::chrome_bridge::ChromeBridgeHub>,
    observer_client_id: &str,
    payload: Value,
) {
    let msg_type = payload
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    match msg_type.as_str() {
        "heartbeat" => {
            hub.handle_observer_heartbeat(observer_client_id).await;
        }
        "pageInfoUpdate" => {
            let markdown = payload
                .get("data")
                .and_then(|v| v.get("markdown"))
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            hub.handle_observer_page_info_update(observer_client_id, markdown)
                .await;
        }
        "command_result" => {
            let data = payload.get("data").cloned().unwrap_or(Value::Null);
            if let Ok(parsed) = serde_json::from_value::<ObserverCommandResultPayload>(data) {
                hub.handle_observer_command_result(observer_client_id, parsed)
                    .await;
            }
        }
        _ => {}
    }
}

async fn handle_control_message(
    hub: &std::sync::Arc<crate::chrome_bridge::ChromeBridgeHub>,
    control_client_id: &str,
    payload: Value,
) {
    let msg_type = payload
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    match msg_type.as_str() {
        "command" => {
            let data = payload.get("data").cloned().unwrap_or(Value::Null);
            if let Ok(parsed) = serde_json::from_value::<ControlCommandPayload>(data) {
                hub.handle_control_command(control_client_id, parsed).await;
            }
        }
        "heartbeat" => {
            hub.send_message_to_control(
                control_client_id,
                serde_json::json!({
                    "type": "heartbeat_ack",
                    "timestamp": chrono::Utc::now().timestamp_millis(),
                }),
            )
            .await;
        }
        _ => {}
    }
}
