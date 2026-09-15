use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;
use std::time::Instant;

use super::approval_overlay::ApprovalRequest;
use super::mcp_server_elicitation;
use super::request_user_input::render as request_user_input_render;
use super::selection_row_layout::{visible_item_window, MAX_POPUP_ROWS};
use super::{BottomPane, PendingInteraction};
use crate::locale::Locale;
use crate::style::{accent_style, attention_style, muted_style};
use crate::wrapping::{word_wrap_line, RtOptions};

pub(crate) fn desired_height_with_locale_for_width(
    pane: &BottomPane,
    locale: Locale,
    width: u16,
) -> u16 {
    // The interaction surface only has top/bottom borders, so its text width is the full
    // terminal width. Measuring with a narrower width would under-allocate the pane and clip
    // wrapped CJK/emoji content on narrow terminals.
    let content_width = width.max(1);
    let content = lines_with_locale(pane, locale, usize::from(content_width), Instant::now());
    let lines = Paragraph::new(content)
        .wrap(Wrap { trim: false })
        .line_count(content_width)
        .saturating_add(2);
    u16::try_from(lines).unwrap_or(u16::MAX).clamp(5, 18)
}

pub(crate) fn render_with_locale(
    frame: &mut Frame<'_>,
    area: Rect,
    pane: &BottomPane,
    locale: Locale,
) {
    let block = Block::default()
        .borders(Borders::TOP | Borders::BOTTOM)
        .border_style(attention_style());
    let inner = block.inner(area);
    let content = lines_with_locale(pane, locale, inner.width as usize, Instant::now());
    frame.render_widget(
        Paragraph::new(content.clone())
            .block(block)
            .wrap(Wrap { trim: false }),
        area,
    );

    match pane.current() {
        Some(PendingInteraction::UserInput(request)) => {
            request_user_input_render::set_cursor_position(frame, inner, request, &content);
        }
        Some(PendingInteraction::McpElicitation(request)) => {
            mcp_server_elicitation::set_cursor_position(frame, inner, request, &content);
        }
        _ => {}
    }
}

fn lines_with_locale(
    pane: &BottomPane,
    locale: Locale,
    width: usize,
    now: Instant,
) -> Vec<Line<'static>> {
    match pane.current() {
        Some(PendingInteraction::Approval(approval)) => {
            let (kind, details) = match &approval.request {
                ApprovalRequest::Exec { params, .. } => (
                    "command",
                    vec![
                        params.command.clone().unwrap_or_default(),
                        params.cwd.clone().unwrap_or_default(),
                        params.reason.clone().unwrap_or_default(),
                    ],
                ),
                ApprovalRequest::ApplyPatch { params, .. } => (
                    "file",
                    vec![
                        params.grant_root.clone().unwrap_or_default(),
                        params.reason.clone().unwrap_or_default(),
                    ],
                ),
                ApprovalRequest::Permissions { params, .. } => (
                    "permissions",
                    vec![
                        params.cwd.clone(),
                        params.reason.clone().unwrap_or_default(),
                        serde_json::to_string(&params.permissions).unwrap_or_default(),
                    ],
                ),
            };
            let mut lines = vec![Line::styled(
                locale.approval_title(kind),
                Style::default().add_modifier(Modifier::BOLD),
            )];
            lines.extend(
                details
                    .into_iter()
                    .filter(|detail| !detail.is_empty())
                    .map(|detail| Line::styled(detail, muted_style())),
            );
            let labels = approval.option_labels();
            let (start, end) = visible_item_window(approval.selected, labels.len(), MAX_POPUP_ROWS);
            lines.extend(
                labels
                    .into_iter()
                    .enumerate()
                    .skip(start)
                    .take(end.saturating_sub(start))
                    .map(|(index, label)| {
                        option_line(
                            index == approval.selected,
                            format!("{}. {}", index + 1, locale.approval_option(&label)),
                        )
                    }),
            );
            if width == usize::MAX {
                lines
            } else {
                lines
                    .into_iter()
                    .flat_map(|line| {
                        word_wrap_line(&line, RtOptions::new(width.max(1)))
                            .into_iter()
                            .map(|wrapped| {
                                let style = wrapped.style;
                                Line::from(
                                    wrapped
                                        .spans
                                        .into_iter()
                                        .map(|span| {
                                            ratatui::text::Span::styled(
                                                span.content.into_owned(),
                                                span.style,
                                            )
                                        })
                                        .collect::<Vec<_>>(),
                                )
                                .style(style)
                            })
                            .collect::<Vec<_>>()
                    })
                    .collect()
            }
        }
        Some(PendingInteraction::UserInput(request)) => {
            request_user_input_render::lines_with_locale_with_width_at(request, locale, width, now)
        }
        Some(PendingInteraction::McpElicitation(request)) => {
            mcp_server_elicitation::lines_with_locale_with_width(request, locale, width)
        }
        None => Vec::new(),
    }
}

