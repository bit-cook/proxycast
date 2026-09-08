//! History-search state and matching rules for `ChatComposer`.

#[derive(Debug, Default)]
pub(super) struct HistorySearchState {
    pub(super) query: String,
    pub(super) draft: String,
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

#[cfg(test)]
mod tests {
    use super::{find_match, find_newer, find_older};

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
}
