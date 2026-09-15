//! Ratatui rendering for the Codex-shaped Agents Overview.

use super::{AgentsOverviewGroup, AgentsOverviewInputMode, AgentsOverviewView};
use crate::bottom_pane::selection_row_layout::{
    wrap_row, SelectionDescriptionLayout, SelectionRow,
};
use crate::locale::Locale;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

pub(crate) fn render(frame: &mut Frame<'_>, area: Rect, view: &AgentsOverviewView, locale: Locale) {
    let width = area
        .width
        .saturating_mul(4)
        .saturating_div(5)
        .clamp(48, 100);
    let height = area
        .height
        .saturating_mul(4)
        .saturating_div(5)
        .clamp(10, 26);
    let popup = Rect::new(
        area.x.saturating_add(area.width.saturating_sub(width) / 2),
        area.y
            .saturating_add(area.height.saturating_sub(height) / 2),
        width.min(area.width),
        height.min(area.height),
    );
    if popup.width < 20 || popup.height < 8 {
        return;
    }
    frame.render_widget(Clear, popup);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Min(4),
            Constraint::Length(1),
        ])
        .split(popup);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!(" {}", locale.agents_overview_title()),
            Style::default().add_modifier(Modifier::BOLD),
        )))
        .block(Block::default().borders(Borders::TOP)),
        chunks[0],
    );
    let mut counts = [0usize; 4];
    for row in view.visible_rows() {
        counts[row.group as usize] += 1;
    }
    let summary_text = format!(
        "{} {}   {} {}   {} {}   {} {}",
        counts[0],
        locale.agents_overview_group_label("need input"),
        counts[1],
        locale.agents_overview_group_label("working"),
        counts[2],
        locale.agents_overview_group_label("ready"),
        counts[3],
        locale.agents_overview_group_label("finished")
    );
    frame.render_widget(
        Paragraph::new(Span::styled(
            summary_text,
            Style::default().fg(Color::DarkGray),
        )),
        chunks[1],
    );

    let body = chunks[2];
    let (list_area, details_area) = if body.width >= 90 {
        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Min(44),
                Constraint::Length(1),
                Constraint::Length(34),
            ])
            .split(body);
        (layout[0], Some(layout[2]))
    } else {
        (body, None)
    };
    let items = view
        .visible_rows()
        .into_iter()
        .map(|row| {
            let marker = match row.group {
                AgentsOverviewGroup::NeedsYou => "●",
                AgentsOverviewGroup::Working => "●",
                AgentsOverviewGroup::Ready => "○",
                AgentsOverviewGroup::Finished => "✓",
            };
            let title = row
                .thread
                .name
                .as_deref()
                .or_else(|| (!row.thread.preview.is_empty()).then_some(row.thread.preview.as_str()))
                .unwrap_or("Untitled task");
            let current = row.is_current.then_some("current");
            let description = if view.status_grouping() {
                current.map(str::to_owned)
            } else {
                let project = format!(
                    "{} · {}",
                    locale.agents_overview_group_label(row.group.label()),
                    row.thread.cwd.display()
                );
                Some(match current {
                    Some(current) => format!("{current}  {project}"),
                    None => project,
                })
            };
            let row = SelectionRow::new(
                title,
                description,
                vec![Span::styled(
                    format!("{marker} "),
                    Style::default().fg(Color::Cyan),
                )],
            );
            ListItem::new(wrap_row(
                &row,
                usize::from(list_area.width.saturating_mul(3).saturating_div(5).max(1)),
                list_area.width.saturating_sub(4),
                SelectionDescriptionLayout::StackBelowWhenNarrow {
                    min_description_width: 16,
                },
            ))
        })
        .collect::<Vec<_>>();
    let mut state = ListState::default();
    if let Some(selected) = view.selected_index() {
        state.select(Some(selected.min(items.len().saturating_sub(1))));
    }
    frame.render_stateful_widget(
        List::new(items)
            .block(Block::default().borders(Borders::LEFT | Borders::RIGHT))
            .highlight_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("› "),
        list_area,
        &mut state,
    );
    if let Some(details_area) = details_area {
        render_details(frame, details_area, view, locale);
    }
    let footer_text = if let Some(mode) = view.input_mode() {
        format!(
            "{}: {}  · enter confirm · esc cancel",
            locale.agents_overview_input_prefix(matches!(mode, AgentsOverviewInputMode::Rename)),
            view.input()
        )
    } else if view.is_searching() {
        format!(
            "{}: {}  · enter open · esc back",
            locale.agents_overview_search_prefix(),
            view.search()
        )
    } else {
        locale.agents_overview_footer().to_string()
    };
    frame.render_widget(
        Paragraph::new(Span::styled(
            footer_text,
            Style::default().fg(Color::DarkGray),
        )),
        chunks[3],
    );
}

