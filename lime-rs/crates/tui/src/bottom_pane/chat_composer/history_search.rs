//! History-search state and matching rules for `ChatComposer`.

use std::ops::Range;

use super::draft_state::ComposerDraft;

#[derive(Debug, Default)]
pub(super) struct HistorySearchState {
    pub(super) query: String,
    pub(super) draft: ComposerDraft,
    pub(super) selected_index: Option<usize>,
}

pub(super) fn find_older(
    history: &[String],
    query: &str,
    selected_index: Option<usize>,
) -> Option<usize> {
    let query = query.to_lowercase();
    if query.is_empty() || history.is_empty() {
        return None;
    }
    let start = selected_index
        .map(|index| index.saturating_sub(1))
        .unwrap_or(history.len() - 1);
    (0..=start).rev().find(|index| {
        history
            .get(*index)
            .is_some_and(|entry| entry.to_lowercase().contains(&query))
    })
}

pub(super) fn find_match(
    history: &[String],
    query: &str,
    selected_index: Option<usize>,
    older: bool,
) -> Option<usize> {
    if older {
        return find_older(history, query, selected_index);
    }
    let query = query.to_lowercase();
    if query.is_empty() || history.is_empty() {
        return None;
    }
    let start = selected_index
        .map(|index| {
            if older {
                index.saturating_sub(1)
            } else {
                index
            }
        })
        .unwrap_or(history.len() - 1);
    (0..=start).rev().find(|index| {
        history
            .get(*index)
            .is_some_and(|entry| entry.to_lowercase().contains(&query))
    })
}

pub(super) fn find_newer(
    history: &[String],
    query: &str,
    selected_index: Option<usize>,
) -> Option<usize> {
    let selected_index = selected_index?;
    let query = query.to_lowercase();
    if query.is_empty() {
        return None;
    }
    ((selected_index + 1)..history.len()).find(|index| {
        history
            .get(*index)
            .is_some_and(|entry| entry.to_lowercase().contains(&query))
    })
}

/// 返回文本中与查询匹配的原始字节范围。
///
/// 搜索导航沿用 `to_lowercase().contains(...)` 的语义；这里对每个原始字符建立
/// lower-case 投影，并把投影命中的范围映射回原文，避免高亮直接按 lower-case
/// 字节偏移切片而破坏 UTF-8 边界。
pub(super) fn match_ranges(text: &str, query: &str) -> Vec<Range<usize>> {
    if query.is_empty() || text.is_empty() {
        return Vec::new();
    }

    let query = query.to_lowercase();
    if query.is_empty() {
        return Vec::new();
    }

    let mut folded = String::new();
    let mut source_ranges = Vec::new();
    for (start, ch) in text.char_indices() {
        let source = start..start + ch.len_utf8();
        for lower in ch.to_lowercase() {
            let folded_start = folded.len();
            folded.push(lower);
            source_ranges.push((folded_start, folded.len(), source.clone()));
        }
    }

    let mut ranges = Vec::new();
    let mut search_from = 0;
    while search_from <= folded.len() {
        let Some(relative_start) = folded[search_from..].find(&query) else {
            break;
        };
        let folded_start = search_from + relative_start;
        let folded_end = folded_start + query.len();
        let Some(first) = source_ranges
            .iter()
            .position(|(start, end, _)| *start <= folded_start && folded_start < *end)
        else {
            break;
        };
        let Some(last) = source_ranges
            .iter()
            .rposition(|(start, end, _)| *start < folded_end && folded_end <= *end)
        else {
            break;
        };
        ranges.push(source_ranges[first].2.start..source_ranges[last].2.end);
        search_from = folded_end;
    }
    ranges
}

#[cfg(test)]
mod tests {
    use super::{find_match, find_newer, find_older, match_ranges};

    fn history() -> Vec<String> {
        vec![
            "git status".to_string(),
            "cargo test".to_string(),
            "git diff".to_string(),
        ]
    }

    #[test]
    fn older_and_newer_matches_are_case_insensitive_and_bounded() {
        let history = history();
        assert_eq!(find_older(&history, "GIT", None), Some(2));
        assert_eq!(find_older(&history, "GIT", Some(2)), Some(0));
        assert_eq!(find_match(&history, "git", Some(2), false), Some(2));
        assert_eq!(find_newer(&history, "git", Some(0)), Some(2));
        assert_eq!(find_newer(&history, "git", Some(2)), None);
    }

    #[test]
    fn empty_queries_and_missing_selection_do_not_match() {
        let history = history();
        assert_eq!(find_older(&history, "", None), None);
        assert_eq!(find_newer(&history, "git", None), None);
    }

    #[test]
    fn match_ranges_map_case_insensitive_unicode_matches_to_original_bytes() {
        assert_eq!(match_ranges("Deploy DEPLOY", "dep"), vec![0..3, 7..10]);
        assert_eq!(match_ranges("Äpfel äPFEL", "äpfel"), vec![0..6, 7..13]);
        assert_eq!(match_ranges("界😀界", "😀"), vec![3..7]);
    }

    #[test]
    fn match_ranges_do_not_match_empty_or_missing_queries() {
        assert!(match_ranges("draft", "").is_empty());
        assert!(match_ranges("draft", "zzz").is_empty());
    }
}
