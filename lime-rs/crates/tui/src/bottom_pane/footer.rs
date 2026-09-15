//! Footer rendering owned by the bottom-pane interaction surface.
//!
//! The footer consumes canonical composer and projection state supplied by the app. It does not
//! own session state or infer runtime status, keeping the same boundary as Codex's footer owner.

use ratatui::layout::{Position, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;
use crate::line_truncation::{line_width, truncate_line_with_ellipsis_if_overflow};
use crate::style::{accent_style, footer_hint_label_style};
use crate::width::usable_content_width_u16;

const FOOTER_INDENT_COLS: u16 = 1;
const FOOTER_CONTEXT_GAP_COLS: u16 = 1;

pub(crate) fn render_footer(frame: &mut Frame<'_>, area: Rect, app: &App) {
    if app.bottom_pane.is_active() {
        let hint = app
            .bottom_pane
            .footer_hint(app.locale, usize::from(area.width.saturating_sub(1)));
        if let Some(hint) = hint {
            let width =
                usable_content_width_u16(area.width, FOOTER_INDENT_COLS).unwrap_or_default();
            let line = Line::from(Span::styled(format!(" {hint}"), footer_hint_label_style()));
            frame.render_widget(
                Paragraph::new(truncate_line_with_ellipsis_if_overflow(line, width)),
                area,
            );
            return;
        }
    }
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
            footer_hint_label_style(),
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
        let mut spans = vec![Span::styled(format!(" {prefix}{query}"), accent_style())];
        append_vim_indicator(&mut spans, vim_indicator.clone());
        let line = truncate_line_with_ellipsis_if_overflow(Line::from(spans), width);
        frame.render_widget(Paragraph::new(line), area);
        return;
    }
    if app.composer.footer_has_draft() {
        let active_turn = app.projection.active_turn_id().is_some();
        let hint_kind = if active_turn {
            SummaryHintKind::QueueMessage
        } else {
            SummaryHintKind::DraftReady
        };
        let default_text = if active_turn {
            app.locale.queue_message_hint()
        } else {
            app.locale.draft_ready_hint()
        };
        render_summary_footer(
            frame,
            area,
            hint_kind,
            default_text,
            active_agent_context(app),
            vim_indicator,
            app.locale.queue_short_hint(),
        );
        return;
    }
    // Keep an actionable footer visible while idle. Codex reserves this row for the shortcut
    // entry point instead of leaving a blank line; an active turn still gets its canonical id.
    let active = app
        .projection
        .active_turn_id()
        .map(|turn| format!(" {} {}", app.locale.turn_label(), turn))
        .unwrap_or_else(|| format!(" {}", app.locale.shortcuts_hint()));
    render_summary_footer(
        frame,
        area,
        SummaryHintKind::None,
        &active,
        active_agent_context(app),
        vim_indicator,
        app.locale.queue_short_hint(),
    );
}

fn active_agent_context(app: &App) -> Option<Line<'static>> {
    app.agent_navigation
        .active_agent_label(app.thread_id.as_deref(), app.primary_thread_id.as_deref())
        .map(|label| Line::from(Span::styled(label, footer_hint_label_style())))
}

