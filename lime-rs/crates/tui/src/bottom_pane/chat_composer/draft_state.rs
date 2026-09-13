//! Editable draft state kept separate from composer control flow.

use std::cell::{RefCell, RefMut};

use super::attachment_state::AttachmentState;
use crate::bottom_pane::textarea::{TextArea, TextAreaState};

/// Minimal composer snapshot shared by history/search and Vim editing.
///
/// Keep this at the draft owner boundary so every temporary composer mode restores the
/// same text, cursor and canonical attachment state. Fields that Lime does not model yet
/// (mentions, text elements and pending pastes) intentionally remain out of this contract.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(super) struct ComposerDraft {
    pub(super) text: String,
    pub(super) cursor: usize,
    pub(super) attachments: AttachmentState,
}

impl ComposerDraft {
    pub(super) fn bytes(&self) -> usize {
        self.text.len()
            + self
                .attachments
                .paths()
                .iter()
                .map(|path| path.as_os_str().len())
                .sum::<usize>()
            + self
                .attachments
                .remote_image_urls()
                .iter()
                .map(String::len)
                .sum::<usize>()
    }
}

#[derive(Debug, Default)]
pub(super) struct DraftState {
    pub(super) textarea: TextArea,
    pub(super) textarea_state: RefCell<TextAreaState>,
    pub(super) saved_draft: Option<ComposerDraft>,
}

impl DraftState {
    pub(super) fn textarea_state_mut(&self) -> RefMut<'_, TextAreaState> {
        self.textarea_state.borrow_mut()
    }
}
