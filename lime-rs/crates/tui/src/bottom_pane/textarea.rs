//! Editable UTF-8 text buffer shared by the TUI composer and focused input overlays.
//!
//! `TextArea` is deliberately independent from submission policy. The parent composer owns
//! history, queueing, and attachments while this module owns cursor-safe editing primitives.

use std::borrow::Cow;
use std::cell::{OnceCell, Ref, RefCell};
use std::ops::Range;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{StatefulWidgetRef, WidgetRef};
use unicode_segmentation::UnicodeSegmentation;

use crate::width::display_width;

mod hyperlinks;
mod vim;
mod vim_search;
mod wrapping;

use self::vim::VimPending;

const WORD_SEPARATORS: &str = "`~!@#$%^&*()-=+[{]}\\|;:'\",.<>/?";

fn is_word_separator(ch: char) -> bool {
    WORD_SEPARATORS.contains(ch)
}

fn split_word_pieces(run: &str) -> Vec<(usize, &str)> {
    let mut pieces = Vec::new();
    for (segment_start, segment) in run.split_word_bound_indices() {
        let mut piece_start = 0;
        let mut chars = segment.char_indices();
        let Some((_, first_char)) = chars.next() else {
            continue;
        };
        let mut in_separator = is_word_separator(first_char);
        for (idx, ch) in chars {
            let is_separator = is_word_separator(ch);
            if is_separator == in_separator {
                continue;
            }
            pieces.push((segment_start + piece_start, &segment[piece_start..idx]));
            piece_start = idx;
            in_separator = is_separator;
        }
        pieces.push((segment_start + piece_start, &segment[piece_start..]));
    }
    pieces
}

fn text_for_display(text: &str) -> Cow<'_, str> {
    if text.contains('\t') {
        Cow::Owned(text.replace('\t', " "))
    } else {
        Cow::Borrowed(text)
    }
}

fn editor_display_width(text: &str) -> usize {
    let display = text_for_display(text);
    display_width(display.as_ref())
}

#[derive(Debug, Default)]
pub(crate) struct TextArea {
    text: String,
    cursor: usize,
    kill_buffer: String,
    wrap_cache: RefCell<Option<WrapCache>>,
    preferred_col: Option<usize>,
    vim_enabled: bool,
    vim_mode: vim::VimMode,
    vim_pending: VimPending,
    vim_search: vim_search::VimSearch,
    vim_replace_steps: Vec<vim::VimReplaceStep>,
}

#[derive(Debug)]
struct WrapCache {
    width: u16,
    lines: Vec<Range<usize>>,
    hyperlinks: OnceCell<hyperlinks::HyperlinkCache>,
}

/// Viewport state kept outside the editable buffer, matching Codex's stateful textarea widget.
#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct TextAreaState {
    /// Index into wrapped lines of the first visible line.
    scroll: u16,
}

