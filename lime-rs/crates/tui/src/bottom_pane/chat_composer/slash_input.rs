//! Slash-command parsing and composer popup construction.
//!
//! This module is the composer-facing fact source.  The command catalog remains in
//! [`crate::slash_command`], while lifecycle and rendering stay in the popup owner.

use super::super::command_popup::CommandPopup;
use crate::slash_command::command_from_prompt as parse_command_from_prompt;

pub(super) fn command_popup(text: &str) -> super::ActivePopup {
    CommandPopup::for_composer(text)
        .map(super::ActivePopup::Command)
        .unwrap_or_default()
}

/// Return the command fragment under the cursor for popup filtering.
///
/// Codex keeps the slash popup available while the cursor is inside the command name, even when
/// the first line already has inline arguments. Once the cursor moves into the argument suffix,
/// the popup is dismissed so it cannot steal normal text editing keys.
pub(super) fn command_popup_filter_text(first_line: &str, cursor: usize) -> Option<String> {
    let (name, _) = command_under_cursor(first_line, cursor)?;
    Some(format!("/{name}"))
}

fn command_under_cursor(first_line: &str, cursor: usize) -> Option<(&str, &str)> {
    if !first_line.starts_with('/')
        || cursor > first_line.len()
        || !first_line.is_char_boundary(cursor)
    {
        return None;
    }

    let name_start = 1;
    let name_end = first_line[name_start..]
        .find(char::is_whitespace)
        .map(|offset| name_start + offset)
        .unwrap_or(first_line.len());
    let cursor = if cursor <= name_start {
        name_end
    } else {
        cursor
    };
    if cursor > name_end {
        return None;
    }

    Some((&first_line[name_start..cursor], &first_line[cursor..]))
}

pub(super) fn command_from_prompt(prompt: &str) -> Option<crate::slash_command::SlashCommand> {
    parse_command_from_prompt(prompt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bottom_pane::chat_composer::popup_state::ActivePopup;

    #[test]
    fn popup_is_created_only_for_a_single_slash_token() {
        assert!(matches!(command_popup("/mo"), ActivePopup::Command(_)));
        assert!(matches!(command_popup("ordinary"), ActivePopup::None));
        assert!(matches!(command_popup("/model args"), ActivePopup::None));
    }

    #[test]
    fn popup_filter_tracks_the_cursor_inside_a_command_name() {
        let text = "/review inline args";
        assert_eq!(
            command_popup_filter_text(text, "/re".len()),
            Some("/re".to_string())
        );
        assert_eq!(
            command_popup_filter_text(text, "/review".len()),
            Some("/review".to_string())
        );
        assert_eq!(command_popup_filter_text(text, text.len()), None);
    }

    #[test]
    fn popup_filter_rejects_non_boundary_utf8_cursor_offsets() {
        let text = "/审查 参数";
        let slash = text.find('/').expect("slash");
        assert_eq!(command_popup_filter_text(text, slash + 2), None);
        assert_eq!(
            command_popup_filter_text(text, "/审".len()),
            Some("/审".to_string())
        );
    }

    #[test]
    fn parser_delegates_to_the_command_catalog() {
        assert_eq!(
            command_from_prompt("/model"),
            Some(crate::slash_command::SlashCommand::Model)
        );
        assert_eq!(command_from_prompt("/unknown"), None);
    }
}
