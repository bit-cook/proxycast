use ratatui::layout::{Position, Rect};
use ratatui::style::Color;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Wrap};
use ratatui::Frame;
use std::time::Instant;
use unicode_segmentation::UnicodeSegmentation;

use crate::bottom_pane::selection_row_layout::{visible_item_window, MAX_POPUP_ROWS};
use crate::line_truncation::line_width;
use crate::locale::Locale;
use crate::style::{accent_style, muted_style};
use crate::width::display_width;
use crate::wrapping::{word_wrap_line, RtOptions};

use super::RequestUserInputOverlay;

#[allow(dead_code)]
pub(in crate::bottom_pane) fn lines_with_locale_with_width(
    request: &RequestUserInputOverlay,
    locale: Locale,
    width: usize,
) -> Vec<Line<'static>> {
    lines_with_locale_with_width_at(request, locale, width, Instant::now())
}

pub(in crate::bottom_pane) fn lines_with_locale_with_width_at(
    request: &RequestUserInputOverlay,
    locale: Locale,
    width: usize,
    now: Instant,
) -> Vec<Line<'static>> {
    let lines = lines_with_locale_unbounded(request, locale, now);
    if width == usize::MAX {
        lines
    } else {
        wrap_request_lines(lines, width.max(1), request.editing)
    }
}

fn own_line(line: Line<'_>) -> Line<'static> {
    let style = line.style;
    let spans = line
        .spans
        .into_iter()
        .map(|span| Span::styled(span.content.into_owned(), span.style))
        .collect::<Vec<_>>();
    Line::from(spans).style(style)
}

/// Wrap prompt/option/footer lines while keeping the editable value on one predictable row.
///
/// The bottom pane's height is derived from this same projection, so a long Chinese or
/// Japanese label cannot push the footer (and therefore the cancel action) out of view.
fn wrap_request_lines(
    lines: Vec<Line<'static>>,
    width: usize,
    editing: bool,
) -> Vec<Line<'static>> {
    let input_index = editing.then(|| lines.len().saturating_sub(1));
    let mut wrapped = Vec::new();
    for (index, line) in lines.into_iter().enumerate() {
        if Some(index) == input_index {
            wrapped.push(truncate_line_word_boundary_with_ellipsis(line, width));
            continue;
        }
        // Keep each option on one row so a long description cannot consume the vertical
        // budget and hide the selected action. The label remains visible; only the secondary
        // description is ellipsized at narrow widths.
        if is_option_line(&line) {
            wrapped.push(truncate_line_word_boundary_with_ellipsis(line, width));
            continue;
        }
        let options = RtOptions::new(width).break_words(true);
        wrapped.extend(word_wrap_line(&line, options).into_iter().map(own_line));
    }
    wrapped
}

fn is_option_line(line: &Line<'_>) -> bool {
    let text = line.to_string();
    let text = text.trim_start_matches(['›', ' ']);
    text.as_bytes().first().is_some_and(u8::is_ascii_digit) && text.contains(". ")
}

fn lines_with_locale_unbounded(
    request: &RequestUserInputOverlay,
    locale: Locale,
    now: Instant,
) -> Vec<Line<'static>> {
    let Some(question) = request.params.questions.get(request.question_index) else {
        return vec![Line::from(locale.no_questions())];
    };
    let mut lines = Vec::new();
    if let Some(countdown) = request.auto_resolution_countdown_text(now, locale) {
        lines.push(Line::styled(countdown, Style::default().fg(Color::Red)));
    }
    lines.extend([
        Line::styled(
            format!(
                "{} ({}/{})",
                question.header,
                request.question_index + 1,
                request.params.questions.len()
            ),
            Style::default().add_modifier(Modifier::BOLD),
        ),
        Line::from(question.question.clone()),
    ]);
    if let Some(options) = question
        .options
        .as_ref()
        .filter(|options| !options.is_empty())
    {
        let total = options.len() + usize::from(question.is_other);
        let (start, end) = visible_item_window(request.selected, total, MAX_POPUP_ROWS);
        for index in start..end {
            if let Some(option) = options.get(index) {
                lines.push(option_line(
                    !request.editing && index == request.selected,
                    format!("{}. {}  {}", index + 1, option.label, option.description),
                ));
            } else if question.is_other && index == options.len() {
                lines.push(option_line(
                    !request.editing && request.selected == options.len(),
                    format!("{}. {}", options.len() + 1, locale.other_option()),
                ));
            }
        }
    }
    if request.editing {
        let value = if question.is_secret {
            "*".repeat(request.composer.text().chars().count())
        } else {
            request.composer.text().to_string()
        };
        lines.push(Line::from(vec![
            Span::styled("› ", accent_style()),
            Span::raw(value),
        ]));
    } else if question.options.is_some() {
        lines.push(Line::styled(locale.add_notes(), muted_style()));
    }
    lines
}

