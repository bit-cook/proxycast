//! Local image attachment bookkeeping for the chat composer.
//!
//! The attachment state is intentionally owned by `ChatComposer`, matching Codex's ownership
//! boundary. Runtime lowering still happens in the App Server session and keeps image parts before
//! text parts in the canonical `UserInput` vector.

use std::path::PathBuf;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct AttachmentState {
    local_images: Vec<PathBuf>,
}

impl AttachmentState {
    pub(super) fn is_empty(&self) -> bool {
        self.local_images.is_empty()
    }

    pub(super) fn len(&self) -> usize {
        self.local_images.len()
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
}
