//! Agent turn lifecycle state for the TUI application.
//!
//! This owner mirrors Codex's turn lifecycle bookkeeping while keeping transport and canonical
//! Thread/Turn/Item projection ownership in the existing App Server path.

use std::collections::HashSet;
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
pub(super) struct TurnLifecycleState {
    /// Tracks whether the canonical projection currently has an active agent turn.
    pub(super) agent_turn_running: bool,
    pub(super) last_turn_id: Option<String>,
    pub(super) budget_limited_turn_ids: HashSet<String>,
    /// Completion labels already inserted into this thread's visible history.
    #[allow(dead_code)]
    pub(super) rendered_completion_turn_ids: HashSet<String>,
    pub(super) active_turn_started_at: Option<Instant>,
}

impl TurnLifecycleState {
    pub(super) fn start(&mut self, turn_id: String, now: Instant) {
        self.agent_turn_running = true;
        self.last_turn_id = Some(turn_id);
        self.active_turn_started_at = Some(now);
    }

    pub(super) fn finish(&mut self) {
        self.agent_turn_running = false;
        self.active_turn_started_at = None;
    }

    pub(super) fn restore_running(&mut self, turn_id: Option<&str>, now: Instant) {
        self.last_turn_id = turn_id.map(str::to_owned);
        self.agent_turn_running = turn_id.is_some();
        self.active_turn_started_at = turn_id.map(|_| now);
    }

    pub(super) fn sync_projection_turn(
        &mut self,
        previous_turn_id: Option<&str>,
        active_turn_id: Option<&str>,
        now: Instant,
    ) {
        if active_turn_id == previous_turn_id {
            return;
        }
        match active_turn_id {
            Some(turn_id) => self.start(turn_id.to_owned(), now),
            None => self.finish(),
        }
    }

    pub(super) fn reset_thread(&mut self) {
        self.finish();
        self.last_turn_id = None;
        self.budget_limited_turn_ids.clear();
        self.rendered_completion_turn_ids.clear();
    }

    pub(super) fn elapsed(&self, active_turn_id: Option<&str>, now: Instant) -> Option<Duration> {
        active_turn_id?;
        Some(
            self.active_turn_started_at
                .map(|started_at| now.saturating_duration_since(started_at))
                .unwrap_or_default(),
        )
    }

    #[allow(dead_code)]
    pub(super) fn mark_budget_limited(&mut self, turn_id: String) {
        self.budget_limited_turn_ids.insert(turn_id);
    }

    #[allow(dead_code)]
    pub(super) fn take_budget_limited(&mut self, turn_id: &str) -> bool {
        self.budget_limited_turn_ids.remove(turn_id)
    }
}

impl super::App {
    pub(crate) fn start_turn(&mut self, turn_id: String) {
        self.projection.start_turn(turn_id.clone());
        self.turn_lifecycle.start(turn_id, Instant::now());
    }

    pub(crate) fn active_turn_elapsed(&self, now: Instant) -> Option<Duration> {
        self.turn_lifecycle
            .elapsed(self.projection.active_turn_id(), now)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_and_finish_update_running_state() {
        let now = Instant::now();
        let mut state = TurnLifecycleState::default();

        state.start("turn-1".to_string(), now);
        assert!(state.agent_turn_running);
        assert_eq!(state.last_turn_id.as_deref(), Some("turn-1"));
        assert!(state.active_turn_started_at.is_some());

        state.finish();
        assert!(!state.agent_turn_running);
        assert!(state.active_turn_started_at.is_none());
    }

    #[test]
    fn projection_transition_starts_and_finishes_timer() {
        let now = Instant::now();
        let mut state = TurnLifecycleState::default();

        state.sync_projection_turn(None, Some("turn-1"), now);
        assert!(state.elapsed(Some("turn-1"), now).is_some());

        state.sync_projection_turn(Some("turn-1"), None, now);
        assert!(state.elapsed(None, now).is_none());
    }

    #[test]
    fn budget_limited_turn_ids_are_consumed() {
        let mut state = TurnLifecycleState::default();

        state.mark_budget_limited("turn-1".to_string());

        assert!(state.take_budget_limited("turn-1"));
        assert!(!state.take_budget_limited("turn-1"));
    }
}
