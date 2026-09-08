//! Session and subagent selection lifecycle for the TUI app.

use super::*;
use crate::app_server_session::AppServerSession;

impl App {
    /// Resume a canonical App Server thread and refresh the local projection/settings snapshot.
    ///
    /// Thread identity, queued input and agent-overview replay all move together so callers do
    /// not accidentally switch only the transport target or only the rendered conversation.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn resume_target_session(
        &mut self,
        session: &mut AppServerSession,
        thread_id: String,
        model: &mut Option<String>,
        model_provider: &mut Option<String>,
        effort: &mut Option<String>,
        permissions: &mut Option<String>,
        options: &crate::runtime::TuiOptions,
    ) -> anyhow::Result<()> {
        if self.thread_id.as_deref() == Some(thread_id.as_str()) {
            return Ok(());
        }
        let response = session.resume_thread(thread_id).await?;
        let resumed_thread_id = response.thread.id.clone();
        let snapshot = self.take_thread_event_snapshot(&resumed_thread_id, true);
        self.bottom_pane.clear();
        self.hydrate_thread(response.thread);
        self.set_thread_id(resumed_thread_id.clone());
        self.restore_thread_input(&resumed_thread_id);
        self.replay_thread_snapshot(snapshot);
        *model = Some(response.model);
        *model_provider = Some(response.model_provider);
        *effort = response.reasoning_effort;
        if options.permissions.is_none() {
            *permissions = session.active_permission_profile().map(str::to_string);
        }
        self.set_cwd(PathBuf::from(response.cwd));
        self.set_settings(
            model.clone(),
            model_provider.clone(),
            effort.clone(),
            permissions.clone(),
        );
        self.refresh_queued_submissions(session).await;
        Ok(())
    }

    pub(super) fn open_agent_picker(&mut self) {
        let picker =
            AgentPicker::from_navigation(&self.agent_navigation, self.primary_thread_id.as_deref());
        if picker.is_empty() {
            self.projection.set_status("no sub-agents available");
        } else {
            self.agent_picker = Some(picker);
        }
    }
}