#[allow(dead_code)]
impl TextArea {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }

    pub(crate) fn cursor(&self) -> usize {
        self.cursor
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    pub(crate) fn replace(&mut self, text: String) {
        self.text = text;
        self.cursor = self.text.len();
        self.preferred_col = None;
        self.vim_pending = VimPending::None;
        self.vim_search.cancel();
        self.vim_replace_steps.clear();
        self.invalidate_wrap_cache();
    }

    pub(crate) fn set_text_clearing_elements(&mut self, text: &str) {
        self.text = text.to_string();
        self.cursor = self.nearest_char_boundary(self.cursor.min(self.text.len()));
        self.preferred_col = None;
        self.vim_pending = VimPending::None;
        self.vim_search.cancel();
        self.vim_replace_steps.clear();
        self.invalidate_wrap_cache();
    }

    pub(crate) fn take(&mut self) -> String {
        self.cursor = 0;
        self.preferred_col = None;
        self.vim_pending = VimPending::None;
        self.vim_search.cancel();
        self.vim_replace_steps.clear();
        let text = std::mem::take(&mut self.text);
        self.invalidate_wrap_cache();
        text
    }

    pub(crate) fn insert(&mut self, value: &str) {
        self.text.insert_str(self.cursor, value);
        self.cursor += value.len();
        self.preferred_col = None;
        self.invalidate_wrap_cache();
    }

    pub(crate) fn insert_str(&mut self, value: &str) {
        self.insert(value);
    }

    pub(crate) fn insert_str_at(&mut self, pos: usize, value: &str) {
        let pos = self.nearest_char_boundary(pos.min(self.text.len()));
        self.text.insert_str(pos, value);
        if pos <= self.cursor {
            self.cursor += value.len();
        }
        self.preferred_col = None;
        self.invalidate_wrap_cache();
    }

    pub(crate) fn replace_range(&mut self, range: Range<usize>, value: &str) {
        let start = self.nearest_char_boundary(range.start.min(self.text.len()));
        let end = self.nearest_char_boundary(range.end.min(self.text.len()));
        if start > end {
            return;
        }
        self.text.replace_range(start..end, value);
        self.cursor = if self.cursor < start {
            self.cursor
        } else if self.cursor <= end {
            start + value.len()
        } else {
            self.cursor.saturating_sub(end - start) + value.len()
        }
        .min(self.text.len());
        self.preferred_col = None;
        self.invalidate_wrap_cache();
    }

    pub(crate) fn set_cursor(&mut self, pos: usize) {
        self.cursor = self.nearest_char_boundary(pos.min(self.text.len()));
        self.preferred_col = None;
    }

    pub(crate) fn move_left(&mut self) {
        self.cursor = self.previous_grapheme_start();
        self.preferred_col = None;
    }

    pub(crate) fn move_right(&mut self) {
        self.cursor = self.next_grapheme_end();
        self.preferred_col = None;
    }

    pub(crate) fn move_line_start(&mut self) {
        self.cursor = self.line_start();
        self.preferred_col = None;
    }

    pub(crate) fn move_line_end(&mut self) {
        self.cursor = self.line_end();
        self.preferred_col = None;
    }

    /// Move to the adjacent visual line while preserving the terminal column.
    ///
    /// When wrapping information is available, navigation follows the same visual rows rendered
    /// by the textarea. Without a cache (for example before the first render), it falls back to
    /// logical-line navigation. The target is always chosen on grapheme boundaries, so wide
    /// characters and combining marks can never leave the cursor inside an UTF-8 sequence.
    pub(crate) fn move_up(&mut self) {
        if !self.move_visual_vertical(-1) {
            self.move_insert_vertical(-1);
        }
    }

    pub(crate) fn move_down(&mut self) {
        if !self.move_visual_vertical(1) {
            self.move_insert_vertical(1);
        }
    }

    /// Returns whether history navigation may consume a vertical key at the current visual row.
    ///
    /// Once the textarea has been rendered, wrapped rows are the editor's navigation surface;
    /// history is only eligible at the outermost row. Before the first render there is no width
    /// to resolve, so callers retain the legacy boundary behavior.
    pub(crate) fn is_vertical_boundary(&self, direction: i8) -> bool {
        let cache_ref = self.wrap_cache.borrow();
        let Some(cache) = cache_ref.as_ref() else {
            return true;
        };
        let Some((row, _)) =
            wrapping::cursor_position(&self.text, &cache.lines, cache.width, self.cursor)
        else {
            return true;
        };
        if direction < 0 {
            row == 0
        } else {
            row + 1 >= cache.lines.len()
        }
    }

    /// Move across wrapped rows when the current render width is known.
    ///
    /// The returned boolean distinguishes an unavailable cache from a real boundary move. A
    /// boundary move still consumes the event and resets the saved column, matching Codex's
    /// behavior when moving above the first or below the last visual row.
    fn move_visual_vertical(&mut self, direction: i8) -> bool {
        enum Target {
            Line {
                start: usize,
                end: usize,
                column: usize,
            },
            Boundary(usize),
        }

        let target = {
            let cache_ref = self.wrap_cache.borrow();
            let Some(cache) = cache_ref.as_ref() else {
                return false;
            };
            let Some((row, current_column)) =
                wrapping::cursor_position(&self.text, &cache.lines, cache.width, self.cursor)
            else {
                return false;
            };
            let column = self
                .preferred_col
                .unwrap_or(current_column)
                .min(usize::from(cache.width.saturating_sub(1)));

            if direction < 0 {
                if let Some(previous) = row.checked_sub(1) {
                    let current = &cache.lines[row];
                    let previous = &cache.lines[previous];
                    let start = previous.start;
                    let mut end = previous.end.saturating_sub(1);
                    if end == current.start {
                        end = self.previous_grapheme_start_at(end).max(start);
                    }
                    Target::Line { start, end, column }
                } else {
                    Target::Boundary(0)
                }
            } else if let Some(next) = cache.lines.get(row + 1) {
                let start = next.start;
                let mut end = next.end.saturating_sub(1);
                if cache
                    .lines
                    .get(row + 2)
                    .is_some_and(|following| following.start == end)
                {
                    end = self.previous_grapheme_start_at(end).max(start);
                }
                Target::Line { start, end, column }
            } else {
                Target::Boundary(self.text.len())
            }
        };

        match target {
            Target::Line { start, end, column } => {
                if self.preferred_col.is_none() {
                    self.preferred_col = Some(column);
                }
                self.move_to_display_col_on_line(start, end, column);
            }
            Target::Boundary(cursor) => {
                self.cursor = cursor;
                self.preferred_col = None;
            }
        }
        true
    }

    fn move_insert_vertical(&mut self, direction: i8) {
        let current_start = self.line_start();
        let current_column = self
            .preferred_col
            .unwrap_or_else(|| editor_display_width(&self.text[current_start..self.cursor]));
        let target_start = if direction < 0 {
            if current_start == 0 {
                self.preferred_col = None;
                return;
            }
            let previous_end = current_start - 1;
            self.text[..previous_end]
                .rfind('\n')
                .map_or(0, |index| index + 1)
        } else {
            let current_end = self.line_end();
            if current_end == self.text.len() {
                self.preferred_col = None;
                return;
            }
            current_end + 1
        };
        let target_end = self.text[target_start..]
            .find('\n')
            .map_or(self.text.len(), |offset| target_start + offset);
        self.cursor = cursor_at_display_column(
            &self.text[target_start..target_end],
            target_start,
            current_column,
        );
        if self.preferred_col.is_none() {
            self.preferred_col = Some(current_column);
        }
    }

    pub(crate) fn remove_previous_grapheme(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        let start = self.previous_grapheme_start();
        self.text.replace_range(start..self.cursor, "");
        self.cursor = start;
        self.preferred_col = None;
        self.invalidate_wrap_cache();
        true
    }

    pub(crate) fn remove_next_grapheme(&mut self) -> bool {
        if self.cursor == self.text.len() {
            return false;
        }
        let end = self.next_grapheme_end();
        self.text.replace_range(self.cursor..end, "");
        self.preferred_col = None;
        self.invalidate_wrap_cache();
        true
    }

    /// Apply the default Codex editor key semantics without submission policy.
    ///
    /// `ChatComposer` owns submit/queue/shortcut dispatch. This method is the shared low-level
    /// editor boundary used by focused overlays and the disconnected composer path.
    pub(crate) fn input(&mut self, event: KeyEvent) {
        if !matches!(event.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return;
        }

        if self.vim_enabled && self.handle_vim_search_key(event) {
            return;
        }

        if self.vim_enabled {
            self.handle_vim_input(event);
        } else {
            self.input_insert_mode(event);
        }
    }

    fn input_insert_mode(&mut self, event: KeyEvent) {
        let control = event.modifiers.contains(KeyModifiers::CONTROL);
        let alt = event.modifiers.contains(KeyModifiers::ALT);
        if crate::key_hint::is_altgr(event.modifiers) {
            if let KeyCode::Char(ch) = event.code {
                self.insert_str(&ch.to_string());
                return;
            }
        }
        match event.code {
            KeyCode::Char('\u{0001}') => self.move_line_start(),
            KeyCode::Char('\u{0002}') => self.move_left(),
            KeyCode::Char('\u{0005}') => self.move_line_end(),
            KeyCode::Char('\u{0006}') => self.move_right(),
            KeyCode::Char('\u{000a}' | '\u{000d}') => self.insert_str("\n"),
            KeyCode::Char('\u{000e}') => self.move_down(),
            KeyCode::Char('\u{0010}') => self.move_up(),
            KeyCode::Char('\u{0008}') => {
                self.remove_previous_grapheme();
            }
            KeyCode::Char('h') if control => {
                self.remove_previous_grapheme();
            }
            KeyCode::Char('m') if control => self.insert_str("\n"),
            KeyCode::Char('b') if control => self.move_left(),
            KeyCode::Char('f') if control => self.move_right(),
            KeyCode::Char('a') if control => self.move_line_start(),
            KeyCode::Char('e') if control => self.move_line_end(),
            KeyCode::Char('p') if control => self.move_up(),
            KeyCode::Char('n') if control => self.move_down(),
            KeyCode::Char('w') if control || alt => self.delete_backward_word(),
            KeyCode::Char('d') if alt => self.delete_forward_word(),
            KeyCode::Char('u') if control => self.kill_to_beginning_of_line(),
            KeyCode::Char('k') if control => self.kill_to_end_of_line(),
            KeyCode::Char('y') if control => self.yank(),
            KeyCode::Backspace if control || alt => self.delete_backward_word(),
            KeyCode::Delete if control || alt => self.delete_forward_word(),
            KeyCode::Backspace => {
                self.remove_previous_grapheme();
            }
            KeyCode::Delete => {
                self.remove_next_grapheme();
            }
            KeyCode::Left => self.move_left(),
            KeyCode::Right => self.move_right(),
            KeyCode::Up => self.move_up(),
            KeyCode::Down => self.move_down(),
            KeyCode::Home => self.move_line_start(),
            KeyCode::End => self.move_line_end(),
            KeyCode::Enter if !control => self.insert_str("\n"),
            KeyCode::Char(ch)
                if !control
                    && !alt
                    && !ch.is_ascii_control()
                    && (event.modifiers.is_empty() || event.modifiers == KeyModifiers::SHIFT) =>
            {
                self.insert_str(&ch.to_string());
            }
            _ => {}
        }
    }

    pub(crate) fn delete_backward(&mut self, n: usize) {
        for _ in 0..n {
            if !self.remove_previous_grapheme() {
                break;
            }
        }
    }

    pub(crate) fn delete_forward(&mut self, n: usize) {
        for _ in 0..n {
            if !self.remove_next_grapheme() {
                break;
            }
        }
    }

    pub(crate) fn delete_forward_kill(&mut self, n: usize) {
        if n == 0 || self.cursor >= self.text.len() {
            return;
        }
        let mut target = self.cursor;
        for _ in 0..n {
            target = self.next_grapheme_end_at(target);
            if target >= self.text.len() {
                break;
            }
        }
        self.kill_range(self.cursor..target);
    }

    pub(crate) fn delete_backward_word(&mut self) {
        let start = self.beginning_of_previous_word();
        self.kill_range(start..self.cursor);
    }

    /// Delete text to the right of the cursor through the next word boundary.
    pub(crate) fn delete_forward_word(&mut self) {
        let end = self.end_of_next_word();
        if end > self.cursor {
            self.kill_range(self.cursor..end);
        }
    }

    /// Kill from the cursor to the end of the current logical line.
    pub(crate) fn kill_to_end_of_line(&mut self) {
        let eol = self.line_end();
        let range = if self.cursor == eol {
            (eol < self.text.len()).then_some(self.cursor..eol + 1)
        } else {
            Some(self.cursor..eol)
        };
        if let Some(range) = range {
            self.kill_range(range);
        }
    }

    /// Kill from the beginning of the current logical line through the cursor.
    pub(crate) fn kill_to_beginning_of_line(&mut self) {
        let bol = self.line_start();
        let range = if self.cursor == bol {
            (bol > 0).then_some(bol - 1..bol)
        } else {
            Some(bol..self.cursor)
        };
        if let Some(range) = range {
            self.kill_range(range);
        }
    }

    /// Insert the most recently killed text at the cursor.
    pub(crate) fn yank(&mut self) {
        if !self.kill_buffer.is_empty() {
            let text = self.kill_buffer.clone();
            self.insert_str(&text);
        }
    }

    pub(crate) fn beginning_of_previous_word(&self) -> usize {
        let prefix = &self.text[..self.cursor];
        let Some((first_non_ws_idx, ch)) = prefix
            .char_indices()
            .rev()
            .find(|&(_, ch)| !ch.is_whitespace())
        else {
            return 0;
        };
        let run_start = prefix[..first_non_ws_idx]
            .char_indices()
            .rev()
            .find(|&(_, ch)| ch.is_whitespace())
            .map_or(0, |(idx, ch)| idx + ch.len_utf8());
        let run_end = first_non_ws_idx + ch.len_utf8();
        let pieces = split_word_pieces(&prefix[run_start..run_end]);
        let mut pieces = pieces.into_iter().rev().peekable();
        let Some((piece_start, piece)) = pieces.next() else {
            return run_start;
        };
        let mut start = run_start + piece_start;
        if piece.chars().all(is_word_separator) {
            while let Some((idx, piece)) = pieces.peek() {
                if !piece.chars().all(is_word_separator) {
                    break;
                }
                start = run_start + *idx;
                pieces.next();
            }
        }
        start
    }

    pub(crate) fn end_of_next_word(&self) -> usize {
        let suffix = &self.text[self.cursor..];
        let Some(first_non_ws) = suffix.find(|ch: char| !ch.is_whitespace()) else {
            return self.text.len();
        };
        let run = &suffix[first_non_ws..];
        let run = &run[..run.find(char::is_whitespace).unwrap_or(run.len())];
        let mut pieces = split_word_pieces(run).into_iter().peekable();
        let Some((start, piece)) = pieces.next() else {
            return self.cursor + first_non_ws;
        };
        let word_start = self.cursor + first_non_ws + start;
        let mut end = word_start + piece.len();
        if piece.chars().all(is_word_separator) {
            while let Some((idx, piece)) = pieces.peek() {
                if !piece.chars().all(is_word_separator) {
                    break;
                }
                end = self.cursor + first_non_ws + *idx + piece.len();
                pieces.next();
            }
        }
        end
    }

    fn kill_range(&mut self, range: Range<usize>) {
        let start = self.nearest_char_boundary(range.start.min(self.text.len()));
        let end = self.nearest_char_boundary(range.end.min(self.text.len()));
        if start >= end {
            return;
        }
        self.kill_buffer = self.text[start..end].to_string();
        self.replace_range(start..end, "");
    }

    pub(crate) fn wrapped_lines(&self, width: u16) -> Ref<'_, Vec<std::ops::Range<usize>>> {
        {
            let mut cache = self.wrap_cache.borrow_mut();
            let needs_recalc = cache.as_ref().is_none_or(|cache| cache.width != width);
            if needs_recalc {
                let display_text = text_for_display(&self.text);
                *cache = Some(WrapCache {
                    width,
                    lines: wrapping::wrapped_lines(display_text.as_ref(), width),
                    hyperlinks: OnceCell::new(),
                });
            }
        }

        let cache = self.wrap_cache.borrow();
        Ref::map(cache, |cache| {
            &cache
                .as_ref()
                .expect("textarea wrap cache initialized")
                .lines
        })
    }

    pub(crate) fn cursor_position(&self, width: u16) -> Option<(usize, usize)> {
        let lines = self.wrapped_lines(width);
        wrapping::cursor_position(&self.text, &lines, width, self.cursor)
    }

    pub(crate) fn cursor_pos(&self, area: Rect) -> Option<(u16, u16)> {
        self.cursor_pos_with_state(area, TextAreaState::default())
    }

    pub(crate) fn cursor_pos_with_state(
        &self,
        area: Rect,
        state: TextAreaState,
    ) -> Option<(u16, u16)> {
        if area.is_empty() {
            return None;
        }
        let lines = self.wrapped_lines(area.width);
        let scroll = self.effective_scroll(area, &lines, state.scroll);
        let (row, column) = wrapping::cursor_position(&self.text, &lines, area.width, self.cursor)?;
        Some((
            area.x.saturating_add(column as u16),
            area.y
                .saturating_add(row.saturating_sub(scroll as usize) as u16)
                .min(area.bottom().saturating_sub(1)),
        ))
    }

    pub(crate) fn desired_height(&self, width: u16) -> u16 {
        self.wrapped_lines(width).len().max(1) as u16
    }

    fn effective_scroll(&self, area: Rect, lines: &[Range<usize>], current: u16) -> u16 {
        if area.height == 0 || lines.is_empty() {
            return 0;
        }
        let total = lines.len() as u16;
        if total <= area.height {
            return 0;
        }
        let cursor_row = wrapping::cursor_position(&self.text, lines, area.width, self.cursor)
            .map(|(row, _)| row as u16)
            .unwrap_or_default();
        let max_scroll = total.saturating_sub(area.height);
        let mut scroll = current.min(max_scroll);
        if cursor_row < scroll {
            scroll = cursor_row;
        } else if cursor_row >= scroll.saturating_add(area.height) {
            scroll = cursor_row
                .saturating_add(1)
                .saturating_sub(area.height)
                .min(max_scroll);
        }
        scroll
    }

    fn invalidate_wrap_cache(&mut self) {
        self.wrap_cache.get_mut().take();
        self.preferred_col = None;
    }

    fn nearest_char_boundary(&self, mut pos: usize) -> usize {
        while pos > 0 && !self.text.is_char_boundary(pos) {
            pos -= 1;
        }
        pos
    }

    fn render_lines(&self, area: Rect, buf: &mut Buffer, lines: &[Range<usize>], scroll: u16) {
        let blank = " ".repeat(usize::from(area.width));
        for row in 0..area.height {
            buf.set_string(area.x, area.y + row, &blank, Style::default());
        }
        let start = usize::from(scroll);
        let end = (start + usize::from(area.height)).min(lines.len());
        for (row, range) in lines[start..end].iter().enumerate() {
            let content_end = range.end.saturating_sub(1).min(self.text.len());
            let content_start = range.start.min(content_end);
            let visible =
                wrapping::visible_prefix(&self.text[content_start..content_end], area.width);
            buf.set_string(
                area.x,
                area.y + row as u16,
                text_for_display(visible),
                Style::default(),
            );
        }
        if let Some(wrap_cache) = self.wrap_cache.borrow().as_ref() {
            wrap_cache
                .hyperlinks
                .get_or_init(|| hyperlinks::HyperlinkCache::new(&self.text, lines))
                .mark(buf, area, &self.text, lines, start..end);
        }
    }

    /// Render the textarea with a fixed-width mask without exposing hyperlink destinations.
    pub(crate) fn render_ref_masked(
        &self,
        area: Rect,
        buf: &mut Buffer,
        state: &mut TextAreaState,
        mask_char: char,
    ) {
        let lines = self.wrapped_lines(area.width);
        state.scroll = self.effective_scroll(area, &lines, state.scroll);
        let start = usize::from(state.scroll);
        let end = (start + usize::from(area.height)).min(lines.len());
        let blank = " ".repeat(usize::from(area.width));
        for row in 0..area.height {
            buf.set_string(area.x, area.y + row, &blank, Style::default());
        }
        for (row, range) in lines[start..end].iter().enumerate() {
            let content_end = range.end.saturating_sub(1).min(self.text.len());
            let content_start = range.start.min(content_end);
            let visible =
                wrapping::visible_prefix(&self.text[content_start..content_end], area.width);
            let masked = visible
                .graphemes(true)
                .flat_map(|grapheme| {
                    std::iter::repeat_n(mask_char, crate::width::display_width(grapheme))
                })
                .collect::<String>();
            buf.set_string(area.x, area.y + row as u16, masked, Style::default());
        }
    }

    /// Render the textarea with render-only highlight ranges and preserve OSC 8 annotations.
    pub(crate) fn render_ref_styled_with_highlights(
        &self,
        area: Rect,
        buf: &mut Buffer,
        state: &mut TextAreaState,
        base_style: Style,
        highlights: &[(Range<usize>, Style)],
    ) {
        let lines = self.wrapped_lines(area.width);
        state.scroll = self.effective_scroll(area, &lines, state.scroll);
        let start = usize::from(state.scroll);
        let end = (start + usize::from(area.height)).min(lines.len());
        let blank = " ".repeat(usize::from(area.width));
        for row in 0..area.height {
            buf.set_string(area.x, area.y + row, &blank, base_style);
        }
        for (row, range) in lines[start..end].iter().enumerate() {
            let content_end = range.end.saturating_sub(1).min(self.text.len());
            let content_start = range.start.min(content_end);
            let visible =
                wrapping::visible_prefix(&self.text[content_start..content_end], area.width);
            let line_range = content_start..content_start + visible.len();
            let y = area.y + row as u16;
            buf.set_stringn(
                area.x,
                y,
                text_for_display(visible),
                usize::from(area.width),
                base_style,
            );
            for (highlight_range, style) in highlights {
                let overlap_start = highlight_range.start.max(line_range.start);
                let overlap_end = highlight_range.end.min(line_range.end);
                if overlap_start >= overlap_end {
                    continue;
                }
                let x = area.x
                    + crate::width::display_width(&self.text[line_range.start..overlap_start])
                        as u16;
                buf.set_stringn(
                    x,
                    y,
                    text_for_display(&self.text[overlap_start..overlap_end]),
                    usize::from(area.width.saturating_sub(x.saturating_sub(area.x))),
                    *style,
                );
            }
        }
        if let Some(wrap_cache) = self.wrap_cache.borrow().as_ref() {
            wrap_cache
                .hyperlinks
                .get_or_init(|| hyperlinks::HyperlinkCache::new(&self.text, &lines))
                .mark(buf, area, &self.text, &lines, start..end);
        }
    }

    fn previous_grapheme_start(&self) -> usize {
        self.previous_grapheme_start_at(self.cursor)
    }

    fn previous_grapheme_start_at(&self, cursor: usize) -> usize {
        self.text[..cursor]
            .grapheme_indices(true)
            .next_back()
            .map(|(index, _)| index)
            .unwrap_or(0)
    }

    fn move_to_display_col_on_line(&mut self, line_start: usize, line_end: usize, target: usize) {
        let mut column: usize = 0;
        for (offset, grapheme) in self.text[line_start..line_end].grapheme_indices(true) {
            let width = editor_display_width(grapheme);
            if column.saturating_add(width) > target {
                self.cursor = line_start + offset;
                return;
            }
            column = column.saturating_add(width);
        }
        self.cursor = line_end;
    }

    fn next_grapheme_end(&self) -> usize {
        self.next_grapheme_end_at(self.cursor)
    }

    fn next_grapheme_end_at(&self, cursor: usize) -> usize {
        self.text[cursor..]
            .graphemes(true)
            .next()
            .map(|grapheme| cursor + grapheme.len())
            .unwrap_or(self.text.len())
    }

    fn line_start(&self) -> usize {
        self.text[..self.cursor]
            .rfind('\n')
            .map(|index| index + 1)
            .unwrap_or(0)
    }

    fn line_end(&self) -> usize {
        self.text[self.cursor..]
            .find('\n')
            .map(|index| self.cursor + index)
            .unwrap_or(self.text.len())
    }
}