fn render_summary_footer(
    frame: &mut Frame<'_>,
    area: Rect,
    hint_kind: SummaryHintKind,
    default_text: &str,
    context: Option<Line<'static>>,
    vim_indicator: Option<Span<'static>>,
    queue_short_text: &str,
) {
    let mut default_spans = vec![Span::styled(
        format!(" {default_text}"),
        footer_hint_label_style(),
    )];
    append_vim_indicator(&mut default_spans, vim_indicator.clone());
    let default_line = Line::from(default_spans);

    let queue_short_line = (hint_kind == SummaryHintKind::QueueMessage).then(|| {
        let mut spans = vec![Span::styled(
            format!(" {queue_short_text}"),
            footer_hint_label_style(),
        )];
        append_vim_indicator(&mut spans, vim_indicator);
        Line::from(spans)
    });
    let context_width = context.as_ref().map(line_width).unwrap_or(0) as u16;
    let (summary, show_context) = single_line_footer_layout(
        area,
        hint_kind,
        default_line.clone(),
        queue_short_line,
        context_width,
    );

    let left = match summary {
        SummaryLeft::Default => default_line,
        SummaryLeft::Custom(line) => line,
        SummaryLeft::None => Line::from(Vec::<Span<'static>>::new()),
    };
    let width = usable_content_width_u16(area.width, FOOTER_INDENT_COLS).unwrap_or_default();
    frame.render_widget(
        Paragraph::new(truncate_line_with_ellipsis_if_overflow(left, width)),
        area,
    );
    if show_context {
        if let Some(context) = context {
            render_context_right(area, frame.buffer_mut(), &context);
        }
    }
}

fn append_vim_indicator(spans: &mut Vec<Span<'static>>, indicator: Option<Span<'static>>) {
    if let Some(indicator) = indicator {
        spans.push(Span::raw("  "));
        spans.push(indicator);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SummaryHintKind {
    None,
    DraftReady,
    QueueMessage,
}

#[derive(Debug)]
pub(crate) enum SummaryLeft {
    Default,
    Custom(Line<'static>),
    None,
}

/// Choose the most useful single-line footer variant for the available width.
///
/// Queue hints are deliberately kept ahead of ambient context: when the full queue hint cannot
/// share a row with the agent label, the context is hidden first and the hint is shortened last.
pub(crate) fn single_line_footer_layout(
    area: Rect,
    hint_kind: SummaryHintKind,
    default_line: Line<'static>,
    queue_short_line: Option<Line<'static>>,
    context_width: u16,
) -> (SummaryLeft, bool) {
    let default_width = line_width(&default_line) as u16;
    if default_width > 0 && can_show_left_with_context(area, default_width, context_width) {
        return (SummaryLeft::Default, true);
    }

    if hint_kind == SummaryHintKind::QueueMessage {
        if let Some(short) = queue_short_line.as_ref() {
            let short_width = line_width(short) as u16;
            if short_width > 0 && can_show_left_with_context(area, short_width, context_width) {
                return (SummaryLeft::Custom(short.clone()), true);
            }
        }
        // Queueing is actionable, so hide passive context before shortening or dropping it.
        if default_width > 0 && left_fits(area, default_width) {
            return (SummaryLeft::Default, false);
        }
        if let Some(short) = queue_short_line {
            if left_fits(area, line_width(&short) as u16) {
                return (SummaryLeft::Custom(short), false);
            }
        }
    } else if default_width > 0 && left_fits(area, default_width) {
        return (SummaryLeft::Default, false);
    }

    // If no left content fits, retain a right context only when it can be rendered on its own.
    (
        SummaryLeft::None,
        context_width > 0 && right_aligned_x(area, context_width).is_some(),
    )
}

fn left_fits(area: Rect, left_width: u16) -> bool {
    left_width <= area.width.saturating_sub(FOOTER_INDENT_COLS)
}

fn right_aligned_x(area: Rect, content_width: u16) -> Option<u16> {
    if area.is_empty() || content_width == 0 {
        return None;
    }
    let max_width = area.width.saturating_sub(FOOTER_INDENT_COLS);
    if max_width == 0 || content_width > max_width {
        return None;
    }
    Some(
        area.x
            .saturating_add(area.width)
            .saturating_sub(content_width)
            .saturating_sub(FOOTER_INDENT_COLS),
    )
}

pub(crate) fn can_show_left_with_context(area: Rect, left_width: u16, context_width: u16) -> bool {
    let available = area.width.saturating_sub(FOOTER_INDENT_COLS);
    if context_width > available {
        return false;
    }
    let Some(context_x) = right_aligned_x(area, context_width) else {
        return true;
    };
    if left_width == 0 {
        return true;
    }
    let left_extent = area
        .x
        .saturating_add(FOOTER_INDENT_COLS)
        .saturating_add(left_width)
        .saturating_add(FOOTER_CONTEXT_GAP_COLS);
    left_extent <= context_x
}

pub(crate) fn render_context_right(
    area: Rect,
    buf: &mut ratatui::buffer::Buffer,
    line: &Line<'static>,
) {
    let Some(mut x) = right_aligned_x(area, line_width(line) as u16) else {
        return;
    };
    let y = area.y + area.height.saturating_sub(1);
    let max_x = area.x.saturating_add(area.width);
    for span in &line.spans {
        if x >= max_x {
            break;
        }
        let span_width = crate::width::display_width(span.content.as_ref()) as u16;
        if span_width == 0 {
            continue;
        }
        let draw_width = span_width.min(max_x.saturating_sub(x));
        buf.set_span(x, y, span, draw_width);
        x = x.saturating_add(span_width);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        line_width, render_footer, single_line_footer_layout, SummaryHintKind, SummaryLeft,
    };
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
    fn idle_footer_keeps_localized_shortcut_entry_point_across_supported_widths() {
        for locale in [
            Locale::ZhCn,
            Locale::ZhTw,
            Locale::EnUs,
            Locale::JaJp,
            Locale::KoKr,
        ] {
            let mut app = App::default();
            app.set_locale(locale);
            for width in [40, 80, 120] {
                let text = rendered_text_at_width(&app, width);
                let compact = text
                    .chars()
                    .filter(|character| !character.is_whitespace())
                    .collect::<String>();
                let expected = locale
                    .shortcuts_hint()
                    .chars()
                    .filter(|character| !character.is_whitespace())
                    .collect::<String>();
                assert!(
                    compact.contains(&expected),
                    "{locale:?} at {width}: {text:?}"
                );
                assert_eq!(
                    text.lines().count(),
                    1,
                    "footer must remain a single row for {locale:?} at {width}: {text:?}"
                );
            }
        }
    }

    #[test]
    fn idle_footer_drops_ambient_agent_context_as_one_unit_when_it_cannot_fit() {
        let mut app = App::default();
        app.set_thread_id("main".to_string());
        app.agent_navigation.upsert(
            "agent-1",
            Some("Robie".to_string()),
            Some("explorer".to_string()),
            false,
        );
        app.set_thread_id("agent-1".to_string());

        let text = rendered_text_at_width(&app, 20);
        assert!(text.contains("? for shortcuts"), "{text}");
        assert!(
            !text.contains("Robie") && !text.contains("explorer"),
            "agent context must be hidden as one unit: {text}"
        );
    }

    #[test]
    fn active_draft_prefers_queue_hint_and_hides_context_when_narrow() {
        let mut app = App::default();
        app.set_thread_id("main".to_string());
        app.agent_navigation.upsert(
            "agent-1",
            Some("Robie".to_string()),
            Some("explorer".to_string()),
            false,
        );
        app.set_thread_id("agent-1".to_string());
        app.start_turn("turn-1".to_string());
        app.composer.insert("draft");

        let text = rendered_text_at_width(&app, 30);
        assert!(text.contains("Tab to queue message"), "{text}");
        assert!(!text.contains("Robie [explorer]"), "{text}");
    }

    #[test]
    fn queue_hint_shortens_before_it_disappears() {
        let default = ratatui::text::Line::from(" Tab to queue message");
        let short = ratatui::text::Line::from(" Tab to queue");
        let (left, show_context) = single_line_footer_layout(
            ratatui::layout::Rect::new(0, 0, 14, 1),
            SummaryHintKind::QueueMessage,
            default,
            Some(short),
            14,
        );
        assert!(!show_context);
        match left {
            SummaryLeft::Custom(line) => assert_eq!(line_width(&line), 13),
            other => panic!("expected shortened queue hint, got {other:?}"),
        }
    }

    #[test]
    fn cjk_and_emoji_context_widths_do_not_overlap_left_hint() {
        let left = ratatui::text::Line::from(" Tab to queue");
        let context = ratatui::text::Line::from("界🙂");
        let (summary, show_context) = single_line_footer_layout(
            ratatui::layout::Rect::new(0, 0, 20, 1),
            SummaryHintKind::QueueMessage,
            left.clone(),
            None,
            line_width(&context) as u16,
        );
        assert!(matches!(summary, SummaryLeft::Default));
        assert!(show_context);
        assert!(line_width(&left) + line_width(&context) + 3 <= 20);
    }

    #[test]
    fn queue_and_draft_footer_hints_cover_all_product_locales() {
        for locale in [
            Locale::ZhCn,
            Locale::ZhTw,
            Locale::EnUs,
            Locale::JaJp,
            Locale::KoKr,
        ] {
            assert!(!locale.queue_message_hint().is_empty());
            assert!(!locale.queue_short_hint().is_empty());
            assert!(!locale.draft_ready_hint().is_empty());
        }
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
