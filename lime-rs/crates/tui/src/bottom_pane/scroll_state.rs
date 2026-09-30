//! Shared selection/scroll state; filtered identities remain with each popup.

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ScrollState {
    pub(crate) selected_idx: Option<usize>,
    pub(crate) scroll_top: usize,
}

impl ScrollState {
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn clamp_selection(&mut self, len: usize) {
        self.selected_idx = (len > 0).then(|| self.selected_idx.unwrap_or(0).min(len - 1));
        self.ensure_visible(len, super::selection_row_layout::MAX_POPUP_ROWS);
    }

    pub(crate) fn move_up_wrap(&mut self, len: usize) {
        self.selected_idx = (len > 0).then(|| match self.selected_idx {
            Some(index) if index > 0 => index - 1,
            Some(_) => len - 1,
            None => 0,
        });
        self.ensure_visible(len, super::selection_row_layout::MAX_POPUP_ROWS);
    }

    pub(crate) fn move_down_wrap(&mut self, len: usize) {
        self.selected_idx = (len > 0).then(|| match self.selected_idx {
            Some(index) if index + 1 < len => index + 1,
            _ => 0,
        });
        self.ensure_visible(len, super::selection_row_layout::MAX_POPUP_ROWS);
    }

    fn ensure_visible(&mut self, len: usize, visible: usize) {
        let Some(selected) = self.selected_idx else {
            self.scroll_top = 0;
            return;
        };
        if len == 0 || visible == 0 {
            self.scroll_top = 0;
        } else if selected < self.scroll_top {
            self.scroll_top = selected;
        } else if selected >= self.scroll_top.saturating_add(visible) {
            self.scroll_top = selected + 1 - visible;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_filter_and_reset_keep_selection_and_window_consistent() {
        let mut state = ScrollState::default();
        state.clamp_selection(10);
        assert_eq!(state.selected_idx, Some(0));
        state.move_up_wrap(10);
        assert_eq!((state.selected_idx, state.scroll_top), (Some(9), 2));
        state.move_down_wrap(10);
        assert_eq!((state.selected_idx, state.scroll_top), (Some(0), 0));
        state.clamp_selection(0);
        assert_eq!(state, ScrollState::default());
        state.move_up_wrap(0);
        state.move_down_wrap(0);
        assert_eq!(state.selected_idx, None);
        state.clamp_selection(3);
        state.reset();
        assert_eq!(state, ScrollState::default());
    }
}
