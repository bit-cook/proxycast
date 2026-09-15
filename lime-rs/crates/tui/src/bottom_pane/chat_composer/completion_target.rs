//! Cursor-neighborhood resolution for sigil-prefixed composer completions.
//!
//! This module is the single owner for `@` file and `$` skill completion targets.  It keeps
//! cursor affinity and shell-syntax arbitration independent from popup presentation.

use crate::bottom_pane::textarea::TextArea;
use std::ops::Range;

fn is_horizontal_whitespace(ch: char) -> bool {
    ch.is_whitespace()
        && !matches!(
            ch,
            '\n' | '\r' | '\u{000B}' | '\u{000C}' | '\u{0085}' | '\u{2028}' | '\u{2029}'
        )
}

fn safe_cursor(text: &str, cursor: usize) -> usize {
    let cursor = cursor.min(text.len());
    if text.is_char_boundary(cursor) {
        return cursor;
    }
    text.char_indices()
        .map(|(index, _)| index)
        .take_while(|index| *index < cursor)
        .last()
        .unwrap_or(0)
}

fn whitespace_delimited_range(
    text: &str,
    cursor: usize,
) -> (Range<usize>, Range<usize>, bool, bool) {
    let cursor = safe_cursor(text, cursor);
    let before = &text[..cursor];
    let after = &text[cursor..];
    let at_whitespace = after.chars().next().is_some_and(char::is_whitespace);
    let after_horizontal = before
        .chars()
        .next_back()
        .is_some_and(is_horizontal_whitespace);
    let next_non_separator = after.chars().find(|ch| !is_horizontal_whitespace(*ch));
    let separator_precedes_token = next_non_separator.is_some_and(|ch| !ch.is_whitespace());
    let at_separator = (at_whitespace || after_horizontal) && separator_precedes_token;

    let end_left = if at_separator {
        before.trim_end_matches(is_horizontal_whitespace).len()
    } else {
        let end_rel = after
            .char_indices()
            .find(|(_, ch)| ch.is_whitespace())
            .map(|(index, _)| index)
            .unwrap_or(after.len());
        cursor + end_rel
    };
    let start_left = text[..end_left]
        .char_indices()
        .rfind(|(_, ch)| ch.is_whitespace())
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(0);

    let right_start = cursor
        + after
            .chars()
            .take_while(|ch| is_horizontal_whitespace(*ch))
            .map(char::len_utf8)
            .sum::<usize>();
    let right_end = right_start
        + text[right_start..]
            .char_indices()
            .find(|(_, ch)| ch.is_whitespace())
            .map(|(index, _)| index)
            .unwrap_or(text.len().saturating_sub(right_start));

    (
        start_left..end_left,
        right_start..right_end,
        at_separator,
        after_horizontal,
    )
}

fn prefixed_candidate(
    text: &str,
    range: Range<usize>,
    prefix: char,
    allow_empty: bool,
) -> Option<(Range<usize>, String)> {
    let token = text.get(range.clone())?;
    let query = token.strip_prefix(prefix)?;
    if !allow_empty && query.is_empty() {
        return None;
    }
    Some((range, query.to_string()))
}

/// Extracts the prefixed completion target nearest to the cursor.
///
/// Horizontal whitespace has same-line affinity; line breaks are hard boundaries. At a separator
/// the right target wins when the cursor starts a new token, while a left shell-like `$` target
/// yields to a completable right target. Returned ranges include the sigil.
pub(super) fn current_prefixed_token_range(
    textarea: &TextArea,
    prefix: char,
    allow_empty: bool,
) -> Option<(Range<usize>, String)> {
    current_prefixed_token_range_with_dollar_predicate(
        textarea,
        prefix,
        allow_empty,
        dollar_query_is_completable,
    )
}

