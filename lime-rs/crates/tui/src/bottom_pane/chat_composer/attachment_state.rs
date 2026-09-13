//! Image attachment bookkeeping for the chat composer.
//!
//! The attachment state is intentionally owned by `ChatComposer`, matching Codex's ownership
//! boundary. Runtime lowering still happens in the App Server session and keeps image parts before
//! text parts in the canonical `UserInput` vector.

use std::path::PathBuf;

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct AttachmentState {
    local_images: Vec<PathBuf>,
    remote_image_urls: Vec<String>,
    selected_remote_image_index: Option<usize>,
}

impl AttachmentState {
    pub(super) fn is_empty(&self) -> bool {
        self.local_images.is_empty() && self.remote_image_urls.is_empty()
    }

    pub(super) fn len(&self) -> usize {
        self.local_images.len() + self.remote_image_urls.len()
    }

    pub(super) fn attach(&mut self, path: PathBuf) {
        self.local_images.push(path);
    }

    pub(super) fn take(&mut self) -> Vec<PathBuf> {
        std::mem::take(&mut self.local_images)
    }

    pub(super) fn restore(&mut self, images: Vec<PathBuf>) {
        self.local_images = images;
    }

    pub(super) fn remote_image_urls(&self) -> &[String] {
        &self.remote_image_urls
    }

    pub(super) fn take_remote_image_urls(&mut self) -> Vec<String> {
        self.selected_remote_image_index = None;
        std::mem::take(&mut self.remote_image_urls)
    }

    pub(super) fn set_remote_image_urls(&mut self, urls: Vec<String>) {
        self.remote_image_urls = urls;
        self.selected_remote_image_index = None;
    }

    pub(super) fn handle_remote_key(
        &mut self,
        key: crossterm::event::KeyEvent,
        cursor: usize,
    ) -> bool {
        use crossterm::event::{KeyCode, KeyEventKind};

        if self.remote_image_urls.is_empty()
            || key.kind != KeyEventKind::Press
            || !key.modifiers.is_empty()
        {
            return false;
        }

        match key.code {
            KeyCode::Up if self.selected_remote_image_index.is_some() || cursor == 0 => {
                self.selected_remote_image_index = Some(match self.selected_remote_image_index {
                    Some(index) => index.saturating_sub(1),
                    None => self.remote_image_urls.len() - 1,
                });
                true
            }
            KeyCode::Down if self.selected_remote_image_index.is_some() => {
                let index = self.selected_remote_image_index.expect("checked above");
                if index + 1 < self.remote_image_urls.len() {
                    self.selected_remote_image_index = Some(index + 1);
                } else {
                    self.selected_remote_image_index = None;
                }
                true
            }
            KeyCode::Delete | KeyCode::Backspace if self.selected_remote_image_index.is_some() => {
                let index = self.selected_remote_image_index.expect("checked above");
                self.remote_image_urls.remove(index);
                self.selected_remote_image_index = if self.remote_image_urls.is_empty() {
                    None
                } else {
                    Some(index.min(self.remote_image_urls.len() - 1))
                };
                true
            }
            _ => {
                self.selected_remote_image_index = None;
                false
            }
        }
    }

    pub(super) fn selected_remote_image_index(&self) -> Option<usize> {
        self.selected_remote_image_index
    }

    pub(super) fn remote_image_lines(&self) -> Vec<Line<'static>> {
        self.remote_image_urls
            .iter()
            .enumerate()
            .map(|(index, _)| {
                let selected = self.selected_remote_image_index == Some(index);
                let mut style = Style::default().fg(Color::Cyan);
                if selected {
                    style = style.add_modifier(Modifier::REVERSED);
                }
                Line::styled(format!("[Image #{}]", index + 1), style)
            })
            .collect()
    }

    pub(super) fn paths(&self) -> &[PathBuf] {
        &self.local_images
    }

    pub(super) fn remove_last(&mut self) -> bool {
        self.local_images.pop().is_some()
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::AttachmentState;

    #[test]
    fn attachments_are_ordered_and_restorable() {
        let mut state = AttachmentState::default();
        state.attach(PathBuf::from("one.png"));
        state.attach(PathBuf::from("two.png"));
        assert_eq!(
            state.paths(),
            &[PathBuf::from("one.png"), PathBuf::from("two.png")]
        );
        let taken = state.take();
        assert!(state.is_empty());
        state.restore(taken);
        assert_eq!(state.len(), 2);
        assert!(state.remove_last());
        assert_eq!(state.paths(), &[PathBuf::from("one.png")]);
    }

    #[test]
    fn remote_images_support_selection_delete_and_round_trip() {
        let mut state = AttachmentState::default();
        state.set_remote_image_urls(vec![
            "https://example.test/one.png".to_string(),
            "https://example.test/two.png".to_string(),
        ]);
        assert_eq!(state.len(), 2);

        let up = crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Up,
            crossterm::event::KeyModifiers::NONE,
        );
        assert!(state.handle_remote_key(up, 0));
        assert_eq!(state.selected_remote_image_index(), Some(1));
        let delete = crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Delete,
            crossterm::event::KeyModifiers::NONE,
        );
        assert!(state.handle_remote_key(delete, 0));
        assert_eq!(state.remote_image_urls(), &["https://example.test/one.png"]);

        let urls = state.take_remote_image_urls();
        assert!(state.remote_image_urls().is_empty());
        state.set_remote_image_urls(urls);
        assert_eq!(state.remote_image_urls(), &["https://example.test/one.png"]);
    }

    #[test]
    fn remote_images_require_selection_before_delete() {
        let mut state = AttachmentState::default();
        state.set_remote_image_urls(vec!["https://example.test/one.png".to_string()]);
        let backspace = crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Backspace,
            crossterm::event::KeyModifiers::NONE,
        );
        assert!(!state.handle_remote_key(backspace, 0));
        assert_eq!(state.remote_image_urls(), &["https://example.test/one.png"]);
    }
}
