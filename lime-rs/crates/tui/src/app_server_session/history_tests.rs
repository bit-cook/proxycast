use app_server_protocol::protocol::v2::{
    ThreadItem, ThreadItemEntry, ThreadItemsListResponse, Turn, TurnItemsView, TurnStatus,
};

use super::*;

fn page(ids: &[&str], next_cursor: Option<&str>) -> ThreadItemsListResponse {
    ThreadItemsListResponse {
        data: ids
            .iter()
            .map(|id| ThreadItemEntry {
                turn_id: "turn-1".to_string(),
                item: ThreadItem::AgentMessage {
                    id: (*id).to_string(),
                    metadata: None,
                    text: (*id).to_string(),
                    phase: None,
                    memory_citation: None,
                    delivery: None,
                },
            })
            .collect(),
        next_cursor: next_cursor.map(str::to_string),
        backwards_cursor: None,
    }
}

#[test]
fn thread_items_page_params_use_codex_descending_shape() {
    let params = thread_items_page_params(
        "thread-1",
        None,
        Some("cursor-1".to_string()),
        HISTORY_ITEM_PAGE_LIMIT,
    );
    assert_eq!(params.thread_id, "thread-1");
    assert_eq!(params.turn_id, None);
    assert_eq!(params.cursor.as_deref(), Some("cursor-1"));
    assert_eq!(params.limit, Some(HISTORY_ITEM_PAGE_LIMIT));
    assert_eq!(params.sort_direction, Some(SortDirection::Desc));
}

#[test]
fn begin_and_apply_page_advance_cursor_and_restore_order() {
    let mut state = ThreadHistoryPagination {
        next_item_cursor: Some("head".to_string()),
        ..ThreadHistoryPagination::default()
    };
    state.loading_older = true;
    state.next_item_cursor = advancing_cursor(
        state.next_item_cursor.as_deref(),
        Some("tail".to_string()),
        &mut state.seen_item_cursors,
    );
    state.loading_older = false;
    let items = page(&["newer", "older"], None)
        .data
        .into_iter()
        .map(|entry| entry.item)
        .rev()
        .collect::<Vec<_>>();
    let ids = items
        .into_iter()
        .map(|item| match item {
            ThreadItem::AgentMessage { id, .. } => id,
            other => panic!("unexpected test item: {other:?}"),
        })
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["older", "newer"]);
    assert_eq!(state.next_item_cursor.as_deref(), Some("tail"));
    assert!(!state.loading_older);
}

#[test]
fn advancing_cursor_rejects_repeated_cursors() {
    let mut seen_cursors = std::collections::HashSet::new();
    assert_eq!(
        advancing_cursor(
            /*current*/ None,
            Some("first".to_string()),
            &mut seen_cursors,
        ),
        Some("first".to_string())
    );
    assert_eq!(
        advancing_cursor(Some("first"), Some("second".to_string()), &mut seen_cursors,),
        Some("second".to_string())
    );
    assert_eq!(
        advancing_cursor(Some("second"), Some("first".to_string()), &mut seen_cursors,),
        None
    );
    assert_eq!(
        advancing_cursor(Some("second"), /*next*/ None, &mut seen_cursors),
        None
    );
}

fn test_turn(id: &str) -> Turn {
    Turn {
        id: id.to_string(),
        items: Vec::new(),
        items_view: TurnItemsView::NotLoaded,
        status: TurnStatus::Completed,
        error: None,
        started_at: Some(1),
        completed_at: Some(2),
        duration_ms: Some(1),
    }
}

#[test]
fn turn_lookup_requires_one_older_context_turn_after_targets() {
    let turns_desc = vec![
        test_turn("newest"),
        test_turn("target"),
        test_turn("previous"),
    ];
    let targets = std::collections::HashSet::from([String::from("target")]);
    assert!(all_target_turns_loaded(&turns_desc, &targets));
    assert!(has_older_turn_context(&turns_desc, &targets));
}

#[test]
fn turn_lookup_keeps_paging_when_target_is_oldest_loaded_turn() {
    let turns_desc = vec![test_turn("newest"), test_turn("target")];
    let targets = std::collections::HashSet::from([String::from("target")]);
    assert!(all_target_turns_loaded(&turns_desc, &targets));
    assert!(!has_older_turn_context(&turns_desc, &targets));
}

#[test]
fn turn_lookup_returns_chronological_order_for_history_filtering() {
    let turns_desc = vec![test_turn("newest"), test_turn("older"), test_turn("oldest")];
    let ids = chronological_turns(turns_desc)
        .into_iter()
        .map(|turn| turn.id)
        .collect::<Vec<_>>();
    assert_eq!(ids, vec!["oldest", "older", "newest"]);
}