pub(super) fn current_prefixed_token_range_with_dollar_predicate(
    textarea: &TextArea,
    prefix: char,
    allow_empty: bool,
    dollar_query_is_completable: impl Fn(&str) -> bool,
) -> Option<(Range<usize>, String)> {
    let text = textarea.text();
    let cursor = safe_cursor(text, textarea.cursor());
    let before = &text[..cursor];
    let after = &text[cursor..];
    let (left_range, right_range, at_separator, after_horizontal) =
        whitespace_delimited_range(text, cursor);
    let left = prefixed_candidate(text, left_range.clone(), prefix, allow_empty);
    let right = prefixed_candidate(text, right_range.clone(), prefix, allow_empty);
    let right_token = text.get(right_range.clone());
    let cursor_starts_token =
        after.chars().next().is_some_and(|ch| !ch.is_whitespace()) && after_horizontal;
    let separator_precedes_completion = right_token
        .and_then(|token| token.chars().next())
        .is_some_and(|ch| matches!(ch, '$' | '@'));

    if cursor_starts_token {
        return right;
    }

    if allow_empty && after.starts_with(prefix) {
        let left_fragment = &text[left_range.start..cursor];
        if let Some(query) = left_fragment.strip_prefix(prefix) {
            if query.bytes().all(is_mention_name_char) {
                return Some((left_range.start..cursor, query.to_string()));
            }
        }
    }

    if at_separator {
        if after_horizontal && !separator_precedes_completion {
            return right;
        }
        if prefix == '$'
            && left
                .as_ref()
                .is_some_and(|(_, query)| !dollar_query_is_completable(query))
            && right
                .as_ref()
                .is_some_and(|(_, query)| dollar_query_is_completable(query))
        {
            return right;
        }
        return left.or(right);
    }

    if after.starts_with(prefix) {
        let starts_token = before.chars().next_back().is_none_or(char::is_whitespace);
        return if starts_token { right.or(left) } else { left };
    }
    left.or(right)
}

pub(super) fn dollar_query_is_completable(query: &str) -> bool {
    matches!(dollar_query_kind(query), DollarQueryKind::Completable)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum DollarQueryKind {
    Completable,
    ShellVariable,
    DefiniteShellParameter,
    AmbiguousShellParameter,
    Invalid,
}

pub(super) fn dollar_query_kind(query: &str) -> DollarQueryKind {
    let name_end = query
        .as_bytes()
        .iter()
        .take_while(|byte| is_mention_name_char(**byte) || **byte == b':')
        .count();
    let name = &query[..name_end];
    let is_shell_var =
        name.bytes().all(|byte| !byte.is_ascii_lowercase()) && is_common_env_var(name);
    let starts_shell_parameter = name
        .as_bytes()
        .first()
        .is_some_and(|byte| *byte == b'-' || byte.is_ascii_digit());
    let numeric = !name.is_empty() && name.bytes().all(|byte| byte.is_ascii_digit());
    if query.is_empty() {
        DollarQueryKind::Completable
    } else if name_end == 0 {
        DollarQueryKind::Invalid
    } else if is_shell_var {
        DollarQueryKind::ShellVariable
    } else if numeric || matches!(name, "-" | "_") {
        DollarQueryKind::DefiniteShellParameter
    } else if starts_shell_parameter {
        DollarQueryKind::AmbiguousShellParameter
    } else {
        DollarQueryKind::Completable
    }
}

pub(super) fn is_mention_name_char(byte: u8) -> bool {
    matches!(byte, b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_' | b'-')
}

fn is_common_env_var(name: &str) -> bool {
    matches!(
        name,
        "PATH"
            | "HOME"
            | "USER"
            | "SHELL"
            | "PWD"
            | "OLDPWD"
            | "TMPDIR"
            | "TEMP"
            | "TMP"
            | "LANG"
            | "TERM"
            | "XDG_CONFIG_HOME"
    )
}

#[cfg(test)]
#[path = "completion_target_tests.rs"]
mod tests;
