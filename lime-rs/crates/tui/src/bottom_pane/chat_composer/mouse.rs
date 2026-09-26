//! Composer pointer routing built on the textarea's last rendered viewport.

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use super::{ActivePopup, ChatComposer};
use crate::tui::TuiEvent;

impl ChatComposer {
    pub(crate) fn end_mouse_drag(&mut self) {
        self.draft.textarea.end_mouse_drag();
    }

    pub(crate) fn clear_mouse_selection(&mut self) {
        self.draft.textarea.clear_mouse_selection();
    }

    pub(crate) fn copy_selection_request(&mut self, event: &TuiEvent) -> Option<(String, bool)> {
        let mouse_copy = matches!(
            event,
            TuiEvent::Mouse(mouse)
                if mouse.kind == MouseEventKind::Down(MouseButton::Right)
                    && self.draft.textarea.contains_mouse(*mouse)
        );
        let keyboard_copy =
            matches!(event, TuiEvent::Key(key) if crate::text_selection::is_copy_key(*key));
        if (!mouse_copy && !keyboard_copy)
            || self.history_search.is_some()
            || self.vim_search_active()
        {
            return None;
        }
        let text = self.draft.textarea.selected_text()?.to_string();
        self.end_mouse_drag();
        Some((text, mouse_copy))
    }

    pub(crate) fn handle_mouse(&mut self, event: MouseEvent) -> bool {
        if self.history_search.is_some() || self.vim_search_active() {
            self.end_mouse_drag();
            return false;
        }
        let state = *self.draft.textarea_state.borrow();
        let handled = self.draft.textarea.handle_mouse(event, state);
        if handled {
            self.attachments.clear_remote_image_selection();
            self.popups.active = ActivePopup::None;
            self.file_search_request = None;
            self.reset_history_navigation();
        }
        handled
    }
}