fn option_line(selected: bool, label: String) -> Line<'static> {
    let prefix = if selected { "› " } else { "  " };
    let style = if selected {
        accent_style()
    } else {
        Style::default()
    };
    Line::styled(format!("{prefix}{label}"), style)
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_server_protocol::protocol::v2::{
        ServerRequest, ToolRequestUserInputOption, ToolRequestUserInputParams,
        ToolRequestUserInputQuestion,
    };
    use app_server_protocol::RequestId;
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

    fn request_with_options(count: usize) -> ServerRequest {
        ServerRequest::ItemToolRequestUserInput {
            id: RequestId::Integer(31),
            params: ToolRequestUserInputParams {
                thread_id: "thread-1".to_string(),
                turn_id: "turn-1".to_string(),
                item_id: "question-1".to_string(),
                questions: vec![ToolRequestUserInputQuestion {
                    id: "choice".to_string(),
                    header: "Next step".to_string(),
                    question: "Choose the next step for this task".to_string(),
                    is_other: false,
                    is_secret: false,
                    options: Some(
                        (0..count)
                            .map(|index| ToolRequestUserInputOption {
                                label: format!("Choice {index}"),
                                description: "A deliberately long description for narrow layout"
                                    .to_string(),
                            })
                            .collect(),
                    ),
                }],
                is_blocking: true,
                auto_resolution_ms: None,
            },
        }
    }

    #[test]
    fn long_request_input_keeps_the_selected_option_inside_the_eight_row_window() {
        let mut pane = BottomPane::default();
        pane.enqueue(request_with_options(12))
            .expect("queue request user input");
        for _ in 0..11 {
            pane.handle_event(Event::Key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)));
        }

        let lines = lines_with_locale(&pane, Locale::EnUs, 80, Instant::now());
        let option_lines = lines
            .iter()
            .filter(|line| line.to_string().contains(". Choice "))
            .collect::<Vec<_>>();
        assert_eq!(option_lines.len(), MAX_POPUP_ROWS);
        assert!(lines
            .iter()
            .any(|line| line.to_string().contains("› 12. Choice 11")));
        assert!(!lines
            .iter()
            .any(|line| line.to_string().contains("1. Choice 0")));
    }

    #[test]
    fn narrow_height_uses_the_same_width_as_rendered_content() {
        let mut pane = BottomPane::default();
        pane.enqueue(request_with_options(2))
            .expect("queue request user input");
        let width = 12u16;
        let content = lines_with_locale(&pane, Locale::EnUs, usize::from(width), Instant::now());
        let expected = Paragraph::new(content)
            .wrap(Wrap { trim: false })
            .line_count(width)
            .saturating_add(2)
            .clamp(5, 18);

        assert_eq!(
            desired_height_with_locale_for_width(&pane, Locale::EnUs, width),
            u16::try_from(expected).expect("height fits in u16")
        );
    }
}
