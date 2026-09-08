//! App Server thread refresh for Agents Overview.

use super::App;
use crate::app_server_session::AppServerSession;
use anyhow::{bail, Context, Result};
use app_server_protocol::protocol::v2::{
    ServerNotification, SortDirection, ThreadListParams, ThreadSortKey, ThreadStatus,
};

impl App {
    #[allow(dead_code)]
    pub(crate) fn track_agents_overview_notification(&mut self, notification: &ServerNotification) {
        let primary = self.primary_thread_id.clone();
        let Some(overview) = self.agents_overview.as_mut() else {
            return;
        };
        let thread_id = match notification {
            ServerNotification::ThreadStarted(params) => Some(params.thread.id.as_str()),
            ServerNotification::ThreadArchived(params) => Some(params.thread_id.as_str()),
            ServerNotification::ThreadDeleted(params) => Some(params.thread_id.as_str()),
            ServerNotification::ThreadClosed(params) => Some(params.thread_id.as_str()),
            ServerNotification::ThreadStatusChanged(params) => Some(params.thread_id.as_str()),
            ServerNotification::ThreadNameUpdated(params) => Some(params.thread_id.as_str()),
            _ => None,
        };
        match notification {
            ServerNotification::ThreadStarted(params) if !params.thread.ephemeral => {
                if let Some(existing) = overview
                    .threads
                    .iter_mut()
                    .find(|thread| thread.id == params.thread.id)
                {
                    *existing = params.thread.clone();
                } else {
                    overview.threads.push(params.thread.clone());
                }
            }
            ServerNotification::ThreadArchived(params) => {
                overview
                    .threads
                    .retain(|thread| thread.id != params.thread_id);
            }
            ServerNotification::ThreadDeleted(params) => {
                overview
                    .threads
                    .retain(|thread| thread.id != params.thread_id);
            }
            ServerNotification::ThreadClosed(params) => {
                if let Some(thread) = overview
                    .threads
                    .iter_mut()
                    .find(|thread| thread.id == params.thread_id)
                {
                    thread.status = ThreadStatus::NotLoaded;
                }
            }
            ServerNotification::ThreadStatusChanged(params) => {
                if let Some(thread) = overview
                    .threads
                    .iter_mut()
                    .find(|thread| thread.id == params.thread_id)
                {
                    thread.status = params.status.clone();
                }
            }
            ServerNotification::ThreadNameUpdated(params) => {
                if let Some(thread) = overview
                    .threads
                    .iter_mut()
                    .find(|thread| thread.id == params.thread_id)
                {
                    thread.name = params.thread_name.clone();
                }
            }
            _ => return,
        }
        if let Some(thread_id) = thread_id {
            overview.refresh_thread_ids.insert(thread_id.to_string());
            if overview.refreshing {
                let pending = overview
                    .refresh_notifications
                    .entry(thread_id.to_string())
                    .or_default();
                pending.retain(|previous| {
                    std::mem::discriminant(previous) != std::mem::discriminant(notification)
                });
                pending.push(notification.clone());
            }
        }
        overview
            .view
            .update_rows(super::agents_overview::build_rows(
                &overview.threads,
                primary.as_deref(),
            ));
        overview.visible_thread_ids = overview
            .view
            .visible_rows()
            .into_iter()
            .map(|row| row.thread.id.clone())
            .collect();
        overview.sync_view_state();
    }

    #[allow(dead_code)]
    pub(crate) async fn refresh_changed_agents_overview_threads(
        &mut self,
        app_server: &AppServerSession,
    ) -> Result<()> {
        self.refresh_agents_overview_threads(app_server).await
    }

    #[allow(dead_code)]
    pub(crate) async fn start_agents_overview_refresh(
        &mut self,
        app_server: &AppServerSession,
    ) -> Result<()> {
        self.refresh_agents_overview_threads(app_server).await
    }

    pub(crate) async fn refresh_agents_overview_threads(
        &mut self,
        app_server: &AppServerSession,
    ) -> Result<()> {
        let Some(overview) = self.agents_overview.as_mut() else {
            return Ok(());
        };
        overview
            .refresh_thread_ids
            .extend(overview.threads.iter().map(|thread| thread.id.clone()));
        if overview.refreshing {
            overview.refresh_pending = true;
            return Ok(());
        }
        let generation = overview.begin_refresh();
        let result = async {
            let mut cursor = None;
            let mut seen = std::collections::HashSet::new();
            let mut threads = Vec::new();
            for _ in 0..16 {
                let page = app_server
                    .thread_list(ThreadListParams {
                        cursor,
                        limit: Some(100),
                        sort_key: Some(ThreadSortKey::RecencyAt),
                        sort_direction: Some(SortDirection::Desc),
                        archived: Some(false),
                        ..ThreadListParams::default()
                    })
                    .await
                    .context("failed to refresh agents overview threads")?;
                threads.extend(page.data);
                let Some(next_cursor) = page.next_cursor else {
                    break;
                };
                if !seen.insert(next_cursor.clone()) {
                    bail!("agents overview thread list repeated cursor {next_cursor}");
                }
                cursor = Some(next_cursor);
            }
            Ok::<_, anyhow::Error>(threads)
        }
        .await;
        match result {
            Ok(threads) => {
                let primary = self.primary_thread_id.clone();
                if let Some(overview) = self.agents_overview.as_mut() {
                    overview.apply_refresh(generation, threads, primary.as_deref());
                }
                let buffered = self
                    .agents_overview
                    .as_mut()
                    .map(|overview| {
                        overview
                            .refresh_notifications
                            .drain()
                            .flat_map(|(_, notifications)| notifications)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                for notification in buffered {
                    self.track_agents_overview_notification(&notification);
                }
                let refresh_pending = self
                    .agents_overview
                    .as_mut()
                    .is_some_and(|overview| overview.take_refresh_pending());
                self.projection.set_status("agents overview refreshed");
                if refresh_pending {
                    Box::pin(self.refresh_agents_overview_threads(app_server)).await?;
                }
                Ok(())
            }
            Err(error) => {
                if let Some(overview) = self.agents_overview.as_mut() {
                    overview.refreshing = false;
                    overview.request_id = None;
                }
                self.projection
                    .set_status(format!("agents overview unavailable: {error}"));
                Err(error)
            }
        }
    }
}
