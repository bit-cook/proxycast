//! Group paginated history items at completed turn boundaries.
//!
//! A page may split a turn. Completion metadata belongs only to the page containing that turn's
//! final item; failed, interrupted, and in-progress turns do not create a boundary.

use std::collections::HashMap;

use app_server_protocol::protocol::v2::{ThreadItem, Turn, TurnStatus};

use crate::projection::CompletionMetadata;

pub(crate) fn group_completed_turn_items(
    items: Vec<ThreadItem>,
    turns: &[Turn],
) -> Vec<(Vec<ThreadItem>, Option<CompletionMetadata>)> {
    let completed_boundaries: HashMap<_, _> = turns
        .iter()
        .filter(|turn| turn.status == TurnStatus::Completed)
        .filter_map(|turn| {
            turn.items.last().map(|item| {
                (
                    thread_item_id(item).to_string(),
                    CompletionMetadata {
                        elapsed_seconds: turn
                            .duration_ms
                            .and_then(|duration| u64::try_from(duration).ok())
                            .map(|duration| duration / 1_000),
                    },
                )
            })
        })
        .collect();

    let mut groups = Vec::new();
    let mut pending = Vec::new();
    for item in items {
        let completed_turn = completed_boundaries.get(thread_item_id(&item)).cloned();
        pending.push(item);
        if completed_turn.is_some() {
            groups.push((std::mem::take(&mut pending), completed_turn));
        }
    }
    if !pending.is_empty() {
        groups.push((pending, None));
    }
    groups
}

fn thread_item_id(item: &ThreadItem) -> &str {
    match item {
        ThreadItem::UserMessage { id, .. }
        | ThreadItem::HookPrompt { id, .. }
        | ThreadItem::AgentMessage { id, .. }
        | ThreadItem::Plan { id, .. }
        | ThreadItem::Reasoning { id, .. }
        | ThreadItem::CommandExecution { id, .. }
        | ThreadItem::FileChange { id, .. }
        | ThreadItem::McpToolCall { id, .. }
        | ThreadItem::DynamicToolCall { id, .. }
        | ThreadItem::CollabAgentToolCall { id, .. }
        | ThreadItem::SubAgentActivity { id, .. }
        | ThreadItem::ImageView { id, .. }
        | ThreadItem::EnteredReviewMode { id, .. }
        | ThreadItem::ExitedReviewMode { id, .. }
        | ThreadItem::ContextCompaction { id, .. }
        | ThreadItem::UnknownItem { id, .. } => id,
        ThreadItem::WebSearch(item) => &item.id,
        ThreadItem::Sleep(item) => &item.id,
        ThreadItem::ImageGeneration(item) => &item.id,
    }
}

#[cfg(test)]
#[path = "history_completion_tests.rs"]
mod tests;
