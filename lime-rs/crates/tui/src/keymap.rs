//! Stable key bindings for the Codex-shaped Agents Overview.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct AgentsKeymap;

impl AgentsKeymap {
    pub(crate) fn resume(self, key: KeyEvent) -> bool {
        Self::control(key, 'o')
    }

    pub(crate) fn search(self, key: KeyEvent) -> bool {
        Self::control(key, 'f')
    }

    pub(crate) fn new_task(self, key: KeyEvent) -> bool {
        Self::control(key, 'n')
    }

    pub(crate) fn rename(self, key: KeyEvent) -> bool {
        Self::control(key, 'r')
    }

    pub(crate) fn stop(self, key: KeyEvent) -> bool {
        Self::control(key, 'x')
    }

    pub(crate) fn toggle_grouping(self, key: KeyEvent) -> bool {
        Self::control(key, 's')
    }

    fn control(key: KeyEvent, character: char) -> bool {
        key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char(character)
    }
}

/// Returns whether a key belongs to the shared insert-mode editor surface.
///
/// Submission, interruption, and popup shortcuts stay at the composer/app
/// layers. This predicate only prevents control-editor keys from being
/// swallowed by those layers before reaching [`TextArea`](crate::bottom_pane::TextArea).
pub(crate) fn is_editor_key_event(key: KeyEvent) -> bool {
    if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
        return false;
    }

    let control = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    if matches!(key.code, KeyCode::Char(_)) && crate::key_hint::is_altgr(key.modifiers) {
        return false;
    }
    match key.code {
        KeyCode::Char(
            'a' | 'b' | 'e' | 'f' | 'h' | 'j' | 'k' | 'm' | 'n' | 'p' | 'u' | 'w' | 'y',
        ) if control => true,
        KeyCode::Char('d') if alt => true,
        KeyCode::Backspace | KeyCode::Delete if control || alt => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{is_editor_key_event, AgentsKeymap};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    #[test]
    fn codex_agents_defaults_are_stable() {
        let keymap = AgentsKeymap;
        assert!(keymap.resume(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL)));
        assert!(keymap.search(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::CONTROL)));
        assert!(keymap.new_task(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL)));
        assert!(keymap.rename(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL)));
        assert!(keymap.stop(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL)));
        assert!(keymap.toggle_grouping(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL,)));
        assert!(!keymap.resume(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE)));
    }

    #[test]
    fn codex_editor_control_aliases_are_routed_to_textarea() {
        for character in [
            'a', 'b', 'e', 'f', 'h', 'j', 'k', 'm', 'n', 'p', 'u', 'w', 'y',
        ] {
            assert!(is_editor_key_event(KeyEvent::new(
                KeyCode::Char(character),
                KeyModifiers::CONTROL,
            )));
        }
        assert!(is_editor_key_event(KeyEvent::new(
            KeyCode::Backspace,
            KeyModifiers::ALT,
        )));
        assert!(!is_editor_key_event(KeyEvent::new(
            KeyCode::Char('c'),
            KeyModifiers::CONTROL,
        )));
        assert!(!is_editor_key_event(KeyEvent::new(
            KeyCode::Char('m'),
            KeyModifiers::NONE,
        )));
    }
}
