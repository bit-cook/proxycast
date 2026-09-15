//! Minimal MCP form elicitation interaction for the TUI.
//!
//! The App Server owns the request contract. This module only translates the supported form
//! schema into a focused terminal editor and emits the typed v2 response.

use std::collections::HashSet;

use app_server_protocol::protocol::v2::{
    McpServerElicitationAction, McpServerElicitationRequest, McpServerElicitationRequestParams,
    McpServerElicitationRequestResponse,
};
use app_server_protocol::RequestId;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::Frame;
use serde_json::{Map, Value};

use super::{AppServerResponse, TextArea, TextAreaState};
use crate::bottom_pane::selection_row_layout::{visible_item_window, MAX_POPUP_ROWS};
use crate::line_truncation::truncate_line_with_ellipsis_if_overflow;
use crate::locale::Locale;
use crate::style::{accent_style, muted_style};
use crate::width::display_width;
use crate::wrapping::{word_wrap_line, RtOptions};

const APPROVAL_META_KIND_KEY: &str = "codex_approval_kind";
const APPROVAL_META_KIND_MCP_TOOL_CALL: &str = "mcp_tool_call";
const APPROVAL_META_KIND_TOOL_SUGGESTION: &str = "tool_suggestion";
const APPROVAL_PERSIST_KEY: &str = "persist";
const APPROVAL_PERSIST_SESSION_VALUE: &str = "session";
const APPROVAL_PERSIST_ALWAYS_VALUE: &str = "always";
const APPROVAL_ACCEPT_ONCE_VALUE: &str = "accept";
const APPROVAL_ACCEPT_SESSION_VALUE: &str = "accept_session";
const APPROVAL_ACCEPT_ALWAYS_VALUE: &str = "accept_always";
const APPROVAL_DECLINE_VALUE: &str = "decline";
const APPROVAL_CANCEL_VALUE: &str = "cancel";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum McpResponseMode {
    FormContent,
    ApprovalAction,
}

#[derive(Debug, Clone, PartialEq)]
struct McpField {
    id: String,
    label: String,
    description: Option<String>,
    required: bool,
    input: McpFieldInput,
}

#[derive(Debug, Clone, PartialEq)]
enum McpFieldInput {
    Text {
        default: Option<String>,
    },
    Select {
        options: Vec<McpOption>,
        default_index: Option<usize>,
    },
}

#[derive(Debug, Clone, PartialEq)]
struct McpOption {
    label: String,
    value: Value,
}

#[derive(Debug, Clone, PartialEq)]
enum McpFieldState {
    Text {
        draft: String,
        committed: bool,
    },
    Select {
        selected: Option<usize>,
        committed: bool,
    },
}

#[derive(Debug)]
pub(super) struct McpServerElicitationOverlay {
    id: RequestId,
    server_name: String,
    message: String,
    response_mode: McpResponseMode,
    fields: Vec<McpField>,
    states: Vec<McpFieldState>,
    current_field: usize,
    text_area: TextArea,
    text_area_state: TextAreaState,
    validation_error: bool,
    done: bool,
}

impl McpServerElicitationOverlay {
    pub(super) fn from_server_request(
        id: RequestId,
        params: &McpServerElicitationRequestParams,
    ) -> Option<Self> {
        let McpServerElicitationRequest::Form {
            message,
            requested_schema,
            meta,
            ..
        } = &params.request;
        let schema = Value::Object(requested_schema.clone());
        let (response_mode, fields) = if is_empty_object_schema(&schema) {
            if is_tool_suggestion(meta.as_ref()) {
                return None;
            }
            (
                McpResponseMode::ApprovalAction,
                approval_fields(meta.as_ref())?,
            )
        } else {
            (McpResponseMode::FormContent, parse_fields(&schema)?)
        };
        if fields.is_empty() {
            return None;
        }

        let states = fields
            .iter()
            .map(|field| match &field.input {
                McpFieldInput::Text { default } => McpFieldState::Text {
                    draft: default.clone().unwrap_or_default(),
                    committed: default
                        .as_deref()
                        .is_some_and(|value| !value.trim().is_empty()),
                },
                McpFieldInput::Select { default_index, .. } => McpFieldState::Select {
                    selected: default_index.or(Some(0)),
                    committed: default_index.is_some(),
                },
            })
            .collect();

        let mut overlay = Self {
            id,
            server_name: params.server_name.clone(),
            message: message.clone(),
            response_mode,
            fields,
            states,
            current_field: 0,
            text_area: TextArea::default(),
            text_area_state: TextAreaState::default(),
            validation_error: false,
            done: false,
        };
        overlay.restore_text_field();
        Some(overlay)
    }

    pub(super) fn handle_key_event(&mut self, key: KeyEvent) -> Option<AppServerResponse> {
        if !matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
            return None;
        }
        if key.code == KeyCode::Esc
            || key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c')
        {
            return Some(self.cancel_response());
        }

        if self.is_select_field() {
            return self.handle_select_key(key);
        }

        if self.handle_field_navigation(key) {
            return None;
        }
        if key.code == KeyCode::Enter && key.modifiers.is_empty() {
            self.commit_current_field();
            return self.advance_or_submit();
        }
        if key.code == KeyCode::Char('j') && key.modifiers == KeyModifiers::CONTROL {
            self.text_area.insert("\n");
            self.mark_text_changed();
            return None;
        }

