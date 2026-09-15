//! Agent picker UI matching Codex's `/subagents` interaction shape.
//!
//! Rows are snapshots of the App Server-backed navigation state. Selecting a row only returns a
//! thread id; `App` and `AppServerSession` own the actual `thread/resume` request.

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use super::agent_navigation::AgentNavigationState;
use crate::bottom_pane::selection_row_layout::{
    centered_popup, visible_item_window, wrap_row, SelectionDescriptionLayout, SelectionRow,
    MAX_POPUP_ROWS,
};
use crate::line_truncation::truncate_line_with_ellipsis_if_overflow;
use crate::locale::Locale;
use crate::multi_agents::format_agent_picker_item_name;
use crate::style::{accent_style, muted_style};

#[derive(Debug, Clone, PartialEq, Eq)]
struct AgentPickerEntry {
    thread_id: String,
    label: String,
    is_running: bool,
    is_closed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AgentPickerAction {
    None,
    Cancel,
    Select(String),
}

#[derive(Debug, Clone)]
pub(crate) struct AgentPicker {
    entries: Vec<AgentPickerEntry>,
    selected: usize,
}

impl AgentPicker {
    #[allow(dead_code)]
    pub(crate) fn from_navigation(
        navigation: &AgentNavigationState,
        primary_thread_id: Option<&str>,
    ) -> Self {
        let entries = navigation
            .ordered_threads()
            .into_iter()
            .map(|(thread_id, entry)| {
                let is_primary = primary_thread_id == Some(thread_id);
                let label = format_agent_picker_item_name(
                    entry.agent_nickname.as_deref(),
                    entry.agent_role.as_deref(),
                    is_primary,
                );
                AgentPickerEntry {
                    thread_id: thread_id.to_string(),
                    label,
                    is_running: entry.is_running,
                    is_closed: entry.is_closed,
                }
            })
            .collect();
        Self {
            entries,
            selected: 0,
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn handle_event(&mut self, event: Event) -> AgentPickerAction {
        match event {
            Event::Key(key) if key.kind == KeyEventKind::Press => {
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('d'))
                {
                    return AgentPickerAction::Cancel;
                }
                match key.code {
                    KeyCode::Esc => AgentPickerAction::Cancel,
                    KeyCode::Up | KeyCode::Char('p') | KeyCode::Char('k')
                        if key.code == KeyCode::Up
                            || key.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        if !self.entries.is_empty() {
                            self.selected = self
                                .selected
                                .checked_sub(1)
                                .unwrap_or(self.entries.len().saturating_sub(1));
                        }
                        AgentPickerAction::None
                    }
                    KeyCode::Down | KeyCode::Char('n') | KeyCode::Char('j')
                        if key.code == KeyCode::Down
                            || key.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        if !self.entries.is_empty() {
                            self.selected = (self.selected + 1) % self.entries.len();
                        }
                        AgentPickerAction::None
                    }
                    KeyCode::Enter => self
                        .entries
                        .get(self.selected)
                        .map(|entry| AgentPickerAction::Select(entry.thread_id.clone()))
                        .unwrap_or(AgentPickerAction::None),
                    _ => AgentPickerAction::None,
                }
            }
            _ => AgentPickerAction::None,
        }
    }

    fn rows(&self) -> impl Iterator<Item = &AgentPickerEntry> {
        self.entries.iter()
    }
}

