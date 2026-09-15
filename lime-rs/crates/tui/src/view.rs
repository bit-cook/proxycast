use ratatui::layout::{Constraint, Direction, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, FrameExt as _, Paragraph};
use ratatui::Frame;

use crate::app::App;
use crate::bottom_pane;
use crate::bottom_pane::command_popup;
use crate::bottom_pane::pending_input_preview;
use crate::line_truncation::truncate_line_with_ellipsis_if_overflow;
use crate::model_picker;
use crate::status_indicator_widget;
use crate::terminal_hyperlinks::HyperlinkParagraph;
use std::time::Instant;

pub(crate) fn render(frame: &mut Frame<'_>, app: &App) {
    let area = frame.area();
    if let Some(picker) = app.resume_picker.as_ref() {
        frame.render_widget(Clear, area);
        crate::resume_picker::render_with_locale(frame, picker, app.locale);
        return;
    }
    if let Some(pager) = app.pager_overlay.as_ref() {
        let transcript_lines = if pager.is_transcript() {
            crate::app::history_ui::render_transcript_content_lines(app, area.width, true)
        } else {
            Vec::new()
        };
        pager.render(frame, area, app.locale, &transcript_lines);
        return;
    }
    if let Some(picker) = app.export_picker.as_ref() {
        crate::app::transcript_export::render_picker(frame, area, picker, app.locale);
        return;
    }
    let active_elapsed = app.active_turn_elapsed(Instant::now());
    let chunks = screen_chunks(area, app, active_elapsed);

    render_transcript(frame, chunks.transcript, app);
    if !app.bottom_pane.is_active() {
        if let Some(elapsed) = active_elapsed {
            let inline_status = active_status_message(app);
            status_indicator_widget::render_with_messages(
                frame,
                chunks.status,
                app.locale,
                elapsed,
                inline_status.as_deref(),
                app.projection.hook_status_message(),
            );
        } else if let Some(status) = transient_status(app) {
            render_transient_status(frame, chunks.status, app.locale, &status);
        }
        pending_input_preview::render(frame, chunks.preview, &app.queued_submissions, app.locale);
    }
    if app.bottom_pane.is_active() {
        bottom_pane::render_with_locale(frame, chunks.input, &app.bottom_pane, app.locale);
    } else {
        render_composer(frame, chunks.input, app);
    }
    bottom_pane::render_footer(frame, chunks.footer, app);
    if !app.bottom_pane.is_active() {
        if let Some(popup) = app.composer.command_popup() {
            command_popup::render(frame, chunks.input, popup, app.locale);
        }
        if let Some(popup) = app.composer.file_search_popup() {
            popup.render(frame, chunks.input, app.locale);
        }
        if let Some(popup) = app.composer.skill_popup() {
            popup.render(frame, chunks.input, app.locale);
        }
    }
    if let Some(picker) = app.model_picker.as_ref() {
        model_picker::render_with_locale(frame, area, picker, app.locale);
    }
    if let Some(overview) = app.agents_overview.as_ref() {
        crate::app::agents_overview_view::render(frame, area, &overview.view, app.locale);
        // Keep app-scoped action feedback visible while the centered overview owns the main
        // viewport. Its popup intentionally leaves the footer row available for this status.
        if let Some(status) = transient_status(app) {
            render_transient_status(frame, chunks.footer, app.locale, &status);
        }
    }
    if let Some(picker) = app.agent_picker.as_ref() {
        if app.agents_overview.is_none() {
            crate::app::agent_picker::render(frame, area, picker, app.locale);
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ScreenChunks {
    transcript: Rect,
    status: Rect,
    preview: Rect,
    input: Rect,
    footer: Rect,
}

fn screen_chunks(
    area: Rect,
    app: &App,
    active_elapsed: Option<std::time::Duration>,
) -> ScreenChunks {
    let status_height = if app.bottom_pane.is_active() {
        0
    } else if let Some(elapsed) = active_elapsed {
        let inline_status = active_status_message(app);
        status_indicator_widget::desired_height_with_messages(
            area.width,
            app.locale,
            elapsed,
            inline_status.as_deref(),
            app.projection.hook_status_message(),
        )
    } else if transient_status(app).is_some() {
        1
    } else {
        0
    };
    let preview_height = if app.bottom_pane.is_active() {
        0
    } else {
        pending_input_preview::desired_height(&app.queued_submissions, area.width, app.locale)
            .min(8)
            .min(area.height.saturating_sub(6 + status_height))
    };
    let input_height = if app.bottom_pane.is_active() {
        bottom_pane::desired_height_with_locale_for_width(&app.bottom_pane, app.locale, area.width)
    } else {
        let desired = app
            .composer
            .desired_height(area.width.saturating_sub(2))
            .saturating_add(u16::try_from(app.composer.pending_image_count()).unwrap_or(u16::MAX))
            .saturating_add(1)
            .clamp(2, 12);
        desired.min(
            area.height
                .saturating_sub(preview_height)
                .saturating_sub(status_height)
                .saturating_sub(2)
                .max(1),
        )
    };
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(status_height),
            Constraint::Length(preview_height),
            Constraint::Length(input_height),
            Constraint::Length(1),
        ])
        .split(area);
    ScreenChunks {
        transcript: chunks[0],
        status: chunks[1],
        preview: chunks[2],
        input: chunks[3],
        footer: chunks[4],
    }
}

fn transient_status(app: &App) -> Option<String> {
    let status = app.status_value();
    if status.is_empty() || status == "ready" {
        None
    } else {
        Some(status)
    }
}

fn active_status_message(app: &App) -> Option<String> {
    let status = transient_status(app)?;
    if status == "running" {
        None
    } else {
        Some(app.locale.status(&status))
    }
}

fn render_transient_status(
    frame: &mut Frame<'_>,
    area: Rect,
    locale: crate::locale::Locale,
    status: &str,
) {
    if area.is_empty() {
        return;
    }
    let line = Line::from(vec![
        Span::styled("• ", crate::style::accent_style()),
        Span::styled(locale.status(status), crate::style::muted_style()),
    ]);
    frame.render_widget(
        Paragraph::new(truncate_line_with_ellipsis_if_overflow(
            line,
            usize::from(area.width),
        )),
        area,
    );
}

pub(crate) fn transcript_page_size(width: u16, height: u16, app: &App) -> usize {
    let transcript = screen_chunks(
        Rect::new(0, 0, width, height),
        app,
        app.active_turn_elapsed(Instant::now()),
    )
    .transcript;
    usize::from(transcript.height.saturating_sub(1).max(1))
}

fn render_transcript(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let lines = crate::app::history_ui::render_transcript_content_lines(app, area.width, false);
    let paragraph = HyperlinkParagraph::new(&lines);
    let scroll = app
        .transcript_viewport
        .resolve(&lines, area, app.transcript_scroll);
    frame.render_widget(paragraph.scroll(scroll), area);
}

#[cfg(test)]
fn transcript_scroll_offset(
    rendered_line_count: usize,
    area: Rect,
    distance_from_bottom: usize,
) -> u16 {
    let max_scroll = rendered_line_count.saturating_sub(usize::from(area.height));
    let offset = max_scroll.saturating_sub(distance_from_bottom.min(max_scroll));
    u16::try_from(offset).unwrap_or(u16::MAX)
}

fn render_composer(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let inner = area;

    let remote_count = app.composer.remote_image_urls().len();
    let mut image_lines = app.composer.remote_image_lines();
    image_lines.extend(
        app.composer
            .pending_images()
            .iter()
            .enumerate()
            .map(|(index, _)| {
                Line::styled(
                    format!("[Image #{}]", remote_count + index + 1),
                    Style::default().fg(Color::Cyan),
                )
            }),
    );
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let image_height = u16::try_from(image_lines.len())
        .unwrap_or(u16::MAX)
        .min(inner.height);
    if image_height > 0 {
        let image_area = Rect::new(inner.x, inner.y, inner.width, image_height);
        frame.render_widget(Paragraph::new(image_lines), image_area);
    }

    let prompt_width = 2_u16.min(inner.width);
    let text_area = Rect::new(
        inner.x.saturating_add(prompt_width),
        inner.y.saturating_add(image_height),
        inner.width.saturating_sub(prompt_width),
        inner.height.saturating_sub(image_height),
    );
    if text_area.is_empty() {
        return;
    }
    let cursor = {
        let mut state = app.composer.textarea_state_mut();
        let highlights = app
            .composer
            .history_search_highlight_ranges()
            .into_iter()
            .map(|range| {
                (
                    range,
                    Style::default()
                        .add_modifier(Modifier::REVERSED)
                        .add_modifier(Modifier::BOLD),
                )
            })
            .collect::<Vec<_>>();
        let prompt = Line::from(Span::styled("› ", crate::style::accent_style()));
        frame.render_widget(
            Paragraph::new(prompt),
            Rect::new(inner.x, text_area.y, prompt_width, 1),
        );
        if highlights.is_empty() {
            if app.composer.is_empty() {
                // Keep attachment rows visible above the input baseline. The prompt occupies its
                // own gutter, while the placeholder is rendered inside the text area so both
                // share the same input baseline.
                let placeholder = Line::from(Span::styled(
                    app.locale.composer_placeholder(),
                    crate::style::muted_style(),
                ));
                frame.render_widget(Paragraph::new(placeholder), text_area);
            } else {
                frame.render_stateful_widget_ref(app.composer.textarea(), text_area, &mut *state);
            }
        } else {
            app.composer.textarea().render_ref_styled_with_highlights(
                text_area,
                frame.buffer_mut(),
                &mut state,
                Style::default(),
                &highlights,
            );
        }
        app.composer
            .textarea()
            .cursor_pos_with_state(text_area, *state)
    };
    if let Some((x, y)) = cursor {
        frame.set_cursor_position(Position::new(x, y));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::locale::Locale;
    use app_server_protocol::protocol::v2::{
        AgentMessageDeltaNotification, CommandExecutionOutputDeltaNotification,
        CommandExecutionRequestApprovalParams, CommandExecutionSource, FileUpdateChange,
        ItemCompletedNotification, ItemStartedNotification, PatchApplyStatus, PatchChangeKind,
        QueuedSubmission, ServerNotification, ServerRequest, ThreadItem,
        ToolRequestUserInputParams, ToolRequestUserInputQuestion, TurnDiffUpdatedNotification,
        TurnPlanStep, TurnPlanStepStatus, TurnPlanUpdatedNotification, UserInput,
    };
    use app_server_protocol::RequestId;
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
    use ratatui::backend::TestBackend;
    use ratatui::widgets::Wrap;
    use ratatui::Terminal;

    fn dispatch_connected_input(app: &mut App, event: Event) -> crate::app::AppAction {
        let event = match event {
            Event::Key(key) => crate::tui::TuiEvent::Key(key),
            Event::Paste(text) => crate::tui::TuiEvent::Paste(text),
            Event::Resize(width, height) => {
                crate::tui::TuiEvent::Resize(ratatui::layout::Size { width, height })
            }
            Event::FocusGained => crate::tui::TuiEvent::FocusGained,
            Event::FocusLost => crate::tui::TuiEvent::FocusLost,
            _ => crate::tui::TuiEvent::Draw,
        };
        app.handle_tui_event(event, true)
    }

    #[test]
    fn wrapped_transcript_scroll_uses_visual_rows() {
        let paragraph = Paragraph::new("abcdefghij").wrap(Wrap { trim: false });
        let narrow = Rect::new(0, 0, 5, 1);

        let narrow_count = paragraph.line_count(narrow.width);
        assert_eq!(transcript_scroll_offset(narrow_count, narrow, 0), 1);
        assert_eq!(transcript_scroll_offset(narrow_count, narrow, 1), 0);
        assert_eq!(
            transcript_scroll_offset(narrow_count, narrow, usize::MAX),
            0
        );
        assert_eq!(
            transcript_scroll_offset(paragraph.line_count(10), Rect::new(0, 0, 10, 1), 0,),
            0
        );
    }

    #[test]
    fn transcript_page_size_tracks_resize() {
        let mut app = App::default();

        assert_eq!(transcript_page_size(80, 10, &app), 6);
        assert_eq!(transcript_page_size(80, 6, &app), 2);
        app.attach_image(std::path::PathBuf::from("/tmp/one.png"));
        app.attach_image(std::path::PathBuf::from("/tmp/two.png"));
        assert_eq!(transcript_page_size(80, 10, &app), 4);
    }

    fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
        let buffer = terminal.backend().buffer();
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| crate::terminal_hyperlinks::strip_osc8(buffer[(x, y)].symbol()))
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn test_backend_renders_streaming_unicode_and_composer() {
        let mut app = App::default();
        app.set_settings(
            Some("fixture-model".to_string()),
            Some("fixture-provider".to_string()),
            Some("high".to_string()),
            Some(":workspace".to_string()),
        );
        app.projection.apply(ServerNotification::AgentMessageDelta(
            AgentMessageDeltaNotification {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                item_id: "item-1".to_string(),
                delta: "你好，terminal".to_string(),
            },
        ));
        app.composer.insert("继续");
        let mut terminal = Terminal::new(TestBackend::new(80, 16)).expect("terminal");

        terminal.draw(|frame| render(frame, &app)).expect("draw");

        let text = buffer_text(&terminal);
        assert!(text.contains("Lime"));
        assert!(text.contains('你'));
        assert!(text.contains('好'));
        assert!(text.contains("terminal"));
        assert!(text.contains('继'));
        assert!(text.contains('续'));
        let compact = text
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>();
        assert!(compact.contains("model:fixture-model"));
        assert!(compact.contains("high"));
    }

    #[test]
    fn test_backend_renders_pending_images_above_composer_text() {
        let mut app = App::default();
        app.attach_image(std::path::PathBuf::from("/tmp/one.png"));
        app.attach_image(std::path::PathBuf::from("/tmp/two.png"));
        app.composer.insert("describe these");
        let mut terminal = Terminal::new(TestBackend::new(40, 10)).expect("terminal");

        terminal.draw(|frame| render(frame, &app)).expect("draw");

        let text = buffer_text(&terminal);
        assert!(text.contains("[Image #1]"));
        assert!(text.contains("[Image #2]"));
        assert!(text.contains("describe these"));
    }

    #[test]
    fn idle_composer_uses_codex_prompt_and_localized_placeholder() {
        let mut app = App::default();
        app.set_locale(Locale::EnUs);
        let mut terminal = Terminal::new(TestBackend::new(80, 10)).expect("terminal");

        terminal.draw(|frame| render(frame, &app)).expect("draw");

        let text = buffer_text(&terminal);
        assert!(text.contains("› "), "missing composer prompt: {text}");
        assert!(
            text.contains("Ask Lime to do anything"),
            "missing composer placeholder: {text}"
        );
    }

    #[test]
    fn transient_status_keeps_action_feedback_visible_without_idle_status_bar() {
        let mut app = App::default();
        app.projection.set_status("interrupting");
        let mut terminal = Terminal::new(TestBackend::new(80, 10)).expect("terminal");

        terminal.draw(|frame| render(frame, &app)).expect("draw");

        let text = buffer_text(&terminal);
        assert!(
            text.contains("• interrupting"),
            "missing transient status: {text}"
        );
        assert!(
            text.contains("› Ask Lime to do anything"),
            "missing composer: {text}"
        );
        assert!(
            !text.contains("Lime ready"),
            "idle header leaked into status: {text}"
        );
    }

    #[test]
    fn test_backend_renders_remote_images_with_selection_highlight() {
        let mut app = App::default();
        app.set_remote_image_urls(vec![
            "https://example.test/one.png".to_string(),
            "https://example.test/two.png".to_string(),
        ]);
        let mut terminal = Terminal::new(TestBackend::new(40, 10)).expect("terminal");

        terminal.draw(|frame| render(frame, &app)).expect("draw");
        let text = buffer_text(&terminal);
        assert!(text.contains("[Image #1]"));
        assert!(text.contains("[Image #2]"));

        let _ = app
            .composer
            .handle_key_event(KeyEvent::new(KeyCode::Up, KeyModifiers::NONE));
        terminal.draw(|frame| render(frame, &app)).expect("redraw");
        let buffer = terminal.backend().buffer();
        let highlighted = (0..buffer.area.height).any(|y| {
            (0..buffer.area.width).any(|x| {
                let cell = &buffer[(x, y)];
                cell.symbol() == "["
                    && cell.style().add_modifier(Modifier::REVERSED) == cell.style()
            })
        });
        assert!(highlighted);
    }

    #[test]
    fn test_backend_renders_canonical_queue_between_transcript_and_composer() {
        let mut app = App::default();
        app.set_queued_submissions(vec![QueuedSubmission {
            id: "queue-1".to_string(),
            input: vec![UserInput::Text {
                text: "follow up after this turn".to_string(),
                text_elements: Vec::new(),
            }],
            client_user_message_id: "client-queue-1".to_string(),
        }]);
        app.composer.insert("current draft");
        let mut terminal = Terminal::new(TestBackend::new(40, 10)).expect("terminal");

        terminal.draw(|frame| render(frame, &app)).expect("draw");

        let text = buffer_text(&terminal);
        let queued_row = text.find("queued (1)").expect("queued header");
        let message_row = text
            .find("follow up after this turn")
            .expect("queued message");
        let composer_row = text.find("current draft").expect("composer");
        assert!(
            queued_row < message_row && message_row < composer_row,
            "{text}"
        );
    }

    #[test]
    fn active_turn_status_precedes_canonical_queue_and_composer() {
        let mut app = App::default();
        app.start_turn("turn-1".to_string());
        app.set_queued_submissions(vec![QueuedSubmission {
            id: "queue-1".to_string(),
            input: vec![UserInput::Text {
                text: "follow up after this turn".to_string(),
                text_elements: Vec::new(),
            }],
            client_user_message_id: "client-queue-1".to_string(),
        }]);
        app.composer.insert("current draft");
        let mut terminal = Terminal::new(TestBackend::new(48, 12)).expect("terminal");

        terminal.draw(|frame| render(frame, &app)).expect("draw");

        let text = buffer_text(&terminal);
        let status_row = text.find("Working (").expect("active status");
        let interrupt_hint = text.find("esc to interrupt").expect("interrupt hint");
        let queued_row = text.find("queued (1)").expect("queued header");
        let composer_row = text.find("current draft").expect("composer");
        assert!(
            status_row < interrupt_hint && interrupt_hint < queued_row && queued_row < composer_row,
            "{text}"
        );
    }

    #[test]
    fn footer_renders_the_active_agent_label() {
        let mut app = App::default();
        app.set_thread_id("main".to_string());
        app.agent_navigation.upsert(
            "agent-1",
            Some("Robie".to_string()),
            Some("explorer".to_string()),
            false,
        );
        app.set_thread_id("agent-1".to_string());

        let mut terminal = Terminal::new(TestBackend::new(64, 8)).expect("terminal");
        terminal.draw(|frame| render(frame, &app)).expect("draw");
        assert!(buffer_text(&terminal).contains("Robie [explorer]"));
    }

    #[test]
    fn active_turn_status_does_not_overflow_a_tiny_terminal() {
        let mut app = App::default();
        app.start_turn("turn-1".to_string());
        app.set_queued_submissions(vec![QueuedSubmission {
            id: "queue-1".to_string(),
            input: vec![UserInput::Text {
                text: "queued text that cannot fit".to_string(),
                text_elements: Vec::new(),
            }],
            client_user_message_id: "client-queue-1".to_string(),
        }]);
        app.composer.insert("界界界界");
        let mut terminal = Terminal::new(TestBackend::new(12, 7)).expect("terminal");

        terminal.draw(|frame| render(frame, &app)).expect("draw");

        let text = buffer_text(&terminal);
        assert_eq!(text.lines().count(), 7, "{text}");
        assert!(text.contains("Working"), "{text}");
        assert!(text.contains('…'), "{text}");
    }

    #[test]
    fn status_and_footer_geometry_remains_stable_across_supported_widths_and_locales() {
        for locale in [
            Locale::ZhCn,
            Locale::ZhTw,
            Locale::EnUs,
            Locale::JaJp,
            Locale::KoKr,
        ] {
            for width in [40, 80, 120] {
                let mut app = App::default();
                app.set_locale(locale);
                app.start_turn(format!("turn-{width}"));
                app.set_queued_submissions(vec![QueuedSubmission {
                    id: format!("queue-{width}"),
                    input: vec![UserInput::Text {
                        text: "queued follow-up".to_string(),
                        text_elements: Vec::new(),
                    }],
                    client_user_message_id: format!("client-{width}"),
                }]);
                app.composer.insert("draft");

                let height = 20;
                let mut terminal =
                    Terminal::new(TestBackend::new(width, height)).expect("terminal");
                terminal.draw(|frame| render(frame, &app)).expect("draw");
                let text = buffer_text(&terminal);
                assert_eq!(text.lines().count(), usize::from(height));
                let compact = text
                    .chars()
                    .filter(|character| !character.is_whitespace())
                    .collect::<String>();
                assert!(
                    compact.contains('›'),
                    "missing composer for {locale:?} at {width}: {text}"
                );
                assert!(
                    compact.contains(
                        &locale
                            .working_label()
                            .chars()
                            .filter(|character| !character.is_whitespace())
                            .collect::<String>()
                    ),
                    "missing active status for {locale:?} at {width}: {text}"
                );
                assert!(
                    compact.contains(
                        &locale
                            .status("queued")
                            .chars()
                            .filter(|character| !character.is_whitespace())
                            .collect::<String>()
                    ),
                    "missing queue preview for {locale:?} at {width}: {text}"
                );

                let chunks = screen_chunks(
                    Rect::new(0, 0, width, height),
                    &app,
                    app.active_turn_elapsed(Instant::now()),
                );
                assert!(chunks.transcript.bottom() <= chunks.status.top());
                assert!(chunks.status.bottom() <= chunks.preview.top());
                assert!(chunks.preview.bottom() <= chunks.input.top());
                assert!(chunks.input.bottom() <= chunks.footer.top());
                assert_eq!(chunks.footer.height, 1);
            }
        }
    }

    #[test]
    fn history_search_footer_shows_localized_query_without_hiding_composer() {
        let mut app = App::default();
        app.set_locale(Locale::ZhCn);
        app.composer.load_history(["git status".to_string()]);
        app.composer.insert("git");
        dispatch_connected_input(
            &mut app,
            Event::Key(crossterm::event::KeyEvent::new(
                crossterm::event::KeyCode::Char('r'),
                crossterm::event::KeyModifiers::CONTROL,
            )),
        );
        for character in "git".chars() {
            dispatch_connected_input(
                &mut app,
                Event::Key(crossterm::event::KeyEvent::new(
                    crossterm::event::KeyCode::Char(character),
                    crossterm::event::KeyModifiers::NONE,
                )),
            );
        }
        let mut terminal = Terminal::new(TestBackend::new(48, 8)).expect("terminal");

        terminal.draw(|frame| render(frame, &app)).expect("draw");

        let text = buffer_text(&terminal);
        let compact = text
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>();
        assert!(compact.contains("反向搜索：git"), "{text}");
        assert!(text.contains("git status"), "{text}");
    }

    #[test]
    fn history_search_preview_highlights_matches_until_accepted() {
        let mut app = App::default();
        app.composer.load_history(["Deploy Lime".to_string()]);
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL)),
        );
        for character in "dep".chars() {
            dispatch_connected_input(
                &mut app,
                Event::Key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE)),
            );
        }

        let area = Rect::new(0, 0, 64, 10);
        let mut terminal =
            Terminal::new(TestBackend::new(area.width, area.height)).expect("terminal");
        terminal.draw(|frame| render(frame, &app)).expect("draw");
        let buffer = terminal.backend().buffer();
        let mut start = None;
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width.saturating_sub(2) {
                if buffer[(x, y)].symbol() == "D"
                    && buffer[(x + 1, y)].symbol() == "e"
                    && buffer[(x + 2, y)].symbol() == "p"
                {
                    start = Some((x, y));
                    break;
                }
            }
            if start.is_some() {
                break;
            }
        }
        let (x, y) = start.expect("history preview");
        for offset in 0..3 {
            let modifiers = buffer[(x + offset, y)].style().add_modifier;
            assert!(modifiers.contains(Modifier::REVERSED));
            assert!(modifiers.contains(Modifier::BOLD));
        }

        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)),
        );
        terminal.draw(|frame| render(frame, &app)).expect("redraw");
        let buffer = terminal.backend().buffer();
        for offset in 0..3 {
            let modifiers = buffer[(x + offset, y)].style().add_modifier;
            assert!(!modifiers.contains(Modifier::REVERSED));
            assert!(!modifiers.contains(Modifier::BOLD));
        }
    }

    #[test]
    fn history_search_footer_cursor_tracks_query_and_clamps_to_narrow_width() {
        let mut app = App::default();
        app.composer.load_history(["git status".to_string()]);
        dispatch_connected_input(
            &mut app,
            Event::Key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL)),
        );
        for character in "git".chars() {
            dispatch_connected_input(
                &mut app,
                Event::Key(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE)),
            );
        }

        let wide = Rect::new(0, 0, 80, 10);
        let mut terminal =
            Terminal::new(TestBackend::new(wide.width, wide.height)).expect("terminal");
        terminal
            .draw(|frame| render(frame, &app))
            .expect("draw wide");
        let footer = screen_chunks(wide, &app, app.active_turn_elapsed(Instant::now())).footer;
        let prefix_width =
            Line::from(format!(" {}", app.locale.history_search_label())).width() as u16;
        assert_eq!(
            terminal.backend().cursor_position(),
            Position::new(footer.x + prefix_width + 3, footer.y)
        );

        let narrow = Rect::new(0, 0, 12, 10);
        let mut terminal =
            Terminal::new(TestBackend::new(narrow.width, narrow.height)).expect("terminal");
        terminal
            .draw(|frame| render(frame, &app))
            .expect("draw narrow");
        let footer = screen_chunks(narrow, &app, app.active_turn_elapsed(Instant::now())).footer;
        assert_eq!(
            terminal.backend().cursor_position(),
            Position::new(footer.right().saturating_sub(1), footer.y)
        );
    }

    #[test]
    fn test_backend_renders_filtered_slash_command_popup_above_composer() {
        let mut app = App::default();
        app.set_locale(Locale::ZhCn);
        for character in ['/', 'p', 'e'] {
            dispatch_connected_input(
                &mut app,
                Event::Key(crossterm::event::KeyEvent::new(
                    crossterm::event::KeyCode::Char(character),
                    crossterm::event::KeyModifiers::NONE,
                )),
            );
        }
        let mut terminal = Terminal::new(TestBackend::new(48, 10)).expect("terminal");

        terminal.draw(|frame| render(frame, &app)).expect("draw");

        let text = buffer_text(&terminal);
        let compact = text
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>();
        assert!(text.contains("› /permissions"));
        assert!(compact.contains("设置权限配置"), "{text}");
        assert_eq!(
            text.matches("/model").count(),
            1,
            "session header only: {text}"
        );
        assert!(text.contains("/pe"));
    }

    #[test]
    fn export_picker_matches_codex_destination_and_filename_flow() {
        let mut app = App::default();
        app.set_thread_id("00000000-0000-0000-0000-000000000123".to_string());
        app.composer.insert("/export");
        assert_eq!(
            dispatch_connected_input(
                &mut app,
                Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE,))
            ),
            crate::app::AppAction::None
        );

        let mut terminal = Terminal::new(TestBackend::new(80, 12)).expect("terminal");
        terminal.draw(|frame| render(frame, &app)).expect("draw");
        let text = buffer_text(&terminal);
        assert!(text.contains("Export conversation"), "{text}");
        assert!(text.contains("Copy to clipboard"), "{text}");
        assert!(text.contains("Save to file"), "{text}");

        assert_eq!(
            dispatch_connected_input(
                &mut app,
                Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE,))
            ),
            crate::app::AppAction::None
        );
        assert_eq!(
            dispatch_connected_input(
                &mut app,
                Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE,))
            ),
            crate::app::AppAction::None
        );
        assert!(app
            .export_picker
            .as_ref()
            .is_some_and(crate::app::transcript_export::ExportPicker::is_filename_prompt));

        terminal
            .draw(|frame| render(frame, &app))
            .expect("draw filename");
        let text = buffer_text(&terminal);
        assert!(text.contains("Save conversation"), "{text}");
        assert!(text.contains("codex-session-00000000-0000-0000-0000-000000000123.md"));

        assert_eq!(
            dispatch_connected_input(
                &mut app,
                Event::Key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE,))
            ),
            crate::app::AppAction::ExportTranscript {
                path: Some(std::path::PathBuf::from(
                    "codex-session-00000000-0000-0000-0000-000000000123.md",
                )),
            }
        );
        assert!(app.export_picker.is_none());
    }

    #[test]
    fn status_pager_owns_the_frame_and_renders_current_session_facts() {
        let mut app = App::default();
        app.set_thread_id("thread-1".to_string());
        app.set_cwd(std::path::PathBuf::from("/workspace"));
        app.set_settings(
            Some("gpt-5".to_string()),
            Some("openai".to_string()),
            Some("high".to_string()),
            Some(":workspace".to_string()),
        );
        app.composer.insert("/status");
        dispatch_connected_input(
            &mut app,
            Event::Key(crossterm::event::KeyEvent::new(
                crossterm::event::KeyCode::Enter,
                crossterm::event::KeyModifiers::NONE,
            )),
        );
        let mut terminal = Terminal::new(TestBackend::new(72, 12)).expect("terminal");

        terminal.draw(|frame| render(frame, &app)).expect("draw");

        let text = buffer_text(&terminal);
        for value in [
            "/ STATUS",
            "thread-1",
            "gpt-5",
            "openai",
            ":workspace",
            "/workspace",
            "100%",
        ] {
            assert!(text.contains(value), "missing {value}: {text}");
        }
        assert!(!text.contains("Lime"));
    }

    #[test]
    fn transcript_overlay_renders_live_canonical_projection_with_markdown_and_links() {
        let destination = "https://example.com/transcript";
        let mut app = App::default();
        app.composer.insert("draft remains private");
        app.projection.apply(ServerNotification::AgentMessageDelta(
            AgentMessageDeltaNotification {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                item_id: "assistant-1".to_string(),
                delta: format!("**first** [link]({destination})"),
            },
        ));
        dispatch_connected_input(
            &mut app,
            Event::Key(crossterm::event::KeyEvent::new(
                crossterm::event::KeyCode::Char('t'),
                crossterm::event::KeyModifiers::CONTROL,
            )),
        );
        let mut terminal = Terminal::new(TestBackend::new(48, 9)).expect("terminal");
        terminal.draw(|frame| render(frame, &app)).expect("draw");

        let text = buffer_text(&terminal);
        assert!(text.contains("/ T R A N S C R I P T"), "{text}");
        assert!(text.contains("first"), "{text}");
        assert!(text.contains("link"), "{text}");
        assert!(!text.contains("draft remains private"), "{text}");
        assert!(terminal.backend().buffer().content.iter().any(|cell| {
            cell.symbol()
                .contains(&format!("\x1b]8;;{destination}\x07"))
        }));

        app.projection.apply(ServerNotification::AgentMessageDelta(
            AgentMessageDeltaNotification {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                item_id: "assistant-2".to_string(),
                delta: "latest canonical update".to_string(),
            },
        ));
        terminal.draw(|frame| render(frame, &app)).expect("redraw");
        assert!(
            buffer_text(&terminal).contains("latest canonical update"),
            "{}",
            buffer_text(&terminal)
        );
    }

    #[test]
    fn user_visible_header_labels_cover_all_product_locales() {
        let cases = [
            (Locale::ZhCn, "模型:fixture-model", "设置已更新"),
            (Locale::ZhTw, "模型:fixture-model", "設定已更新"),
            (Locale::EnUs, "model:fixture-model", "settings updated"),
            (Locale::JaJp, "モデル:fixture-model", "設定を更新しました"),
            (Locale::KoKr, "모델:fixture-model", "설정이 업데이트됨"),
        ];
        for (locale, model_label, _status_label) in cases {
            let mut app = App::default();
            app.set_locale(locale);
            app.set_settings(
                Some("fixture-model".to_string()),
                Some("fixture-provider".to_string()),
                Some("high".to_string()),
                Some(":workspace".to_string()),
            );
            app.set_cwd(std::path::PathBuf::from("/workspace"));
            let mut terminal = Terminal::new(TestBackend::new(80, 16)).expect("terminal");
            terminal.draw(|frame| render(frame, &app)).expect("draw");
            let text = buffer_text(&terminal);
            let compact = text
                .chars()
                .filter(|character| !character.is_whitespace())
                .collect::<String>();
            assert!(compact.contains(model_label), "{locale:?}: {text}");
            assert!(compact.contains("/workspace"), "{locale:?}: {text}");
        }
    }

    #[test]
    fn test_backend_renders_specialized_plan_and_patch_layouts() {
        let mut app = App::default();
        app.projection.apply(ServerNotification::TurnPlanUpdated(
            TurnPlanUpdatedNotification {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                explanation: None,
                plan: vec![TurnPlanStep {
                    step: "run tests".to_string(),
                    status: TurnPlanStepStatus::InProgress,
                }],
            },
        ));
        app.projection.apply(ServerNotification::TurnDiffUpdated(
            TurnDiffUpdatedNotification {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                diff: "+new line".to_string(),
            },
        ));
        let mut terminal = Terminal::new(TestBackend::new(40, 10)).expect("terminal");

        terminal.draw(|frame| render(frame, &app)).expect("draw");

        let text = buffer_text(&terminal);
        assert!(text.contains("• [~] run tests"));
        assert!(text.contains("Δ +new line"));
    }

    #[test]
    fn test_backend_renders_markdown_and_numbered_diff() {
        let mut app = App::default();
        app.projection.apply(ServerNotification::AgentMessageDelta(
            AgentMessageDeltaNotification {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                item_id: "assistant-1".to_string(),
                delta: "## Result\n\nRead [the guide](https://example.com/guide).".to_string(),
            },
        ));
        app.projection.apply(ServerNotification::TurnDiffUpdated(
            TurnDiffUpdatedNotification {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                diff: "@@ -1 +1 @@\n-old\n+new".to_string(),
            },
        ));
        let mut terminal = Terminal::new(TestBackend::new(80, 16)).expect("terminal");

        terminal.draw(|frame| render(frame, &app)).expect("draw");

        let text = buffer_text(&terminal);
        assert!(text.contains("## Result"));
        assert!(text.contains("the guide (https://example.com/guide)"));
        assert!(text.contains("1 -old"));
        assert!(text.contains("1 +new"));
    }

    #[test]
    fn test_backend_marks_markdown_links_with_osc8() {
        let destination = "https://example.com/guide";
        let mut app = App::default();
        app.projection.apply(ServerNotification::AgentMessageDelta(
            AgentMessageDeltaNotification {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                item_id: "assistant-1".to_string(),
                delta: format!("Read [the guide]({destination})."),
            },
        ));
        let mut terminal = Terminal::new(TestBackend::new(40, 10)).expect("terminal");

        terminal.draw(|frame| render(frame, &app)).expect("draw");

        let buffer = terminal.backend().buffer();
        assert!(buffer.content.iter().any(|cell| {
            cell.symbol()
                .contains(&format!("\x1b]8;;{destination}\x07"))
        }));
        let visible = buffer_text(&terminal);
        assert!(visible.contains("the guide"));
        assert!(visible.contains("https://example.com/guide"));
    }

    #[test]
    fn test_backend_renders_completed_item_summaries() {
        let mut app = App::default();
        app.projection.apply(ServerNotification::ItemCompleted(
            ItemCompletedNotification {
                item: ThreadItem::CommandExecution {
                    id: "command-1".to_string(),
                    metadata: None,
                    plugin_id: None,
                    script_path: None,
                    command: "cargo test -p tui".to_string(),
                    cwd: "/workspace".to_string(),
                    process_id: None,
                    source: CommandExecutionSource::Agent,
                    status: app_server_protocol::protocol::v2::CommandExecutionStatus::Completed,
                    command_actions: Vec::new(),
                    aggregated_output: Some("ok".to_string()),
                    exit_code: Some(0),
                    duration_ms: Some(42),
                    terminal_interactions: Vec::new(),
                },
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                completed_at_ms: 1,
            },
        ));
        app.projection.apply(ServerNotification::ItemCompleted(
            ItemCompletedNotification {
                item: ThreadItem::FileChange {
                    id: "patch-1".to_string(),
                    metadata: None,
                    changes: vec![FileUpdateChange {
                        path: "src/lib.rs".to_string(),
                        kind: PatchChangeKind::Add,
                        diff: "+new".to_string(),
                    }],
                    status: PatchApplyStatus::Completed,
                },
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                completed_at_ms: 2,
            },
        ));
        let mut terminal = Terminal::new(TestBackend::new(80, 16)).expect("terminal");

        terminal.draw(|frame| render(frame, &app)).expect("draw");

        let text = buffer_text(&terminal);
        assert!(text.contains("cargo test -p tui [completed]"));
        assert!(text.contains("- exit 0"));
        assert!(text.contains("- duration 42ms"));
        assert!(text.contains("src/lib.rs (+1 -0) [completed]"));
        assert!(text.contains("- files: 1"));
        assert!(text.contains("+new"));
    }

    #[test]
    fn test_backend_renders_live_command_output_below_the_command() {
        let mut app = App::default();
        app.projection
            .apply(ServerNotification::ItemStarted(ItemStartedNotification {
                item: ThreadItem::CommandExecution {
                    id: "command-1".to_string(),
                    metadata: None,
                    plugin_id: None,
                    script_path: None,
                    command: "printf data".to_string(),
                    cwd: "/workspace".to_string(),
                    process_id: None,
                    source: CommandExecutionSource::Agent,
                    status: app_server_protocol::protocol::v2::CommandExecutionStatus::InProgress,
                    command_actions: Vec::new(),
                    aggregated_output: None,
                    exit_code: None,
                    duration_ms: None,
                    terminal_interactions: Vec::new(),
                },
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                started_at_ms: 1,
            }));
        for delta in ["std", "out\nstderr\n"] {
            app.projection
                .apply(ServerNotification::CommandExecutionOutputDelta(
                    CommandExecutionOutputDeltaNotification {
                        thread_id: "thread-1".to_string(),
                        turn_id: "turn-1".to_string(),
                        item_id: "command-1".to_string(),
                        delta: delta.to_string(),
                    },
                ));
        }
        let mut terminal = Terminal::new(TestBackend::new(60, 10)).expect("terminal");

        terminal.draw(|frame| render(frame, &app)).expect("draw");

        let text = buffer_text(&terminal);
        let command = text.find("$ printf data [running]").expect("command row");
        let stdout = text.find("stdout").expect("stdout row");
        let stderr = text.find("stderr").expect("stderr row");
        assert!(command < stdout && stdout < stderr, "{text}");
        assert!(!text.contains("datastdout"), "{text}");
    }

    #[test]
    fn test_backend_renders_bounded_command_output_marker() {
        let mut app = App::default();
        app.projection
            .apply(ServerNotification::ItemStarted(ItemStartedNotification {
                item: ThreadItem::CommandExecution {
                    id: "command-large".to_string(),
                    metadata: None,
                    plugin_id: None,
                    script_path: None,
                    command: "printf output".to_string(),
                    cwd: "/workspace".to_string(),
                    process_id: None,
                    source: CommandExecutionSource::Agent,
                    status: app_server_protocol::protocol::v2::CommandExecutionStatus::InProgress,
                    command_actions: Vec::new(),
                    aggregated_output: None,
                    exit_code: None,
                    duration_ms: None,
                    terminal_interactions: Vec::new(),
                },
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                started_at_ms: 1,
            }));
        let output = (0..101)
            .map(|index| format!("line-{index}"))
            .collect::<Vec<_>>()
            .join("\n");
        app.projection
            .apply(ServerNotification::CommandExecutionOutputDelta(
                CommandExecutionOutputDeltaNotification {
                    thread_id: "thread-1".to_string(),
                    turn_id: "turn-1".to_string(),
                    item_id: "command-large".to_string(),
                    delta: output,
                },
            ));
        let mut terminal = Terminal::new(TestBackend::new(80, 110)).expect("terminal");

        terminal.draw(|frame| render(frame, &app)).expect("draw");

        let text = buffer_text(&terminal);
        assert!(text.contains("… 1 lines omitted …"), "{text}");
        assert!(text.contains("line-0"), "{text}");
        assert!(text.contains("line-100"), "{text}");
        assert!(!text.contains("line-50"), "{text}");
    }

    #[test]
    fn narrow_terminal_does_not_overflow_or_panic() {
        let mut app = App::default();
        app.projection.set_status("a-status-that-does-not-fit");
        app.composer.insert("界界界界");
        let mut terminal = Terminal::new(TestBackend::new(8, 6)).expect("terminal");

        terminal.draw(|frame| render(frame, &app)).expect("draw");

        let text = buffer_text(&terminal);
        assert_eq!(text.lines().count(), 6);
        assert!(
            text.contains("╰"),
            "narrow session header remains bounded: {text}"
        );
        assert!(
            !text.contains("Lime"),
            "narrow header should clip safely: {text}"
        );
    }

    #[test]
    fn approval_replaces_the_composer_with_actionable_options() {
        let mut app = App::default();
        app.composer.insert("unsent draft");
        app.bottom_pane
            .enqueue(ServerRequest::ItemCommandExecutionRequestApproval {
                id: RequestId::Integer(7),
                params: CommandExecutionRequestApprovalParams {
                    thread_id: "thread-1".to_string(),
                    turn_id: "turn-1".to_string(),
                    item_id: "command-1".to_string(),
                    started_at_ms: 1,
                    approval_id: None,
                    reason: Some("run the focused regression".to_string()),
                    network_approval_context: None,
                    command: Some("cargo test -p tui".to_string()),
                    cwd: Some("/workspace".to_string()),
                    available_decisions: None,
                },
            })
            .expect("queue approval");
        let mut terminal = Terminal::new(TestBackend::new(60, 16)).expect("terminal");

        terminal.draw(|frame| render(frame, &app)).expect("draw");

        let text = buffer_text(&terminal);
        assert!(text.contains("Approve command?"));
        assert!(text.contains("cargo test -p tui"));
        assert!(text.contains("Allow once"));
        assert!(!text.contains("unsent draft"));
    }

    #[test]
    fn approval_hides_an_open_slash_command_popup() {
        let mut app = App::default();
        dispatch_connected_input(
            &mut app,
            Event::Key(crossterm::event::KeyEvent::new(
                crossterm::event::KeyCode::Char('/'),
                crossterm::event::KeyModifiers::NONE,
            )),
        );
        app.bottom_pane
            .enqueue(ServerRequest::ItemCommandExecutionRequestApproval {
                id: RequestId::Integer(9),
                params: CommandExecutionRequestApprovalParams {
                    thread_id: "thread-1".to_string(),
                    turn_id: "turn-1".to_string(),
                    item_id: "command-1".to_string(),
                    started_at_ms: 1,
                    approval_id: None,
                    reason: None,
                    network_approval_context: None,
                    command: Some("cargo test".to_string()),
                    cwd: Some("/workspace".to_string()),
                    available_decisions: None,
                },
            })
            .expect("queue approval");
        let mut terminal = Terminal::new(TestBackend::new(72, 12)).expect("terminal");

        terminal.draw(|frame| render(frame, &app)).expect("draw");

        let text = buffer_text(&terminal);
        assert!(text.contains("Approve command?"));
        assert!(!text.contains("/model"));
        assert!(!text.contains("copy the last response"));
    }

    #[test]
    fn approval_narrow_layout_keeps_primary_controls_visible() {
        let mut app = App::default();
        app.bottom_pane
            .enqueue(ServerRequest::ItemCommandExecutionRequestApproval {
                id: RequestId::Integer(10),
                params: CommandExecutionRequestApprovalParams {
                    thread_id: "thread-1".to_string(),
                    turn_id: "turn-1".to_string(),
                    item_id: "command-1".to_string(),
                    started_at_ms: 1,
                    approval_id: None,
                    reason: Some("a long reason that should wrap safely".to_string()),
                    network_approval_context: None,
                    command: Some("cargo test -p tui --lib".to_string()),
                    cwd: Some("/workspace/project".to_string()),
                    available_decisions: None,
                },
            })
            .expect("queue approval");
        let mut terminal = Terminal::new(TestBackend::new(40, 20)).expect("terminal");

        terminal.draw(|frame| render(frame, &app)).expect("draw");

        let text = buffer_text(&terminal);
        assert!(text.contains("Allow once"), "{text}");
        assert!(text.contains("Enter confirm"), "{text}");
        assert!(text.contains("Esc cancel"), "{text}");
        assert!(text.lines().all(|line| line.chars().count() <= 40));
    }

    #[test]
    fn request_user_input_narrow_layout_keeps_submit_and_cancel_visible() {
        let mut app = App::default();
        app.bottom_pane
            .enqueue(ServerRequest::ItemToolRequestUserInput {
                id: RequestId::Integer(11),
                params: ToolRequestUserInputParams {
                    thread_id: "thread-1".to_string(),
                    turn_id: "turn-1".to_string(),
                    item_id: "question-1".to_string(),
                    questions: vec![ToolRequestUserInputQuestion {
                        id: "mode".to_string(),
                        header: "Mode".to_string(),
                        question: "Choose the next step for this task".to_string(),
                        is_other: false,
                        is_secret: false,
                        options: Some(vec![
                            app_server_protocol::protocol::v2::ToolRequestUserInputOption {
                                label: "Run tests".to_string(),
                                description: "Pick the most relevant crate and validate behavior"
                                    .to_string(),
                            },
                            app_server_protocol::protocol::v2::ToolRequestUserInputOption {
                                label: "Review diff".to_string(),
                                description: "Summarize the current changes".to_string(),
                            },
                        ]),
                    }],
                    is_blocking: true,
                    auto_resolution_ms: None,
                },
            })
            .expect("queue user input");
        let mut terminal = Terminal::new(TestBackend::new(40, 20)).expect("terminal");

        terminal.draw(|frame| render(frame, &app)).expect("draw");

        let text = buffer_text(&terminal);
        assert!(text.contains("1. Run tests"), "{text}");
        assert!(text.contains("Enter submit"), "{text}");
        assert!(text.contains("Esc cancel"), "{text}");
        assert!(text.lines().all(|line| line.chars().count() <= 40));
    }

    #[test]
    fn interactive_overlays_remain_actionable_across_supported_widths_and_locales() {
        for locale in [
            Locale::ZhCn,
            Locale::ZhTw,
            Locale::EnUs,
            Locale::JaJp,
            Locale::KoKr,
        ] {
            for width in [40, 80, 120] {
                let mut approval = App::default();
                approval.set_locale(locale);
                approval
                    .bottom_pane
                    .enqueue(ServerRequest::ItemCommandExecutionRequestApproval {
                        id: RequestId::Integer(12),
                        params: CommandExecutionRequestApprovalParams {
                            thread_id: "thread-1".to_string(),
                            turn_id: "turn-1".to_string(),
                            item_id: "command-1".to_string(),
                            started_at_ms: 1,
                            approval_id: None,
                            reason: Some("focused regression".to_string()),
                            network_approval_context: None,
                            command: Some("cargo test -p tui".to_string()),
                            cwd: Some("/workspace".to_string()),
                            available_decisions: None,
                        },
                    })
                    .expect("queue approval");
                let mut terminal = Terminal::new(TestBackend::new(width, 20)).expect("terminal");
                terminal
                    .draw(|frame| render(frame, &approval))
                    .expect("draw approval");
                let approval_text = buffer_text(&terminal);
                let approval_compact = approval_text
                    .chars()
                    .filter(|character| !character.is_whitespace())
                    .collect::<String>();
                assert!(
                    approval_compact.contains(
                        &locale
                            .approval_title("command")
                            .chars()
                            .filter(|character| !character.is_whitespace())
                            .collect::<String>(),
                    ),
                    "approval title for {locale:?} at {width}: {approval_text}"
                );
                assert!(
                    approval_compact.contains(
                        &locale
                            .approval_controls()
                            .chars()
                            .filter(|character| !character.is_whitespace())
                            .collect::<String>(),
                    ),
                    "approval controls for {locale:?} at {width}: {approval_text}"
                );

                let mut question = App::default();
                question.set_locale(locale);
                question
                    .bottom_pane
                    .enqueue(ServerRequest::ItemToolRequestUserInput {
                        id: RequestId::Integer(13),
                        params: ToolRequestUserInputParams {
                            thread_id: "thread-1".to_string(),
                            turn_id: "turn-1".to_string(),
                            item_id: "question-1".to_string(),
                            questions: vec![ToolRequestUserInputQuestion {
                                id: "mode".to_string(),
                                header: "Mode".to_string(),
                                question: "Choose one option".to_string(),
                                is_other: false,
                                is_secret: false,
                                options: Some(vec![
                                    app_server_protocol::protocol::v2::ToolRequestUserInputOption {
                                        label: "Fast".to_string(),
                                        description: "Continue immediately".to_string(),
                                    },
                                ]),
                            }],
                            is_blocking: true,
                            auto_resolution_ms: None,
                        },
                    })
                    .expect("queue user input");
                terminal
                    .draw(|frame| render(frame, &question))
                    .expect("draw user input");
                let question_text = buffer_text(&terminal);
                let question_compact = question_text
                    .chars()
                    .filter(|character| !character.is_whitespace())
                    .collect::<String>();
                assert!(
                    question_compact.contains(
                        &locale
                            .request_submit_hint()
                            .chars()
                            .filter(|character| !character.is_whitespace())
                            .collect::<String>(),
                    ),
                    "submit control for {locale:?} at {width}: {question_text}"
                );
                assert!(
                    question_compact.contains(
                        &locale
                            .request_cancel_hint()
                            .chars()
                            .filter(|character| !character.is_whitespace())
                            .collect::<String>(),
                    ),
                    "cancel control for {locale:?} at {width}: {question_text}"
                );
                assert!(
                    question_compact.contains("1.Fast"),
                    "option for {locale:?} at {width}: {question_text}"
                );
            }
        }
    }

    #[test]
    fn secret_user_input_is_masked_in_the_test_backend() {
        let mut app = App::default();
        app.bottom_pane
            .enqueue(ServerRequest::ItemToolRequestUserInput {
                id: RequestId::Integer(8),
                params: ToolRequestUserInputParams {
                    thread_id: "thread-1".to_string(),
                    turn_id: "turn-1".to_string(),
                    item_id: "question-1".to_string(),
                    questions: vec![ToolRequestUserInputQuestion {
                        id: "token".to_string(),
                        header: "Token".to_string(),
                        question: "Enter token".to_string(),
                        is_other: false,
                        is_secret: true,
                        options: None,
                    }],
                    is_blocking: true,
                    auto_resolution_ms: None,
                },
            })
            .expect("queue user input");
        dispatch_connected_input(&mut app, Event::Paste("sensitive".to_string()));
        let mut terminal = Terminal::new(TestBackend::new(40, 10)).expect("terminal");

        terminal.draw(|frame| render(frame, &app)).expect("draw");

        let text = buffer_text(&terminal);
        assert!(text.contains("*********"));
        assert!(!text.contains("sensitive"));
    }
}
