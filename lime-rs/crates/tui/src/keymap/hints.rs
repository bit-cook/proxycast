//! Visible shortcuts are projected from the same resolved bindings as event routing.

use super::*;

impl TranscriptKeymap {
    pub(crate) fn reserves_global_key(&self, key: KeyEvent) -> bool {
        [
            &self.open_agents,
            &self.open_transcript,
            &self.find_transcript,
        ]
        .into_iter()
        .any(|set| {
            set.shortcuts.iter().any(|shortcut| match shortcut {
                Shortcut::Single(binding) => binding.is_pressed(key),
                Shortcut::Chord { prefix, .. } => prefix.is_pressed(key),
            })
        })
    }

    pub(crate) fn pager_find_hint(&self) -> String {
        binding_labels([&self.find])
    }

    pub(crate) fn pager_page_down_hint(&self) -> String {
        binding_labels([&self.page_down])
    }

    pub(crate) fn pager_close_hint(&self) -> String {
        binding_labels([&self.close_transcript, &self.close])
    }

    pub(crate) fn open_transcript_hint(&self) -> Option<String> {
        self.open_transcript.labels().next()
    }

    pub(crate) fn global_hint(&self, action: GlobalKeymapAction) -> Option<String> {
        match action {
            GlobalKeymapAction::OpenAgents => &self.open_agents,
            GlobalKeymapAction::OpenTranscript => &self.open_transcript,
            GlobalKeymapAction::FindTranscript => &self.find_transcript,
        }
        .labels()
        .next()
    }

    pub(crate) fn pager_hint(&self, action: PagerKeymapAction) -> Option<String> {
        match action {
            PagerKeymapAction::ScrollUp => &self.scroll_up,
            PagerKeymapAction::ScrollDown => &self.scroll_down,
            PagerKeymapAction::PageUp => &self.page_up,
            PagerKeymapAction::PageDown => &self.page_down,
            PagerKeymapAction::HalfPageUp => &self.half_page_up,
            PagerKeymapAction::HalfPageDown => &self.half_page_down,
            PagerKeymapAction::JumpTop => &self.jump_top,
            PagerKeymapAction::JumpBottom => &self.jump_bottom,
            PagerKeymapAction::Close => &self.close,
            PagerKeymapAction::CloseTranscript => &self.close_transcript,
            PagerKeymapAction::Find => &self.find,
        }
        .labels()
        .next()
    }
}
