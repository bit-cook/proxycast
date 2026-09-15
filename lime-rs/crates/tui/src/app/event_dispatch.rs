//! Application action dispatch for the TUI.
//!
//! This is the Codex-shaped owner for actions that require App Server state. Input decoding stays
//! in `input.rs`; terminal-specific actions remain in the runtime because they need the live
//! terminal guard. The split keeps the central loop focused on event ordering and lifecycle.

use anyhow::Result;

use super::*;
use crate::app_server_session::AppServerSession;
use crate::settings::{cycle_setting, EFFORTS};

pub(crate) struct EventContext<'a> {
    pub(crate) session: &'a mut AppServerSession,
    pub(crate) model: &'a mut Option<String>,
    pub(crate) model_provider: &'a mut Option<String>,
    pub(crate) effort: &'a mut Option<String>,
    pub(crate) permissions: &'a mut Option<String>,
}

pub(crate) enum EventDispatch {
    Handled,
    Unhandled(AppAction),
}

impl App {
    /// Dispatch one App action that belongs to the App Server-backed application layer.
    pub(crate) async fn handle_event(
        &mut self,
        event: AppAction,
        context: EventContext<'_>,
    ) -> Result<EventDispatch> {
        match event {
            action @ (AppAction::DecreaseEffort | AppAction::IncreaseEffort) => {
                let direction = if matches!(action, AppAction::DecreaseEffort) {
                    -1
                } else {
                    1
                };
                let next = cycle_setting(&EFFORTS, context.effort.as_deref(), direction);
                match context
                    .session
                    .update_settings(None, None, Some(next.clone()), None)
                    .await
                {
                    Ok(()) => {
                        *context.effort = Some(next.clone());
                        self.set_settings(
                            context.model.clone(),
                            context.model_provider.clone(),
                            context.effort.clone(),
                            context.permissions.clone(),
                        );
                        self.projection.set_status(format!("effort: {next}"));
                    }
                    Err(error) => self.projection.set_status(error.to_string()),
                }
                Ok(EventDispatch::Handled)
            }
            action @ (AppAction::PreviousPermissions | AppAction::NextPermissions) => {
                let direction = if matches!(action, AppAction::PreviousPermissions) {
                    -1
                } else {
                    1
                };
                let next = self.cycle_permission_profile(context.permissions.as_deref(), direction);
                match context
                    .session
                    .update_settings(None, None, None, Some(next.clone()))
                    .await
                {
                    Ok(()) => {
                        *context.permissions = Some(next.clone());
                        self.set_settings(
                            context.model.clone(),
                            context.model_provider.clone(),
                            context.effort.clone(),
                            context.permissions.clone(),
                        );
                        self.projection.set_status(format!("permissions: {next}"));
                    }
                    Err(error) => self.projection.set_status(error.to_string()),
                }
                Ok(EventDispatch::Handled)
            }
            AppAction::Respond(response) => {
                self.note_outbound_response(&response);
                if let Err(error) = context.session.respond(response).await {
                    self.projection.set_status(error.to_string());
                }
                Ok(EventDispatch::Handled)
            }
            AppAction::SelectModel(selection) => {
                match context
                    .session
                    .update_settings(
                        Some(selection.model.clone()),
                        Some(selection.provider.clone()),
                        None,
                        None,
                    )
                    .await
                {
                    Ok(()) => {
                        *context.model = Some(selection.model);
                        *context.model_provider = Some(selection.provider);
                        self.set_settings(
                            context.model.clone(),
                            context.model_provider.clone(),
                            context.effort.clone(),
                            context.permissions.clone(),
                        );
                        self.projection.set_status("settings updated");
                    }
                    Err(error) => self.projection.set_status(error.to_string()),
                }
                Ok(EventDispatch::Handled)
            }
            AppAction::ChangeCollaborationMode(collaboration_mode) => {
                match context
                    .session
                    .update_collaboration_mode(collaboration_mode.clone())
                    .await
                {
                    Ok(()) => {
                        *context.model = Some(collaboration_mode.settings.model.clone());
                        *context.effort = collaboration_mode.settings.reasoning_effort.clone();
                        self.set_settings(
                            context.model.clone(),
                            context.model_provider.clone(),
                            context.effort.clone(),
                            context.permissions.clone(),
                        );
                        self.collaboration_mode = Some(collaboration_mode);
                        self.projection.set_status("collaboration mode updated");
                    }
                    Err(error) => self.projection.set_status(error.to_string()),
                }
                Ok(EventDispatch::Handled)
            }
            AppAction::RefreshAgentsOverview => {
                if let Err(error) = self.refresh_agents_overview_threads(context.session).await {
                    self.projection.set_status(error.to_string());
                }
                Ok(EventDispatch::Handled)
            }
            AppAction::DispatchAgentsOverviewTask { prompt, cwd } => {
                match self
                    .dispatch_agents_overview_task(context.session, prompt, cwd)
                    .await
                {
                    Ok((thread_id, turn_id)) => self
                        .projection
                        .set_status(format!("background task started: {thread_id} ({turn_id})")),
                    Err(error) => self
                        .projection
                        .set_status(format!("background task failed: {error}")),
                }
                Ok(EventDispatch::Handled)
            }
            AppAction::RenameAgentsOverviewThread { thread_id, name } => {
                match context.session.thread_set_name(thread_id, name).await {
                    Ok(()) => {
                        self.projection.set_status("agent renamed");
                        self.refresh_agents_overview_threads(context.session)
                            .await
                            .ok();
                    }
                    Err(error) => self
                        .projection
                        .set_status(format!("agent rename failed: {error}")),
                }
                Ok(EventDispatch::Handled)
            }
            AppAction::StopAgentsOverviewThread { thread_id } => {
                match self
                    .stop_agents_overview_thread(context.session, thread_id)
                    .await
                {
                    Ok(Some(turn_id)) => self
                        .projection
                        .set_status(format!("stopping background turn {turn_id}")),
                    Ok(None) => self.projection.set_status("background task is idle"),
                    Err(error) => self
                        .projection
                        .set_status(format!("background task stop failed: {error}")),
                }
                Ok(EventDispatch::Handled)
            }
            AppAction::FetchMcpInventory { detail } => {
                match context.session.list_mcp_server_statuses(detail).await {
                    Ok(statuses) => self.open_mcp_inventory(statuses, detail),
                    Err(error) => self
                        .projection
                        .set_status(format!("MCP inventory failed: {error}")),
                }
                Ok(EventDispatch::Handled)
            }
            other => Ok(EventDispatch::Unhandled(other)),
        }
    }
}
