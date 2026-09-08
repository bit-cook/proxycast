//! Stable key bindings for the Codex-shaped Agents Overview.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

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

#[cfg(test)]
mod tests {
    use super::AgentsKeymap;
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
}