        let before = (self.text_area.text().to_string(), self.text_area.cursor());
        self.text_area.input(key);
        if before != (self.text_area.text().to_string(), self.text_area.cursor()) {
            self.mark_text_changed();
        }
        None
    }

    pub(super) fn handle_paste(&mut self, text: &str) {
        if self.is_select_field() || text.is_empty() {
            return;
        }
        self.text_area.insert(text);
        self.mark_text_changed();
    }

    fn handle_select_key(&mut self, key: KeyEvent) -> Option<AppServerResponse> {
        if self.handle_field_navigation(key) {
            return None;
        }
        let options_len = self.current_options().len();
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if let Some((selected, committed)) = self.current_select_state_mut() {
                    *selected = Some(match *selected {
                        Some(0) | None => options_len.saturating_sub(1),
                        Some(index) => index.saturating_sub(1),
                    });
                    *committed = false;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if let Some((selected, committed)) = self.current_select_state_mut() {
                    *selected = Some(match *selected {
                        Some(index) => (index + 1) % options_len,
                        None => 0,
                    });
                    *committed = false;
                }
            }
            KeyCode::Backspace | KeyCode::Delete => {
                if let Some((selected, committed)) = self.current_select_state_mut() {
                    *selected = None;
                    *committed = false;
                }
            }
            KeyCode::Char(' ') => self.commit_current_field(),
            KeyCode::Enter if key.modifiers.is_empty() => {
                self.commit_current_field();
                return self.advance_or_submit();
            }
            KeyCode::Char(ch) => {
                let digit = ch.to_digit(10)?;
                if digit == 0 {
                    return None;
                }
                let index = digit as usize - 1;
                if index < options_len {
                    if let Some((selected, committed)) = self.current_select_state_mut() {
                        *selected = Some(index);
                        *committed = true;
                    }
                    return self.advance_or_submit();
                }
            }
            _ => {}
        }
        None
    }

    fn handle_field_navigation(&mut self, key: KeyEvent) -> bool {
        let previous = matches!(key.code, KeyCode::BackTab | KeyCode::PageUp)
            || key.code == KeyCode::Char('p') && key.modifiers == KeyModifiers::CONTROL;
        let next = matches!(key.code, KeyCode::Tab | KeyCode::PageDown)
            || key.code == KeyCode::Char('n') && key.modifiers == KeyModifiers::CONTROL;
        let horizontal_previous = self.is_select_field()
            && matches!(key.code, KeyCode::Left | KeyCode::Char('h'))
            && key.modifiers.is_empty();
        let horizontal_next = self.is_select_field()
            && matches!(key.code, KeyCode::Right | KeyCode::Char('l'))
            && key.modifiers.is_empty();
        if previous || horizontal_previous {
            self.move_field(false);
            true
        } else if next || horizontal_next {
            self.move_field(true);
            true
        } else {
            false
        }
    }

    fn move_field(&mut self, next: bool) {
        if self.fields.len() < 2 {
            return;
        }
        self.save_text_draft();
        let offset = if next { 1 } else { self.fields.len() - 1 };
        self.current_field = (self.current_field + offset) % self.fields.len();
        self.validation_error = false;
        self.restore_text_field();
    }

    fn commit_current_field(&mut self) {
        let index = self.current_field;
        if self.is_text_field() {
            let text = self.text_area.text().to_string();
            if let Some(McpFieldState::Text { draft, committed }) = self.states.get_mut(index) {
                *draft = text;
                *committed = !draft.trim().is_empty();
            }
        } else if let Some(McpFieldState::Select {
            selected,
            committed,
        }) = self.states.get_mut(index)
        {
            *committed = selected.is_some();
        }
        self.validation_error = false;
    }

    fn save_text_draft(&mut self) {
        if !self.is_text_field() {
            return;
        }
        let text = self.text_area.text().to_string();
        if let Some(McpFieldState::Text { draft, committed }) =
            self.states.get_mut(self.current_field)
        {
            if *draft != text {
                *committed = false;
            }
            *draft = text;
        }
    }

    fn restore_text_field(&mut self) {
        let text = match self.states.get(self.current_field) {
            Some(McpFieldState::Text { draft, .. }) => draft.clone(),
            _ => String::new(),
        };
        self.text_area.replace(text);
        self.text_area_state = TextAreaState::default();
    }

    fn mark_text_changed(&mut self) {
        if let Some(McpFieldState::Text { committed, .. }) = self.states.get_mut(self.current_field)
        {
            *committed = false;
        }
        self.validation_error = false;
    }

    fn advance_or_submit(&mut self) -> Option<AppServerResponse> {
        if self.current_field + 1 < self.fields.len() {
            self.move_field(true);
            None
        } else {
            self.submit_answers()
        }
    }

    fn submit_answers(&mut self) -> Option<AppServerResponse> {
        self.save_text_draft();
        let Some(index) = self
            .fields
            .iter()
            .enumerate()
            .find(|(index, field)| field.required && self.field_value(*index).is_none())
            .map(|(index, _)| index)
        else {
            if self.response_mode == McpResponseMode::ApprovalAction {
                let selected = self
                    .field_value(0)
                    .and_then(|value| value.as_str().map(str::to_owned));
                let (action, content, meta) = match selected.as_deref() {
                    Some(APPROVAL_ACCEPT_ONCE_VALUE) => (
                        McpServerElicitationAction::Accept,
                        Some(Value::Object(Map::new())),
                        None,
                    ),
                    Some(APPROVAL_ACCEPT_SESSION_VALUE) => (
                        McpServerElicitationAction::Accept,
                        Some(Value::Object(Map::new())),
                        Some(Map::from_iter([(
                            APPROVAL_PERSIST_KEY.to_string(),
                            Value::String(APPROVAL_PERSIST_SESSION_VALUE.to_string()),
                        )])),
                    ),
                    Some(APPROVAL_ACCEPT_ALWAYS_VALUE) => (
                        McpServerElicitationAction::Accept,
                        Some(Value::Object(Map::new())),
                        Some(Map::from_iter([(
                            APPROVAL_PERSIST_KEY.to_string(),
                            Value::String(APPROVAL_PERSIST_ALWAYS_VALUE.to_string()),
                        )])),
                    ),
                    Some(APPROVAL_DECLINE_VALUE) => {
                        (McpServerElicitationAction::Decline, None, None)
                    }
                    Some(APPROVAL_CANCEL_VALUE) => (McpServerElicitationAction::Cancel, None, None),
                    _ => return None,
                };
                self.done = true;
                return Some(AppServerResponse::McpElicitation {
                    id: self.id.clone(),
                    response: McpServerElicitationRequestResponse {
                        action,
                        content,
                        meta,
                    },
                });
            }
            let content = self
                .fields
                .iter()
                .enumerate()
                .filter_map(|(index, field)| {
                    self.field_value(index)
                        .map(|value| (field.id.clone(), value))
                })
                .collect::<Map<_, _>>();
            self.done = true;
            return Some(AppServerResponse::McpElicitation {
                id: self.id.clone(),
                response: McpServerElicitationRequestResponse {
                    action: McpServerElicitationAction::Accept,
                    content: Some(Value::Object(content)),
                    meta: None,
                },
            });
        };

        self.current_field = index;
        self.validation_error = true;
        self.restore_text_field();
        None
    }

    fn cancel_response(&mut self) -> AppServerResponse {
        self.done = true;
        AppServerResponse::McpElicitation {
            id: self.id.clone(),
            response: McpServerElicitationRequestResponse {
                action: McpServerElicitationAction::Cancel,
                content: None,
                meta: None,
            },
        }
    }

    fn field_value(&self, index: usize) -> Option<Value> {
        let field = self.fields.get(index)?;
        let state = self.states.get(index)?;
        match (&field.input, state) {
            (McpFieldInput::Text { .. }, McpFieldState::Text { draft, committed }) => committed
                .then(|| draft.trim().to_string())
                .filter(|value| !value.is_empty())
                .map(Value::String),
            (
                McpFieldInput::Select { options, .. },
                McpFieldState::Select {
                    selected,
                    committed,
                },
            ) => committed
                .then(|| {
                    selected.and_then(|index| options.get(index).map(|option| option.value.clone()))
                })
                .flatten(),
            _ => None,
        }
    }

    fn current_options(&self) -> &[McpOption] {
        match self
            .fields
            .get(self.current_field)
            .map(|field| &field.input)
        {
            Some(McpFieldInput::Select { options, .. }) => options,
            _ => &[],
        }
    }

    fn current_select_state_mut(&mut self) -> Option<(&mut Option<usize>, &mut bool)> {
        match self.states.get_mut(self.current_field) {
            Some(McpFieldState::Select {
                selected,
                committed,
            }) => Some((selected, committed)),
            _ => None,
        }
    }

    fn is_select_field(&self) -> bool {
        matches!(
            self.fields
                .get(self.current_field)
                .map(|field| &field.input),
            Some(McpFieldInput::Select { .. })
        )
    }

    fn is_text_field(&self) -> bool {
        !self.is_select_field()
    }

    #[cfg(test)]
    pub(super) fn is_complete(&self) -> bool {
        self.done
    }
}

