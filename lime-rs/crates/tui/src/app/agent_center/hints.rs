//! The compact footer and help consume the same resolved task bindings as dispatch.

use super::*;
use crate::keymap::AgentsKeymapAction;
use crate::locale::Locale;
use crate::style::{footer_hint_label_style, key_hint_style};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::Stylize;
use ratatui::text::{Line, Span};

pub(super) fn hint_line(items: &[(String, String)]) -> Line<'static> {
    let mut spans = Vec::new();
    for (key, label) in items.iter().filter(|(key, _)| !key.is_empty()) {
        if !spans.is_empty() {
            spans.push(Span::raw("  "));
        }
        spans.push(Span::styled(key.clone(), key_hint_style()));
        spans.push(Span::styled(
            format!(" {label}"),
            footer_hint_label_style().not_bold(),
        ));
    }
    Line::from(spans)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::agents_overview_view::{AgentsOverviewAction, AgentsOverviewInputMode};
    use crossterm::event::Event;

    #[test]
    fn footer_yields_to_task_keys_and_restores_cancel_while_editing() {
        let mut config = lime_core::config::TuiKeymap::default();
        let binding = |key: &str| {
            lime_core::config::KeybindingsSpec::One(lime_core::config::KeybindingSpec(key.into()))
        };
        config.agents.new_task = Some(binding("up"));
        config.agents.resume = Some(binding("esc"));
        let runtime = crate::keymap::RuntimeKeymap::from_config(&config).unwrap();
        let mut view =
            AgentsOverviewView::new_with_keymap(Vec::new(), None, runtime.agents().clone());
        let hints = view.center_footer_hints(Locale::EnUs);
        assert!(hints
            .iter()
            .any(|(key, action)| key == "ctrl+p/↓" && action == "move"));
        assert!(!hints.iter().any(|(_, action)| action == "back"));
        assert_eq!(
            view.handle_event(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))),
            AgentsOverviewAction::OpenResumePicker
        );
        view.handle_event(Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)));
        assert_eq!(view.input_mode(), Some(AgentsOverviewInputMode::NewTask));
        assert!(view
            .center_footer_hints(Locale::EnUs)
            .iter()
            .any(|(key, action)| key == "esc" && action == "back"));
        assert_eq!(
            view.handle_event(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))),
            AgentsOverviewAction::None
        );
        assert_eq!(view.input_mode(), None);
    }
}

impl AgentsOverviewView {
    fn list_hint(&self, code: KeyCode) -> Option<String> {
        let candidates: &[(KeyCode, KeyModifiers, &str)] = match code {
            KeyCode::Up => &[
                (KeyCode::Up, KeyModifiers::NONE, "↑"),
                (KeyCode::Char('p'), KeyModifiers::CONTROL, "ctrl+p"),
                (KeyCode::Char('k'), KeyModifiers::CONTROL, "ctrl+k"),
                (KeyCode::Char('k'), KeyModifiers::NONE, "k"),
            ],
            KeyCode::Down => &[
                (KeyCode::Down, KeyModifiers::NONE, "↓"),
                (KeyCode::Char('n'), KeyModifiers::CONTROL, "ctrl+n"),
                (KeyCode::Char('j'), KeyModifiers::CONTROL, "ctrl+j"),
                (KeyCode::Char('j'), KeyModifiers::NONE, "j"),
            ],
            KeyCode::PageUp => &[
                (KeyCode::PageUp, KeyModifiers::NONE, "pgup"),
                (KeyCode::Char('b'), KeyModifiers::CONTROL, "ctrl+b"),
            ],
            KeyCode::PageDown => &[
                (KeyCode::PageDown, KeyModifiers::NONE, "pgdn"),
                (KeyCode::Char('f'), KeyModifiers::CONTROL, "ctrl+f"),
            ],
            KeyCode::Enter => &[(KeyCode::Enter, KeyModifiers::NONE, "enter")],
            KeyCode::Esc => &[(KeyCode::Esc, KeyModifiers::NONE, "esc")],
            _ => return None,
        };
        candidates
            .iter()
            .find(|(code, modifiers, _)| {
                !self
                    .agents_keymap
                    .reserves_key(KeyEvent::new(*code, *modifiers))
            })
            .map(|(_, _, label)| label.to_string())
    }

