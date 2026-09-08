use super::*;
use crate::app::agents_overview_view::AgentsOverviewGroup;
use app_server_protocol::protocol::v2::{
    AgentMessageDeltaNotification, ServerNotification, SessionSource, ThreadHistoryMode,
    ThreadNameUpdatedNotification, ThreadStatus,
};
use std::path::PathBuf;

fn thread(id: &str, parent: Option<&str>, status: ThreadStatus) -> Thread {
    Thread {
        id: id.to_string(),
        extra: None,
        session_id: id.to_string(),
        forked_from_id: None,
        parent_thread_id: parent.map(str::to_string),
        preview: String::new(),
        ephemeral: false,
        section: None,
        section_entered_at: None,
        project_id: None,
        history_mode: ThreadHistoryMode::Legacy,
        model_provider: "test".to_string(),
        created_at: 1,
        updated_at: 1,
        recency_at: Some(1),
        status,
        path: None,
        cwd: PathBuf::from("/workspace"),
        cli_version: "test".to_string(),
        source: SessionSource::Cli,
        can_accept_direct_input: Some(true),
        thread_source: None,
        agent_nickname: None,
        agent_role: None,
        git_info: None,
        name: Some(id.to_string()),
        turns: Vec::new(),
    }
}

#[test]
fn agents_overview_group_bubbles_child_status_and_hides_ephemeral_threads() {
    let mut root = thread("root", None, ThreadStatus::Idle);
    let child = thread(
        "child",
        Some("root"),
        ThreadStatus::Active {
            active_flags: Vec::new(),
        },
    );
    let mut ephemeral = thread("ephemeral", None, ThreadStatus::Idle);
    ephemeral.ephemeral = true;
    let rows = build_rows(&[root.clone(), child, ephemeral], Some("root"));
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].group, AgentsOverviewGroup::Working);
    root.status = ThreadStatus::SystemError;
    assert_eq!(
        build_rows(&[root], None)[0].group,
        AgentsOverviewGroup::NeedsYou
    );
}

#[test]
fn agents_overview_refresh_ignores_stale_generation() {
    let mut state = AgentsOverviewState::new(None);
    let first = state.begin_refresh();
    state.refreshing = false;
    state.request_id = None;
    let second = state.begin_refresh();
    assert!(!state.apply_refresh(first, Vec::new(), None));
    assert!(state.apply_refresh(second, Vec::new(), None));
}

#[test]
fn agents_overview_refresh_retains_locally_observed_threads() {
    let mut state = AgentsOverviewState::new(None);
    state.replace_threads(vec![thread("local", None, ThreadStatus::Idle)], None);
    state.replace_threads(vec![thread("recent", None, ThreadStatus::Idle)], None);
    let ids = state
        .threads
        .iter()
        .map(|thread| thread.id.as_str())
        .collect::<Vec<_>>();
    assert!(ids.contains(&"local"));
    assert!(ids.contains(&"recent"));
}

#[test]
fn agents_overview_refresh_coalesces_requests_and_exposes_codex_state_fields() {
    let mut state = AgentsOverviewState::new(None);
    assert!(!state.initialized);
    assert!(state.request_id.is_none());
    let first = state.begin_refresh();
    assert_eq!(state.request_id, Some(first));
    assert!(state.refreshing);
    let second = state.begin_refresh();
    assert!(state.refresh_pending);
    assert_eq!(first, second);
    assert!(state.take_refresh_pending());
    assert!(!state.take_refresh_pending());
}

#[test]
fn agents_overview_buffers_latest_notification_during_refresh() {
    let mut app = super::super::App::default();
    let mut state = AgentsOverviewState::new(None);
    state.replace_threads(vec![thread("background", None, ThreadStatus::Idle)], None);
    state.refreshing = true;
    app.agents_overview = Some(state);

    let notification = ServerNotification::ThreadNameUpdated(ThreadNameUpdatedNotification {
        thread_id: "background".to_string(),
        thread_name: Some("renamed".to_string()),
    });
    app.track_agents_overview_notification(&notification);
    app.track_agents_overview_notification(&ServerNotification::ThreadNameUpdated(
        ThreadNameUpdatedNotification {
            thread_id: "background".to_string(),
            thread_name: Some("renamed-again".to_string()),
        },
    ));

    let overview = app.agents_overview.as_ref().expect("overview state");
    assert_eq!(overview.refresh_notifications["background"].len(), 1);
    assert_eq!(overview.visible_thread_ids, vec!["background"]);
    assert_eq!(
        overview
            .view_state
            .lock()
            .expect("view state")
            .visible_rows()[0]
            .thread
            .name
            .as_deref(),
        Some("renamed-again")
    );
}

#[test]
fn background_thread_notifications_do_not_mutate_the_current_projection() {
    let mut app = super::super::App::default();
    app.set_thread_id("current".to_string());
    app.agents_overview = Some(AgentsOverviewState::new(Some("current")));

    app.apply_notification(ServerNotification::AgentMessageDelta(
        AgentMessageDeltaNotification {
            thread_id: "background".to_string(),
            turn_id: "turn-background".to_string(),
            item_id: "message-background".to_string(),
            delta: "background answer".to_string(),
        },
    ));

    assert!(app.projection.entries().is_empty());
    app.apply_notification(ServerNotification::AgentMessageDelta(
        AgentMessageDeltaNotification {
            thread_id: "current".to_string(),
            turn_id: "turn-current".to_string(),
            item_id: "message-current".to_string(),
            delta: "current answer".to_string(),
        },
    ));
    assert_eq!(app.projection.final_answer(), "current answer");
}
