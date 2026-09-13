//! Surface-independent literal Vim search queries.
//!
//! Matches are discovered only at UTF-8 grapheme boundaries so callers can safely
//! move or edit the returned ranges without splitting a user-visible grapheme.

use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum SearchDirection {
    #[default]
    Forward,
    Backward,
}

impl SearchDirection {
    pub(crate) fn reversed(self) -> Self {
        match self {
            Self::Forward => Self::Backward,
            Self::Backward => Self::Forward,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SearchQuery {
    pub(crate) text: String,
    pub(crate) direction: SearchDirection,
}

pub(crate) fn matching_ranges<'a>(
    text: &'a str,
    query: &'a str,
) -> impl Iterator<Item = Range<usize>> + 'a {
    let text = if query.is_empty() { "" } else { text };
    text.grapheme_indices(true)
        .filter(move |&(start, _)| text[start..].starts_with(query))
        .map(move |(start, _)| {
            let end = text[start..]
                .grapheme_indices(true)
                .map(|(offset, grapheme)| start + offset + grapheme.len())
                .find(|&end| end >= start + query.len())
                .unwrap_or(text.len());
            start..end
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_ranges_stay_on_grapheme_boundaries() {
        let text = "a👩🏽‍💻a";
        let ranges = matching_ranges(text, "👩🏽‍💻").collect::<Vec<_>>();
        assert_eq!(ranges, vec![1.."a👩🏽‍💻".len()]);
    }

    #[test]
    fn empty_query_has_no_matches() {
        assert_eq!(matching_ranges("abc", "").count(), 0);
    }
}
