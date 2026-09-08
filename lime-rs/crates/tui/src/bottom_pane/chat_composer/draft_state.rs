//! Editable draft state kept separate from composer control flow.

use std::cell::{RefCell, RefMut};

use crate::bottom_pane::textarea::{TextArea, TextAreaState};

#[derive(Debug, Default)]
pub(super) struct DraftState {
    pub(super) textarea: TextArea,
    pub(super) textarea_state: RefCell<TextAreaState>,
    pub(super) saved_draft: Option<String>,
}

impl DraftState {
    pub(super) fn textarea_state_mut(&self) -> RefMut<'_, TextAreaState> {
        self.textarea_state.borrow_mut()
    }
}
