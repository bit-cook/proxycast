use app_server_protocol::protocol::v2::{ThreadItem, ThreadItemEntry, ThreadItemsListResponse};

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