fn cursor_at_display_column(line: &str, line_start: usize, target_column: usize) -> usize {
    if target_column == 0 {
        return line_start;
    }

    let mut column: usize = 0;
    for (offset, grapheme) in line.grapheme_indices(true) {
        column = column.saturating_add(editor_display_width(grapheme));
        if column > target_column {
            return line_start + offset;
        }
    }
    line_start + line.len()
}

impl WidgetRef for &TextArea {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let lines = self.wrapped_lines(area.width);
        self.render_lines(area, buf, &lines, 0);
    }
}

impl StatefulWidgetRef for &TextArea {
    type State = TextAreaState;

    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let lines = self.wrapped_lines(area.width);
        state.scroll = self.effective_scroll(area, &lines, state.scroll);
        self.render_lines(area, buf, &lines, state.scroll);
    }
}

#[cfg(test)]
mod tests {
    use super::{TextArea, TextAreaState};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::layout::Rect;
    use ratatui::widgets::StatefulWidgetRef;

    #[test]
    fn unicode_cursor_and_grapheme_deletion_match_codex_textarea_semantics() {
        let mut textarea = TextArea::default();
        textarea.insert("a👩🏽‍💻界");
        textarea.move_left();
        assert!(textarea.remove_previous_grapheme());
        assert_eq!(textarea.text(), "a界");
        assert_eq!(textarea.cursor(), 1);
    }

