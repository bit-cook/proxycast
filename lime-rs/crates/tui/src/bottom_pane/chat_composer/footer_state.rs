//! Footer presentation state owned by the chat composer.
//!
//! This mirrors Codex's owner boundary: transient footer state lives beside composer input, while
//! the app view only renders the selected mode and text.

use std::time::Instant;

use ratatui::text::Line;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum FooterMode {
    #[default]
    ComposerEmpty,
    ComposerHasDraft,
    HistorySearch,
}

#[derive(Clone, Debug)]
pub(super) struct FooterFlash {
    pub(super) line: Line<'static>,
    pub(super) expires_at: Instant,
}

#[derive(Debug)]
pub(super) struct FooterState {
    pub(super) mode: FooterMode,
    pub(super) flash: Option<FooterFlash>,
}

impl Default for FooterState {
    fn default() -> Self {
        Self {
            mode: FooterMode::ComposerEmpty,
            flash: None,
        }
    }
}

impl FooterState {
    pub(super) fn flash_line(&self) -> Option<&Line<'static>> {
        self.flash
            .as_ref()
            .filter(|flash| Instant::now() < flash.expires_at)
            .map(|flash| &flash.line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::text::Line;

    #[test]
    fn flash_state_expires_without_mutating_mode() {
        let state = FooterState {
            flash: Some(FooterFlash {
                line: Line::raw("saved"),
                expires_at: Instant::now() + std::time::Duration::from_secs(1),
            }),
            ..FooterState::default()
        };
        assert!(state.flash_line().is_some());
        assert_eq!(state.mode, FooterMode::ComposerEmpty);
    }
}
