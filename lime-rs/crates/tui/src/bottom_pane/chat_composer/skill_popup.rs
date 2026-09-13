//! Skill completion popup for `$` mentions.
//!
//! The popup is presentation-only. Skill metadata comes from the App Server `skills/list`
//! response and the composer remains responsible for replacing the active draft token.

use app_server_protocol::protocol::v2::SkillMetadata;
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;

use crate::line_truncation::truncate_line_with_ellipsis_if_overflow;
use crate::locale::Locale;

const MAX_ROWS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillPopupAction {
    Pass,
    Consumed,
    Cancel,
    Complete,
}

#[derive(Debug, Clone)]
pub(crate) struct SkillPopup {
    query: String,
    skills: Vec<SkillMetadata>,
    selected: usize,
}

impl SkillPopup {
    pub(crate) fn new(skills: Vec<SkillMetadata>, query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            skills,
            selected: 0,
        }
    }

    pub(crate) fn set_query(&mut self, query: impl Into<String>) {
        self.query = query.into();
        self.selected = self.selected.min(self.matches().len().saturating_sub(1));
    }

    pub(crate) fn set_skills(&mut self, skills: Vec<SkillMetadata>) {
        self.skills = skills;
        self.selected = self.selected.min(self.matches().len().saturating_sub(1));
    }

    pub(crate) fn selected_skill(&self) -> Option<&SkillMetadata> {
        self.matches()
            .get(self.selected)
            .and_then(|index| self.skills.get(*index))
    }

    pub(crate) fn handle_event(&mut self, event: &Event) -> SkillPopupAction {
        let Event::Key(key) = event else {
            return SkillPopupAction::Pass;
        };
        if key.kind != KeyEventKind::Press {
            return SkillPopupAction::Pass;
        }
        match key.code {
            KeyCode::Up | KeyCode::Char('p')
                if key.code == KeyCode::Up || key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                let len = self.matches().len();
                if len > 0 {
                    self.selected = self.selected.checked_sub(1).unwrap_or(len - 1);
                }
                SkillPopupAction::Consumed
            }
            KeyCode::Down | KeyCode::Char('n')
                if key.code == KeyCode::Down || key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                let len = self.matches().len();
                if len > 0 {
                    self.selected = (self.selected + 1) % len;
                }
                SkillPopupAction::Consumed
            }
            KeyCode::Esc => SkillPopupAction::Cancel,
            KeyCode::Enter | KeyCode::Tab if self.selected_skill().is_some() => {
                SkillPopupAction::Complete
            }
            KeyCode::Enter | KeyCode::Tab => SkillPopupAction::Consumed,
            _ => SkillPopupAction::Pass,
        }
    }

    pub(crate) fn render(&self, frame: &mut Frame<'_>, composer_area: Rect, locale: Locale) {
        if composer_area.y == 0 || composer_area.width == 0 {
            return;
        }
        let matches = self.matches();
        let height = u16::try_from(matches.len().clamp(1, MAX_ROWS))
            .unwrap_or(u16::MAX)
            .min(composer_area.y);
        let area = Rect::new(
            composer_area.x,
            composer_area.y.saturating_sub(height),
            composer_area.width,
            height,
        );
        let lines = if matches.is_empty() {
            vec![Line::styled(
                format!("  {}", locale.skill_popup_no_matches()),
                Style::default().add_modifier(Modifier::DIM),
            )]
        } else {
            matches
                .into_iter()
                .enumerate()
                .map(|(index, skill_index)| {
                    let skill = &self.skills[skill_index];
                    let selected = index == self.selected;
                    let style = if selected {
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    let name = skill_display_name(skill);
                    let description = skill_description(skill);
                    let mut spans = vec![
                        Span::styled(if selected { "> " } else { "  " }, style),
                        Span::styled(format!("${name}"), style),
                    ];
                    if !description.is_empty() {
                        spans.push(Span::raw("  "));
                        spans.push(Span::styled(
                            description,
                            Style::default().add_modifier(Modifier::DIM),
                        ));
                    }
                    truncate_line_with_ellipsis_if_overflow(
                        Line::from(spans),
                        usize::from(area.width),
                    )
                })
                .collect()
        };
        frame.render_widget(Clear, area);
        frame.render_widget(Paragraph::new(lines), area);
    }

    fn matches(&self) -> Vec<usize> {
        let query = self.query.trim().to_ascii_lowercase();
        let mut matches = self
            .skills
            .iter()
            .enumerate()
            .filter_map(|(index, skill)| {
                let display = skill_display_name(skill);
                let score = fuzzy_score(&display, &query)?;
                Some((index, score))
            })
            .collect::<Vec<_>>();
        matches.sort_by(|(left_index, left_score), (right_index, right_score)| {
            left_score.cmp(right_score).then_with(|| {
                skill_display_name(&self.skills[*left_index])
                    .cmp(&skill_display_name(&self.skills[*right_index]))
            })
        });
        matches.into_iter().map(|(index, _)| index).collect()
    }
}