    #[test]
    fn line_navigation_stays_inside_the_current_logical_line() {
        let mut textarea = TextArea::default();
        textarea.insert("first\nsecond");
        textarea.move_line_start();
        assert_eq!(textarea.cursor(), 6);
        textarea.move_line_end();
        assert_eq!(textarea.cursor(), 12);
    }

    #[test]
    fn codex_cursor_pos_api_keeps_cursor_visible_in_a_scrolled_viewport() {
        let mut textarea = TextArea::new();
        textarea.insert_str("abcdefghij");
        textarea.set_cursor(textarea.text().len());

        let area = Rect::new(2, 3, 4, 2);
        let state = TextAreaState::default();
        let (x, y) = textarea
            .cursor_pos_with_state(area, state)
            .expect("cursor position");

        assert_eq!((x, y), (area.x + 2, area.y + 1));
    }

    #[test]
    fn codex_buffer_replacement_preserves_cursor_only_at_valid_boundaries() {
        let mut textarea = TextArea::new();
        textarea.insert_str("ab");
        textarea.set_cursor(1);
        textarea.set_text_clearing_elements("cd");

        assert_eq!(textarea.text(), "cd");
        assert_eq!(textarea.cursor(), 1);
    }

    #[test]
    fn stateful_render_clears_rows_outside_the_current_draft() {
        let area = Rect::new(0, 0, 6, 2);
        let mut textarea = TextArea::new();
        textarea.insert_str("longer draft");
        let mut buffer = ratatui::buffer::Buffer::empty(area);
        let mut state = TextAreaState::default();
        StatefulWidgetRef::render_ref(&&textarea, area, &mut buffer, &mut state);

        textarea.set_text_clearing_elements("ok");
        StatefulWidgetRef::render_ref(&&textarea, area, &mut buffer, &mut state);

        assert_eq!(buffer[(0, 0)].symbol(), "o");
        assert_eq!(buffer[(2, 0)].symbol(), " ");
        assert_eq!(buffer[(0, 1)].symbol(), " ");
    }

