//! Popup lifecycle state for the chat composer.
//!
//! The composer owns popup visibility and dismissal.  `App` only routes events and renders the
//! current projection, which keeps slash completion state from being duplicated across hosts.

use super::super::command_popup::CommandPopup;
use super::file_search_popup::FileSearchPopup;
use super::skill_popup::SkillPopup;
use std::ops::Range;

/// One token occurrence whose autocomplete popup should remain hidden.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct DismissedToken {
    query: String,
    token: String,
    occurrence: usize,
}

impl DismissedToken {
    pub(super) fn new(text: &str, range: Range<usize>, query: String) -> Self {
        let token = text[range.clone()].to_string();
        let occurrence = token_occurrences_before(text, &token, range.start);
        Self {
            query,
            token,
            occurrence,
        }
    }

    pub(super) fn matches(&self, text: &str, range: &Range<usize>, query: &str) -> bool {
        self.query == query
            && text.get(range.clone()) == Some(self.token.as_str())
            && token_occurrences_before(text, &self.token, range.start) == self.occurrence
    }
}

fn token_occurrences_before(text: &str, token: &str, before: usize) -> usize {
    text[..before]
        .match_indices(token)
        .filter(|(start, _)| {
            let end = *start + token.len();
            let starts_at_boundary = *start == 0
                || text[..*start]
                    .chars()
                    .next_back()
                    .is_some_and(char::is_whitespace);
            let ends_at_boundary = text[end..].chars().next().is_none_or(char::is_whitespace);
            starts_at_boundary && ends_at_boundary
        })
        .count()
}

/// At most one composer popup is active at a time.
#[derive(Debug, Default)]
pub(super) enum ActivePopup {
    #[default]
    None,
    Command(CommandPopup),
    File(FileSearchPopup),
    Skill(SkillPopup),
}

#[derive(Debug, Default)]
pub(super) struct PopupState {
    pub(super) active: ActivePopup,
    pub(super) dismissed_command_token: Option<String>,
    pub(super) dismissed_file_token: Option<DismissedToken>,
    pub(super) dismissed_skill_token: Option<DismissedToken>,
    pub(super) file_search_requested_query: Option<String>,
}

impl ActivePopup {
    pub(super) fn as_command_mut(&mut self) -> Option<&mut CommandPopup> {
        match self {
            Self::Command(popup) => Some(popup),
            Self::File(_) | Self::None => None,
            Self::Skill(_) => None,
        }
    }
}

impl PopupState {
    pub(super) fn active(&self) -> bool {
        !matches!(self.active, ActivePopup::None)
    }

    pub(super) fn dismiss_command(&mut self, token: impl Into<String>) {
        self.dismissed_command_token = Some(token.into());
        self.active = ActivePopup::None;
    }

    pub(super) fn clear_dismissal(&mut self) {
        self.dismissed_command_token = None;
    }

    pub(super) fn dismissed_for(&self, token: Option<&str>) -> bool {
        self.dismissed_command_token.as_deref() == token
    }
}

#[cfg(test)]
#[path = "popup_state_tests.rs"]
mod tests;
