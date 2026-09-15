use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;

use crate::line_truncation::truncate_line_with_ellipsis_if_overflow;
use crate::locale::Locale;
use crate::slash_command::{command_filter, SlashCommand};
use crate::style::{accent_style, muted_style};

const MAX_VISIBLE_ROWS: usize = 8;

/// Actions returned to the composer host after handling one popup event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CommandPopupAction {
    Pass,
    Consumed,
    Cancel,
    Complete(SlashCommand),
    Execute(SlashCommand),
}

/// Slash-command completion state owned by the bottom pane composer.
///
/// The command catalog remains in `slash_command`; this type owns only the popup
/// lifecycle, selection, and terminal rendering. Keeping those responsibilities
/// together matches the Codex bottom-pane owner without introducing a second
/// command parser.
#[derive(Debug, Clone)]
pub(crate) struct CommandPopup {
    filter: String,
    selected: usize,
}

impl CommandPopup {
    pub(crate) fn for_composer(text: &str) -> Option<Self> {
        let filter = command_filter(text)?.to_ascii_lowercase();
        let popup = Self {
            filter,
            selected: 0,
        };
        (!popup.matches().is_empty()).then_some(popup)
    }

    pub(crate) fn update(&mut self, text: &str) -> bool {
        let Some(filter) = command_filter(text) else {
            return false;
        };
        self.filter = filter.to_ascii_lowercase();
        let matches = self.matches();
        if matches.is_empty() {
            return false;
        }
        self.selected = self.selected.min(matches.len() - 1);
        true
    }

    pub(crate) fn commands(&self) -> Vec<SlashCommand> {
        self.matches()
    }

    pub(crate) fn selected(&self) -> Option<SlashCommand> {
        self.matches().get(self.selected).copied()
    }

    pub(crate) fn handle_event(&mut self, event: &Event) -> CommandPopupAction {
        let Event::Key(key) = event else {
            return CommandPopupAction::Pass;
        };
        if key.kind != KeyEventKind::Press {
            return CommandPopupAction::Pass;
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('p') | KeyCode::Char('k')
                if key.code == KeyCode::Up || key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                let len = self.matches().len();
                if len > 0 {
                    self.selected = self.selected.checked_sub(1).unwrap_or(len - 1);
                }
                CommandPopupAction::Consumed
            }
            KeyCode::Down | KeyCode::Char('n') | KeyCode::Char('j')
                if key.code == KeyCode::Down || key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                let len = self.matches().len();
                if len > 0 {
                    self.selected = (self.selected + 1) % len;
                }
                CommandPopupAction::Consumed
            }
            KeyCode::Esc => CommandPopupAction::Cancel,
            KeyCode::Tab => self
                .selected()
                .map(CommandPopupAction::Complete)
                .unwrap_or(CommandPopupAction::Consumed),
            KeyCode::Enter => self
                .selected()
                .map(|command| {
                    if command.requires_argument() {
                        CommandPopupAction::Complete(command)
                    } else {
                        CommandPopupAction::Execute(command)
                    }
                })
                .unwrap_or(CommandPopupAction::Consumed),
            _ => CommandPopupAction::Pass,
        }
    }

    fn matches(&self) -> Vec<SlashCommand> {
        SlashCommand::ALL
            .into_iter()
            .filter(|command| command.command().starts_with(&self.filter))
            .collect()
    }
}

