//! Bounded literal search shared by the compact and detailed transcript surfaces.
//!
//! Canonical history remains in `ConversationProjection`. This owner only keeps an ephemeral
//! query, source offsets, and presentation-local navigation state. Source revisions restart the
//! bounded scan; older history is requested through the app's existing pager.

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::ops::Range;

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::style::Modifier;
use ratatui::text::Span;
use unicode_segmentation::UnicodeSegmentation;

use crate::terminal_hyperlinks::{wrapped_line_starts, HyperlinkLine, HyperlinkParagraph};

pub(crate) const MAX_SEARCH_QUERY_BYTES: usize = 4096;
const SEARCH_WINDOW_BYTES: usize = 16 * 1024;
const SEARCH_LINES_PER_FRAME: usize = 8;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum SearchBoundary {
    #[default]
    None,
    Older,
    Newer,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum SearchHistoryState {
    #[default]
    Idle,
    Loading,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SearchAction {
    Consumed,
    ScheduleFrame,
    LoadOlderHistory,
    Closed { restore_scroll: Option<usize> },
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct SearchMatch {
    pub(crate) line: usize,
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) row_start: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SearchDirection {
    Older,
    Newer,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SearchSource {
    lines: Vec<String>,
    width: u16,
    excluded_lines: HashSet<usize>,
    row_starts: Vec<usize>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct MatchIdentity {
    text: String,
    start: usize,
    end: usize,
}

#[derive(Debug)]
struct ScanState {
    line: Option<usize>,
    end: usize,
    matches: Vec<SearchMatch>,
}

impl ScanState {
    fn new(source: &SearchSource) -> Self {
        let line = source.lines.len().checked_sub(1);
        let end = line
            .and_then(|line| source.lines.get(line))
            .map_or(0, String::len);
        Self {
            line,
            end,
            matches: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub(crate) struct TranscriptSearch {
    active: bool,
    query: String,
    folded_query: String,
    restore_on_close: bool,
    saved_scroll: usize,
    cursor: Cell<usize>,
    matches: RefCell<Vec<SearchMatch>>,
    matches_valid: Cell<bool>,
    boundary: Cell<SearchBoundary>,
    source: RefCell<Option<SearchSource>>,
    scan: RefCell<Option<ScanState>>,
    preferred_match: RefCell<Option<MatchIdentity>>,
    pending_direction: Cell<Option<SearchDirection>>,
    wants_history: Cell<bool>,
    history_state: Cell<SearchHistoryState>,
    query_truncated: bool,
    viewport_scroll: Cell<Option<usize>>,
}

impl Default for TranscriptSearch {
    fn default() -> Self {
        Self {
            active: false,
            query: String::new(),
            folded_query: String::new(),
            restore_on_close: false,
            saved_scroll: 0,
            cursor: Cell::new(0),
            matches: RefCell::new(Vec::new()),
            matches_valid: Cell::new(false),
            boundary: Cell::new(SearchBoundary::None),
            source: RefCell::new(None),
            scan: RefCell::new(None),
            preferred_match: RefCell::new(None),
            pending_direction: Cell::new(None),
            wants_history: Cell::new(false),
            history_state: Cell::new(SearchHistoryState::Idle),
            query_truncated: false,
            viewport_scroll: Cell::new(None),
        }
    }
}

impl TranscriptSearch {
    pub(crate) fn begin(&mut self, saved_scroll: usize, restore_on_close: bool) -> bool {
        if self.active {
            return false;
        }
        self.active = true;
        self.restore_on_close = restore_on_close;
        self.saved_scroll = saved_scroll;
        self.viewport_scroll.set(None);
        self.history_state.set(SearchHistoryState::Idle);
        self.restart_scan(/*retain_match*/ false);
        true
    }

    pub(crate) fn close(&mut self) -> Option<usize> {
        if !self.active {
            return None;
        }
        let restore = self.restore_on_close.then_some(self.saved_scroll);
        self.clear();
        restore
    }

    pub(crate) fn clear(&mut self) {
        self.active = false;
        self.query.clear();
        self.folded_query.clear();
        self.restore_on_close = false;
        self.saved_scroll = 0;
        self.cursor.set(0);
        self.matches.borrow_mut().clear();
        self.matches_valid.set(false);
        self.boundary.set(SearchBoundary::None);
        self.source.borrow_mut().take();
        self.scan.borrow_mut().take();
        self.preferred_match.borrow_mut().take();
        self.pending_direction.set(None);
        self.wants_history.set(false);
        self.history_state.set(SearchHistoryState::Idle);
        self.query_truncated = false;
        self.viewport_scroll.set(None);
    }

    pub(crate) fn is_active(&self) -> bool {
        self.active
    }

    pub(crate) fn query(&self) -> &str {
        &self.query
    }

    pub(crate) fn cursor(&self) -> usize {
        self.cursor.get()
    }

    pub(crate) fn match_count(&self) -> usize {
        self.matches.borrow().len()
    }

    pub(crate) fn matches_valid(&self) -> bool {
        self.matches_valid.get()
    }

    pub(crate) fn boundary(&self) -> SearchBoundary {
        self.boundary.get()
    }

    pub(crate) fn query_truncated(&self) -> bool {
        self.query_truncated
    }

    pub(crate) fn history_state(&self) -> SearchHistoryState {
        self.history_state.get()
    }

    pub(crate) fn begin_history_load(&self) {
        self.history_state.set(SearchHistoryState::Loading);
    }

    pub(crate) fn complete_history_load(&self) {
        self.history_state.set(SearchHistoryState::Idle);
    }

    pub(crate) fn fail_history_load(&self) {
        self.history_state.set(SearchHistoryState::Failed);
    }

    pub(crate) fn needs_frame(&self) -> bool {
        self.active && self.scan.borrow().is_some() && !self.matches_valid.get()
    }

    pub(crate) fn take_history_request(&self, older_history_available: bool) -> bool {
        self.active
            && older_history_available
            && self.history_state.get() != SearchHistoryState::Loading
            && self.wants_history.replace(false)
    }

    pub(crate) fn handle_event(
        &mut self,
        event: &Event,
        older_history_available: bool,
    ) -> SearchAction {
        if !self.active {
            return SearchAction::Consumed;
        }
        if let Event::Paste(text) = event {
            let pasted = text.replace(['\r', '\n'], " ");
            let remaining = MAX_SEARCH_QUERY_BYTES.saturating_sub(self.query.len());
            let end = pasted
                .grapheme_indices(true)
                .map(|(offset, grapheme)| offset + grapheme.len())
                .take_while(|end| *end <= remaining)
                .last()
                .unwrap_or_default();
            if end > 0 {
                self.query.push_str(&pasted[..end]);
                self.query_changed();
            }
            self.query_truncated = end < pasted.len();
            return SearchAction::ScheduleFrame;
        }
        let Event::Key(key) = event else {
            return SearchAction::Consumed;
        };
        if key.kind != KeyEventKind::Press {
            return SearchAction::Consumed;
        }
        match key.code {
            KeyCode::Esc => SearchAction::Closed {
                restore_scroll: self.close(),
            },
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                SearchAction::Closed {
                    restore_scroll: self.close(),
                }
            }
            KeyCode::Home => self.request_older_history(older_history_available),
            KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.navigate(SearchDirection::Older, older_history_available)
            }
            KeyCode::Enter
                if key.modifiers.is_empty() || key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.navigate(SearchDirection::Newer, older_history_available)
            }
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.navigate(SearchDirection::Newer, older_history_available)
            }
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.navigate(SearchDirection::Older, older_history_available)
            }
            KeyCode::Backspace | KeyCode::Delete => {
                let end = self
                    .query
                    .grapheme_indices(true)
                    .next_back()
                    .map(|(offset, _)| offset)
                    .unwrap_or_default();
                self.query.truncate(end);
                self.query_changed();
                SearchAction::ScheduleFrame
            }
            KeyCode::Char(character)
                if !key.modifiers.intersects(
                    KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SUPER,
                ) && self.query.len().saturating_add(character.len_utf8())
                    <= MAX_SEARCH_QUERY_BYTES =>
            {
                self.query.push(character);
                self.query_changed();
                SearchAction::ScheduleFrame
            }
            _ => SearchAction::Consumed,
        }
    }

    pub(crate) fn prepare(
        &self,
        lines: &[HyperlinkLine],
        width: u16,
        excluded_lines: &HashSet<usize>,
        selection_active: bool,
    ) {
        if !self.active {
            return;
        }
        let source = SearchSource {
            lines: lines.iter().map(line_text).collect(),
            width,
            excluded_lines: excluded_lines.clone(),
            row_starts: wrapped_line_starts(lines, width),
        };
        let changed = self.source.borrow().as_ref() != Some(&source);
        if changed {
            let preferred = self.selected_identity();
            self.source.replace(Some(source));
            self.preferred_match.replace(preferred);
            self.restart_scan_shared();
        }
        if selection_active || self.query.is_empty() {
            return;
        }
        self.advance_scan();
    }

    pub(crate) fn highlighted_lines(&self, lines: &[HyperlinkLine]) -> Vec<HyperlinkLine> {
        highlight_search_lines(lines, &self.matches.borrow(), self.cursor.get())
    }

    pub(crate) fn selected_match(&self) -> Option<SearchMatch> {
        self.matches.borrow().get(self.cursor.get()).copied()
    }

    pub(crate) fn resolve_main_scroll(
        &self,
        lines: &[HyperlinkLine],
        width: u16,
        height: u16,
    ) -> usize {
        let total_height = HyperlinkParagraph::new(lines).line_count(width);
        let max_scroll = total_height.saturating_sub(usize::from(height));
        let mut scroll = self
            .viewport_scroll
            .get()
            .unwrap_or_else(|| max_scroll.saturating_sub(self.saved_scroll.min(max_scroll)))
            .min(max_scroll);
        if let Some(selected) = self.selected_match() {
            let page_height = usize::from(height).max(1);
            if selected.row_start < scroll {
                scroll = selected.row_start;
            } else if selected.row_start >= scroll.saturating_add(page_height) {
                scroll = selected
                    .row_start
                    .saturating_add(1)
                    .saturating_sub(page_height);
            }
            scroll = scroll.min(max_scroll);
        }
        self.viewport_scroll.set(Some(scroll));
        scroll
    }

    fn navigate(&self, direction: SearchDirection, older_history_available: bool) -> SearchAction {
        if self.query.is_empty() {
            return SearchAction::Consumed;
        }
        if !self.matches_valid.get() {
            self.pending_direction.set(Some(direction));
            return SearchAction::ScheduleFrame;
        }
        let count = self.matches.borrow().len();
        if count == 0 {
            return self.request_older_history(older_history_available);
        }
        let current = self.cursor.get().min(count.saturating_sub(1));
        let next = match direction {
            SearchDirection::Older => current.checked_sub(1),
            SearchDirection::Newer => (current + 1 < count).then_some(current + 1),
        };
        let Some(next) = next else {
            self.boundary.set(match direction {
                SearchDirection::Older => SearchBoundary::Older,
                SearchDirection::Newer => SearchBoundary::Newer,
            });
            return if direction == SearchDirection::Older {
                self.request_older_history(older_history_available)
            } else {
                SearchAction::Consumed
            };
        };
        self.cursor.set(next);
        self.boundary.set(SearchBoundary::None);
        self.viewport_scroll.set(None);
        SearchAction::Consumed
    }

    fn request_older_history(&self, older_history_available: bool) -> SearchAction {
        if self.query.is_empty()
            || !older_history_available
            || self.history_state.get() == SearchHistoryState::Loading
        {
            SearchAction::Consumed
        } else {
            self.wants_history.set(false);
            SearchAction::LoadOlderHistory
        }
    }

    fn query_changed(&mut self) {
        self.folded_query = self.query.chars().flat_map(char::to_lowercase).collect();
        self.query_truncated = false;
        self.restart_scan(/*retain_match*/ false);
    }

    fn restart_scan(&self, retain_match: bool) {
        if !retain_match {
            self.preferred_match.borrow_mut().take();
        }
        self.restart_scan_shared();
    }

    fn restart_scan_shared(&self) {
        self.cursor.set(0);
        self.matches.borrow_mut().clear();
        self.matches_valid.set(self.query.is_empty());
        self.boundary.set(SearchBoundary::None);
        self.pending_direction.set(None);
        self.wants_history.set(false);
        self.viewport_scroll.set(None);
        let scan = (!self.query.is_empty())
            .then(|| self.source.borrow().as_ref().map(ScanState::new))
            .flatten();
        self.scan.replace(scan);
    }

    fn selected_identity(&self) -> Option<MatchIdentity> {
        let source = self.source.borrow();
        let source = source.as_ref()?;
        let selected = self.selected_match()?;
        Some(MatchIdentity {
            text: source.lines.get(selected.line)?.clone(),
            start: selected.start,
            end: selected.end,
        })
    }

    fn advance_scan(&self) {
        if self.matches_valid.get() {
            return;
        }
        let source = self.source.borrow();
        let Some(source) = source.as_ref() else {
            return;
        };
        let overlap = self.folded_query.len().saturating_mul(4);
        let extent = SEARCH_WINDOW_BYTES.saturating_add(overlap);
        let mut scanned_bytes = 0usize;
        let mut scanned_lines = 0usize;
        let mut scan_ref = self.scan.borrow_mut();
        let Some(scan) = scan_ref.as_mut() else {
            self.matches_valid.set(true);
            return;
        };

        while let Some(line_index) = scan.line {
            let Some(text) = source.lines.get(line_index) else {
                scan.line = line_index.checked_sub(1);
                continue;
            };
            if source.excluded_lines.contains(&line_index) {
                scan.line = line_index.checked_sub(1);
                scan.end = scan
                    .line
                    .and_then(|line| source.lines.get(line))
                    .map_or(0, String::len);
                scanned_lines += 1;
                if scanned_lines >= SEARCH_LINES_PER_FRAME {
                    break;
                }
                continue;
            }
            scan.end = floor_char_boundary(text, scan.end.min(text.len()));
            let start = floor_char_boundary(text, scan.end.saturating_sub(extent));
            let window = start..scan.end;
            for range in find_literal_ranges(&text[window.clone()], &self.folded_query) {
                scan.matches.push(SearchMatch {
                    line: line_index,
                    start: window.start + range.start,
                    end: window.start + range.end,
                    row_start: source
                        .row_starts
                        .get(line_index)
                        .copied()
                        .unwrap_or_default(),
                });
            }
            scanned_bytes = scanned_bytes.saturating_add(window.len());
            if start == 0 {
                scan.line = line_index.checked_sub(1);
                scan.end = scan
                    .line
                    .and_then(|line| source.lines.get(line))
                    .map_or(0, String::len);
                scanned_lines += 1;
            } else {
                scan.end = start.saturating_add(overlap).min(scan.end);
            }
            self.publish_matches(&scan.matches, source);
            if scanned_bytes >= SEARCH_WINDOW_BYTES || scanned_lines >= SEARCH_LINES_PER_FRAME {
                break;
            }
        }

        if scan.line.is_none() {
            self.publish_matches(&scan.matches, source);
            self.matches_valid.set(true);
            *scan_ref = None;
            self.finish_scan(source);
        }
    }

    fn publish_matches(&self, matches: &[SearchMatch], source: &SearchSource) {
        let current = self.matches.borrow().get(self.cursor.get()).copied();
        let mut next = matches.to_vec();
        next.sort_unstable();
        next.dedup();
        let had_matches = !self.matches.borrow().is_empty();
        let preferred = self.preferred_match.borrow();
        let preferred_cursor = preferred.as_ref().and_then(|preferred| {
            next.iter().position(|candidate| {
                candidate.start == preferred.start
                    && candidate.end == preferred.end
                    && source.lines.get(candidate.line) == Some(&preferred.text)
            })
        });
        let current_cursor =
            current.and_then(|current| next.iter().position(|candidate| *candidate == current));
        *self.matches.borrow_mut() = next;
        let count = self.matches.borrow().len();
        if count == 0 {
            self.cursor.set(0);
        } else if let Some(cursor) = preferred_cursor.or(current_cursor) {
            self.cursor.set(cursor);
        } else if !had_matches {
            self.cursor.set(count - 1);
        } else {
            self.cursor.set(self.cursor.get().min(count - 1));
        }
    }

    fn finish_scan(&self, source: &SearchSource) {
        self.preferred_match.borrow_mut().take();
        if let Some(direction) = self.pending_direction.take() {
            let _ = self.navigate(direction, /*older_history_available*/ false);
        }
        if self.matches.borrow().is_empty() || self.boundary.get() == SearchBoundary::Older {
            self.wants_history.set(true);
        }
        let _ = source;
    }
}

fn line_text(line: &HyperlinkLine) -> String {
    line.line
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect()
}

fn floor_char_boundary(text: &str, mut offset: usize) -> usize {
    offset = offset.min(text.len());
    while !text.is_char_boundary(offset) {
        offset = offset.saturating_sub(1);
    }
    offset
}

pub(crate) fn find_literal_ranges(text: &str, query: &str) -> Vec<Range<usize>> {
    let folded_query = query
        .chars()
        .flat_map(char::to_lowercase)
        .collect::<String>();
    if folded_query.is_empty() {
        return Vec::new();
    }
    let mut folded = String::new();
    let mut spans = Vec::new();
    for (start, character) in text.char_indices() {
        for lowercase in character.to_lowercase() {
            folded.push(lowercase);
            spans.push((folded.len(), start..start + character.len_utf8()));
        }
    }

    let mut matches = Vec::new();
    let mut search_from = 0;
    while let Some(relative) = folded[search_from..].find(&folded_query) {
        let start = search_from + relative;
        let end = start + folded_query.len();
        let first = spans.partition_point(|(folded_end, _)| *folded_end <= start);
        let last = spans.partition_point(|(folded_end, _)| *folded_end < end);
        if let (Some(first), Some(last)) = (spans.get(first), spans.get(last)) {
            matches.push(first.1.start..last.1.end);
        }
        search_from = start.saturating_add(folded_query.len().max(1));
        if search_from >= folded.len() {
            break;
        }
    }
    matches
}

fn highlight_search_lines(
    lines: &[HyperlinkLine],
    matches: &[SearchMatch],
    selected: usize,
) -> Vec<HyperlinkLine> {
    lines
        .iter()
        .enumerate()
        .map(|(line_index, source)| {
            let line_matches = matches
                .iter()
                .enumerate()
                .filter(|(_, search_match)| search_match.line == line_index)
                .collect::<Vec<_>>();
            if line_matches.is_empty() {
                return source.clone();
            }
            let mut line = source.clone();
            let mut offset = 0usize;
            let mut spans = Vec::new();
            for span in &line.line.spans {
                let span_start = offset;
                let span_end = span_start + span.content.len();
                for (grapheme_offset, grapheme) in span.content.grapheme_indices(true) {
                    let start = span_start + grapheme_offset;
                    let end = start + grapheme.len();
                    let matching = line_matches.iter().filter(|(_, search_match)| {
                        start < search_match.end && search_match.start < end
                    });
                    let mut style = span.style;
                    if matching.clone().next().is_some() {
                        let selected_match = matching.clone().any(|(index, _)| *index == selected);
                        let modifier = if selected_match {
                            Modifier::REVERSED | Modifier::BOLD
                        } else {
                            Modifier::REVERSED
                        };
                        style = style.add_modifier(modifier);
                    }
                    spans.push(Span::styled(grapheme.to_string(), style));
                }
                offset = span_end;
            }
            line.line.spans = spans;
            line
        })
        .collect()
}

#[cfg(test)]
#[path = "search_tests.rs"]
mod tests;
