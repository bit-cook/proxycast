use super::*;
use crate::bottom_pane::textarea::TextArea;
use pretty_assertions::assert_eq;

fn textarea(text: &str, cursor: usize) -> TextArea {
    let mut textarea = TextArea::new();
    textarea.insert_str(text);
    textarea.set_cursor(cursor);
    textarea
}

#[test]
fn current_prefixed_token_affinity_does_not_cross_line_break() {
    for (text, prefix, allow_empty) in [
        ("@file\n  continue", '@', false),
        ("continue  \n@file", '@', false),
        ("$skill\n  continue", '$', true),
        ("continue  \n$skill", '$', true),
    ] {
        let cursor = text.find("  ").expect("indentation present") + 1;
        let textarea = textarea(text, cursor);
        assert_eq!(
            current_prefixed_token_range(&textarea, prefix, allow_empty),
            None
        );
    }
}

#[test]
fn current_prefixed_token_affinity_does_not_cross_separator_before_plain_text() {
    for (text, prefix, allow_empty) in [("@old  word", '@', false), ("$old  word", '$', true)] {
        let cursor = text.find("  ").expect("separator present") + 1;
        let textarea = textarea(text, cursor);
        assert_eq!(
            current_prefixed_token_range(&textarea, prefix, allow_empty),
            None
        );
    }
}

#[test]
fn current_prefixed_token_prefers_right_at_token_start() {
    let text = "$old $new";
    let right_start = "$old ".len();
    let textarea = textarea(text, right_start);
    assert_eq!(
        current_prefixed_token_range(&textarea, '$', true),
        Some((right_start..text.len(), "new".to_string()))
    );
}

#[test]
fn current_prefixed_token_preserves_separator_affinity_between_sigil_targets() {
    let text = "$old  $new";
    let separator_start = "$old".len();
    let at_separator = textarea(text, separator_start + 1);
    assert_eq!(
        current_prefixed_token_range(&at_separator, '$', true),
        Some((0.."$old".len(), "old".to_string()))
    );
    let at_right_start = textarea(text, "$old  ".len());
    assert_eq!(
        current_prefixed_token_range(&at_right_start, '$', true),
        Some(("$old  ".len()..text.len(), "new".to_string()))
    );
}

#[test]
fn current_prefixed_token_handles_cursor_inside_utf8_codepoint_fail_closed() {
    let text = "@界";
    let cursor = "@".len() + 1;
    assert!(!text.is_char_boundary(cursor));
    assert_eq!(
        current_prefixed_token_range(&textarea(text, cursor), '@', false),
        Some((0..text.len(), "界".to_string()))
    );
}

#[test]
fn current_prefixed_token_does_not_cross_trailing_horizontal_whitespace_into_plain_text() {
    let text = "$figma  next";
    let cursor = "$figma ".len();
    assert_eq!(
        current_prefixed_token_range(&textarea(text, cursor), '$', true),
        None
    );
}

#[test]
fn current_prefixed_token_keeps_nested_dollar_prefix_in_same_token() {
    let text = "$HOME/$USER";
    let textarea = textarea(text, text.find("$USER").expect("nested prefix present"));
    assert_eq!(
        current_prefixed_token_range(&textarea, '$', true),
        Some((0..text.len(), "HOME/$USER".to_string()))
    );
}

#[test]
fn current_prefixed_token_accepts_utf8_cursor_boundaries() {
    let text = "前 @界🙂 后";
    let start = text.find('@').expect("mention prefix");
    let end = text.find(" 后").expect("trailing separator");
    let expected = Some((start..end, "界🙂".to_string()));
    for cursor in [start, start + 1, end] {
        assert_eq!(
            current_prefixed_token_range(&textarea(text, cursor), '@', false),
            expected
        );
    }
}

#[test]
fn dollar_query_classifies_shell_and_skill_syntax() {
    assert_eq!(dollar_query_kind(""), DollarQueryKind::Completable);
    assert_eq!(
        dollar_query_kind("home:search"),
        DollarQueryKind::Completable
    );
    assert_eq!(dollar_query_kind("HOME"), DollarQueryKind::ShellVariable);
    for query in ["0", "1", "12", "-", "_"] {
        assert_eq!(
            dollar_query_kind(query),
            DollarQueryKind::DefiniteShellParameter
        );
        assert!(!dollar_query_is_completable(query));
    }
    for query in ["1_suffix", "1foo", "-x"] {
        assert_eq!(
            dollar_query_kind(query),
            DollarQueryKind::AmbiguousShellParameter
        );
        assert!(!dollar_query_is_completable(query));
    }
    assert_eq!(dollar_query_kind("{HOME}"), DollarQueryKind::Invalid);
}

#[test]
fn dollar_query_classifies_shell_parameter_edge_cases() {
    assert_eq!(
        dollar_query_kind("$HOME".trim_start_matches('$')),
        DollarQueryKind::ShellVariable
    );
    assert_eq!(
        dollar_query_kind("$1".trim_start_matches('$')),
        DollarQueryKind::DefiniteShellParameter
    );
    assert_eq!(
        dollar_query_kind("$1_suffix".trim_start_matches('$')),
        DollarQueryKind::AmbiguousShellParameter
    );
    assert_eq!(
        dollar_query_kind("$-x".trim_start_matches('$')),
        DollarQueryKind::AmbiguousShellParameter
    );
    assert_eq!(
        dollar_query_kind("$-".trim_start_matches('$')),
        DollarQueryKind::DefiniteShellParameter
    );
    assert_eq!(
        dollar_query_kind("$_".trim_start_matches('$')),
        DollarQueryKind::DefiniteShellParameter
    );
}
