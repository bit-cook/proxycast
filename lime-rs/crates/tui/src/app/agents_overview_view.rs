//! Codex-shaped Agents Overview interaction state.
//!
//! The view owns only terminal selection and filtering. Thread metadata and lifecycle remain
//! owned by App Server and are refreshed through `agents_overview_threads`.

#[path = "agents_overview_render.rs"]
mod render;
pub(crate) use render::render;

use app_server_protocol::protocol::v2::{Thread, ThreadActiveFlag, ThreadStatus};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use std::path::PathBuf;

use crate::key_hint::is_plain_text_key_event;
use crate::keymap::AgentsKeymap;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum AgentsOverviewGroup {
    NeedsYou,
    Working,
    Ready,
    Finished,
}

impl AgentsOverviewGroup {
    pub(crate) fn for_status(status: &ThreadStatus) -> Self {
        match status {
            ThreadStatus::Active { active_flags }
                if active_flags.contains(&ThreadActiveFlag::WaitingOnApproval)
                    || active_flags.contains(&ThreadActiveFlag::WaitingOnUserInput) =>
            {
                Self::NeedsYou
            }
            ThreadStatus::Active { .. } => Self::Working,
            ThreadStatus::Idle => Self::Ready,
            ThreadStatus::SystemError => Self::NeedsYou,
            ThreadStatus::NotLoaded => Self::Finished,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::NeedsYou => "Needs input",
            Self::Working => "Working",
            Self::Ready => "Ready",
            Self::Finished => "Finished",
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct AgentsOverviewRow {
    pub(crate) thread: Thread,
    pub(crate) group: AgentsOverviewGroup,
    pub(crate) is_current: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AgentsOverviewInputMode {
    NewTask,
    Rename,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum AgentsOverviewAction {
    None,
    Cancel,
    Refresh,
    Select,
    Dispatch {
        prompt: String,
        cwd: Option<PathBuf>,
    },
    Rename {
        thread_id: String,
        name: String,
    },
    Stop {
        thread_id: String,
    },
    OpenResumePicker,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct AgentsOverviewView {
    pub(crate) rows: Vec<AgentsOverviewRow>,
    selected: usize,
    search: String,
    searching: bool,
    status_grouping: bool,
    input: String,
    input_mode: Option<AgentsOverviewInputMode>,
    agents_keymap: AgentsKeymap,
}

impl AgentsOverviewView {
    pub(crate) fn new(rows: Vec<AgentsOverviewRow>, selected_thread_id: Option<&str>) -> Self {
        let mut view = Self {
            rows,
            selected: 0,
            search: String::new(),
            searching: false,
            status_grouping: false,
            input: String::new(),
            input_mode: None,
            agents_keymap: AgentsKeymap,
        };
        let visible = view.visible_rows();
        view.selected = selected_thread_id
            .and_then(|thread_id| visible.iter().position(|row| row.thread.id == thread_id))
            .or_else(|| visible.iter().position(|row| row.is_current))
            .unwrap_or(0);
        view
    }

    pub(crate) fn selected_thread_id(&self) -> Option<&str> {
        self.visible_rows()
            .get(self.selected)
            .map(|row| row.thread.id.as_str())
    }

    pub(crate) fn selected_index(&self) -> Option<usize> {
        (!self.visible_rows().is_empty()).then_some(self.selected)
    }

    pub(crate) fn is_searching(&self) -> bool {
        self.searching
    }

    pub(crate) fn search(&self) -> &str {
        &self.search
    }

    pub(crate) fn input(&self) -> &str {
        &self.input
    }

    pub(crate) fn input_mode(&self) -> Option<AgentsOverviewInputMode> {
        self.input_mode
    }

    #[allow(dead_code)]
    pub(crate) fn status_grouping(&self) -> bool {
        self.status_grouping
    }

    pub(crate) fn update_rows(&mut self, rows: Vec<AgentsOverviewRow>) {
        let selected = self.selected_thread_id().map(str::to_owned);
        let search = self.search.clone();
        let searching = self.searching;
        let status_grouping = self.status_grouping;
        let input = self.input.clone();
        let input_mode = self.input_mode;
        *self = Self::new(rows, selected.as_deref());
        self.search = search;
        self.searching = searching;
        self.status_grouping = status_grouping;
        self.input = input;
        self.input_mode = input_mode;
        self.selected = selected
            .as_deref()
            .and_then(|thread_id| {
                self.visible_rows()
                    .iter()
                    .position(|row| row.thread.id == thread_id)
            })
            .unwrap_or_else(|| {
                self.selected
                    .min(self.visible_rows().len().saturating_sub(1))
            });
    }

    pub(crate) fn visible_rows(&self) -> Vec<&AgentsOverviewRow> {
        let query = self.search.trim().to_ascii_lowercase();
        let mut rows = self
            .rows
            .iter()
            .filter(|row| {
                query.is_empty()
                    || row
                        .thread
                        .name
                        .as_deref()
                        .unwrap_or_default()
                        .to_ascii_lowercase()
                        .contains(&query)
                    || row.thread.preview.to_ascii_lowercase().contains(&query)
                    || row
                        .thread
                        .cwd
                        .to_string_lossy()
                        .to_ascii_lowercase()
                        .contains(&query)
                    || row.thread.id.to_ascii_lowercase().contains(&query)
            })
            .collect::<Vec<_>>();
        if self.status_grouping {
            rows.sort_by(|left, right| {
                left.group
                    .cmp(&right.group)
                    .then_with(|| right.thread.updated_at.cmp(&left.thread.updated_at))
                    .then_with(|| left.thread.id.cmp(&right.thread.id))
            });
        } else {
            rows.sort_by(|left, right| {
                left.thread
                    .cwd
                    .cmp(&right.thread.cwd)
                    .then_with(|| right.thread.updated_at.cmp(&left.thread.updated_at))
            });
        }
        rows
    }

    pub(crate) fn handle_event(&mut self, event: Event) -> AgentsOverviewAction {
        if let Event::Paste(text) = event {
            if let Some(mode) = self.input_mode {
                match mode {
                    AgentsOverviewInputMode::NewTask | AgentsOverviewInputMode::Rename => {
                        self.input.push_str(&text);
                    }
                }
            } else if self.searching {
                if let Some(query) = crate::clipboard_paste::normalize_pasted_search_query(&text) {
                    self.search.push_str(&query);
                }
                self.selected = self
                    .selected
                    .min(self.visible_rows().len().saturating_sub(1));
            }
            return AgentsOverviewAction::None;
        }
        let Event::Key(key) = event else {
            return AgentsOverviewAction::None;
        };
        if key.kind != KeyEventKind::Press {
            return AgentsOverviewAction::None;
        }
        if key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('d'))
        {
            return AgentsOverviewAction::Cancel;
        }
        if let Some(mode) = self.input_mode {
            match key.code {
                KeyCode::Esc => {
                    self.input.clear();
                    self.input_mode = None;
                }
                KeyCode::Backspace if key.modifiers.is_empty() => {
                    self.input.pop();
                }
                KeyCode::Enter => {
                    let input = std::mem::take(&mut self.input);
                    self.input_mode = None;
                    if input.trim().is_empty() {
                        return AgentsOverviewAction::None;
                    }
                    match mode {
                        AgentsOverviewInputMode::NewTask => {
                            let cwd = (!self.status_grouping)
                                .then(|| self.selected_row().map(|row| row.thread.cwd.clone()))
                                .flatten();
                            return AgentsOverviewAction::Dispatch { prompt: input, cwd };
                        }
                        AgentsOverviewInputMode::Rename => {
                            if let Some(thread_id) =
                                self.selected_row().map(|row| row.thread.id.clone())
                            {
                                return AgentsOverviewAction::Rename {
                                    thread_id,
                                    name: input.trim().to_string(),
                                };
                            }
                        }
                    }
                }
                KeyCode::Char(character) if is_plain_text_key_event(key) => {
                    self.input.push(character);
                }
                _ => {}
            }
            return AgentsOverviewAction::None;
        }
        if key.code == KeyCode::Esc {
            if self.searching {
                self.searching = false;
                self.search.clear();
                self.selected = 0;
                return AgentsOverviewAction::None;
            }
            return AgentsOverviewAction::Cancel;
        }
        if (key.code == KeyCode::Char('/') && key.modifiers.is_empty())
            || self.agents_keymap.search(key)
        {
            self.searching = !self.searching;
            if !self.searching {
                self.search.clear();
            }
            self.selected = 0;
            return AgentsOverviewAction::None;
        }
        if self.searching {
            match key.code {
                KeyCode::Backspace => {
                    self.search.pop();
                }
                KeyCode::Char(character) if is_plain_text_key_event(key) => {
                    self.search.push(character);
                }
                KeyCode::Up | KeyCode::Char('k') => self.move_selection(false),
                KeyCode::Down | KeyCode::Char('j') => self.move_selection(true),
                KeyCode::Enter => {
                    self.searching = false;
                    return AgentsOverviewAction::Select;
                }
                _ => {}
            }
            self.selected = self
                .selected
                .min(self.visible_rows().len().saturating_sub(1));
            return AgentsOverviewAction::None;
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(false),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(true),
            KeyCode::Enter => return AgentsOverviewAction::Select,
            KeyCode::Char('o') if self.agents_keymap.resume(key) => {
                return AgentsOverviewAction::OpenResumePicker;
            }
            KeyCode::Char('n') if self.agents_keymap.new_task(key) => {
                self.input.clear();
                self.input_mode = Some(AgentsOverviewInputMode::NewTask);
                return AgentsOverviewAction::None;
            }
            KeyCode::Char('r') if self.agents_keymap.rename(key) => {
                if let Some(row) = self.selected_row() {
                    self.input = row.thread.name.clone().unwrap_or_default();
                    self.input_mode = Some(AgentsOverviewInputMode::Rename);
                }
                return AgentsOverviewAction::None;
            }
            KeyCode::Char('x') if self.agents_keymap.stop(key) => {
                if let Some(row) = self.selected_row() {
                    if matches!(row.thread.status, ThreadStatus::Active { .. }) {
                        return AgentsOverviewAction::Stop {
                            thread_id: row.thread.id.clone(),
                        };
                    }
                }
                return AgentsOverviewAction::None;
            }
            KeyCode::Char('s') if self.agents_keymap.toggle_grouping(key) => {
                self.status_grouping = !self.status_grouping;
                self.selected = self
                    .selected
                    .min(self.visible_rows().len().saturating_sub(1));
                return AgentsOverviewAction::None;
            }
            KeyCode::Char('r') if key.modifiers.is_empty() => {
                return AgentsOverviewAction::Refresh;
            }
            KeyCode::Char('g') if key.modifiers.is_empty() => {
                self.status_grouping = !self.status_grouping;
                self.selected = self
                    .selected
                    .min(self.visible_rows().len().saturating_sub(1));
            }
            _ => {}
        }
        AgentsOverviewAction::None
    }

    pub(crate) fn selected_row(&self) -> Option<&AgentsOverviewRow> {
        self.visible_rows().get(self.selected).copied()
    }

    fn move_selection(&mut self, forward: bool) {
        let count = self.visible_rows().len();
        if count == 0 {
            self.selected = 0;
            return;
        }
        self.selected = if forward {
            (self.selected + 1) % count
        } else {
            self.selected.checked_sub(1).unwrap_or(count - 1)
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_server_protocol::protocol::v2::ThreadHistoryMode;
    use crossterm::event::KeyEvent;
    use std::path::PathBuf;

    fn thread(id: &str, status: ThreadStatus, updated_at: i64) -> Thread {
        Thread {
            id: id.to_string(),
            extra: None,
            session_id: format!("session-{id}"),
            forked_from_id: None,
            parent_thread_id: None,
            preview: format!("preview-{id}"),
            ephemeral: false,
            section: None,
            section_entered_at: None,
            project_id: None,
            history_mode: ThreadHistoryMode::Legacy,
            model_provider: "test".to_string(),
            created_at: updated_at,
            updated_at,
            recency_at: Some(updated_at),
            status,
            path: None,
            cwd: PathBuf::from("/workspace"),
            cli_version: "test".to_string(),
            source: app_server_protocol::protocol::v2::SessionSource::Cli,
            can_accept_direct_input: Some(true),
            thread_source: None,
            agent_nickname: None,
            agent_role: None,
            git_info: None,
            name: Some(id.to_string()),
            turns: Vec::new(),
        }
    }

    fn row(id: &str, group: AgentsOverviewGroup, current: bool) -> AgentsOverviewRow {
        AgentsOverviewRow {
            thread: thread(id, ThreadStatus::Idle, 1),
            group,
            is_current: current,
        }
    }

    #[test]
    fn overview_groups_and_selects_current_thread() {
        let view = AgentsOverviewView::new(
            vec![
                row("ready", AgentsOverviewGroup::Ready, false),
                row("main", AgentsOverviewGroup::Working, true),
            ],
            None,
        );
        assert_eq!(view.selected_thread_id(), Some("main"));
        assert_eq!(
            AgentsOverviewGroup::for_status(&ThreadStatus::SystemError),
            AgentsOverviewGroup::NeedsYou
        );
    }

    #[test]
    fn overview_search_filters_and_enter_selects() {
        let mut view = AgentsOverviewView::new(
            vec![
                row("alpha", AgentsOverviewGroup::Ready, false),
                row("beta", AgentsOverviewGroup::Finished, false),
            ],
            None,
        );
        assert_eq!(
            view.handle_event(Event::Key(KeyEvent::new(
                KeyCode::Char('/'),
                KeyModifiers::NONE
            ))),
            AgentsOverviewAction::None
        );
        assert_eq!(
            view.handle_event(Event::Key(KeyEvent::new(
                KeyCode::Char('l'),
                KeyModifiers::NONE
            ))),
            AgentsOverviewAction::None
        );
        assert_eq!(view.visible_rows().len(), 1);
        assert_eq!(
            view.handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE
            ))),
            AgentsOverviewAction::Select
        );
        assert_eq!(view.selected_thread_id(), Some("alpha"));
    }

    #[test]
    fn overview_escape_and_refresh_are_explicit_actions() {
        let mut view = AgentsOverviewView::default();
        assert_eq!(
            view.handle_event(Event::Key(KeyEvent::new(
                KeyCode::Char('r'),
                KeyModifiers::NONE
            ))),
            AgentsOverviewAction::Refresh
        );
        assert_eq!(
            view.handle_event(Event::Key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))),
            AgentsOverviewAction::Cancel
        );
    }

    #[test]
    fn overview_codex_shortcuts_dispatch_rename_stop_and_resume() {
        let mut view = AgentsOverviewView::new(
            vec![row("ready", AgentsOverviewGroup::Ready, false), {
                let mut active = row("active", AgentsOverviewGroup::Working, true);
                active.thread.status = ThreadStatus::Active {
                    active_flags: Vec::new(),
                };
                active
            }],
            Some("active"),
        );
        assert_eq!(view.selected_thread_id(), Some("active"));

        assert_eq!(
            view.handle_event(Event::Key(KeyEvent::new(
                KeyCode::Char('n'),
                KeyModifiers::CONTROL,
            ))),
            AgentsOverviewAction::None
        );
        for character in "run checks".chars() {
            assert_eq!(
                view.handle_event(Event::Key(KeyEvent::new(
                    KeyCode::Char(character),
                    KeyModifiers::NONE,
                ))),
                AgentsOverviewAction::None
            );
        }
        assert_eq!(
            view.handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            ))),
            AgentsOverviewAction::Dispatch {
                prompt: "run checks".to_string(),
                cwd: Some(PathBuf::from("/workspace")),
            }
        );

