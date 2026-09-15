//! Submission lowering at the TUI boundary.
//!
//! This module owns the Lime subset of Codex `chatwidget/input_submission`:
//! composer results become app actions, canonical queued submissions can be
//! edited without losing attachments, and the composer owns attachment state.
//! Transport execution remains in the runtime and App Server session.

use super::*;
use crate::bottom_pane::pending_input_preview::can_restore_submission;
use crate::bottom_pane::InputResult;
use app_server_protocol::protocol::v2::UserInput;

impl App {
    pub(crate) fn attach_image(&mut self, path: PathBuf) {
        self.composer.attach_image(path);
    }

    pub(crate) fn take_pending_images(&mut self) -> Vec<PathBuf> {
        self.composer.take_pending_images()
    }

    pub(crate) fn restore_pending_images(&mut self, images: Vec<PathBuf>) {
        self.composer.restore_pending_images(images);
    }

    pub(crate) fn take_remote_image_urls(&mut self) -> Vec<String> {
        self.composer.take_remote_image_urls()
    }

    pub(crate) fn set_remote_image_urls(&mut self, urls: Vec<String>) {
        self.composer.set_remote_image_urls(urls);
    }

    pub(crate) fn set_queued_submissions(&mut self, submissions: Vec<QueuedSubmission>) {
        self.queued_submissions = submissions;
    }

    pub(crate) fn upsert_queued_submission(&mut self, submission: QueuedSubmission) {
        if let Some(existing) = self
            .queued_submissions
            .iter_mut()
            .find(|existing| existing.id == submission.id)
        {
            *existing = submission;
        } else {
            self.queued_submissions.push(submission);
        }
    }

    pub(crate) fn restore_queued_submission_for_edit(
        &mut self,
        submission: QueuedSubmission,
    ) -> bool {
        if !self.composer.is_empty()
            || self.composer.has_pending_images()
            || !can_restore_submission(&submission)
        {
            return false;
        }
        let submission_id = submission.id.clone();
        let mut text = String::new();
        let mut local_images = Vec::new();
        let mut remote_images = Vec::new();
        let mut skills = Vec::new();
        for input in submission.input {
            match input {
                UserInput::Text { text: value, .. } => text = value,
                UserInput::LocalImage { path, .. } => local_images.push(PathBuf::from(path)),
                UserInput::Image { url, .. } => remote_images.push(url),
                UserInput::Skill { name, .. } => skills.push(format!("${name}")),
                _ => return false,
            }
        }
        self.queued_submissions
            .retain(|queued| queued.id != submission_id);
        if !skills.is_empty() {
            let prefix = skills.join(" ");
            text = if text.is_empty() {
                prefix
            } else {
                format!("{prefix} {text}")
            };
        }
        self.replace_composer(text);
        self.composer.restore_pending_images(local_images);
        self.composer.set_remote_image_urls(remote_images);
        self.clear_command_popup();
        true
    }

    pub(super) fn map_composer_action(&mut self, action: InputResult) -> AppAction {
        match action {
            InputResult::Submitted(text) => {
                self.clear_command_popup();
                AppAction::Submit(text)
            }
            InputResult::Queued(text) => {
                self.clear_command_popup();
                AppAction::Queue(text)
            }
            InputResult::Interrupt => {
                let cleared = self.composer.clear_for_ctrl_c().is_some();
                if cleared {
                    self.clear_command_popup();
                    // Codex treats Ctrl-C as composer cancellation when a draft is present.
                    // Do not also interrupt the active turn: a follow-up Ctrl-C can then be
                    // handled after the terminal projection settles.
                    AppAction::None
                } else {
                    AppAction::Interrupt
                }
            }
            InputResult::DecreaseEffort => AppAction::DecreaseEffort,
            InputResult::IncreaseEffort => AppAction::IncreaseEffort,
            InputResult::PreviousPermissions => AppAction::PreviousPermissions,
            InputResult::NextPermissions => AppAction::NextPermissions,
            InputResult::OpenExternalEditor => {
                self.request_external_editor_launch();
                AppAction::None
            }
            InputResult::OpenAgentsOverview => {
                self.open_agents_overview();
                AppAction::RefreshAgentsOverview
            }
            InputResult::Quit => AppAction::Quit,
            InputResult::Changed => {
                if self.composer.history_search_active() || self.composer.vim_search_active() {
                    self.clear_command_popup();
                } else {
                    self.sync_command_popup();
                }
                AppAction::None
            }
            InputResult::None => AppAction::None,
        }
    }
}