pub(super) fn lines_with_locale(
    overlay: &McpServerElicitationOverlay,
    locale: Locale,
) -> Vec<Line<'static>> {
    lines_with_locale_inner(overlay, locale, None)
}

pub(super) fn lines_with_locale_with_width(
    overlay: &McpServerElicitationOverlay,
    locale: Locale,
    width: usize,
) -> Vec<Line<'static>> {
    let width = width.max(1);
    if width == usize::MAX {
        return lines_with_locale(overlay, locale);
    }
    let lines = lines_with_locale_inner(overlay, locale, Some(width));

    let footer_index = lines.len().saturating_sub(1);
    lines
        .into_iter()
        .enumerate()
        .flat_map(|(index, line)| {
            if index == footer_index {
                return vec![line].into_iter();
            }
            if is_input_line(&line) {
                return split_input_lines(line)
                    .into_iter()
                    .map(|line| truncate_line_with_ellipsis_if_overflow(line, width))
                    .collect::<Vec<_>>()
                    .into_iter();
            }
            if is_option_line(&line) {
                return vec![truncate_line_with_ellipsis_if_overflow(line, width)].into_iter();
            }
            word_wrap_line(&line, RtOptions::new(width).break_words(true))
                .into_iter()
                .map(line_to_owned)
                .collect::<Vec<_>>()
                .into_iter()
        })
        .collect()
}

