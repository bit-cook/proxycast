//! Pause-aware elapsed time for the live status indicator.
//!
//! The timer is intentionally independent from turn accounting. App/runtime may pause the
//! display while an approval or user-input request is visible without changing canonical turn
//! state.

use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub(crate) struct StatusTimer {
    pub(crate) display_started_at: Option<Instant>,
    elapsed_running: Duration,
    last_resume_at: Instant,
    is_paused: bool,
}

impl Default for StatusTimer {
    fn default() -> Self {
        Self {
            display_started_at: None,
            elapsed_running: Duration::ZERO,
            last_resume_at: Instant::now(),
            is_paused: false,
        }
    }
}

impl StatusTimer {
    pub(crate) fn reset(&mut self, elapsed: Duration) {
        self.elapsed_running = elapsed;
        self.last_resume_at = Instant::now();
    }

    pub(crate) fn pause_at(&mut self, now: Instant) {
        if !self.is_paused {
            self.elapsed_running += now.saturating_duration_since(self.last_resume_at);
            self.is_paused = true;
        }
    }

    pub(crate) fn resume_at(&mut self, now: Instant) {
        if self.is_paused {
            self.last_resume_at = now;
            self.is_paused = false;
        }
    }

    pub(crate) fn elapsed_at(&self, now: Instant) -> Duration {
        self.elapsed_running
            + if self.is_paused {
                Duration::ZERO
            } else {
                now.saturating_duration_since(self.last_resume_at)
            }
    }
}

#[cfg(test)]
#[path = "timer_tests.rs"]
mod tests;
