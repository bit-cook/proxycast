//! Load older transcript pages without creating a TUI-local history store.

use anyhow::Result;
use app_server_protocol::protocol::v2::Turn;
use std::collections::HashSet;

use super::App;
use crate::app_server_session::{AppServerSession, InitialHistoryPage, HISTORY_ITEM_PAGE_LIMIT};
use crate::history_filter::hidden_user_message_ids;

#[path = "history_completion.rs"]
pub(crate) mod completion;

impl App {
    /// Prepend the initial paginated page using the same optional Turn enrichment as older pages.
    pub(crate) fn prepend_initial_history_page(&mut self, page: InitialHistoryPage) {
        if let Some(turns) = page.turns {
            let hidden_ids = hidden_user_message_ids(&turns);
            let groups = completion::group_completed_turn_items(page.items, &turns);
            self.projection
                .prepend_grouped_items_with_hidden_ids(groups, &hidden_ids);
        } else {
            self.projection.prepend_items(page.items);
        }
    }

    /// Load one bounded page shared by terminal scrollback and the transcript overlay.
    pub(crate) async fn request_older_history_page(
        &mut self,
        app_server: &mut AppServerSession,
    ) -> Result<bool> {
        let Some(thread_id) = self.thread_id.clone() else {
            return Ok(false);
        };
        let Some(cursor) = app_server.begin_older_history_page(&thread_id) else {
            return Ok(false);
        };
        let result = app_server
            .thread_items_page(
                thread_id.clone(),
                Some(cursor.clone()),
                HISTORY_ITEM_PAGE_LIMIT,
            )
            .await;
        if self.thread_id.as_deref() != Some(thread_id.as_str()) {
            app_server.cancel_older_history_page(&thread_id);
            return Ok(false);
        }
        // Turn metadata is optional for older App Servers. When available it lets the
        // projection place completion separators at the exact last item of each completed turn.
        let turn_ids = result.as_ref().ok().map(|page| {
            page.data
                .iter()
                .map(|entry| entry.turn_id.clone())
                .collect::<HashSet<_>>()
        });
        let turns = match turn_ids {
            Some(turn_ids) if !turn_ids.is_empty() => app_server
                .thread_turns_for_items(thread_id.clone(), &turn_ids)
                .await
                .ok(),
            _ => None,
        };
        self.handle_older_history_page_with_turns(
            app_server,
            &thread_id,
            &cursor,
            result,
            turns.as_deref(),
        )
    }

    #[allow(dead_code)]
    pub(crate) fn handle_older_history_page(
        &mut self,
        app_server: &mut AppServerSession,
        thread_id: &str,
        cursor: &str,
        result: Result<app_server_protocol::protocol::v2::ThreadItemsListResponse>,
    ) -> Result<bool> {
        self.handle_older_history_page_with_turns(app_server, thread_id, cursor, result, None)
    }

    fn handle_older_history_page_with_turns(
        &mut self,
        app_server: &mut AppServerSession,
        thread_id: &str,
        cursor: &str,
        result: Result<app_server_protocol::protocol::v2::ThreadItemsListResponse>,
        turns: Option<&[Turn]>,
    ) -> Result<bool> {
        if self.thread_id.as_deref() != Some(thread_id) {
            app_server.cancel_older_history_page(thread_id);
            return Ok(false);
        }
        let page = match result {
            Ok(page) => page,
            Err(error) => {
                app_server.cancel_older_history_page(thread_id);
                return Err(error);
            }
        };
        let items = app_server.apply_older_history_page(thread_id, cursor, page)?;
        if let Some(turns) = turns {
            let hidden_ids = hidden_user_message_ids(turns);
            self.projection.remove_hidden_entries(&hidden_ids);
            let groups = completion::group_completed_turn_items(items, turns);
            self.projection
                .prepend_grouped_items_with_hidden_ids(groups, &hidden_ids);
        } else {
            self.projection.prepend_items(items);
        }
        self.scrollback_has_older_history = app_server.has_older_history(thread_id);
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_server_protocol::protocol::v2::{TurnItemsView, TurnStatus, UserInput};

    fn user_message(id: &str, text: &str) -> app_server_protocol::protocol::v2::ThreadItem {
        app_server_protocol::protocol::v2::ThreadItem::UserMessage {
            id: id.to_string(),
            metadata: None,
            client_id: None,
            content: vec![UserInput::Text {
                text: text.to_string(),
                text_elements: Vec::new(),
            }],
        }
    }

    fn answer(id: &str, text: &str) -> app_server_protocol::protocol::v2::ThreadItem {
        app_server_protocol::protocol::v2::ThreadItem::AgentMessage {
            id: id.to_string(),
            metadata: None,
            text: text.to_string(),
            phase: None,
            memory_citation: None,
            delivery: None,
        }
    }

    #[test]
    fn initial_history_page_uses_turn_metadata_for_filtering_and_completion() {
        let items = vec![
            app_server_protocol::protocol::v2::ThreadItem::EnteredReviewMode {
                id: "review-enter".to_string(),
                metadata: None,
                review: "review".to_string(),
            },
            user_message("review-prompt", "hidden"),
            app_server_protocol::protocol::v2::ThreadItem::ExitedReviewMode {
                id: "review-exit".to_string(),
                metadata: None,
                review: "review".to_string(),
            },
            answer("answer", "done"),
        ];
        let turn = Turn {
            id: "turn-1".to_string(),
            items: items.clone(),
            items_view: TurnItemsView::Full,
            status: TurnStatus::Completed,
            error: None,
            started_at: Some(1),
            completed_at: Some(4),
            duration_ms: Some(3_000),
        };
        let mut app = App::default();
        app.prepend_initial_history_page(InitialHistoryPage {
            items,
            turns: Some(vec![turn]),
        });

        assert_eq!(
            app.projection
                .entries()
                .iter()
                .map(|entry| entry.id.as_str())
                .collect::<Vec<_>>(),
            vec!["review-enter", "review-exit", "answer"]
        );
        assert_eq!(
            app.projection
                .completion_after("answer")
                .and_then(|boundary| boundary.elapsed_seconds),
            Some(3)
        );
    }

    #[test]
    fn initial_history_page_without_turn_metadata_stays_item_only() {
        let items = vec![user_message("prompt", "visible"), answer("answer", "done")];
        let mut app = App::default();
        app.prepend_initial_history_page(InitialHistoryPage { items, turns: None });

        assert_eq!(app.projection.entries().len(), 2);
        assert!(app.projection.completion_after("answer").is_none());
    }
}