fn render_details(frame: &mut Frame<'_>, area: Rect, view: &AgentsOverviewView, locale: Locale) {
    let Some(row) = view.selected_row() else {
        return;
    };
    let title = row
        .thread
        .name
        .as_deref()
        .filter(|name| !name.trim().is_empty())
        .or_else(|| (!row.thread.preview.is_empty()).then_some(row.thread.preview.as_str()))
        .unwrap_or("Untitled task");
    let status = locale.agents_overview_group_label(row.group.label());
    let mut lines = vec![
        Line::from(Span::styled(
            "Task details",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::default(),
        Line::from(Span::styled(
            title,
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled(status, Style::default().fg(Color::Cyan)),
            Span::styled(
                if row.is_current { "  current" } else { "" },
                Style::default().fg(Color::DarkGray),
            ),
        ]),
        Line::default(),
        Line::from(Span::styled(
            "Project",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(row.thread.cwd.display().to_string()),
        Line::default(),
        Line::from(Span::styled(
            "Latest activity",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(if row.thread.preview.is_empty() {
            "No activity yet.".to_string()
        } else {
            row.thread.preview.clone()
        }),
    ];
    if let Some(branch) = row
        .thread
        .git_info
        .as_ref()
        .and_then(|git| git.branch.as_ref())
    {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            "Branch",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(branch.clone()));
    }
    frame.render_widget(
        Paragraph::new(lines).wrap(ratatui::widgets::Wrap { trim: true }),
        area,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::agents_overview_view::AgentsOverviewRow;
    use app_server_protocol::protocol::v2::{SessionSource, ThreadHistoryMode, ThreadStatus};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use std::path::PathBuf;

    fn row(id: &str) -> AgentsOverviewRow {
        AgentsOverviewRow {
            thread: app_server_protocol::protocol::v2::Thread {
                id: id.to_string(),
                extra: None,
                session_id: id.to_string(),
                forked_from_id: None,
                parent_thread_id: None,
                preview: "preview".to_string(),
                ephemeral: false,
                section: None,
                section_entered_at: None,
                project_id: None,
                history_mode: ThreadHistoryMode::Legacy,
                model_provider: "test".to_string(),
                created_at: 1,
                updated_at: 1,
                recency_at: Some(1),
                status: ThreadStatus::Idle,
                path: None,
                cwd: PathBuf::from("/workspace"),
                cli_version: "test".to_string(),
                source: SessionSource::Cli,
                can_accept_direct_input: Some(true),
                thread_source: None,
                agent_nickname: None,
                agent_role: None,
                git_info: None,
                name: Some(id.to_string()),
                turns: Vec::new(),
            },
            group: AgentsOverviewGroup::Ready,
            is_current: false,
        }
    }

    #[test]
    fn overview_render_is_bounded_on_wide_and_narrow_terminals() {
        let view = AgentsOverviewView::new(vec![row("alpha"), row("beta")], None);
        for (width, height) in [(120, 30), (24, 10), (16, 8)] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("terminal");
            terminal
                .draw(|frame| render(frame, frame.area(), &view, Locale::EnUs))
                .expect("draw");
        }
    }

    #[test]
    fn overview_render_shows_localized_new_task_input_without_overflow() {
        let mut view = AgentsOverviewView::new(vec![row("alpha")], None);
        view.handle_event(crossterm::event::Event::Key(
            crossterm::event::KeyEvent::new(
                crossterm::event::KeyCode::Char('n'),
                crossterm::event::KeyModifiers::CONTROL,
            ),
        ));
        for character in "检查任务".chars() {
            view.handle_event(crossterm::event::Event::Key(
                crossterm::event::KeyEvent::new(
                    crossterm::event::KeyCode::Char(character),
                    crossterm::event::KeyModifiers::NONE,
                ),
            ));
        }
        let mut terminal = Terminal::new(TestBackend::new(24, 10)).expect("terminal");
        terminal
            .draw(|frame| render(frame, frame.area(), &view, Locale::ZhCn))
            .expect("draw");
        let text = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();
        let compact = text.chars().filter(|character| !character.is_whitespace());
        let compact = compact.collect::<String>();
        assert!(compact.contains("新建任务"), "{text}");
        assert!(compact.contains("检查任务"), "{text}");
    }

    #[test]
    fn overview_render_uses_selection_row_for_status_and_current_context() {
        let mut current = row("current");
        current.group = AgentsOverviewGroup::Ready;
        current.is_current = true;
        current.thread.cwd = PathBuf::from("/workspace/project");
        let view = AgentsOverviewView::new(vec![current], Some("current"));
        let mut terminal = Terminal::new(TestBackend::new(80, 16)).expect("terminal");
        terminal
            .draw(|frame| render(frame, frame.area(), &view, Locale::EnUs))
            .expect("draw");
        let text = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect::<String>();
        assert!(text.contains("○"), "selection row marker missing: {text}");
        assert!(text.contains("current"), "current context missing: {text}");
        assert!(text.contains("Ready"), "status description missing: {text}");
        let compact = text
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>();
        assert!(compact.contains("/"), "cwd prefix missing: {text}");
        assert!(
            compact.contains("workspace/project"),
            "cwd description missing: {text}"
        );
    }
}