fn lines_with_locale_inner(
    overlay: &McpServerElicitationOverlay,
    locale: Locale,
    width: Option<usize>,
) -> Vec<Line<'static>> {
    let Some(field) = overlay.fields.get(overlay.current_field) else {
        return vec![Line::from(locale.mcp_elicitation_invalid())];
    };
    let mut lines = vec![
        Line::styled(
            locale.mcp_elicitation_title(&overlay.server_name),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Line::from(overlay.message.clone()),
        Line::styled(
            locale.mcp_elicitation_progress(overlay.current_field + 1, overlay.fields.len()),
            muted_style(),
        ),
    ];
    if !field.label.is_empty() {
        lines.push(Line::styled(
            format!("{}{}", field.label, if field.required { " *" } else { "" }),
            Style::default().add_modifier(Modifier::BOLD),
        ));
    }
    if let Some(description) = field.description.as_ref().filter(|value| !value.is_empty()) {
        lines.push(Line::styled(description.clone(), muted_style()));
    }

    match &field.input {
        McpFieldInput::Text { .. } => {
            let value = overlay.text_area.text();
            let value = if value.is_empty() {
                locale
                    .mcp_elicitation_text_placeholder(field.required)
                    .to_string()
            } else {
                value.to_string()
            };
            let style = if overlay.text_area.text().is_empty() {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            };
            lines.push(Line::from(vec![
                Span::styled("› ", accent_style()),
                Span::styled(value, style),
            ]));
        }
        McpFieldInput::Select { options, .. } => {
            let selected = match overlay.states.get(overlay.current_field) {
                Some(McpFieldState::Select { selected, .. }) => *selected,
                _ => None,
            };
            let selected_index = selected.unwrap_or(0);
            let (start, end) = visible_item_window(selected_index, options.len(), MAX_POPUP_ROWS);
            for (index, option) in options.iter().enumerate().skip(start).take(end - start) {
                let is_selected = selected == Some(index);
                let prefix = if is_selected { "›" } else { " " };
                let label = match (&option.value, overlay.response_mode) {
                    (Value::Bool(value), _) => {
                        locale.mcp_elicitation_boolean_option(*value).to_string()
                    }
                    (Value::String(value), McpResponseMode::ApprovalAction) => {
                        let localized = locale.mcp_elicitation_approval_option(value);
                        if localized.is_empty() {
                            option.label.clone()
                        } else {
                            localized.to_string()
                        }
                    }
                    _ => option.label.clone(),
                };
                let style = if is_selected {
                    accent_style()
                } else {
                    Style::default()
                };
                lines.push(Line::styled(
                    format!("{prefix} {}. {label}", index + 1),
                    style,
                ));
            }
        }
    }
    if overlay.validation_error {
        lines.push(Line::styled(
            locale.mcp_elicitation_required_error(),
            Style::default().fg(Color::Red),
        ));
    }
    lines.extend(footer_control_lines(
        locale,
        overlay.is_select_field(),
        width,
    ));
    lines
}

fn is_option_line(line: &Line<'_>) -> bool {
    let text = line.to_string();
    let text = text.trim_start_matches(['›', '>', ' ']);
    text.as_bytes().first().is_some_and(u8::is_ascii_digit) && text.contains(". ")
}

fn is_input_line(line: &Line<'_>) -> bool {
    line.to_string().starts_with(['›', '>'])
}

