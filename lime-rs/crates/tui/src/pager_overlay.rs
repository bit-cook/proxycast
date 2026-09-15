use std::cell::{Cell, RefCell};

use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;

use crate::line_truncation::truncate_line_with_ellipsis_if_overflow;
use crate::locale::Locale;
use crate::terminal_hyperlinks::{HyperlinkLine, HyperlinkParagraph};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PagerAction {
    Consumed,
    LoadOlderHistory,
    Close,
}

pub(crate) struct StatusFacts<'a> {
    pub(crate) thread_id: Option<&'a str>,
    pub(crate) model: Option<&'a str>,
    pub(crate) provider: Option<&'a str>,
    pub(crate) effort: Option<&'a str>,
    pub(crate) permissions: Option<&'a str>,
    pub(crate) cwd: &'a str,
    pub(crate) status: &'a str,
}

#[derive(Debug)]
pub(crate) struct PagerOverlay {
    title: String,
    static_lines: Option<Vec<HyperlinkLine>>,
    scroll: Cell<usize>,
    page_height: Cell<usize>,
    max_scroll: Cell<usize>,
    pinned_to_bottom: Cell<bool>,
    older_history_available: Cell<bool>,
    /// Previous transcript input used to preserve the visible logical anchor when the
    /// projection prepends history or the terminal width changes. Static pagers do not
    /// populate this cache.
    previous_transcript_lines: RefCell<Option<Vec<HyperlinkLine>>>,
    previous_content_width: Cell<u16>,
}

impl PagerOverlay {
    pub(crate) fn status(locale: Locale, facts: StatusFacts<'_>) -> Self {
        let value = |value: Option<&str>| value.unwrap_or(locale.not_set_label()).to_string();
        let status = if facts.status.is_empty() {
            locale.ready_label().to_string()
        } else {
            locale.status(facts.status)
        };
        let fields = [
            (locale.thread_label(), value(facts.thread_id)),
            (locale.model_label(), value(facts.model)),
            (locale.provider_label(), value(facts.provider)),
            (locale.effort_label(), value(facts.effort)),
            (locale.permissions_label(), value(facts.permissions)),
            (locale.cwd_label(), facts.cwd.to_string()),
            (locale.state_label(), status),
        ];
        let lines = fields
            .into_iter()
            .map(|(label, value)| {
                Line::from(vec![
                    Span::styled(
                        format!("{label}: "),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(value),
                ])
            })
            .collect();
        Self::new(locale.status_title().to_string(), lines)
    }

    pub(crate) fn new(title: String, lines: Vec<Line<'static>>) -> Self {
        Self {
            title,
            static_lines: Some(lines.into_iter().map(HyperlinkLine::new).collect()),
            scroll: Cell::new(0),
            page_height: Cell::new(1),
            max_scroll: Cell::new(0),
            pinned_to_bottom: Cell::new(false),
            older_history_available: Cell::new(false),
            previous_transcript_lines: RefCell::new(None),
            previous_content_width: Cell::new(0),
        }
    }

    pub(crate) fn transcript(locale: Locale) -> Self {
        Self {
            title: locale.transcript_title().to_string(),
            static_lines: None,
            scroll: Cell::new(usize::MAX),
            page_height: Cell::new(1),
            max_scroll: Cell::new(0),
            pinned_to_bottom: Cell::new(true),
            older_history_available: Cell::new(false),
            previous_transcript_lines: RefCell::new(None),
            previous_content_width: Cell::new(0),
        }
    }

    pub(crate) fn is_transcript(&self) -> bool {
        self.static_lines.is_none()
    }

    pub(crate) fn set_older_history_available(&self, available: bool) {
        self.older_history_available.set(available);
    }

    /// Keep the transcript at the actual beginning after Home loads all older pages.
    pub(crate) fn reset_transcript_anchor_at_top(&self) {
        if !self.is_transcript() {
            return;
        }
        self.pinned_to_bottom.set(false);
        self.scroll.set(0);
        *self.previous_transcript_lines.borrow_mut() = None;
        self.previous_content_width.set(0);
    }

    pub(crate) fn handle_event(&mut self, event: &Event) -> PagerAction {
        let Event::Key(key) = event else {
            return PagerAction::Consumed;
        };
        if key.kind != KeyEventKind::Press {
            return PagerAction::Consumed;
        }
        let load_older = self.is_transcript()
            && self.older_history_available.get()
            && matches!(key.code, KeyCode::Up | KeyCode::PageUp | KeyCode::Home);
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => return PagerAction::Close,
            KeyCode::Char(value)
                if self.is_transcript()
                    && key.modifiers.contains(KeyModifiers::CONTROL)
                    && value.eq_ignore_ascii_case(&'t') =>
            {
                return PagerAction::Close;
            }
            KeyCode::Up => {
                self.pinned_to_bottom.set(false);
                self.scroll.set(self.scroll.get().saturating_sub(1));
            }
            KeyCode::Down => self.scroll.set(self.scroll.get().saturating_add(1)),
            KeyCode::PageUp => {
                self.pinned_to_bottom.set(false);
                self.scroll.set(
                    self.scroll
                        .get()
                        .saturating_sub(self.page_height.get().max(1)),
                );
            }
            KeyCode::PageDown => {
                self.scroll.set(
                    self.scroll
                        .get()
                        .saturating_add(self.page_height.get().max(1)),
                );
            }
            KeyCode::Home => {
                self.pinned_to_bottom.set(false);
                self.scroll.set(0);
            }
            KeyCode::End => {
                self.pinned_to_bottom.set(true);
                self.scroll.set(self.max_scroll.get());
            }
            _ => return PagerAction::Consumed,
        }
        self.scroll
            .set(self.scroll.get().min(self.max_scroll.get()));
        if matches!(key.code, KeyCode::Down | KeyCode::PageDown) {
            self.pinned_to_bottom
                .set(self.scroll.get() == self.max_scroll.get());
        }
        if load_older && self.scroll.get() == 0 {
            PagerAction::LoadOlderHistory
        } else {
            PagerAction::Consumed
        }
    }