pub(crate) fn render(
    frame: &mut Frame<'_>,
    composer_area: Rect,
    popup: &CommandPopup,
    locale: Locale,
) {
    let commands = popup.commands();
    if commands.is_empty() || composer_area.y == 0 || composer_area.width == 0 {
        return;
    }
    let visible_rows = commands
        .len()
        .min(MAX_VISIBLE_ROWS)
        .min(usize::from(composer_area.y));
    let height = u16::try_from(visible_rows).unwrap_or(u16::MAX);
    if height == 0 {
        return;
    }
    let area = Rect::new(
        composer_area.x,
        composer_area.y.saturating_sub(height),
        composer_area.width,
        height,
    );
    // Keep the selected command inside the visible window as the user navigates a long catalog.
    // The popup remains anchored to the composer; only its slice moves.
    let selected = popup
        .selected()
        .and_then(|selected| commands.iter().position(|command| *command == selected));
    let start = selected
        .map(|selected| selected.saturating_sub(visible_rows.saturating_sub(1)))
        .unwrap_or_default();
    let lines = commands
        .into_iter()
        .enumerate()
        .skip(start)
        .take(visible_rows)
        .map(|(index, command)| {
            let selected = index == popup.selected;
            let marker = if selected { "› " } else { "  " };
            let command_style = if selected {
                accent_style()
            } else {
                Style::default().add_modifier(Modifier::BOLD)
            };
            truncate_line_with_ellipsis_if_overflow(
                Line::from(vec![
                    Span::styled(marker, command_style),
                    Span::styled(format!("/{}", command.command()), command_style),
                    Span::raw("  "),
                    Span::styled(command.description(locale), muted_style()),
                ]),
                usize::from(area.width),
            )
        })
        .collect::<Vec<_>>();
    frame.render_widget(Clear, area);
    frame.render_widget(Paragraph::new(lines), area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyModifiers};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn filters_by_prefix_and_wraps_selection() {
        let mut popup = CommandPopup::for_composer("/").expect("popup");
        assert_eq!(popup.selected(), Some(SlashCommand::Model));
        assert_eq!(
            popup.handle_event(&Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE,))),
            CommandPopupAction::Consumed
        );
        assert_eq!(popup.selected(), SlashCommand::ALL.last().copied());
        assert!(popup.update("/per"));
        assert_eq!(popup.commands(), vec![SlashCommand::Permissions]);
        assert_eq!(popup.selected(), Some(SlashCommand::Permissions));
        assert!(!popup.update("/unknown"));
    }

    #[test]
    fn enter_executes_immediate_commands_and_completes_argument_commands() {
        let event = Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        let mut model = CommandPopup::for_composer("/m").expect("model popup");
        assert_eq!(
            model.handle_event(&event),
            CommandPopupAction::Execute(SlashCommand::Model)
        );
        let mut effort = CommandPopup::for_composer("/e").expect("effort popup");
        assert_eq!(
            effort.handle_event(&event),
            CommandPopupAction::Complete(SlashCommand::Effort)
        );
    }

    #[test]
    fn control_navigation_matches_codex_list_bindings() {
        let mut popup = CommandPopup::for_composer("/").expect("popup");
        let first = popup.selected().expect("first command");

        popup.handle_event(&Event::Key(KeyEvent::new(
            KeyCode::Char('n'),
            KeyModifiers::CONTROL,
        )));
        let second = popup.selected().expect("second command");
        assert_ne!(second, first);

        popup.handle_event(&Event::Key(KeyEvent::new(
            KeyCode::Char('p'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(popup.selected(), Some(first));

        popup.handle_event(&Event::Key(KeyEvent::new(
            KeyCode::Char('k'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(
            popup.selected(),
            Some(popup.commands().last().copied().expect("last"))
        );
    }

    #[test]
    fn non_key_and_key_release_events_do_not_change_popup_state() {
        let mut popup = CommandPopup::for_composer("/m").expect("popup");
        let selected = popup.selected();
        assert_eq!(
            popup.handle_event(&Event::Resize(20, 5)),
            CommandPopupAction::Pass
        );
        assert_eq!(popup.selected(), selected);
        let mut release = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        release.kind = KeyEventKind::Release;
        assert_eq!(
            popup.handle_event(&Event::Key(release)),
            CommandPopupAction::Pass
        );
        assert_eq!(popup.selected(), selected);
    }

    #[test]
    fn empty_filter_keeps_catalog_order_and_narrow_render_is_bounded() {
        let popup = CommandPopup::for_composer("/").expect("popup");
        assert_eq!(popup.commands().first(), Some(&SlashCommand::Model));
        let mut terminal = Terminal::new(TestBackend::new(10, 4)).expect("terminal");
        terminal
            .draw(|frame| render(frame, Rect::new(0, 3, 10, 1), &popup, Locale::EnUs))
            .expect("draw");
        let row = (0..10)
            .map(|x| terminal.backend().buffer()[(x, 2)].symbol())
            .collect::<String>();
        assert!(row.chars().count() <= 10);
    }

    #[test]
    fn test_backend_renders_commands_and_localized_descriptions() {
        let popup = CommandPopup::for_composer("/").expect("popup");
        let mut terminal = Terminal::new(TestBackend::new(72, 10)).expect("terminal");
        terminal
            .draw(|frame| render(frame, Rect::new(0, 8, 72, 2), &popup, Locale::ZhCn))
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
        let compact = text
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>();

        assert!(text.contains("/model"));
        assert!(compact.contains("选择模型"), "{text}");
        assert!(text.contains("/copy"));
    }

    #[test]
    fn long_catalog_keeps_selected_row_visible_with_a_bounded_popup() {
        let mut popup = CommandPopup::for_composer("/").expect("popup");
        for _ in 0..10 {
            let _ = popup.handle_event(&Event::Key(KeyEvent::new(
                KeyCode::Down,
                KeyModifiers::NONE,
            )));
        }
        let selected = popup.selected().expect("selected command");
        let mut terminal = Terminal::new(TestBackend::new(72, 16)).expect("terminal");
        terminal
            .draw(|frame| render(frame, Rect::new(0, 15, 72, 1), &popup, Locale::EnUs))
            .expect("draw");
        let buffer = terminal.backend().buffer();
        let rows_with_marker = (0..buffer.area.height)
            .filter(|y| {
                let row = (0..buffer.area.width)
                .map(|x| buffer[(x, *y)].symbol())
                    .collect::<String>();
                row.contains(&format!("› /{}", selected.command()))
            })
            .count();
        assert_eq!(rows_with_marker, 1);
        assert!(popup.commands().len() > MAX_VISIBLE_ROWS);
    }
}