/// Split the editable line at explicit newlines before applying the width projection.
///
/// `Line` may contain a newline inside a span, but ratatui lays out each physical row
/// independently. Keeping the split here makes the rendered rows and cursor calculation share
/// the same source of truth, including continuation rows that do not carry the `›` prompt.
fn split_input_lines(line: Line<'static>) -> Vec<Line<'static>> {
    let Line {
        style,
        alignment,
        spans,
    } = line;
    let mut rows = vec![Vec::<Span<'static>>::new()];
    for span in spans {
        let content = span.content.into_owned();
        let mut segments = content.split('\n').peekable();
        while let Some(segment) = segments.next() {
            if !segment.is_empty() {
                rows.last_mut()
                    .expect("input row exists")
                    .push(Span::styled(segment.to_string(), span.style));
            }
            if segments.peek().is_some() {
                rows.push(Vec::new());
            }
        }
    }
    rows.into_iter()
        .map(|spans| Line {
            style,
            alignment,
            spans,
        })
        .collect()
}

fn line_to_owned(line: Line<'_>) -> Line<'static> {
    let style = line.style;
    let spans = line
        .spans
        .into_iter()
        .map(|span| Span::styled(span.content.into_owned(), span.style))
        .collect::<Vec<_>>();
    Line::from(spans).style(style)
}

fn footer_control_lines(locale: Locale, select: bool, width: Option<usize>) -> Vec<Line<'static>> {
    let full = locale.mcp_elicitation_controls(select);
    let Some(width) = width else {
        return vec![Line::styled(full, muted_style())];
    };
    let width = width.max(1);
    if display_width(&full) <= width {
        return vec![Line::styled(full, muted_style())];
    }

    // Locale strings intentionally keep two spaces between tips. Reusing those boundaries lets
    // the wide layout remain byte-for-byte stable while allowing narrow terminals to pack each
    // action independently. Submit and cancel stay ahead of navigation hints so the primary
    // action set remains visible when the footer needs multiple rows.
    let segments = full
        .split("  ")
        .filter(|segment| !segment.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if segments.len() < 2 {
        return vec![Line::styled(
            compact_control_segment(&full, width),
            muted_style(),
        )];
    }
    let submit_index = segments
        .iter()
        .position(|segment| contains_submit_hint(segment))
        .unwrap_or(0);
    let cancel_index = segments
        .iter()
        .position(|segment| contains_cancel_hint(segment))
        .unwrap_or(segments.len().saturating_sub(1));
    let cancel_segment = (cancel_index != submit_index).then(|| segments[cancel_index].clone());
    let mut ordered = Vec::with_capacity(segments.len());
    ordered.push(segments[submit_index].clone());
    for (index, segment) in segments.into_iter().enumerate() {
        if index != submit_index && index != cancel_index {
            ordered.push(segment);
        }
    }
    if let Some(cancel_segment) = cancel_segment {
        ordered.push(cancel_segment);
    }

    let mut rows = Vec::<String>::new();
    let mut current = String::new();
    for segment in ordered {
        let segment = compact_control_segment(&segment, width);
        if current.is_empty() {
            current = segment;
            continue;
        }
        let candidate = format!("{current}  {segment}");
        if display_width(&candidate) <= width {
            current = candidate;
        } else {
            rows.push(current);
            current = segment;
        }
    }
    if !current.is_empty() {
        rows.push(current);
    }
    rows.into_iter()
        .map(|line| Line::styled(line, muted_style()))
        .collect()
}

fn contains_submit_hint(segment: &str) -> bool {
    segment.contains("Enter")
        || segment.contains("↵")
        || segment.contains("确认")
        || segment.contains("確定")
        || segment.contains("확인")
}

fn contains_cancel_hint(segment: &str) -> bool {
    segment.contains("Esc")
        || segment.contains('⎋')
        || segment.contains("取消")
        || segment.contains("キャンセル")
        || segment.contains("취소")
}

fn compact_control_segment(segment: &str, width: usize) -> String {
    if display_width(segment) <= width {
        return segment.to_owned();
    }
    let candidates = if contains_cancel_hint(segment) {
        vec!["Esc", "⎋"]
    } else if contains_submit_hint(segment) {
        vec!["Enter", "↵"]
    } else if segment.contains('↑') || segment.contains('↓') {
        vec!["↑/↓", "↑↓", "↑", "↓"]
    } else if segment.contains("Tab")
        || segment.contains("欄位")
        || segment.contains("字段")
        || segment.contains("フィールド")
        || segment.contains("필드")
    {
        vec!["Tab", "⇥"]
    } else {
        Vec::new()
    };
    if let Some(candidate) = candidates
        .into_iter()
        .find(|candidate| display_width(candidate) <= width)
    {
        return candidate.to_owned();
    }
    truncate_line_with_ellipsis_if_overflow(Line::from(segment.to_owned()), width).to_string()
}

pub(super) fn set_cursor_position(
    frame: &mut Frame<'_>,
    inner: Rect,
    overlay: &McpServerElicitationOverlay,
    content: &[Line<'static>],
) {
    if !overlay.is_text_field() || inner.is_empty() {
        return;
    }
    let before = &overlay.text_area.text()[..overlay.text_area.cursor()];
    let current_line = before.rsplit('\n').next().unwrap_or(before);
    let input_row = content
        .iter()
        .position(is_input_line)
        .unwrap_or_else(|| content.len().saturating_sub(1));
    let prefix_width = if before.contains('\n') { 0 } else { 2 };
    let x = inner
        .x
        .saturating_add(prefix_width)
        .saturating_add(u16::try_from(display_width(current_line)).unwrap_or(u16::MAX))
        .min(inner.right().saturating_sub(1));
    let input_row = input_row.saturating_add(before.matches('\n').count());
    let y = inner.y.saturating_add(
        u16::try_from(input_row)
            .unwrap_or(u16::MAX)
            .min(inner.height.saturating_sub(1)),
    );
    frame.set_cursor_position(Position::new(x, y));
}

fn parse_fields(schema: &Value) -> Option<Vec<McpField>> {
    let schema = schema.as_object()?;
    if schema.get("type").and_then(Value::as_str) != Some("object") {
        return None;
    }
    let properties = schema.get("properties")?.as_object()?;
    let required = parse_required(schema.get("required"), properties)?;
    properties
        .iter()
        .map(|(id, property)| parse_field(id, property, required.contains(id)))
        .collect()
}

fn is_empty_object_schema(schema: &Value) -> bool {
    schema
        .as_object()
        .and_then(|schema| {
            schema
                .get("type")
                .and_then(Value::as_str)
                .map(|type_name| (schema, type_name))
        })
        .is_some_and(|(schema, type_name)| {
            type_name == "object"
                && schema
                    .get("properties")
                    .and_then(Value::as_object)
                    .is_some_and(Map::is_empty)
        })
}

fn is_tool_suggestion(meta: Option<&Value>) -> bool {
    meta.and_then(Value::as_object)
        .and_then(|meta| meta.get(APPROVAL_META_KIND_KEY))
        .and_then(Value::as_str)
        == Some(APPROVAL_META_KIND_TOOL_SUGGESTION)
}

fn approval_fields(meta: Option<&Value>) -> Option<Vec<McpField>> {
    let is_tool_call = meta
        .and_then(Value::as_object)
        .and_then(|meta| meta.get(APPROVAL_META_KIND_KEY))
        .and_then(Value::as_str)
        == Some(APPROVAL_META_KIND_MCP_TOOL_CALL);
    let mut options = vec![McpOption {
        label: APPROVAL_ACCEPT_ONCE_VALUE.to_string(),
        value: Value::String(APPROVAL_ACCEPT_ONCE_VALUE.to_string()),
    }];
    if approval_supports_persist_mode(meta, APPROVAL_PERSIST_SESSION_VALUE) {
        options.push(McpOption {
            label: APPROVAL_ACCEPT_SESSION_VALUE.to_string(),
            value: Value::String(APPROVAL_ACCEPT_SESSION_VALUE.to_string()),
        });
    }
    if approval_supports_persist_mode(meta, APPROVAL_PERSIST_ALWAYS_VALUE) {
        options.push(McpOption {
            label: APPROVAL_ACCEPT_ALWAYS_VALUE.to_string(),
            value: Value::String(APPROVAL_ACCEPT_ALWAYS_VALUE.to_string()),
        });
    }
    if !is_tool_call {
        options.push(McpOption {
            label: APPROVAL_DECLINE_VALUE.to_string(),
            value: Value::String(APPROVAL_DECLINE_VALUE.to_string()),
        });
    }
    options.push(McpOption {
        label: APPROVAL_CANCEL_VALUE.to_string(),
        value: Value::String(APPROVAL_CANCEL_VALUE.to_string()),
    });
    Some(vec![McpField {
        id: "__approval".to_string(),
        label: String::new(),
        description: None,
        required: true,
        input: McpFieldInput::Select {
            options,
            default_index: None,
        },
    }])
}

fn approval_supports_persist_mode(meta: Option<&Value>, expected_mode: &str) -> bool {
    let Some(persist) = meta
        .and_then(Value::as_object)
        .and_then(|meta| meta.get(APPROVAL_PERSIST_KEY))
    else {
        return false;
    };
    match persist {
        Value::String(value) => value == expected_mode,
        Value::Array(values) => values
            .iter()
            .filter_map(Value::as_str)
            .any(|value| value == expected_mode),
        _ => false,
    }
}

fn parse_required(
    value: Option<&Value>,
    properties: &Map<String, Value>,
) -> Option<HashSet<String>> {
    let Some(value) = value else {
        return Some(HashSet::new());
    };
    if value.is_null() {
        return Some(HashSet::new());
    }
    let required = value.as_array()?;
    let mut names = HashSet::new();
    for value in required {
        let name = value.as_str()?.to_string();
        if !properties.contains_key(&name) {
            return None;
        }
        names.insert(name);
    }
    Some(names)
}

fn parse_field(id: &str, property: &Value, required: bool) -> Option<McpField> {
    let property = property.as_object()?;
    let type_name = property.get("type")?.as_str()?;
    let label = string_property(property, "title")?.unwrap_or_else(|| id.to_string());
    let description = string_property(property, "description")?;
    let input = match type_name {
        "string" if property.contains_key("enum") || property.contains_key("oneOf") => {
            McpFieldInput::Select {
                options: parse_options(property)?,
                default_index: parse_default_index(property)?,
            }
        }
        "string" => McpFieldInput::Text {
            default: optional_string_property(property, "default")?,
        },
        "boolean" => McpFieldInput::Select {
            options: vec![
                McpOption {
                    label: "true".to_string(),
                    value: Value::Bool(true),
                },
                McpOption {
                    label: "false".to_string(),
                    value: Value::Bool(false),
                },
            ],
            default_index: match property.get("default") {
                None | Some(Value::Null) => None,
                Some(value) => Some(usize::from(!value.as_bool()?)),
            },
        },
        _ => return None,
    };
    Some(McpField {
        id: id.to_string(),
        label,
        description,
        required,
        input,
    })
}

fn parse_options(property: &Map<String, Value>) -> Option<Vec<McpOption>> {
    if property.contains_key("enum") && property.contains_key("oneOf") {
        return None;
    }
    if let Some(values) = property.get("enum") {
        let values = values.as_array()?;
        if values.is_empty() {
            return None;
        }
        let labels = match property.get("enumNames") {
            None | Some(Value::Null) => None,
            Some(Value::Array(labels)) => Some(
                labels
                    .iter()
                    .map(Value::as_str)
                    .collect::<Option<Vec<_>>>()?,
            ),
            Some(_) => return None,
        };
        if labels
            .as_ref()
            .is_some_and(|labels| labels.len() != values.len())
        {
            return None;
        }
        return values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                let value = value.as_str()?.to_string();
                Some(McpOption {
                    label: labels
                        .as_ref()
                        .and_then(|labels| labels.get(index).copied())
                        .unwrap_or(value.as_str())
                        .to_string(),
                    value: Value::String(value),
                })
            })
            .collect();
    }

    let values = property.get("oneOf")?.as_array()?;
    if values.is_empty() {
        return None;
    }
    values
        .iter()
        .map(|value| {
            let option = value.as_object()?;
            Some(McpOption {
                label: option.get("title")?.as_str()?.to_string(),
                value: Value::String(option.get("const")?.as_str()?.to_string()),
            })
        })
        .collect()
}

fn parse_default_index(property: &Map<String, Value>) -> Option<Option<usize>> {
    let Some(default) = property.get("default") else {
        return Some(None);
    };
    let default = default.as_str()?;
    let options = parse_options(property)?;
    Some(
        options
            .iter()
            .position(|option| option.value.as_str() == Some(default)),
    )
}

fn string_property(property: &Map<String, Value>, name: &str) -> Option<Option<String>> {
    match property.get(name) {
        None | Some(Value::Null) => Some(None),
        Some(Value::String(value)) => Some(Some(value.clone())),
        Some(_) => None,
    }
}

fn optional_string_property(property: &Map<String, Value>, name: &str) -> Option<Option<String>> {
    string_property(property, name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_server_protocol::protocol::v2::McpServerElicitationRequest;
    use crossterm::event::KeyModifiers;
    use ratatui::backend::TestBackend;
    use ratatui::layout::Rect;
    use ratatui::Terminal;
    use serde_json::json;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn params(schema: Value) -> McpServerElicitationRequestParams {
        params_with_meta(schema, None)
    }

    fn params_with_meta(schema: Value, meta: Option<Value>) -> McpServerElicitationRequestParams {
        let Value::Object(requested_schema) = schema else {
            panic!("test schema must be an object");
        };
        McpServerElicitationRequestParams {
            thread_id: "thread-1".to_string(),
            turn_id: Some("turn-1".to_string()),
            server_name: "server-1".to_string(),
            request: McpServerElicitationRequest::Form {
                meta,
                message: "Choose values".to_string(),
                requested_schema,
            },
        }
    }

    fn overlay(schema: Value) -> McpServerElicitationOverlay {
        McpServerElicitationOverlay::from_server_request(RequestId::Integer(7), &params(schema))
            .expect("supported MCP form")
    }

    #[test]
    fn parses_string_boolean_and_single_select_fields() {
        let overlay = overlay(json!({
            "type": "object",
            "properties": {
                "name": { "type": "string", "title": "Name" },
                "confirmed": { "type": "boolean", "default": true },
                "mode": { "type": "string", "enum": ["fast", "safe"], "enumNames": ["Fast", "Safe"] }
            },
            "required": ["name", "confirmed", "mode"]
        }));

        assert_eq!(overlay.fields.len(), 3);
        assert!(matches!(
            overlay.fields[0].input,
            McpFieldInput::Text { .. }
        ));
        assert!(matches!(
            overlay.fields[1].input,
            McpFieldInput::Select { .. }
        ));
        assert!(matches!(
            overlay.fields[2].input,
            McpFieldInput::Select { .. }
        ));
        assert_eq!(overlay.field_value(1), Some(Value::Bool(true)));
    }

    #[test]
    fn parses_titled_single_select_and_rejects_unsupported_shapes() {
        let overlay = overlay(json!({
            "type": "object",
            "properties": {
                "mode": { "type": "string", "oneOf": [
                    { "const": "fast", "title": "Fast" },
                    { "const": "safe", "title": "Safe" }
                ] }
            }
        }));
        assert_eq!(overlay.current_options().len(), 2);

        for schema in [
            json!({ "type": "object", "properties": { "count": { "type": "number" } } }),
            json!({ "type": "object", "properties": { "tags": { "type": "string", "enum": [], "enumNames": [] } } }),
        ] {
            assert!(McpServerElicitationOverlay::from_server_request(
                RequestId::Integer(1),
                &params(schema)
            )
            .is_none());
        }
    }

    #[test]
    fn empty_schema_uses_generic_approval_actions() {
        let mut overlay = McpServerElicitationOverlay::from_server_request(
            RequestId::Integer(8),
            &params(json!({ "type": "object", "properties": {} })),
        )
        .expect("empty object schema should use approval surface");

        assert_eq!(overlay.response_mode, McpResponseMode::ApprovalAction);
        assert_eq!(overlay.current_options().len(), 3);
        let response = overlay
            .handle_key_event(key(KeyCode::Enter))
            .expect("default allow response");
        let AppServerResponse::McpElicitation { response, .. } = response else {
            unreachable!();
        };
        assert_eq!(response.action, McpServerElicitationAction::Accept);
        assert_eq!(response.content, Some(json!({})));
        assert!(response.meta.is_none());
    }

    #[test]
    fn empty_schema_decline_and_persisted_accept_are_typed() {
        let mut decline = McpServerElicitationOverlay::from_server_request(
            RequestId::Integer(9),
            &params(json!({ "type": "object", "properties": {} })),
        )
        .expect("generic approval surface");
        decline.handle_key_event(key(KeyCode::Down));
        let response = decline
            .handle_key_event(key(KeyCode::Enter))
            .expect("decline response");
        let AppServerResponse::McpElicitation { response, .. } = response else {
            unreachable!();
        };
        assert_eq!(response.action, McpServerElicitationAction::Decline);
        assert!(response.content.is_none());

        let mut persisted = McpServerElicitationOverlay::from_server_request(
            RequestId::Integer(10),
            &params_with_meta(
                json!({ "type": "object", "properties": {} }),
                Some(json!({ "persist": ["session", "always"] })),
            ),
        )
        .expect("persisted approval surface");
        assert_eq!(persisted.current_options().len(), 5);
        persisted.handle_key_event(key(KeyCode::Down));
        let response = persisted
            .handle_key_event(key(KeyCode::Enter))
            .expect("session accept response");
        let AppServerResponse::McpElicitation { response, .. } = response else {
            unreachable!();
        };
        assert_eq!(response.action, McpServerElicitationAction::Accept);
        assert_eq!(response.content, Some(json!({})));
        assert_eq!(
            response.meta,
            Some(Map::from_iter([(
                "persist".to_string(),
                Value::String("session".to_string()),
            )]))
        );
    }

    #[test]
    fn tool_suggestion_empty_schema_remains_fail_closed_without_a_consumer() {
        let request = params_with_meta(
            json!({ "type": "object", "properties": {} }),
            Some(json!({ "codex_approval_kind": "tool_suggestion" })),
        );
        assert!(
            McpServerElicitationOverlay::from_server_request(RequestId::Integer(11), &request)
                .is_none()
        );
    }

    #[test]
    fn enter_collects_multiple_fields_and_emits_typed_content() {
        let mut overlay = overlay(json!({
            "type": "object",
            "properties": {
                "name": { "type": "string" },
                "confirmed": { "type": "boolean" }
            },
            "required": ["name", "confirmed"]
        }));
        overlay.handle_paste("Ada");
        assert!(overlay.handle_key_event(key(KeyCode::Enter)).is_none());
        assert_eq!(overlay.current_field, 1);
        let response = overlay
            .handle_key_event(key(KeyCode::Enter))
            .expect("elicitation response");
        assert!(matches!(
            response,
            AppServerResponse::McpElicitation {
                response: McpServerElicitationRequestResponse {
                    action: McpServerElicitationAction::Accept,
                    content: Some(Value::Object(_)),
                    ..
                },
                ..
            }
        ));
        let AppServerResponse::McpElicitation { response, .. } = response else {
            unreachable!();
        };
        assert_eq!(
            response.content,
            Some(json!({ "name": "Ada", "confirmed": true }))
        );
    }

    #[test]
    fn required_validation_moves_to_missing_field_and_cancel_is_typed() {
        let mut overlay = overlay(json!({
            "type": "object",
            "properties": { "name": { "type": "string" } },
            "required": ["name"]
        }));
        assert!(overlay.handle_key_event(key(KeyCode::Enter)).is_none());
        assert!(overlay.validation_error);
        assert!(!overlay.is_complete());

        let response = overlay
            .handle_key_event(key(KeyCode::Esc))
            .expect("cancel response");
        assert!(matches!(
            response,
            AppServerResponse::McpElicitation {
                response: McpServerElicitationRequestResponse {
                    action: McpServerElicitationAction::Cancel,
                    content: None,
                    ..
                },
                ..
            }
        ));
        assert!(overlay.is_complete());
    }

    #[test]
    fn select_fields_support_horizontal_navigation_and_digits() {
        let mut overlay = overlay(json!({
            "type": "object",
            "properties": {
                "first": { "type": "boolean" },
                "second": { "type": "string", "enum": ["a", "b"] }
            }
        }));
        assert!(overlay.handle_key_event(key(KeyCode::Char(' '))).is_none());
        assert!(overlay.handle_key_event(key(KeyCode::Right)).is_none());
        assert_eq!(overlay.current_field, 1);
        let response = overlay
            .handle_key_event(key(KeyCode::Char('2')))
            .expect("last field response");
        let AppServerResponse::McpElicitation { response, .. } = response else {
            unreachable!();
        };
        assert_eq!(
            response.content,
            Some(json!({ "first": true, "second": "b" }))
        );
    }

    #[test]
    fn form_surface_localizes_generated_labels_and_controls() {
        let overlay = overlay(json!({
            "type": "object",
            "properties": { "confirmed": { "type": "boolean" } }
        }));
        for locale in [
            Locale::ZhCn,
            Locale::ZhTw,
            Locale::EnUs,
            Locale::JaJp,
            Locale::KoKr,
        ] {
            let lines = lines_with_locale(&overlay, locale);
            let text = lines
                .iter()
                .flat_map(|line| line.spans.iter())
                .map(|span| span.content.as_ref())
                .collect::<String>();
            assert!(text.contains("server-1"));
            assert!(text.contains(&locale.mcp_elicitation_progress(1, 1)));
            assert!(text.contains(locale.mcp_elicitation_boolean_option(true)));
            assert!(text.contains(locale.mcp_elicitation_controls(true).trim()));
        }
    }

    #[test]
    fn narrow_select_surface_bounds_rows_and_keeps_cancel_visible() {
        let options = (0..12)
            .map(|index| format!("choice-{index}"))
            .collect::<Vec<_>>();
        let mut overlay = overlay(json!({
            "type": "object",
            "properties": {
                "choice": { "type": "string", "enum": options }
            }
        }));
        for _ in 0..11 {
            assert!(overlay.handle_key_event(key(KeyCode::Down)).is_none());
        }

        for width in [1, 4, 12, 24] {
            let lines = lines_with_locale_with_width(&overlay, Locale::EnUs, width);
            if width >= 8 {
                let option_lines = lines
                    .iter()
                    .filter(|line| is_option_line(line))
                    .collect::<Vec<_>>();
                assert_eq!(option_lines.len(), MAX_POPUP_ROWS);
                assert!(option_lines
                    .iter()
                    .any(|line| line.to_string().starts_with('›')));
            }
            assert!(lines
                .iter()
                .all(|line| display_width(&line.to_string()) <= width));
            let footer = lines.last().expect("footer line").to_string();
            if width >= display_width("Esc") {
                assert!(footer.contains("Esc"), "footer={footer:?}, width={width}");
            }
        }
    }

    #[test]
    fn narrow_footer_wraps_actions_in_priority_order_for_all_locales() {
        let overlay = overlay(json!({
            "type": "object",
            "properties": {
                "choice": { "type": "string", "enum": ["one", "two"] },
                "note": { "type": "string" }
            },
            "required": ["choice", "note"]
        }));

        for locale in [
            Locale::ZhCn,
            Locale::ZhTw,
            Locale::EnUs,
            Locale::JaJp,
            Locale::KoKr,
        ] {
            let lines = lines_with_locale_with_width(&overlay, locale, 8);
            let footer = lines
                .iter()
                .skip_while(|line| {
                    !line.to_string().contains("Enter")
                        && !line.to_string().contains("确认")
                        && !line.to_string().contains("確定")
                        && !line.to_string().contains("확인")
                })
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            assert!(!footer.is_empty(), "missing submit footer for {locale:?}");
            assert!(
                footer.last().is_some_and(|line| line.contains("Esc")
                    || line.contains("取消")
                    || line.contains("キャンセル")
                    || line.contains("취소")
                    || line.contains('⎋')),
                "cancel action must remain last for {locale:?}: {footer:?}"
            );
            assert!(
                lines
                    .iter()
                    .all(|line| display_width(&line.to_string()) <= 8),
                "footer overflow for {locale:?}: {lines:?}"
            );
        }
    }

    #[test]
    fn narrow_multiline_text_keeps_physical_rows_and_cursor_aligned() {
        let mut overlay = overlay(json!({
            "type": "object",
            "properties": {
                "answer": { "type": "string", "title": "回答" }
            },
            "required": ["answer"]
        }));
        overlay.handle_paste("你好\n👩🏽‍💻");

        let width = 12usize;
        let lines = lines_with_locale_with_width(&overlay, Locale::ZhCn, width);
        let input_index = lines.iter().position(is_input_line).expect("editable row");
        assert_eq!(lines[input_index].to_string(), "› 你好");
        assert_eq!(lines[input_index + 1].to_string(), "👩🏽‍💻");
        assert!(lines
            .iter()
            .all(|line| display_width(&line.to_string()) <= width));

        let mut terminal = Terminal::new(TestBackend::new(width as u16, 16)).expect("terminal");
        terminal
            .draw(|frame| {
                set_cursor_position(frame, Rect::new(0, 0, width as u16, 16), &overlay, &lines)
            })
            .expect("draw");
        let cursor = terminal.backend().cursor_position();
        assert_eq!(
            cursor.y,
            u16::try_from(input_index + 1).expect("cursor row")
        );
        assert_eq!(cursor.x, display_width("👩🏽‍💻") as u16);
    }
}