fn skill_display_name(skill: &SkillMetadata) -> String {
    skill
        .interface
        .as_ref()
        .and_then(|interface| interface.display_name.as_deref())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or(&skill.name)
        .to_string()
}

fn skill_description(skill: &SkillMetadata) -> String {
    skill
        .interface
        .as_ref()
        .and_then(|interface| interface.short_description.as_deref())
        .or(skill.short_description.as_deref())
        .unwrap_or(&skill.description)
        .trim()
        .to_string()
}

/// Returns a small deterministic score for case-insensitive subsequence matches.
fn fuzzy_score(value: &str, query: &str) -> Option<usize> {
    if query.is_empty() {
        return Some(0);
    }
    let value = value.to_ascii_lowercase();
    if value.contains(query) {
        return Some(value.find(query).unwrap_or(0));
    }
    let mut cursor = 0;
    let mut first = None;
    let mut gaps = 0;
    for query_char in query.chars() {
        let relative = value[cursor..].find(query_char)?;
        let index = cursor + relative;
        first.get_or_insert(index);
        gaps += relative;
        cursor = index + query_char.len_utf8();
    }
    Some(first.unwrap_or(0) + gaps + value.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_server_protocol::protocol::v2::SkillScope;
    use crossterm::event::KeyEvent;

    fn skill(name: &str, description: &str) -> SkillMetadata {
        SkillMetadata {
            name: name.to_string(),
            description: description.to_string(),
            short_description: None,
            interface: None,
            dependencies: None,
            path: format!("/skills/{name}/SKILL.md").into(),
            scope: SkillScope::User,
            enabled: true,
        }
    }

    #[test]
    fn filters_and_ranks_skill_names_case_insensitively() {
        let mut popup = SkillPopup::new(
            vec![skill("deploy", "release"), skill("code-review", "review")],
            "cr",
        );
        assert_eq!(
            popup.selected_skill().map(|skill| skill.name.as_str()),
            Some("code-review")
        );
        popup.set_query("DEP");
        assert_eq!(
            popup.selected_skill().map(|skill| skill.name.as_str()),
            Some("deploy")
        );
    }

    #[test]
    fn selection_wraps_and_enter_completes() {
        let mut popup = SkillPopup::new(vec![skill("one", ""), skill("two", "")], "");
        popup.handle_event(&Event::Key(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)));
        assert_eq!(
            popup.selected_skill().map(|skill| skill.name.as_str()),
            Some("two")
        );
        assert_eq!(
            popup.handle_event(&Event::Key(KeyEvent::new(
                KeyCode::Enter,
                KeyModifiers::NONE
            ))),
            SkillPopupAction::Complete
        );
    }

    #[test]
    fn control_p_and_control_n_cycle_selection() {
        let mut popup = SkillPopup::new(vec![skill("one", ""), skill("two", "")], "");
        popup.handle_event(&Event::Key(KeyEvent::new(
            KeyCode::Char('n'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(
            popup.selected_skill().map(|skill| skill.name.as_str()),
            Some("two")
        );
        popup.handle_event(&Event::Key(KeyEvent::new(
            KeyCode::Char('p'),
            KeyModifiers::CONTROL,
        )));
        assert_eq!(
            popup.selected_skill().map(|skill| skill.name.as_str()),
            Some("one")
        );
    }
}
