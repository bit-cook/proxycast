//! Pagination completion boundaries preserve Codex item grouping semantics.

use super::*;
use app_server_protocol::protocol::v2::{ThreadItem, TurnItemsView, TurnStatus};

fn turn(id: &str, status: TurnStatus, item_ids: &[&str]) -> Turn {
    Turn {
        id: id.to_string(),
        items: item_ids
            .iter()
            .map(|id| ThreadItem::UserMessage {
                id: id.to_string(),
                metadata: None,
                client_id: None,
                content: Vec::new(),
            })
            .collect(),
        items_view: TurnItemsView::Summary,
        status,
        error: None,
        started_at: Some(1_700_000_000),
        completed_at: Some(1_700_000_001),
        duration_ms: Some(125_000),
    }
}

#[test]
fn split_turn_only_completes_on_the_page_with_its_last_item() {
    let turns = vec![turn(
        "turn",
        TurnStatus::Completed,
        &["first", "second", "last"],
    )];
    assert!(
        group_completed_turn_items(turns[0].items[1..].to_vec(), &turns)[0]
            .1
            .is_some()
    );
    assert!(
        group_completed_turn_items(turns[0].items[..1].to_vec(), &turns)[0]
            .1
            .is_none()
    );
}

#[test]
fn multiple_turns_keep_item_groups_and_completion_order() {
    let turns = vec![
        turn("first", TurnStatus::Completed, &["a", "b"]),
        turn("second", TurnStatus::Completed, &["c", "d"]),
    ];
    let items = turns
        .iter()
        .flat_map(|turn| turn.items.clone())
        .collect::<Vec<_>>();
    let groups = group_completed_turn_items(items, &turns);
    assert_eq!(groups.len(), 2);
    assert_eq!(groups[0].0.len(), 2);
    assert_eq!(groups[1].0.len(), 2);
    assert_eq!(
        groups[0]
            .1
            .as_ref()
            .and_then(|boundary| boundary.elapsed_seconds),
        Some(125)
    );
    assert_eq!(
        groups[1]
            .1
            .as_ref()
            .and_then(|boundary| boundary.elapsed_seconds),
        Some(125)
    );
}

#[test]
fn unsuccessful_and_running_turns_do_not_create_completion_boundaries() {
    let turns = vec![
        turn("failed", TurnStatus::Failed, &["a"]),
        turn("interrupted", TurnStatus::Interrupted, &["b"]),
        turn("running", TurnStatus::InProgress, &["c"]),
    ];
    let items = turns
        .iter()
        .flat_map(|turn| turn.items.clone())
        .collect::<Vec<_>>();
    let groups = group_completed_turn_items(items, &turns);
    assert_eq!(groups.len(), 1);
    assert!(groups[0].1.is_none());
}