        assert_eq!(
            view.handle_event(Event::Key(KeyEvent::new(
                KeyCode::Char('r'),
                KeyModifiers::CONTROL,
            ))),
            AgentsOverviewAction::None
        );
        assert_eq!(
            view.handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            ))),
            AgentsOverviewAction::Rename {
                thread_id: "active".to_string(),
                name: "active".to_string(),
            }
        );
        assert_eq!(
            view.handle_event(Event::Key(KeyEvent::new(
                KeyCode::Char('x'),
                KeyModifiers::CONTROL,
            ))),
            AgentsOverviewAction::Stop {
                thread_id: "active".to_string(),
            }
        );
        assert_eq!(
            view.handle_event(Event::Key(KeyEvent::new(
                KeyCode::Char('o'),
                KeyModifiers::CONTROL,
            ))),
            AgentsOverviewAction::OpenResumePicker
        );
    }

    #[test]
    fn overview_rename_accepts_shifted_characters() {
        let mut view = AgentsOverviewView::new(
            vec![row("active", AgentsOverviewGroup::Working, true)],
            Some("active"),
        );
        assert_eq!(
            view.handle_event(Event::Key(KeyEvent::new(
                KeyCode::Char('r'),
                KeyModifiers::CONTROL,
            ))),
            AgentsOverviewAction::None
        );
        for _ in 0.."active".len() {
            view.handle_event(Event::Key(KeyEvent::new(
                KeyCode::Backspace,
                KeyModifiers::NONE,
            )));
        }
        for (character, modifiers) in [
            ('G', KeyModifiers::SHIFT),
            ('a', KeyModifiers::NONE),
            ('t', KeyModifiers::NONE),
            ('e', KeyModifiers::NONE),
        ] {
            view.handle_event(Event::Key(KeyEvent::new(
                KeyCode::Char(character),
                modifiers,
            )));
        }
        assert_eq!(
            view.handle_event(Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE,
            ))),
            AgentsOverviewAction::Rename {
                thread_id: "active".to_string(),
                name: "Gate".to_string(),
            }
        );
    }
}
