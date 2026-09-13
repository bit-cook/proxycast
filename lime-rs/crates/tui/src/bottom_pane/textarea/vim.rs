use super::split_word_pieces;
use super::TextArea;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::style::{Color, Style};
use ratatui::text::Span;
use std::ops::Range;
use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum VimMode {
    Normal,
    #[default]
    Insert,
    Replace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VimOperator {
    Delete,
    Change,
    Yank,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum VimPending {
    #[default]
    None,
    Operator(VimOperator),
    TextObject {
        operator: VimOperator,
        scope: VimTextObjectScope,
    },
    Find {
        motion: VimFindMotion,
        operator: Option<VimOperator>,
    },
    ReplaceChar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VimFindMotion {
    Forward,
    Backward,
    TillForward,
    TillBackward,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VimTextObjectScope {
    Inner,
    Around,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VimTextObject {
    Word,
    BigWord,
    Parentheses,
    Brackets,
    Braces,
    DoubleQuote,
    SingleQuote,
    Backtick,
}

#[derive(Debug)]
pub(super) struct VimReplaceStep {
    cursor_before: usize,
    start: usize,
    inserted_len: usize,
    original: String,
}

impl TextArea {
    pub(crate) fn set_vim_enabled(&mut self, enabled: bool) {
        self.vim_enabled = enabled;
        self.vim_pending = VimPending::None;
        self.vim_search.cancel();
        self.vim_replace_steps.clear();
        self.preferred_col = None;
        self.vim_mode = if enabled {
            VimMode::Normal
        } else {
            VimMode::Insert
        };
    }

    pub(crate) fn is_vim_enabled(&self) -> bool {
        self.vim_enabled
    }

    pub(crate) fn is_vim_normal_mode(&self) -> bool {
        self.vim_enabled && self.vim_mode == VimMode::Normal
    }

    fn vim_normal_end_cursor(&self) -> usize {
        if self.text.is_empty() {
            0
        } else {
            self.text[..self.text.len()]
                .grapheme_indices(true)
                .next_back()
                .map(|(index, _)| index)
                .unwrap_or(0)
        }
    }

    pub(crate) fn is_vim_operator_pending(&self) -> bool {
        !matches!(self.vim_pending, VimPending::None)
    }

    pub(crate) fn should_handle_vim_insert_escape(&self, event: KeyEvent) -> bool {
        self.vim_enabled
            && event.code == KeyCode::Esc
            && event.modifiers == KeyModifiers::NONE
            && matches!(event.kind, KeyEventKind::Press | KeyEventKind::Repeat)
            && (self.vim_mode != VimMode::Normal || self.is_vim_operator_pending())
    }

    pub(crate) fn vim_mode_indicator_span(&self) -> Option<Span<'static>> {
        let (label, color) = match self.vim_mode {
            VimMode::Normal => ("Vim: Normal", Color::Magenta),
            VimMode::Insert => ("Vim: Insert", Color::Green),
            VimMode::Replace => ("Vim: Replace", Color::Cyan),
        };
        self.vim_enabled
            .then(|| Span::styled(label, Style::default().fg(color)))
    }

    fn enter_vim_insert_mode(&mut self) {
        if self.vim_enabled {
            self.vim_mode = VimMode::Insert;
            self.vim_pending = VimPending::None;
        }
    }

    fn enter_vim_normal_mode(&mut self) {
        if self.vim_enabled {
            self.vim_mode = VimMode::Normal;
            self.vim_pending = VimPending::None;
            self.vim_replace_steps.clear();
            self.preferred_col = None;
            if !self.text.is_empty() {
                self.cursor = self.vim_normal_end_cursor().min(self.cursor);
            }
        }
    }

    pub(super) fn handle_vim_input(&mut self, event: KeyEvent) {
        if self.vim_mode == VimMode::Insert {
            if event.code == KeyCode::Esc && event.modifiers == KeyModifiers::NONE {
                self.leave_vim_insert_mode();
            } else {
                self.input_insert_mode(event);
            }
            return;
        }
        if self.vim_mode == VimMode::Replace {
            if event.code == KeyCode::Esc && event.modifiers == KeyModifiers::NONE {
                self.enter_vim_normal_mode();
            } else {
                self.input_replace_mode(event);
            }
            return;
        }

        if let VimPending::Find { motion, operator } = self.vim_pending {
            self.vim_pending = VimPending::None;
            if let Some(target) = plain_char(event).and_then(|value| value.chars().next()) {
                self.find_vim_character(motion, operator, target);
            }
            return;
        }

        if let VimPending::TextObject { operator, scope } = self.vim_pending {
            self.vim_pending = VimPending::None;
            if let Some(object) = self.vim_text_object(event) {
                self.apply_vim_text_object(operator, scope, object);
            }
            return;
        }

        if let VimPending::ReplaceChar = self.vim_pending {
            self.vim_pending = VimPending::None;
            if event.code == KeyCode::Esc {
                return;
            }
            if let Some(value) = plain_char(event) {
                self.replace_current_grapheme(&value);
            }
            return;
        }

        if let VimPending::Operator(operator) = self.vim_pending {
            if event.modifiers == KeyModifiers::NONE {
                let scope = match event.code {
                    KeyCode::Char('i') => Some(VimTextObjectScope::Inner),
                    KeyCode::Char('a') => Some(VimTextObjectScope::Around),
                    _ => None,
                };
                if let Some(scope) = scope {
                    self.vim_pending = VimPending::TextObject { operator, scope };
                    return;
                }
                let motion = match event.code {
                    KeyCode::Char('f') => Some(VimFindMotion::Forward),
                    KeyCode::Char('F') => Some(VimFindMotion::Backward),
                    KeyCode::Char('t') => Some(VimFindMotion::TillForward),
                    KeyCode::Char('T') => Some(VimFindMotion::TillBackward),
                    _ => None,
                };
                if let Some(motion) = motion {
                    self.vim_pending = VimPending::Find {
                        motion,
                        operator: Some(operator),
                    };
                    return;
                }
            }
            self.vim_pending = VimPending::None;
            self.handle_vim_operator(operator, event);
            return;
        }

        match event.code {
            KeyCode::Esc => self.vim_pending = VimPending::None,
            KeyCode::Char('i') | KeyCode::Insert => self.enter_vim_insert_mode(),
            KeyCode::Char('a') => {
                self.move_right_normal();
                self.enter_vim_insert_mode();
            }
            KeyCode::Char('A') => {
                self.move_line_end();
                self.enter_vim_insert_mode();
            }
            KeyCode::Char('I') => {
                self.move_line_start();
                self.enter_vim_insert_mode();
            }
            KeyCode::Char('o') => {
                let end = self.line_end();
                if end < self.text.len() {
                    self.text.insert(end, '\n');
                } else {
                    self.text.push('\n');
                }
                self.cursor = end + 1;
                self.invalidate_wrap_cache();
                self.enter_vim_insert_mode();
            }
            KeyCode::Char('O') => {
                let start = self.line_start();
                self.text.insert(start, '\n');
                self.cursor = start;
                self.invalidate_wrap_cache();
                self.enter_vim_insert_mode();
            }
            KeyCode::Char('R') => {
                self.vim_replace_steps.clear();
                self.vim_mode = VimMode::Replace;
            }
            KeyCode::Char('r') => self.vim_pending = VimPending::ReplaceChar,
            KeyCode::Char('h') | KeyCode::Left => self.move_left_normal(),
            KeyCode::Char('l') | KeyCode::Right => self.move_right_normal(),
            KeyCode::Char('j') | KeyCode::Down => self.move_vertical(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_vertical(-1),
            KeyCode::Char('0') | KeyCode::Home => self.move_line_start(),
            KeyCode::Char('$') | KeyCode::End => self.set_cursor(self.vim_line_end()),
            KeyCode::Char('w') => self.move_word_forward(),
            KeyCode::Char('b') => self.set_cursor(self.beginning_of_previous_word()),
            KeyCode::Char('e') => self.set_cursor(self.word_end_cursor()),
            KeyCode::Char('f') => {
                self.vim_pending = VimPending::Find {
                    motion: VimFindMotion::Forward,
                    operator: None,
                };
            }
            KeyCode::Char('F') => {
                self.vim_pending = VimPending::Find {
                    motion: VimFindMotion::Backward,
                    operator: None,
                };
            }
            KeyCode::Char('t') => {
                self.vim_pending = VimPending::Find {
                    motion: VimFindMotion::TillForward,
                    operator: None,
                };
            }
            KeyCode::Char('T') => {
                self.vim_pending = VimPending::Find {
                    motion: VimFindMotion::TillBackward,
                    operator: None,
                };
            }
            KeyCode::Char('x') => {
                self.remove_next_grapheme();
            }
            KeyCode::Char('X') => {
                self.remove_previous_grapheme();
            }
            KeyCode::Char('s') if self.remove_next_grapheme() => self.enter_vim_insert_mode(),
            KeyCode::Char('D') => self.delete_to_line_end(false),
            KeyCode::Char('C') => self.delete_to_line_end(true),
            KeyCode::Char('d') => self.vim_pending = VimPending::Operator(VimOperator::Delete),
            KeyCode::Char('c') => self.vim_pending = VimPending::Operator(VimOperator::Change),
            KeyCode::Char('y') => self.vim_pending = VimPending::Operator(VimOperator::Yank),
            KeyCode::Char('p') => {
                self.paste_after_cursor();
            }
            _ => {}
        }
    }

    fn input_replace_mode(&mut self, event: KeyEvent) {
        match event.code {
            KeyCode::Backspace => {
                if !self.restore_vim_replaced_character() {
                    self.remove_previous_grapheme();
                }
            }
            KeyCode::Left => {
                self.vim_replace_steps.clear();
                self.move_left();
            }
            KeyCode::Right => {
                self.vim_replace_steps.clear();
                self.move_right_normal();
            }
            KeyCode::Enter => self.insert_vim_replace_text("\n"),
            _ => {
                if let Some(value) = plain_char(event) {
                    self.replace_vim_text(&value);
                }
            }
        }
    }

    fn replace_vim_text(&mut self, value: &str) {
        let cursor_before = self.cursor;
        let start = self.cursor;
        let end = if start < self.text.len() {
            let next = self.next_grapheme_end();
            if self.text[start..next] == *"\n" {
                start
            } else {
                next
            }
        } else {
            start
        };
        let original = self.text[start..end].to_string();
        self.text.replace_range(start..end, value);
        self.cursor = start + value.len();
        self.vim_replace_steps.push(VimReplaceStep {
            cursor_before,
            start,
            inserted_len: value.len(),
            original,
        });
        self.invalidate_wrap_cache();
    }

    fn insert_vim_replace_text(&mut self, value: &str) {
        let start = self.cursor;
        self.text.insert_str(start, value);
        self.cursor += value.len();
        self.vim_replace_steps.push(VimReplaceStep {
            cursor_before: start,
            start,
            inserted_len: value.len(),
            original: String::new(),
        });
        self.invalidate_wrap_cache();
    }

    fn restore_vim_replaced_character(&mut self) -> bool {
        let Some(step) = self.vim_replace_steps.pop() else {
            return false;
        };
        if self.cursor != step.start + step.inserted_len {
            self.vim_replace_steps.clear();
            return false;
        }
        self.text
            .replace_range(step.start..step.start + step.inserted_len, &step.original);
        self.cursor = step.cursor_before;
        self.invalidate_wrap_cache();
        true
    }

    fn vim_text_object(&self, event: KeyEvent) -> Option<VimTextObject> {
        if event
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return None;
        }
        match event.code {
            KeyCode::Char('w') => Some(VimTextObject::Word),
            KeyCode::Char('W') => Some(VimTextObject::BigWord),
            KeyCode::Char('(') | KeyCode::Char(')') => Some(VimTextObject::Parentheses),
            KeyCode::Char('[') | KeyCode::Char(']') => Some(VimTextObject::Brackets),
            KeyCode::Char('{') | KeyCode::Char('}') => Some(VimTextObject::Braces),
            KeyCode::Char('"') => Some(VimTextObject::DoubleQuote),
            KeyCode::Char('\'') => Some(VimTextObject::SingleQuote),
            KeyCode::Char('\u{60}') => Some(VimTextObject::Backtick),
            _ => None,
        }
    }

    fn apply_vim_text_object(
        &mut self,
        operator: VimOperator,
        scope: VimTextObjectScope,
        object: VimTextObject,
    ) {
        let Some(range) = self.text_object_range(object, scope) else {
            return;
        };
        match operator {
            VimOperator::Delete => self.kill_range(range),
            VimOperator::Change => {
                self.kill_range(range);
                self.enter_vim_insert_mode();
            }
            VimOperator::Yank => {
                self.kill_buffer = self.text[range].to_string();
            }
        }
    }

    fn text_object_range(
        &self,
        object: VimTextObject,
        scope: VimTextObjectScope,
    ) -> Option<Range<usize>> {
        match object {
            VimTextObject::Word => self.word_text_object_range(scope, false),
            VimTextObject::BigWord => self.word_text_object_range(scope, true),
            VimTextObject::Parentheses => self.paired_text_object_range(scope, '(', ')'),
            VimTextObject::Brackets => self.paired_text_object_range(scope, '[', ']'),
            VimTextObject::Braces => self.paired_text_object_range(scope, '{', '}'),
            VimTextObject::DoubleQuote => self.quoted_text_object_range(scope, '"'),
            VimTextObject::SingleQuote => self.quoted_text_object_range(scope, '\''),
            VimTextObject::Backtick => self.quoted_text_object_range(scope, '\u{60}'),
        }
    }

    fn word_text_object_range(
        &self,
        scope: VimTextObjectScope,
        big_word: bool,
    ) -> Option<Range<usize>> {
        let mut runs = Vec::new();
        let mut start = None;
        for (idx, ch) in self.text.char_indices() {
            if ch.is_whitespace() {
                if let Some(run_start) = start.take() {
                    runs.push(run_start..idx);
                }
            } else if start.is_none() {
                start = Some(idx);
            }
        }
        if let Some(run_start) = start {
            runs.push(run_start..self.text.len());
        }
        let run = runs
            .iter()
            .find(|range| range.start <= self.cursor && self.cursor < range.end)
            .or_else(|| runs.iter().find(|range| range.end == self.cursor))?
            .clone();
        let inner = if big_word {
            run
        } else {
            split_word_pieces(&self.text[run.clone()])
                .into_iter()
                .map(|(offset, piece)| run.start + offset..run.start + offset + piece.len())
                .find(|range| range.start <= self.cursor && self.cursor < range.end)
                .or_else(|| {
                    split_word_pieces(&self.text[run.clone()])
                        .into_iter()
                        .last()
                        .map(|(offset, piece)| run.start + offset..run.start + offset + piece.len())
                })?
        };
        Some(match scope {
            VimTextObjectScope::Inner => inner,
            VimTextObjectScope::Around => {
                let following = self.following_whitespace_end(inner.end);
                if following > inner.end {
                    inner.start..following
                } else {
                    self.preceding_whitespace_start(inner.start)..inner.end
                }
            }
        })
    }

    fn following_whitespace_end(&self, start: usize) -> usize {
        let mut end = start;
        for (offset, ch) in self.text[start..].char_indices() {
            if !ch.is_whitespace() {
                break;
            }
            end = start + offset + ch.len_utf8();
        }
        end
    }

    fn preceding_whitespace_start(&self, end: usize) -> usize {
        let mut start = end;
        for (idx, ch) in self.text[..end].char_indices().rev() {
            if !ch.is_whitespace() {
                break;
            }
            start = idx;
        }
        start
    }

    fn paired_text_object_range(
        &self,
        scope: VimTextObjectScope,
        open: char,
        close: char,
    ) -> Option<Range<usize>> {
        let mut stack = Vec::new();
        let mut best = None;
        for (idx, ch) in self.text.char_indices() {
            if ch == open {
                stack.push(idx);
            } else if ch == close {
                let Some(open_idx) = stack.pop() else {
                    continue;
                };
                if open_idx <= self.cursor && self.cursor <= idx {
                    let candidate = match scope {
                        VimTextObjectScope::Inner => open_idx + open.len_utf8()..idx,
                        VimTextObjectScope::Around => open_idx..idx + close.len_utf8(),
                    };
                    if best
                        .as_ref()
                        .is_none_or(|current: &Range<usize>| candidate.len() < current.len())
                    {
                        best = Some(candidate);
                    }
                }
            }
        }
        best
    }

    fn quoted_text_object_range(
        &self,
        scope: VimTextObjectScope,
        quote: char,
    ) -> Option<Range<usize>> {
        let line_start = self.line_start();
        let line_end = self.line_end();
        let mut open = None;
        let mut best = None;
        for (offset, ch) in self.text[line_start..line_end].char_indices() {
            let idx = line_start + offset;
            if ch != quote || self.is_escaped(idx) {
                continue;
            }
            if let Some(open_idx) = open.take() {
                if open_idx <= self.cursor && self.cursor <= idx {
                    let candidate = match scope {
                        VimTextObjectScope::Inner => open_idx + quote.len_utf8()..idx,
                        VimTextObjectScope::Around => open_idx..idx + quote.len_utf8(),
                    };
                    if best
                        .as_ref()
                        .is_none_or(|current: &Range<usize>| candidate.len() < current.len())
                    {
                        best = Some(candidate);
                    }
                }
            } else {
                open = Some(idx);
            }
        }
        best
    }

    fn is_escaped(&self, pos: usize) -> bool {
        let mut backslashes = 0;
        for ch in self.text[..pos].chars().rev() {
            if ch != '\\' {
                break;
            }
            backslashes += 1;
        }
        backslashes % 2 == 1
    }

    fn find_vim_character(
        &mut self,
        motion: VimFindMotion,
        operator: Option<VimOperator>,
        target: char,
    ) -> bool {
        let origin = self.cursor;
        let line_start = self.line_start();
        let line_end = self.line_end();
        let found = match motion {
            VimFindMotion::Forward | VimFindMotion::TillForward => {
                let start = self.next_grapheme_end_at(origin);
                if start >= line_end {
                    return false;
                }
                self.text[start..line_end]
                    .grapheme_indices(true)
                    .find(|(_, grapheme)| grapheme.starts_with(target))
                    .map(|(offset, grapheme)| start + offset..start + offset + grapheme.len())
            }
            VimFindMotion::Backward | VimFindMotion::TillBackward => self.text[line_start..origin]
                .grapheme_indices(true)
                .rev()
                .find(|(_, grapheme)| grapheme.starts_with(target))
                .map(|(offset, grapheme)| {
                    let start = line_start + offset;
                    start..start + grapheme.len()
                }),
        };
        let Some(position) = found else {
            return false;
        };

        if let Some(operator) = operator {
            let range = match motion {
                VimFindMotion::Forward => origin..position.end,
                VimFindMotion::Backward => position.start..origin,
                VimFindMotion::TillForward => origin..position.start,
                VimFindMotion::TillBackward => position.end..origin,
            };
            match operator {
                VimOperator::Delete => self.kill_range(range),
                VimOperator::Change => {
                    self.kill_range(range);
                    self.enter_vim_insert_mode();
                }
                VimOperator::Yank => {
                    self.kill_buffer = self.text[range].to_string();
                }
            }
        } else {
            let destination = match motion {
                VimFindMotion::Forward | VimFindMotion::Backward => position.start,
                VimFindMotion::TillForward => self.text[..position.start]
                    .grapheme_indices(true)
                    .next_back()
                    .map(|(offset, _)| offset)
                    .unwrap_or(origin),
                VimFindMotion::TillBackward => position.end,
            };
            self.set_cursor(destination.min(self.vim_normal_end_cursor()));
        }
        true
    }

    fn leave_vim_insert_mode(&mut self) {
        if self.cursor > self.line_start() {
            self.cursor = self.previous_grapheme_start();
        }
        self.enter_vim_normal_mode();
    }

    fn handle_vim_operator(&mut self, operator: VimOperator, event: KeyEvent) {
        if event.code == KeyCode::Esc {
            return;
        }
        let range = if matches!(event.code, KeyCode::Char('d' | 'c' | 'y')) {
            Some(self.current_line_range())
        } else {
            self.range_for_motion(event)
        };
        let Some(range) = range else {
            return;
        };
        match operator {
            VimOperator::Delete => self.kill_range(range),
            VimOperator::Yank => self.kill_buffer = self.text[range].to_string(),
            VimOperator::Change => {
                self.kill_range(range);
                self.enter_vim_insert_mode();
            }
        }
    }

    fn range_for_motion(&self, event: KeyEvent) -> Option<std::ops::Range<usize>> {
        let target = match event.code {
            KeyCode::Char('h') | KeyCode::Left => self.previous_grapheme_start(),
            KeyCode::Char('l') | KeyCode::Right => self.next_grapheme_end(),
            KeyCode::Char('w') => self.next_word_start(),
            KeyCode::Char('b') => self.beginning_of_previous_word(),
            KeyCode::Char('e') => self.word_end_cursor(),
            KeyCode::Char('0') | KeyCode::Home => self.line_start(),
            KeyCode::Char('$') | KeyCode::End => self.line_end(),
            _ => return None,
        };
        let start = self.cursor.min(target);
        let mut end = self.cursor.max(target);
        if start == end {
            return None;
        }
        if matches!(event.code, KeyCode::Char('w')) && target > self.cursor {
            end = end.min(self.text.len());
        }
        Some(start..end)
    }

    fn current_line_range(&self) -> std::ops::Range<usize> {
        let start = self.line_start();
        let end = self.line_end();
        if end < self.text.len() {
            start..end + 1
        } else if start > 0 && self.text.as_bytes().get(start - 1) == Some(&b'\n') {
            start.saturating_sub(1)..end
        } else {
            start..end
        }
    }

    fn delete_to_line_end(&mut self, enter_insert: bool) {
        let end = self.line_end();
        if self.cursor < end {
            self.kill_range(self.cursor..end);
        }
        if enter_insert {
            self.enter_vim_insert_mode();
        }
    }

    fn paste_after_cursor(&mut self) {
        if self.kill_buffer.is_empty() {
            return;
        }
        let at = self.next_grapheme_end();
        self.text.insert_str(at, &self.kill_buffer.clone());
        self.cursor = at + self.kill_buffer.len();
        self.invalidate_wrap_cache();
    }

    fn replace_current_grapheme(&mut self, value: &str) {
        if self.cursor < self.text.len() {
            let end = self.next_grapheme_end();
            if self.text[self.cursor..end] != *"\n" {
                self.text.replace_range(self.cursor..end, value);
                self.invalidate_wrap_cache();
                return;
            }
        }
        self.text.insert_str(self.cursor, value);
        self.cursor += value.len();
        self.invalidate_wrap_cache();
    }

    fn move_left_normal(&mut self) {
        self.move_left();
    }

    fn move_right_normal(&mut self) {
        let next = self.next_grapheme_end().min(self.vim_normal_end_cursor());
        self.set_cursor(next);
    }

    fn move_word_forward(&mut self) {
        self.set_cursor(self.next_word_start());
    }

    fn next_word_start(&self) -> usize {
        let suffix = &self.text[self.cursor..];
        let mut saw_word = false;
        let mut saw_separator = false;
        for (offset, ch) in suffix.char_indices() {
            if ch.is_whitespace() {
                if saw_word {
                    saw_separator = true;
                }
                continue;
            }
            if saw_separator {
                return self.cursor + offset;
            }
            saw_word = true;
        }
        self.text.len()
    }

    fn word_end_cursor(&self) -> usize {
        let start = self.next_word_start();
        let mut end = start;
        for (offset, ch) in self.text[start..].char_indices() {
            if ch.is_whitespace() {
                break;
            }
            end = start + offset + ch.len_utf8();
        }
        end.saturating_sub(1)
    }

    fn vim_line_end(&self) -> usize {
        let end = self.line_end();
        if end > self.line_start() {
            self.text[..end]
                .grapheme_indices(true)
                .next_back()
                .map(|(index, _)| index)
                .unwrap_or(end)
        } else {
            end
        }
    }

    fn move_vertical(&mut self, delta: isize) {
        if delta < 0 {
            self.move_up();
        } else {
            self.move_down();
        }
        self.cursor = self.cursor.min(self.vim_normal_end_cursor());
    }
}

fn plain_char(event: KeyEvent) -> Option<String> {
    if !matches!(event.kind, KeyEventKind::Press | KeyEventKind::Repeat)
        || event
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        return None;
    }
    match event.code {
        KeyCode::Char(ch) if !ch.is_ascii_control() => Some(ch.to_string()),
        _ => None,
    }
}

#[cfg(test)]
#[path = "vim_commands_tests.rs"]
mod tests;
