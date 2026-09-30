//! Case-insensitive subsequence matching with original-character highlight indices.

pub(crate) fn fuzzy_match(haystack: &str, needle: &str) -> Option<(Vec<usize>, i32)> {
    if needle.is_empty() {
        return Some((Vec::new(), i32::MAX));
    }
    let lowered = haystack
        .chars()
        .enumerate()
        .flat_map(|(index, ch)| ch.to_lowercase().map(move |ch| (ch, index)))
        .collect::<Vec<_>>();
    let needle = needle.to_lowercase().chars().collect::<Vec<_>>();
    let mut cursor = 0;
    let mut first = None;
    let mut last = 0;
    let mut indices = Vec::with_capacity(needle.len());
    for ch in &needle {
        let position = lowered[cursor..]
            .iter()
            .position(|(value, _)| value == ch)?
            + cursor;
        indices.push(lowered[position].1);
        first.get_or_insert(position);
        last = position;
        cursor = position + 1;
    }
    let first = first?;
    let span = last
        .saturating_sub(first)
        .saturating_add(1)
        .saturating_sub(needle.len());
    let score = i32::try_from(span).unwrap_or(i32::MAX) - if first == 0 { 100 } else { 0 };
    indices.dedup();
    Some((indices, score))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unicode_lowercase_expansion_retains_original_highlight_indices() {
        assert_eq!(fuzzy_match("İstanbul", "is"), Some((vec![0, 1], -99)));
        assert_eq!(fuzzy_match("İ", "i\u{307}"), Some((vec![0], -100)));
        assert_eq!(fuzzy_match("FooBar", "foo"), Some((vec![0, 1, 2], -100)));
        assert_eq!(fuzzy_match("", ""), Some((vec![], i32::MAX)));
        assert!(fuzzy_match("straße", "strasse").is_none());
    }

    #[test]
    fn contiguous_and_prefix_matches_rank_first() {
        let score = |value| fuzzy_match(value, "abc").unwrap().1;
        assert!(score("abc") < score("a-b-c"));
        assert!(score("abc") < score("xabc"));
    }
}
