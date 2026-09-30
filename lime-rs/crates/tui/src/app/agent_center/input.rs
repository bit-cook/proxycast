//! Metadata editing yields to text input; task shortcuts and paging share Codex priority.

use super::super::{AgentsOverviewAction, AgentsOverviewInputMode};
use super::*;
use crate::clipboard_paste::normalize_pasted_search_query;
use crate::key_hint::is_plain_text_key_event;
use crate::keymap::{AgentsKeymapAction, KeymapMatch};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use unicode_segmentation::UnicodeSegmentation;

fn delete_last_grapheme(text: &mut String) {
    if let Some((offset, _)) = text.grapheme_indices(true).next_back() {
        text.truncate(offset);
    }
}

impl AgentsOverviewView {
    pub(crate) fn handle_event(&mut self, event: Event) -> AgentsOverviewAction {
        if let Event::Paste(text) = event {
            self.key_chord_matcher.reset();
            if let Some(text) = normalize_pasted_search_query(&text) {
                if self.input_mode.is_some() {
                    self.input.push_str(&text);
                } else if self.searching {
                    self.search.push_str(&text);
                    self.selected = 0;
                    self.scroll.set(0);
                }
            }
            return AgentsOverviewAction::None;
        }
        let Event::Key(key) = event else {
            return AgentsOverviewAction::None;
        };
        if key.kind != KeyEventKind::Press {
            return AgentsOverviewAction::None;
        }
        if (key.code == KeyCode::Esc
            && (self.editing_metadata() || self.help || !self.agents_keymap.reserves_key(key)))
            || (key.modifiers.contains(KeyModifiers::CONTROL)
                && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('d')))
        {
            self.key_chord_matcher.reset();
            if self.editing_metadata() {
                let selected = self.selected_thread_id().map(str::to_owned);
                self.searching = false;
                self.search.clear();
                self.input_mode = None;
                self.input.clear();
                self.rename_target = None;
                self.task_cwd = None;
                self.select_thread(selected.as_deref());
            } else if self.help {
                self.help = false;
            } else {
                return AgentsOverviewAction::Cancel;
            }
            return AgentsOverviewAction::None;
        }
        if let Some(mode) = self.input_mode {
            match key.code {
                KeyCode::Backspace if key.modifiers.is_empty() => {
                    delete_last_grapheme(&mut self.input)
                }
                KeyCode::Enter => {
                    let input = std::mem::take(&mut self.input);
                    self.input_mode = None;
                    let target = self.rename_target.take();
                    let cwd = self.task_cwd.take();
                    self.select_thread(target.as_deref());
                    if input.trim().is_empty() {
                        return AgentsOverviewAction::None;
                    }
                    return match mode {
                        AgentsOverviewInputMode::NewTask => {
                            AgentsOverviewAction::Dispatch { prompt: input, cwd }
                        }
                        AgentsOverviewInputMode::Rename => target
                            .filter(|id| self.rows.iter().any(|row| row.thread.id == *id))
                            .map(|thread_id| AgentsOverviewAction::Rename {
                                thread_id,
                                name: input.trim().to_string(),
                            })
                            .unwrap_or(AgentsOverviewAction::None),
                    };
                }
                KeyCode::Char(character) if is_plain_text_key_event(key) => {
                    self.input.push(character)
                }
                _ => {}
            }
            return AgentsOverviewAction::None;
        }
        if self.searching {
            match key.code {
                KeyCode::Backspace if key.modifiers.is_empty() => {
                    delete_last_grapheme(&mut self.search);
                    self.selected = 0;
                    self.scroll.set(0);
                }
                KeyCode::Char(character) if is_plain_text_key_event(key) => {
                    self.search.push(character);
                    self.selected = 0;
                    self.scroll.set(0);
                }
                KeyCode::Enter => return self.activate(),
                _ => self.navigate(key),
            }
            self.clamp_selection();
            return AgentsOverviewAction::None;
        }
        if self.help {
            if key.code == KeyCode::Char('?') && is_plain_text_key_event(key) {
                self.help = false;
            }
            return AgentsOverviewAction::None;
        }
        match self
            .agents_keymap
            .dispatch(&mut self.key_chord_matcher, key)
        {
            KeymapMatch::Completed(action) => {
                match action {
                    AgentsKeymapAction::Resume => return AgentsOverviewAction::OpenResumePicker,
                    AgentsKeymapAction::Search => {
                        self.searching = true;
                        self.selected = 0;
                        self.scroll.set(0);
                    }
                    AgentsKeymapAction::NewTask => {
                        self.task_cwd = matches!(
                            self.grouping,
                            super::super::grouping::AgentsOverviewGrouping::Project
                        )
                        .then(|| self.selected_row().map(|row| row.thread.cwd.clone()))
                        .flatten();
                        self.input.clear();
                        self.input_mode = Some(AgentsOverviewInputMode::NewTask);
                    }
                    AgentsKeymapAction::Rename => {
                        if let Some((id, name)) = self.selected_row().map(|row| {
                            (
                                row.thread.id.clone(),
                                row.thread.name.clone().unwrap_or_default(),
                            )
                        }) {
                            self.rename_target = Some(id);
                            self.input = name;
                            self.input_mode = Some(AgentsOverviewInputMode::Rename);
                        }
                    }
                    AgentsKeymapAction::Stop => {
                        if let Some(row) = self.selected_row() {
                            if matches!(
                                row.thread.status,
                                app_server_protocol::protocol::v2::ThreadStatus::Active { .. }
                            ) {
                                return AgentsOverviewAction::Stop {
                                    thread_id: row.thread.id.clone(),
                                };
                            }
                        }
                    }
                    AgentsKeymapAction::ToggleGrouping => {
                        let id = self.selected_thread_id().map(str::to_owned);
                        use super::super::grouping::AgentsOverviewGrouping;
                        self.grouping = match self.grouping {
                            AgentsOverviewGrouping::Project => AgentsOverviewGrouping::Status,
                            AgentsOverviewGrouping::Status => AgentsOverviewGrouping::Project,
                        };
                        self.scroll.set(0);
                        self.select_thread(id.as_deref());
                    }
                }
                return AgentsOverviewAction::None;
            }
            KeymapMatch::Pending | KeymapMatch::Cancelled => return AgentsOverviewAction::None,
            KeymapMatch::PassThrough => {}
        }
        if matches!(key.code, KeyCode::Tab | KeyCode::BackTab)
            && !key
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            let id = self.selected_thread_id().map(str::to_owned);
            let step =
                if key.code == KeyCode::BackTab || key.modifiers.contains(KeyModifiers::SHIFT) {
                    TASK_FILTERS.len() - 1
                } else {
                    1
                };
            self.status_filter = (self.status_filter + step) % TASK_FILTERS.len();
            self.scroll.set(0);
            self.select_thread(id.as_deref());
        } else if key.code == KeyCode::Char('?') && is_plain_text_key_event(key) {
            self.help = true;
        } else if key.code == KeyCode::Enter || key.code == KeyCode::Right {
            return self.activate();
        } else {
            self.navigate(key);
        }
        AgentsOverviewAction::None
    }

    fn navigate(&mut self, key: KeyEvent) {
        match (key.code, key.modifiers) {
            (KeyCode::Up | KeyCode::Char('k'), KeyModifiers::NONE)
            | (KeyCode::Char('p') | KeyCode::Char('k'), KeyModifiers::CONTROL) => {
                self.move_selection(false)
            }
            (KeyCode::Down | KeyCode::Char('j'), KeyModifiers::NONE)
            | (KeyCode::Char('n') | KeyCode::Char('j'), KeyModifiers::CONTROL) => {
                self.move_selection(true)
            }
            (KeyCode::PageDown, KeyModifiers::NONE)
            | (KeyCode::Char('f'), KeyModifiers::CONTROL) => self.page_selection(true),
            (KeyCode::PageUp, KeyModifiers::NONE) | (KeyCode::Char('b'), KeyModifiers::CONTROL) => {
                self.page_selection(false)
            }
            (KeyCode::Home, KeyModifiers::NONE) => {
                self.selected = 0;
                self.scroll.set(0);
            }
            (KeyCode::End, KeyModifiers::NONE) => {
                self.selected = self.item_count().saturating_sub(1)
            }
            _ => {}
        }
    }

    fn activate(&mut self) -> AgentsOverviewAction {
        if self.selected_is_load_more() {
            return if self.loading_more {
                AgentsOverviewAction::None
            } else {
                AgentsOverviewAction::LoadMore
            };
        }
        if self.selected_row().is_none() {
            return AgentsOverviewAction::None;
        }
        self.searching = false;
        // Keep the selected row identity available to the App action reducer until it opens.
        AgentsOverviewAction::Select
    }
}