    pub(crate) fn render(
        &self,
        frame: &mut Frame<'_>,
        area: Rect,
        locale: Locale,
        transcript_lines: &[HyperlinkLine],
    ) {
        frame.render_widget(Clear, area);
        if area.width == 0 || area.height == 0 {
            return;
        }

        let header = Rect::new(area.x, area.y, area.width, 1);
        let footer_height = u16::from(area.height >= 2);
        let separator_height = u16::from(area.height >= 3);
        let content_y = area.y.saturating_add(1);
        let content_height = area
            .height
            .saturating_sub(1 + footer_height + separator_height);
        let content = Rect::new(area.x, content_y, area.width, content_height);
        let separator = Rect::new(area.x, content.bottom(), area.width, separator_height);
        let footer = Rect::new(area.x, separator.bottom(), area.width, footer_height);

        frame.render_widget(
            Paragraph::new(truncate_line_with_ellipsis_if_overflow(
                Line::styled(
                    format!("/ {} ", self.title),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                usize::from(header.width),
            )),
            header,
        );

        let lines = self.static_lines.as_deref().unwrap_or(transcript_lines);
        if self.is_transcript() {
            self.remap_transcript_anchor(lines, content.width);
        }
        let paragraph = HyperlinkParagraph::new(lines);
        let total_height = paragraph.line_count(content.width);
        let page_height = usize::from(content.height);
        let max_scroll = total_height.saturating_sub(page_height);
        let scroll = if self.pinned_to_bottom.get() {
            max_scroll
        } else {
            self.scroll.get().min(max_scroll)
        };
        self.scroll.set(scroll);
        self.page_height.set(page_height.max(1));
        self.max_scroll.set(max_scroll);
        frame.render_widget(
            paragraph.scroll(u16::try_from(scroll).unwrap_or(u16::MAX)),
            content,
        );

        let visible_rows = total_height
            .saturating_sub(scroll)
            .min(usize::from(content.height));
        for row in visible_rows..usize::from(content.height) {
            frame.render_widget(
                Paragraph::new("~").style(Style::default().fg(Color::DarkGray)),
                Rect::new(
                    content.x,
                    content
                        .y
                        .saturating_add(u16::try_from(row).unwrap_or(u16::MAX)),
                    content.width,
                    1,
                ),
            );
        }

        if separator.height > 0 {
            let percent = scroll
                .saturating_mul(100)
                .checked_div(max_scroll)
                .unwrap_or(100);
            let percentage = format!(" {percent}% ");
            let line = format!(
                "{}{}",
                "─".repeat(usize::from(area.width).saturating_sub(percentage.len())),
                percentage
            );
            frame.render_widget(
                Paragraph::new(truncate_line_with_ellipsis_if_overflow(
                    Line::styled(line, Style::default().fg(Color::DarkGray)),
                    usize::from(separator.width),
                )),
                separator,
            );
        }
        if footer.height > 0 {
            frame.render_widget(
                Paragraph::new(truncate_line_with_ellipsis_if_overflow(
                    Line::styled(
                        if self.is_transcript() {
                            locale.transcript_pager_footer()
                        } else {
                            locale.pager_footer()
                        },
                        Style::default().fg(Color::DarkGray),
                    ),
                    usize::from(footer.width),
                )),
                footer,
            );
        }
        if self.is_transcript() {
            *self.previous_transcript_lines.borrow_mut() = Some(lines.to_vec());
            self.previous_content_width.set(content.width);
        }
    }

    /// Preserve the same logical transcript row across prepended history and width reflow.
    ///
    /// `scroll` is measured in wrapped terminal rows, while the projection is a sequence of
    /// logical `HyperlinkLine`s. We first identify the old logical line and intra-line offset,
    /// then map that line through a pure prefix/suffix or unchanged-index relationship. If the
    /// user is pinned to the bottom, normal tail-following remains authoritative.
    fn remap_transcript_anchor(&self, lines: &[HyperlinkLine], width: u16) {
        if self.pinned_to_bottom.get() {
            return;
        }
        let Some(previous) = self.previous_transcript_lines.borrow().as_ref().cloned() else {
            return;
        };
        if previous.is_empty() || lines.is_empty() {
            return;
        }
        let old_width = self.previous_content_width.get();
        if old_width == 0 || width == 0 {
            return;
        }

        let old_starts = wrapped_line_starts(&previous, old_width);
        let old_scroll = self.scroll.get();
        let old_line = old_starts
            .iter()
            .enumerate()
            .rev()
            .find(|(_, start)| **start <= old_scroll)
            .map(|(index, start)| (index, old_scroll.saturating_sub(*start)))
            .unwrap_or((0, old_scroll));

        let common_prefix = previous
            .iter()
            .zip(lines)
            .take_while(|(old, new)| old == new)
            .count();
        let common_suffix = previous
            .iter()
            .rev()
            .zip(lines.iter().rev())
            .take_while(|(old, new)| old == new)
            .count()
            .min(previous.len().saturating_sub(common_prefix));
        let mapped_line = if lines.len() == previous.len()
            && (common_prefix > 0 || common_suffix > 0 || previous.len() == 1)
        {
            // Streaming updates replace the active canonical line in place. The visible text (and
            // therefore `HyperlinkLine` equality) changes, but its logical transcript identity is
            // stable, so keep the same line index while recomputing its wrapped height below.
            old_line.0
        } else if lines.len() >= previous.len()
            && lines[lines.len() - previous.len()..] == previous[..]
        {
            old_line.0 + lines.len() - previous.len()
        } else if lines.len() >= previous.len() && lines[..previous.len()] == previous[..] {
            old_line.0
        } else {
            return;
        };

        let new_starts = wrapped_line_starts(lines, width);
        let Some(new_start) = new_starts.get(mapped_line).copied() else {
            return;
        };
        let new_height = new_starts
            .get(mapped_line + 1)
            .copied()
            .unwrap_or_else(|| HyperlinkParagraph::new(&lines[mapped_line..]).line_count(width));
        self.scroll
            .set(new_start.saturating_add(old_line.1.min(new_height.saturating_sub(1))));
    }
}

fn wrapped_line_starts(lines: &[HyperlinkLine], width: u16) -> Vec<usize> {
    let mut starts = Vec::with_capacity(lines.len());
    let mut offset = 0usize;
    for line in lines {
        starts.push(offset);
        offset = offset.saturating_add(
            HyperlinkParagraph::new(std::slice::from_ref(line))
                .line_count(width)
                .max(1),
        );
    }
    starts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyEvent, KeyModifiers};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn key(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
        let buffer = terminal.backend().buffer();
        (0..buffer.area.height)
            .map(|y| {
                (0..buffer.area.width)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn static_overlay_wraps_scrolls_jumps_and_closes() {
        let mut overlay = PagerOverlay::new(
            "STATUS".to_string(),
            (0..8)
                .map(|index| {
                    Line::raw(format!(
                        "line {index}: a very long status value that wraps in a narrow terminal"
                    ))
                })
                .collect(),
        );
        let mut terminal = Terminal::new(TestBackend::new(24, 5)).expect("terminal");
        terminal
            .draw(|frame| overlay.render(frame, frame.area(), Locale::EnUs, &[]))
            .expect("draw");
        assert!(overlay.max_scroll.get() > 0);
        assert!(buffer_text(&terminal).contains("line 0: a very long"));

        assert_eq!(
            overlay.handle_event(&key(KeyCode::Down)),
            PagerAction::Consumed
        );
        assert_eq!(overlay.scroll.get(), 1);
        assert_eq!(
            overlay.handle_event(&key(KeyCode::PageDown)),
            PagerAction::Consumed
        );
        assert_eq!(overlay.scroll.get(), 3);
        assert_eq!(
            overlay.handle_event(&key(KeyCode::Up)),
            PagerAction::Consumed
        );
        assert_eq!(overlay.scroll.get(), 2);

        assert_eq!(
            overlay.handle_event(&key(KeyCode::End)),
            PagerAction::Consumed
        );
        assert_eq!(overlay.scroll.get(), overlay.max_scroll.get());
        assert_eq!(
            overlay.handle_event(&key(KeyCode::PageUp)),
            PagerAction::Consumed
        );
        assert!(overlay.scroll.get() < overlay.max_scroll.get());
        assert_eq!(
            overlay.handle_event(&key(KeyCode::Home)),
            PagerAction::Consumed
        );
        assert_eq!(overlay.scroll.get(), 0);
        assert_eq!(overlay.handle_event(&key(KeyCode::Esc)), PagerAction::Close);
    }

    #[test]
    fn resize_recomputes_scroll_bounds_and_tiny_areas_do_not_panic() {
        let overlay = PagerOverlay::new(
            "STATUS".to_string(),
            vec![Line::raw(
                "a long value that needs several rows in a narrow viewport",
            )],
        );
        let mut narrow = Terminal::new(TestBackend::new(10, 5)).expect("terminal");
        narrow
            .draw(|frame| overlay.render(frame, frame.area(), Locale::EnUs, &[]))
            .expect("narrow draw");
        assert!(overlay.max_scroll.get() > 0);
        overlay.scroll.set(overlay.max_scroll.get());

        let mut wide = Terminal::new(TestBackend::new(80, 12)).expect("terminal");
        wide.draw(|frame| overlay.render(frame, frame.area(), Locale::EnUs, &[]))
            .expect("wide draw");
        assert_eq!(overlay.max_scroll.get(), 0);
        assert_eq!(overlay.scroll.get(), 0);
        assert_eq!(overlay.page_height.get(), 9);

        let mut tiny = Terminal::new(TestBackend::new(1, 1)).expect("terminal");
        tiny.draw(|frame| {
            overlay.render(frame, Rect::new(0, 0, 0, 0), Locale::EnUs, &[]);
            overlay.render(frame, frame.area(), Locale::EnUs, &[]);
        })
        .expect("tiny draw");
    }

    #[test]
    fn status_overlay_uses_current_values_without_a_second_session_model() {
        let overlay = PagerOverlay::status(
            Locale::EnUs,
            StatusFacts {
                thread_id: Some("thread-1"),
                model: Some("gpt-5"),
                provider: Some("openai"),
                effort: Some("high"),
                permissions: Some(":workspace"),
                cwd: "/workspace",
                status: "running",
            },
        );
        let text = overlay
            .static_lines
            .as_ref()
            .expect("status lines")
            .iter()
            .flat_map(|line| &line.line.spans)
            .map(|span| span.content.as_ref())
            .collect::<String>();

        for value in [
            "thread-1",
            "gpt-5",
            "openai",
            "high",
            ":workspace",
            "/workspace",
            "running",
        ] {
            assert!(text.contains(value), "missing {value}: {text}");
        }
    }

    #[test]
    fn status_overlay_localizes_an_empty_projection_status_as_ready() {
        for locale in [
            Locale::ZhCn,
            Locale::ZhTw,
            Locale::EnUs,
            Locale::JaJp,
            Locale::KoKr,
        ] {
            let overlay = PagerOverlay::status(
                locale,
                StatusFacts {
                    thread_id: None,
                    model: None,
                    provider: None,
                    effort: None,
                    permissions: None,
                    cwd: "/workspace",
                    status: "",
                },
            );
            let text = overlay
                .static_lines
                .as_ref()
                .expect("status lines")
                .iter()
                .flat_map(|line| &line.line.spans)
                .map(|span| span.content.as_ref())
                .collect::<String>();

            assert!(text.contains(locale.ready_label()), "{locale:?}: {text}");
        }
    }

    #[test]
    fn transcript_overlay_starts_at_tail_follows_updates_and_closes_with_ctrl_t() {
        let mut overlay = PagerOverlay::transcript(Locale::EnUs);
        let initial = (0..8)
            .map(|index| HyperlinkLine::from(format!("line {index}")))
            .collect::<Vec<_>>();
        let mut terminal = Terminal::new(TestBackend::new(24, 6)).expect("terminal");
        terminal
            .draw(|frame| overlay.render(frame, frame.area(), Locale::EnUs, &initial))
            .expect("initial draw");
        assert_eq!(overlay.scroll.get(), overlay.max_scroll.get());
        assert!(buffer_text(&terminal).contains("line 7"));

        let updated = (0..10)
            .map(|index| HyperlinkLine::from(format!("line {index}")))
            .collect::<Vec<_>>();
        terminal
            .draw(|frame| overlay.render(frame, frame.area(), Locale::EnUs, &updated))
            .expect("updated draw");
        assert_eq!(overlay.scroll.get(), overlay.max_scroll.get());
        assert!(buffer_text(&terminal).contains("line 9"));

        assert_eq!(
            overlay.handle_event(&Event::Key(KeyEvent::new(
                KeyCode::Char('t'),
                KeyModifiers::CONTROL,
            ))),
            PagerAction::Close
        );
    }

    #[test]
    fn transcript_overlay_requests_older_history_when_scrolled_to_the_top() {
        let mut overlay = PagerOverlay::transcript(Locale::EnUs);
        overlay.set_older_history_available(true);
        let lines = (0..12)
            .map(|index| HyperlinkLine::from(format!("line {index}")))
            .collect::<Vec<_>>();
        let mut terminal = Terminal::new(TestBackend::new(24, 6)).expect("terminal");
        terminal
            .draw(|frame| overlay.render(frame, frame.area(), Locale::EnUs, &lines))
            .expect("initial draw");

        assert_eq!(
            overlay.handle_event(&key(KeyCode::Home)),
            PagerAction::LoadOlderHistory
        );
        assert_eq!(overlay.scroll.get(), 0);
        assert!(!overlay.pinned_to_bottom.get());
        assert_eq!(
            overlay.handle_event(&key(KeyCode::PageUp)),
            PagerAction::LoadOlderHistory
        );
    }

    #[test]
    fn static_overlay_never_requests_older_history() {
        let mut overlay = PagerOverlay::new("STATUS".to_string(), vec![Line::raw("line")]);
        assert_eq!(
            overlay.handle_event(&key(KeyCode::Home)),
            PagerAction::Consumed
        );
    }

    #[test]
    fn transcript_overlay_without_older_history_consumes_top_navigation() {
        let mut overlay = PagerOverlay::transcript(Locale::EnUs);
        assert_eq!(
            overlay.handle_event(&key(KeyCode::Home)),
            PagerAction::Consumed
        );
    }

    #[test]
    fn transcript_overlay_reset_anchor_keeps_newly_loaded_beginning_visible() {
        let overlay = PagerOverlay::transcript(Locale::EnUs);
        let old_lines = vec![HyperlinkLine::from("oldest loaded")];
        let mut terminal = Terminal::new(TestBackend::new(24, 6)).expect("terminal");
        terminal
            .draw(|frame| overlay.render(frame, frame.area(), Locale::EnUs, &old_lines))
            .expect("initial draw");

        overlay.reset_transcript_anchor_at_top();
        let lines = vec![
            HyperlinkLine::from("actual oldest"),
            HyperlinkLine::from("oldest loaded"),
        ];
        terminal
            .draw(|frame| overlay.render(frame, frame.area(), Locale::EnUs, &lines))
            .expect("expanded draw");

        assert_eq!(overlay.scroll.get(), 0);
    }

    #[test]
    fn transcript_overlay_preserves_manual_scroll_when_projection_grows() {
        let mut overlay = PagerOverlay::transcript(Locale::EnUs);
        let initial = (0..10)
            .map(|index| HyperlinkLine::from(format!("line {index}")))
            .collect::<Vec<_>>();
        let mut terminal = Terminal::new(TestBackend::new(24, 6)).expect("terminal");
        terminal
            .draw(|frame| overlay.render(frame, frame.area(), Locale::EnUs, &initial))
            .expect("initial draw");
        overlay.handle_event(&key(KeyCode::Up));
        let manual_scroll = overlay.scroll.get();

        let updated = (0..12)
            .map(|index| HyperlinkLine::from(format!("line {index}")))
            .collect::<Vec<_>>();
        terminal
            .draw(|frame| overlay.render(frame, frame.area(), Locale::EnUs, &updated))
            .expect("updated draw");

        assert_eq!(overlay.scroll.get(), manual_scroll);
        assert!(!overlay.pinned_to_bottom.get());
    }

    #[test]
    fn transcript_overlay_preserves_manual_anchor_when_history_is_prepended() {
        let mut overlay = PagerOverlay::transcript(Locale::EnUs);
        let initial = (0..20)
            .map(|index| HyperlinkLine::from(format!("line {index}")))
            .collect::<Vec<_>>();
        let mut terminal = Terminal::new(TestBackend::new(24, 6)).expect("terminal");
        terminal
            .draw(|frame| overlay.render(frame, frame.area(), Locale::EnUs, &initial))
            .expect("initial draw");
        overlay.handle_event(&key(KeyCode::PageUp));
        let previous_scroll = overlay.scroll.get();
        assert!(!overlay.pinned_to_bottom.get());

        let mut updated = (0..3)
            .map(|index| HyperlinkLine::from(format!("older {index}")))
            .collect::<Vec<_>>();
        updated.extend(initial);
        terminal
            .draw(|frame| overlay.render(frame, frame.area(), Locale::EnUs, &updated))
            .expect("prepended draw");

        assert_eq!(overlay.scroll.get(), previous_scroll + 3);
        assert!(!overlay.pinned_to_bottom.get());
    }

    #[test]
    fn transcript_overlay_remaps_manual_anchor_when_width_reflows() {
        let overlay = PagerOverlay::transcript(Locale::EnUs);
        let mut lines = vec![
            HyperlinkLine::from("a long first transcript line that wraps after resize"),
            HyperlinkLine::from("anchor line"),
        ];
        lines.extend((0..8).map(|index| HyperlinkLine::from(format!("tail {index}"))));
        let mut wide = Terminal::new(TestBackend::new(48, 6)).expect("wide terminal");
        wide.draw(|frame| overlay.render(frame, frame.area(), Locale::EnUs, &lines))
            .expect("wide draw");
        overlay.scroll.set(wrapped_line_starts(&lines, 48)[1]);
        overlay.pinned_to_bottom.set(false);

        let mut narrow = Terminal::new(TestBackend::new(16, 6)).expect("narrow terminal");
        narrow
            .draw(|frame| overlay.render(frame, frame.area(), Locale::EnUs, &lines))
            .expect("narrow draw");

        let expected = wrapped_line_starts(&lines, 16)[1];
        assert!(expected > 1);
        assert_eq!(overlay.scroll.get(), expected);
        assert!(!overlay.pinned_to_bottom.get());
    }

    #[test]
    fn transcript_overlay_remaps_manual_anchor_when_streaming_line_grows_in_place() {
        let overlay = PagerOverlay::transcript(Locale::EnUs);
        let initial = vec![
            HyperlinkLine::from("stable header"),
            HyperlinkLine::from("anchor line"),
            HyperlinkLine::from("tail line"),
        ];
        let mut terminal = Terminal::new(TestBackend::new(18, 4)).expect("terminal");
        terminal
            .draw(|frame| overlay.render(frame, frame.area(), Locale::EnUs, &initial))
            .expect("initial draw");
        overlay.pinned_to_bottom.set(false);
        overlay.scroll.set(wrapped_line_starts(&initial, 18)[1]);

        let updated = vec![
            HyperlinkLine::from("stable header that grew while streaming"),
            HyperlinkLine::from("anchor line"),
            HyperlinkLine::from("tail line"),
        ];
        terminal
            .draw(|frame| overlay.render(frame, frame.area(), Locale::EnUs, &updated))
            .expect("streaming redraw");

        assert_eq!(
            overlay.scroll.get(),
            wrapped_line_starts(&updated, 18)[1],
            "the same logical anchor should remain visible after an in-place stream update"
        );
        assert!(!overlay.pinned_to_bottom.get());
    }
}
