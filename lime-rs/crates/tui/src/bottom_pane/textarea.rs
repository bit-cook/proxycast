//! Editable UTF-8 text buffer shared by the TUI composer and focused input overlays.
//!
//! `TextArea` is deliberately independent from submission policy. The parent composer owns
//! history, queueing, and attachments while this module owns cursor-safe editing primitives.

use std::borrow::Cow;
use std::ops::Range;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::{StatefulWidgetRef, WidgetRef};
use unicode_segmentation::UnicodeSegmentation;

mod wrapping;

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

#[derive(Debug, Default)]
pub(crate) struct TextArea {
    text: String,
    cursor: usize,
    kill_buffer: String,
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
    }

    pub(crate) fn set_text_clearing_elements(&mut self, text: &str) {
        self.text = text.to_string();
        self.cursor = self.nearest_char_boundary(self.cursor.min(self.text.len()));
    }

    pub(crate) fn take(&mut self) -> String {
        self.cursor = 0;
        std::mem::take(&mut self.text)
    }

    pub(crate) fn insert(&mut self, value: &str) {
        self.text.insert_str(self.cursor, value);
        self.cursor += value.len();
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
    }

    pub(crate) fn set_cursor(&mut self, pos: usize) {
        self.cursor = self.nearest_char_boundary(pos.min(self.text.len()));
    }

    pub(crate) fn move_left(&mut self) {
        self.cursor = self.previous_grapheme_start();
    }

    pub(crate) fn move_right(&mut self) {
        self.cursor = self.next_grapheme_end();
    }

    pub(crate) fn move_line_start(&mut self) {
        self.cursor = self.line_start();
    }

    pub(crate) fn move_line_end(&mut self) {
        self.cursor = self.line_end();
    }

    pub(crate) fn remove_previous_grapheme(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        let start = self.previous_grapheme_start();
        self.text.replace_range(start..self.cursor, "");
        self.cursor = start;
        true
    }

    pub(crate) fn remove_next_grapheme(&mut self) -> bool {
        if self.cursor == self.text.len() {
            return false;
        }
        let end = self.next_grapheme_end();
        self.text.replace_range(self.cursor..end, "");
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

        let control = event.modifiers.contains(KeyModifiers::CONTROL);
        let alt = event.modifiers.contains(KeyModifiers::ALT);
        match event.code {
            KeyCode::Char('b') if control => self.move_left(),
            KeyCode::Char('f') if control => self.move_right(),
            KeyCode::Char('a') if control => self.move_line_start(),
            KeyCode::Char('e') if control => self.move_line_end(),
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
            KeyCode::Home => self.move_line_start(),
            KeyCode::End => self.move_line_end(),
            KeyCode::Enter if !control && !alt => self.insert_str("\n"),
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

    pub(crate) fn wrapped_lines(&self, width: u16) -> Vec<std::ops::Range<usize>> {
        wrapping::wrapped_lines(&self.text, width)
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
    }

    fn previous_grapheme_start(&self) -> usize {
        self.text[..self.cursor]
            .grapheme_indices(true)
            .next_back()
            .map(|(index, _)| index)
            .unwrap_or(0)
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