pub(in crate::bottom_pane) fn set_cursor_position(
    frame: &mut Frame<'_>,
    inner: Rect,
    request: &RequestUserInputOverlay,
    content: &[Line<'static>],
) {
    if !request.editing || inner.width == 0 || inner.height == 0 {
        return;
    }
    let value = &request.composer.text()[..request.composer.cursor()];
    let is_secret = request
        .params
        .questions
        .get(request.question_index)
        .is_some_and(|question| question.is_secret);
    let display_value = if is_secret {
        "*".repeat(value.chars().count())
    } else {
        value.to_string()
    };
    let input_prefix = format!("› {display_value}");
    let input_rows_before_cursor = Paragraph::new(Line::from(input_prefix))
        .wrap(Wrap { trim: false })
        .line_count(inner.width.max(1));
    let input_line_index = content.len().saturating_sub(1);
    let preceding_rows = Paragraph::new(content[..input_line_index].to_vec())
        .wrap(Wrap { trim: false })
        .line_count(inner.width.max(1));
    let value_width = request
        .params
        .questions
        .get(request.question_index)
        .filter(|question| question.is_secret)
        .map_or_else(|| display_width(value), |_| value.chars().count());
    let current_line = value.rsplit('\n').next().unwrap_or(value);
    let current_line_width = if is_secret {
        current_line.chars().count()
    } else {
        display_width(current_line)
    };
    let prefix_width = usize::from(!value.contains('\n')) * 2;
    let x = u16::try_from(if value.contains('\n') {
        current_line_width
    } else {
        value_width
    })
    .unwrap_or(u16::MAX)
    .saturating_add(prefix_width as u16)
    .min(inner.width.saturating_sub(1));
    let input_row = preceding_rows.saturating_add(input_rows_before_cursor.saturating_sub(1));
    let y = u16::try_from(input_row)
        .unwrap_or(u16::MAX)
        .min(inner.height.saturating_sub(1));
    frame.set_cursor_position(Position::new(
        inner.x.saturating_add(x),
        inner.y.saturating_add(y),
    ));
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

/// Truncate a styled line at a grapheme-safe word boundary and append an ellipsis.
///
/// The available width reserves one cell for the ellipsis. Whitespace is preferred as the
/// break point, while a grapheme boundary is used when no word boundary fits.
pub(super) fn truncate_line_word_boundary_with_ellipsis(
    line: Line<'static>,
    max_width: usize,
) -> Line<'static> {
    if max_width == 0 {
        return Line::from(Vec::<Span<'static>>::new());
    }

    if line_width(&line) <= max_width {
        return line;
    }

    let ellipsis = "…";
    let ellipsis_width = display_width(ellipsis);
    if ellipsis_width >= max_width {
        return Line::from(ellipsis);
    }
    let limit = max_width.saturating_sub(ellipsis_width);

    #[derive(Clone, Copy)]
    struct BreakPoint {
        span_idx: usize,
        byte_end: usize,
    }

    let mut used = 0usize;
    let mut last_fit = None;
    let mut last_word_break = None;
    let mut overflowed = false;

    'outer: for (span_idx, span) in line.spans.iter().enumerate() {
        for (byte_idx, grapheme) in span.content.as_ref().grapheme_indices(true) {
            let grapheme_width = display_width(grapheme);
            if used.saturating_add(grapheme_width) > limit {
                overflowed = true;
                break 'outer;
            }
            used = used.saturating_add(grapheme_width);
            let break_point = BreakPoint {
                span_idx,
                byte_end: byte_idx + grapheme.len(),
            };
            last_fit = Some(break_point);
            if grapheme.chars().all(char::is_whitespace) {
                last_word_break = Some(break_point);
            }
        }
    }

    if !overflowed {
        return line;
    }

    let Some(chosen_break) = last_word_break.or(last_fit) else {
        return Line::from(ellipsis);
    };

    let line_style = line.style;
    let mut spans_out = Vec::new();
    for (idx, span) in line.spans.into_iter().enumerate() {
        if idx < chosen_break.span_idx {
            spans_out.push(span);
            continue;
        }
        if idx == chosen_break.span_idx {
            let text = span.content.into_owned();
            let truncated = text[..chosen_break.byte_end].to_string();
            if !truncated.is_empty() {
                spans_out.push(Span::styled(truncated, span.style));
            }
        }
        break;
    }

    while let Some(last) = spans_out.last_mut() {
        let trimmed = last
            .content
            .trim_end_matches(char::is_whitespace)
            .to_string();
        if trimmed.is_empty() {
            spans_out.pop();
        } else {
            last.content = trimmed.into();
            break;
        }
    }

    let ellipsis_style = spans_out
        .last()
        .map(|span| span.style)
        .unwrap_or(line_style);
    spans_out.push(Span::styled(ellipsis, ellipsis_style));
    Line::from(spans_out).style(line_style)
}

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
