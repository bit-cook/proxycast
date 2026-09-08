//! Key input matching shared by TUI selection surfaces.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Returns whether an event should be treated as literal text input.
pub(crate) fn is_plain_text_key_event(event: KeyEvent) -> bool {
    matches!(
        event,
        KeyEvent {
            code: KeyCode::Char(ch),
            modifiers,
            ..
        } if !ch.is_ascii_control()
            && !modifiers.contains(KeyModifiers::CONTROL)
            && !modifiers.contains(KeyModifiers::ALT)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shifted_characters_are_plain_text() {
        assert!(is_plain_text_key_event(KeyEvent::new(
            KeyCode::Char('G'),
            KeyModifiers::SHIFT,
        )));
    }

    #[test]
    fn control_and_alt_characters_are_not_plain_text() {
        assert!(!is_plain_text_key_event(KeyEvent::new(
            KeyCode::Char('g'),
            KeyModifiers::CONTROL,
        )));
        assert!(!is_plain_text_key_event(KeyEvent::new(
            KeyCode::Char('g'),
            KeyModifiers::ALT,
        )));
    }
}
