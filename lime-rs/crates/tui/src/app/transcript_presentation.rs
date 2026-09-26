//! Session-local ownership for compact and detailed transcript reading positions.
//!
//! Compact history keeps its position in `App::transcript_scroll`. The detailed Ctrl+T surface
//! retains its pager while closed so the two presentations can be browsed independently without
//! moving presentation state into the canonical Thread/Turn/Item projection.

use super::App;
use crate::locale::Locale;
use crate::keymap::TranscriptKeymap;
use crate::pager_overlay::PagerOverlay;

#[derive(Debug, Default)]
pub(super) struct TranscriptPresentation {
    detailed: Option<PagerOverlay>,
}

impl TranscriptPresentation {
    fn open(&mut self, locale: Locale, keymap: TranscriptKeymap) -> PagerOverlay {
        self.detailed
            .take()
            .unwrap_or_else(|| PagerOverlay::transcript(locale))
            .with_keymap(keymap)
    }

    fn retain(&mut self, mut pager: PagerOverlay) {
        if !pager.is_transcript() {
            return;
        }
        pager.suspend_transcript_interaction();
        self.detailed = Some(pager);
    }

    fn clear(&mut self) {
        self.detailed = None;
    }

    #[cfg(test)]
    pub(super) fn has_detailed_bookmark(&self) -> bool {
        self.detailed.is_some()
    }
}

impl App {
    pub(super) fn open_transcript_pager(&mut self) {
        self.finish_main_transcript_selection(false);
        if let Some(scroll) = self.transcript_search.close() {
            self.transcript_scroll = scroll;
        }
        self.composer.clear_command_popup();
        self.dismiss_pager_overlay();
        let pager = self
            .transcript_presentation
            .open(self.locale, self.runtime_keymap.transcript().clone());
        pager.set_older_history_available(self.scrollback_has_older_history);
        self.pager_overlay = Some(pager);
    }

    pub(super) fn dismiss_pager_overlay(&mut self) {
        let Some(pager) = self.pager_overlay.take() else {
            return;
        };
        self.transcript_presentation.retain(pager);
    }

    pub(super) fn reset_transcript_presentation(&mut self) {
        if self
            .pager_overlay
            .as_ref()
            .is_some_and(PagerOverlay::is_transcript)
        {
            self.pager_overlay = None;
        }
        self.transcript_presentation.clear();
    }
}