pub(crate) fn render(frame: &mut Frame<'_>, area: Rect, picker: &AgentPicker, locale: Locale) {
    let width = area.width.saturating_mul(4).saturating_div(5).clamp(32, 72);
    let height = area
        .height
        .saturating_mul(3)
        .saturating_div(4)
        .clamp(7, MAX_POPUP_ROWS as u16 + 4);
    let popup = centered_popup(area, width, height);

    frame.render_widget(Clear, popup);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(popup);
    frame.render_widget(
        Paragraph::new(truncate_line_with_ellipsis_if_overflow(
            Line::from(vec![Span::styled(
                format!(" {} ", locale.agent_picker_title()),
                Style::default().add_modifier(Modifier::BOLD),
            )]),
            usize::from(chunks[0].width.saturating_sub(2)),
        ))
        .block(Block::default().borders(Borders::TOP)),
        chunks[0],
    );

    let row_width = chunks[1].width.saturating_sub(2);
    let desc_col = usize::from(row_width.saturating_mul(2).saturating_div(5).max(1));
    let rows = picker.rows().collect::<Vec<_>>();
    let max_visible = MAX_POPUP_ROWS.min(usize::from(chunks[1].height).max(1));
    let (start, end) = visible_item_window(picker.selected, rows.len(), max_visible);
    let items = rows
        .into_iter()
        .skip(start)
        .take(end.saturating_sub(start))
        .map(|entry| {
            let status = if entry.is_closed {
                format!(" [{}]", locale.status("closed"))
            } else if entry.is_running {
                format!(" [{}]", locale.status("running"))
            } else {
                String::new()
            };
            let row = SelectionRow::new(
                entry.label.clone(),
                (!status.is_empty()).then_some(status),
                Vec::new(),
            );
            ListItem::new(wrap_row(
                &row,
                desc_col,
                row_width,
                SelectionDescriptionLayout::StackBelowWhenNarrow {
                    min_description_width: 10,
                },
            ))
        })
        .collect::<Vec<_>>();
    let mut state = ListState::default();
    if !items.is_empty() {
        state.select(Some(picker.selected.saturating_sub(start)));
    }
    frame.render_stateful_widget(
        List::new(items)
            .block(Block::default().borders(Borders::LEFT | Borders::RIGHT))
            .highlight_style(accent_style())
            .highlight_symbol("› "),
        chunks[1],
        &mut state,
    );
    let footer = if picker.is_empty() {
        locale.agent_picker_empty()
    } else {
        locale.agent_picker_footer()
    };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(footer, muted_style()))),
        chunks[2],
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyEvent;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn picker() -> AgentPicker {
        let mut navigation = AgentNavigationState::default();
        navigation.upsert("main", None, None, false);
        navigation.upsert(
            "agent-1",
            Some("Robie".to_string()),
            Some("explorer".to_string()),
            false,
        );
        AgentPicker::from_navigation(&navigation, Some("main"))
    }

    #[test]
    fn picker_selects_rows_in_stable_navigation_order() {
        let mut picker = picker();
        assert_eq!(
            picker.handle_event(Event::Key(crossterm::event::KeyEvent::new(
                KeyCode::Down,
                KeyModifiers::NONE,
            ))),
            AgentPickerAction::None
        );
        assert_eq!(
            picker.handle_event(Event::Key(crossterm::event::KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            ))),
            AgentPickerAction::Select("agent-1".to_string())
        );
    }

    #[test]
    fn picker_navigation_wraps_and_accepts_control_bindings() {
        let mut agent_picker = picker();

        agent_picker.handle_event(Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)));
        assert_eq!(
            agent_picker.handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            ))),
            AgentPickerAction::Select("agent-1".to_string())
        );

        let mut agent_picker = picker();
        agent_picker.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char('n'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(
            agent_picker.handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            ))),
            AgentPickerAction::Select("agent-1".to_string())
        );
        agent_picker.handle_event(Event::Key(KeyEvent::new(
            KeyCode::Char('k'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(
            agent_picker.handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            ))),
            AgentPickerAction::Select("main".to_string())
        );
    }

    #[test]
    fn picker_escape_cancels_and_narrow_render_stays_bounded() {
        let mut picker = picker();
        assert_eq!(
            picker.handle_event(Event::Key(crossterm::event::KeyEvent::new(
                KeyCode::Esc,
                KeyModifiers::NONE,
            ))),
            AgentPickerAction::Cancel
        );
        let mut terminal = Terminal::new(TestBackend::new(16, 7)).expect("terminal");
        terminal
            .draw(|frame| render(frame, frame.area(), &picker, Locale::EnUs))
            .expect("draw");
    }

    #[test]
    fn picker_copy_and_status_render_for_every_product_locale() {
        let picker = picker();
        let locales = [
            Locale::ZhCn,
            Locale::ZhTw,
            Locale::EnUs,
            Locale::JaJp,
            Locale::KoKr,
        ];
        for locale in locales {
            let mut terminal = Terminal::new(TestBackend::new(64, 12)).expect("terminal");
            terminal
                .draw(|frame| render(frame, frame.area(), &picker, locale))
                .expect("draw");
            let buffer = terminal.backend().buffer();
            let text = (0..buffer.area.height)
                .map(|y| {
                    (0..buffer.area.width)
                        .map(|x| buffer[(x, y)].symbol())
                        .collect::<String>()
                })
                .collect::<Vec<_>>()
                .join("\n");
            let compact = |value: &str| {
                value
                    .chars()
                    .filter(|character| !character.is_whitespace())
                    .collect::<String>()
            };
            assert!(
                compact(&text).contains(&compact(locale.agent_picker_title())),
                "locale={locale:?} text={text:?}"
            );
            assert!(
                compact(&text).contains(&compact(locale.agent_picker_footer())),
                "locale={locale:?} text={text:?}"
            );
        }
    }

    #[test]
    fn long_agent_catalog_keeps_selected_row_inside_bounded_popup() {
        let mut navigation = AgentNavigationState::default();
        for index in 0..20 {
            navigation.upsert(
                format!("agent-{index:02}"),
                Some(format!("Agent {index:02}")),
                Some("worker".to_string()),
                false,
            );
        }
        let mut picker = AgentPicker::from_navigation(&navigation, None);
        for _ in 0..12 {
            picker.handle_event(Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)));
        }

        let mut terminal = Terminal::new(TestBackend::new(80, 20)).expect("terminal");
        terminal
            .draw(|frame| render(frame, frame.area(), &picker, Locale::EnUs))
            .expect("draw");
        let buffer = terminal.backend().buffer();
        let text = (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            text.contains("Agent 12"),
            "selected agent was clipped: {text}"
        );
        assert!(text.lines().all(|line| line.chars().count() <= 80));
    }
}