    pub(super) fn center_filter_hint(&self) -> String {
        [
            (KeyCode::Tab, KeyModifiers::NONE, "tab"),
            (KeyCode::BackTab, KeyModifiers::SHIFT, "shift+tab"),
        ]
        .into_iter()
        .filter(|(code, modifiers, _)| {
            !self
                .agents_keymap
                .reserves_key(KeyEvent::new(*code, *modifiers))
        })
        .map(|(_, _, label)| label)
        .collect::<Vec<_>>()
        .join("/")
    }

    pub(super) fn center_footer_hints(&self, locale: Locale) -> Vec<(String, String)> {
        let label = |name| locale.agent_center_label(name).to_string();
        let mut hints = Vec::new();
        if !self.help
            && !self.editing_metadata()
            && !self
                .agents_keymap
                .reserves_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE))
        {
            hints.push(("?".into(), label("help")));
        }
        let cancel = if self.help || self.editing_metadata() {
            Some("esc".into())
        } else {
            self.list_hint(KeyCode::Esc)
        };
        if let Some(cancel) = cancel {
            hints.push((cancel, label("back")));
        }
        if self.help {
            return hints;
        }
        if !self.editing_metadata() {
            let navigation = [KeyCode::Up, KeyCode::Down]
                .into_iter()
                .filter_map(|code| self.list_hint(code))
                .collect::<Vec<_>>()
                .join("/");
            if !navigation.is_empty() {
                hints.push((navigation, label("move")));
            }
        }
        let action = match self.input_mode {
            Some(super::super::AgentsOverviewInputMode::Rename) => "rename",
            Some(super::super::AgentsOverviewInputMode::NewTask) => "confirm",
            None => "open",
        };
        let accept = if self.editing_metadata() {
            Some("enter".into())
        } else {
            self.list_hint(KeyCode::Enter)
        };
        if let Some(accept) = accept {
            hints.push((accept, label(action)));
        }
        if !self.editing_metadata() {
            if let Some(key) = self.agents_keymap.primary_hint(AgentsKeymapAction::NewTask) {
                hints.push((key, label("new")));
            }
        }
        hints
    }

    pub(super) fn center_help_lines(&self, locale: Locale, width: u16) -> Vec<Line<'static>> {
        use crate::shortcut_help::Group;
        let mut navigate = Group {
            title: locale.agent_center_label("Navigate"),
            entries: Vec::new(),
        };
        for (code, action) in [
            (KeyCode::Up, "Up"),
            (KeyCode::Down, "Down"),
            (KeyCode::Enter, "Open"),
            (KeyCode::PageUp, "Page up"),
            (KeyCode::PageDown, "Page down"),
        ] {
            navigate.push(self.list_hint(code), locale.agent_center_label(action));
        }
        navigate.push(Some("ctrl+c".into()), locale.agent_center_label("Quit"));
        let mut tasks = Group {
            title: locale.agent_center_label("Tasks"),
            entries: Vec::new(),
        };
        for (action, label) in [
            (AgentsKeymapAction::NewTask, "New"),
            (AgentsKeymapAction::Resume, "Resume"),
            (AgentsKeymapAction::Rename, "Rename"),
            (AgentsKeymapAction::Stop, "Stop"),
        ] {
            tasks.push(
                self.agents_keymap.primary_hint(action),
                locale.agent_center_label(label),
            );
        }
        let mut view = Group {
            title: locale.agent_center_label("View"),
            entries: Vec::new(),
        };
        view.push(
            Some(self.center_filter_hint()),
            locale.agent_center_label("Filter"),
        );
        for (action, label) in [
            (AgentsKeymapAction::Search, "Search"),
            (AgentsKeymapAction::ToggleGrouping, "Group"),
        ] {
            view.push(
                self.agents_keymap.primary_hint(action),
                locale.agent_center_label(label),
            );
        }
        let mut lines = vec![
            locale.agent_center_label("Task shortcuts").bold().into(),
            Line::default(),
        ];
        lines.extend(crate::shortcut_help::group_lines(
            [navigate, tasks, view],
            width,
        ));
        lines
    }
}
