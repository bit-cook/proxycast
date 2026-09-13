//! Footer rendering owned by the bottom-pane interaction surface.
//!
//! The footer consumes canonical composer and projection state supplied by the app. It does not
//! own session state or infer runtime status, keeping the same boundary as Codex's footer owner.

use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;
use crate::line_truncation::truncate_line_with_ellipsis_if_overflow;
use crate::width::usable_content_width_u16;

pub(crate) fn render_footer(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let vim_indicator = app.composer.vim_mode_indicator_span();
    if let Some(line) = app.composer.footer_flash() {
        let mut spans = line.spans.clone();
        append_vim_indicator(&mut spans, vim_indicator.clone());
        frame.render_widget(Paragraph::new(Line::from(spans)), area);
        return;
    }
    if let Some(query) = app.composer.history_search_query() {
        let width = usable_content_width_u16(area.width, 1).unwrap_or_default();
        let mut spans = vec![Span::styled(
            format!(" {}{}", app.locale.history_search_label(), query),
            Style::default().fg(Color::DarkGray),
        )];
        append_vim_indicator(&mut spans, vim_indicator.clone());
        let line = truncate_line_with_ellipsis_if_overflow(Line::from(spans), width);
        frame.render_widget(Paragraph::new(line), area);
        if let Some((x, y)) = app
            .composer
            .history_search_cursor_position(area, app.locale.history_search_label())
        {
            frame.set_cursor_position(Position::new(x, y));
        }
        return;
    }
    if let Some((query, direction)) = app.composer.vim_search_query() {
        let prefix = match direction {
            crate::vim_search::SearchDirection::Forward => "/",
            crate::vim_search::SearchDirection::Backward => "?",
        };
        let width = usable_content_width_u16(area.width, 1).unwrap_or_default();
        let mut spans = vec![Span::styled(
            format!(" {prefix}{query}"),
            Style::default().fg(Color::Cyan),
        )];
        append_vim_indicator(&mut spans, vim_indicator.clone());
        let line = truncate_line_with_ellipsis_if_overflow(Line::from(spans), width);
        frame.render_widget(Paragraph::new(line), area);
        return;
    }
    if app.composer.footer_has_draft() {
        let width = usable_content_width_u16(area.width, 1).unwrap_or_default();
        let mut spans = vec![Span::styled(
            " draft ready",
            Style::default().fg(Color::DarkGray),
        )];
        append_vim_indicator(&mut spans, vim_indicator.clone());
        let line = truncate_line_with_ellipsis_if_overflow(Line::from(spans), width);
        frame.render_widget(Paragraph::new(line), area);
        return;
    }
    let active = app
        .projection
        .active_turn_id()
        .map(|turn| format!(" {} {}", app.locale.turn_label(), turn))
        .unwrap_or_default();
    let active_agent = app
        .agent_navigation
        .active_agent_label(app.thread_id.as_deref(), app.primary_thread_id.as_deref())
        .map(|label| format!("  {label}"))
        .unwrap_or_default();
    let width = usable_content_width_u16(area.width, 1).unwrap_or_default();
    let mut spans = vec![Span::styled(
        format!(" {active}{active_agent}"),
        Style::default().fg(Color::DarkGray),
    )];
    append_vim_indicator(&mut spans, vim_indicator);
    let line = truncate_line_with_ellipsis_if_overflow(Line::from(spans), width);
    frame.render_widget(Paragraph::new(line), area);
}

fn append_vim_indicator(spans: &mut Vec<Span<'static>>, indicator: Option<Span<'static>>) {
    if let Some(indicator) = indicator {
        spans.push(Span::raw("  "));
        spans.push(indicator);
    }
}

#[cfg(test)]
mod tests {
    use super::render_footer;
    use crate::app::App;
    use crate::locale::Locale;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn rendered_text_at_width(app: &App, width: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(width, 1)).expect("terminal");
        terminal
            .draw(|frame| render_footer(frame, frame.area(), app))
            .expect("draw footer");
        let buffer = terminal.backend().buffer();
        (0..buffer.area.width)
            .map(|x| buffer[(x, 0)].symbol())
            .collect::<String>()
    }

    fn rendered_text(app: &App) -> String {
        rendered_text_at_width(app, 64)
    }

    #[test]
    fn renders_composer_draft_state() {
        let mut app = App::default();
        app.composer.insert("draft");

        assert!(rendered_text(&app).contains("draft ready"));
    }

    #[test]
    fn renders_localized_history_search_query() {
        let mut app = App::default();
        app.set_locale(Locale::ZhCn);
        app.composer.load_history(["git status".to_string()]);
        app.composer.insert("git");
        app.composer
            .handle_key_event(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL));
        app.composer
            .handle_key_event(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::NONE));

        let text = rendered_text(&app);
        let compact = text
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>();
        assert!(compact.contains("反向搜索：g"), "{text}");
    }

    #[test]
    fn renders_active_agent_context() {
        let mut app = App::default();
        app.set_thread_id("main".to_string());
        app.agent_navigation.upsert(
            "agent-1",
            Some("Robie".to_string()),
            Some("explorer".to_string()),
            false,
        );
        app.set_thread_id("agent-1".to_string());

        let text = rendered_text(&app);
        assert!(text.contains("Robie [explorer]"), "{text}");
    }

    #[test]
    fn renders_vim_mode_indicator_and_truncates_it_in_a_narrow_terminal() {
        let mut app = App::default();
        app.composer.set_vim_enabled(true);

        assert!(rendered_text(&app).contains("Vim: Normal"));

        let narrow = rendered_text_at_width(&app, 10);
        assert_eq!(narrow.chars().count(), 10, "{narrow}");
        assert!(narrow.contains('…'), "{narrow}");
    }

    #[test]
    fn renders_vim_search_query_before_submission() {
        let mut app = App::default();
        app.composer.set_vim_enabled(true);
        app.composer.insert("alpha beta");
        app.composer
            .handle_key_event(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
        app.composer
            .handle_key_event(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE));

        let text = rendered_text(&app);
        assert!(text.contains("/b"), "{text}");
    }
}
