//! Bounded App Server transcript loading for resume and scrollback views.

use std::collections::HashSet;

use anyhow::{Context, Result};
use app_server_protocol::protocol::v2::{
    SortDirection, ThreadItem, ThreadItemsListParams, ThreadItemsListResponse,
    METHOD_THREAD_ITEMS_LIST,
};

use super::AppServerSession;

pub(crate) const HISTORY_ITEM_PAGE_LIMIT: u32 = 100;

pub(crate) fn thread_items_page_params(
    thread_id: impl Into<String>,
    turn_id: Option<&str>,
    cursor: Option<String>,
    limit: u32,
) -> ThreadItemsListParams {
    ThreadItemsListParams {
        thread_id: thread_id.into(),
        turn_id: turn_id.map(str::to_string),
        cursor,
        limit: Some(limit),
        sort_direction: Some(SortDirection::Desc),
    }
}

fn advancing_cursor(
    current: Option<&str>,
    next: Option<String>,
    seen_cursors: &mut HashSet<String>,
) -> Option<String> {
    if let Some(current) = current {
        seen_cursors.insert(current.to_string());
    }
    next.filter(|next| seen_cursors.insert(next.clone()))
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ThreadHistoryPagination {
    next_item_cursor: Option<String>,
    seen_item_cursors: HashSet<String>,
    loading_older: bool,
}

impl AppServerSession {
    pub(crate) async fn hydrate_initial_thread_history(
        &mut self,
        thread_id: impl Into<String>,
        item_cursor: Option<String>,
    ) -> Result<Vec<ThreadItem>> {
        let thread_id = thread_id.into();
        let page = self
            .thread_items_page(thread_id.clone(), item_cursor, HISTORY_ITEM_PAGE_LIMIT)
            .await?;
        let next_item_cursor = page.next_cursor.clone();
        let items = page
            .data
            .into_iter()
            .map(|entry| entry.item)
            .rev()
            .collect();
        self.initialize_history_pagination(thread_id, next_item_cursor);
        Ok(items)
    }

    pub(crate) fn initialize_history_pagination(
        &mut self,
        thread_id: impl Into<String>,
        next_item_cursor: Option<String>,
    ) {
        self.history_pagination.insert(
            thread_id.into(),
            ThreadHistoryPagination {
                next_item_cursor,
                ..ThreadHistoryPagination::default()
            },
        );
    }

    pub(crate) fn has_older_history(&self, thread_id: &str) -> bool {
        self.history_pagination
            .get(thread_id)
            .is_some_and(|page| page.next_item_cursor.is_some())
    }

    pub(crate) fn begin_older_history_page(&mut self, thread_id: &str) -> Option<String> {
        let page = self.history_pagination.get_mut(thread_id)?;
        if page.loading_older {
            return None;
        }
        let cursor = page.next_item_cursor.clone()?;
        page.loading_older = true;
        Some(cursor)
    }

    pub(crate) fn cancel_older_history_page(&mut self, thread_id: &str) {
        if let Some(page) = self.history_pagination.get_mut(thread_id) {
            page.loading_older = false;
        }
    }

    pub(crate) fn apply_older_history_page(
        &mut self,
        thread_id: &str,
        cursor: &str,
        page: ThreadItemsListResponse,
    ) -> Result<Vec<ThreadItem>> {
        let Some(state) = self.history_pagination.get_mut(thread_id) else {
            return Ok(Vec::new());
        };
        if !state.loading_older || state.next_item_cursor.as_deref() != Some(cursor) {
            return Ok(Vec::new());
        }
        state.next_item_cursor =
            advancing_cursor(Some(cursor), page.next_cursor, &mut state.seen_item_cursors);
        state.loading_older = false;
        Ok(page
            .data
            .into_iter()
            .map(|entry| entry.item)
            .rev()
            .collect())
    }

    pub(crate) async fn thread_items_page(
        &self,
        thread_id: impl Into<String>,
        cursor: Option<String>,
        limit: u32,
    ) -> Result<ThreadItemsListResponse> {
        self.request_handle
            .request(
                METHOD_THREAD_ITEMS_LIST,
                thread_items_page_params(thread_id, None, cursor, limit),
            )
            .await
            .context("failed to load App Server thread item page")
    }
}

#[cfg(test)]
#[path = "history_tests.rs"]
mod tests;