    #[test]
    fn codex_word_delete_and_line_kill_preserve_unicode_boundaries() {
        let mut textarea = TextArea::new();
        textarea.insert_str("alpha 你👍 beta");
        textarea.set_cursor(textarea.text().len());
        textarea.delete_backward_word();
        assert_eq!(textarea.text(), "alpha 你👍 ");
        textarea.yank();
        assert_eq!(textarea.text(), "alpha 你👍 beta");

        textarea.set_cursor(9);
        textarea.kill_to_beginning_of_line();
        assert_eq!(textarea.text(), "👍 beta");
        textarea.yank();
        assert_eq!(textarea.text(), "alpha 你👍 beta");
    }

    #[test]
    fn codex_input_dispatches_emacs_style_editor_bindings() {
        let mut textarea = TextArea::new();
        textarea.input(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        textarea.input(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE));
        textarea.input(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::NONE));
        textarea.input(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::CONTROL));
        textarea.input(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL));
        assert_eq!(textarea.text(), "ab");
        textarea.input(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::CONTROL));
        assert_eq!(textarea.text(), "abc");
    }

    #[test]
    fn codex_control_h_deletes_and_control_m_inserts_newline() {
        let mut textarea = TextArea::new();
        textarea.insert_str("ab");
        textarea.input(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::CONTROL));
        assert_eq!(textarea.text(), "a");
        textarea.input(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::CONTROL));
        assert_eq!(textarea.text(), "a\n");
    }

    #[test]
    fn c0_line_feed_and_emacs_vertical_motion_match_codex_textarea_semantics() {
        let mut textarea = TextArea::new();
        textarea.insert_str("ab\ncdef");
        textarea.set_cursor(2);
        textarea.input(KeyEvent::new(KeyCode::Char('\u{000a}'), KeyModifiers::NONE));
        assert_eq!(textarea.text(), "ab\n\ncdef");

        textarea.replace("ab\ncdef".to_string());
        textarea.set_cursor(2);
        textarea.input(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL));
        assert_eq!(textarea.cursor(), 5);
        textarea.input(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL));
        assert_eq!(textarea.cursor(), 2);
    }

    #[test]
    fn vertical_motion_preserves_display_column_for_wide_graphemes() {
        let mut textarea = TextArea::new();
        textarea.insert_str("界a\nxy");
        textarea.set_cursor(3);
        textarea.move_down();
        assert_eq!(textarea.cursor(), 7);
        textarea.move_up();
        assert_eq!(textarea.cursor(), 3);
    }

    #[test]
    fn codex_word_boundaries_handle_cjk_and_separator_runs() {
        let mut textarea = TextArea::new();
        textarea.insert_str("你好::world");
        textarea.set_cursor(textarea.text().len());
        assert_eq!(textarea.beginning_of_previous_word(), 8);
        textarea.delete_backward_word();
        assert_eq!(textarea.text(), "你好::");
        textarea.set_cursor(0);
        assert_eq!(textarea.end_of_next_word(), 3);
    }
}
