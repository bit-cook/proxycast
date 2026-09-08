use super::*;
use app_server_protocol::protocol::v2::{
    AgentMessageDeltaNotification, ServerNotification, ServerRequest,
    ServerRequestResolvedNotification, ThreadClosedNotification, Turn, TurnCompletedNotification,
    TurnItemsView, TurnStatus,
};
use app_server_protocol::RequestId;

fn delta(thread_id: &str, turn_id: &str, item_id: &str, text: &str) -> ServerNotification {
    ServerNotification::AgentMessageDelta(AgentMessageDeltaNotification {
        thread_id: thread_id.to_string(),
        turn_id: turn_id.to_string(),
        item_id: item_id.to_string(),
        delta: text.to_string(),
    })
}

#[test]
fn thread_event_store_coalesces_only_adjacent_matching_agent_message_deltas() {
    let mut store = ThreadEventStore::new(/*capacity*/ 4);
    store.push_notification(delta("thread", "turn", "item", "hello"));
    store.push_notification(delta("thread", "turn", "item", " world"));
    store.push_notification(delta("thread", "turn", "other", "bar"));

    let snapshot = store.snapshot();
    assert_eq!(snapshot.events.len(), 2);
    let ThreadBufferedEvent::Notification(first) = &snapshot.events[0] else {
        panic!("expected notification");
    };
    let ServerNotification::AgentMessageDelta(first) = first.as_ref() else {
        panic!("expected agent delta");
    };
    assert_eq!(first.delta, "hello world");
}

#[test]
fn thread_event_store_preserves_requests_while_evicting_old_notifications() {
    let mut store = ThreadEventStore::new(/*capacity*/ 4);
    let request = ServerRequest::CurrentTimeRead {
        id: RequestId::Integer(1),
        params: app_server_protocol::protocol::v2::CurrentTimeReadParams {
            thread_id: "thread".to_string(),
        },
    };
    assert!(store.push_request(request));
    for index in 0..6 {
        store.push_notification(delta("thread", "turn", &format!("item-{index}"), "x"));
    }

    let snapshot = store.snapshot();
    assert_eq!(snapshot.events.len(), 4);
    assert!(snapshot
        .events
        .iter()
        .any(|event| matches!(event, ThreadBufferedEvent::Request(_))));
}

#[test]
fn thread_event_store_tracks_delta_bytes_after_coalescing() {
    let mut store = ThreadEventStore::new(/*capacity*/ 4);
    store.push_notification(delta("thread", "turn", "item", "a"));
    store.push_notification(delta("thread", "turn", "item", "bc"));
    assert_eq!(store.buffered_agent_message_delta_bytes, 3);
}

fn user_input_request(id: i64, thread_id: &str) -> ServerRequest {
    ServerRequest::ItemToolRequestUserInput {
        id: RequestId::Integer(id),
        params: app_server_protocol::protocol::v2::ToolRequestUserInputParams {
            thread_id: thread_id.to_string(),
            turn_id: "turn".to_string(),
            item_id: format!("item-{id}"),
            questions: Vec::new(),
            auto_resolution_ms: None,
        },
    }
}

#[test]
fn thread_event_store_does_not_replay_server_resolved_request() {
    let mut store = ThreadEventStore::new(4);
    store.push_request(user_input_request(7, "thread"));
    store.push_notification(ServerNotification::ServerRequestResolved(
        ServerRequestResolvedNotification {
            thread_id: "thread".to_string(),
            request_id: RequestId::Integer(7),
        },
    ));

    assert!(store
        .snapshot()
        .events
        .iter()
        .all(|event| { !matches!(event, ThreadBufferedEvent::Request(_)) }));
}

#[test]
fn thread_event_store_drops_requests_when_turn_completes_or_thread_closes() {
    let mut store = ThreadEventStore::new(4);
    store.push_request(user_input_request(8, "thread"));
    store.push_notification(ServerNotification::TurnCompleted(
        TurnCompletedNotification {
            thread_id: "thread".to_string(),
            turn: Turn {
                id: "turn".to_string(),
                items: Vec::new(),
                items_view: TurnItemsView::Full,
                status: TurnStatus::Completed,
                error: None,
                started_at: None,
                completed_at: None,
                duration_ms: None,
            },
        },
    ));
    assert!(store
        .snapshot()
        .events
        .iter()
        .all(|event| { !matches!(event, ThreadBufferedEvent::Request(_)) }));

    store.push_request(user_input_request(9, "thread"));
    store.push_notification(ServerNotification::ThreadClosed(ThreadClosedNotification {
        thread_id: "thread".to_string(),
    }));
    assert!(store
        .snapshot()
        .events
        .iter()
        .all(|event| { !matches!(event, ThreadBufferedEvent::Request(_)) }));
}

#[test]
fn thread_event_store_request_capacity_fails_closed_without_evicting_requests() {
    let mut store = ThreadEventStore::new(1);
    assert!(store.push_request(user_input_request(10, "thread")));
    assert!(!store.push_request(user_input_request(11, "thread")));
    let snapshot = store.snapshot();
    assert_eq!(snapshot.events.len(), 1);
    assert!(matches!(
        snapshot.events.first(),
        Some(ThreadBufferedEvent::Request(request)) if request.id() == &RequestId::Integer(10)
    ));
}
