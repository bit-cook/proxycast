//! Load older transcript pages without creating a TUI-local history store.

use anyhow::Result;

use super::App;
use crate::app_server_session::{AppServerSession, HISTORY_ITEM_PAGE_LIMIT};

impl App {
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
        self.handle_older_history_page(app_server, &thread_id, &cursor, result)
    }

    pub(crate) fn handle_older_history_page(
        &mut self,
        app_server: &mut AppServerSession,
        thread_id: &str,
        cursor: &str,
        result: Result<app_server_protocol::protocol::v2::ThreadItemsListResponse>,
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
        self.projection.prepend_items(items);
        self.scrollback_has_older_history = app_server.has_older_history(thread_id);
        Ok(true)
    }
}
