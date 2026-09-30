//! Single-line suggestion rows keep overflow hints outside selectable content.

use super::scroll_state::ScrollState;
use super::selection_popup_common;
use super::selection_row_layout::SelectionRow;
use ratatui::layout::Rect;
use ratatui::text::Line;
use ratatui::widgets::{Clear, Paragraph};
use ratatui::Frame;

pub(super) fn render_rows_single_line(
    frame: &mut Frame<'_>,
    area: Rect,
    rows: &[SelectionRow],
    state: &ScrollState,
    empty: &str,
) {
    frame.render_widget(Clear, area);
    let padding = u16::from(area.height >= 3);
    let body = Rect::new(
        area.x,
        area.y + padding,
        area.width,
        area.height.saturating_sub(padding * 2),
    );
    let rendered = selection_popup_common::render_rows_single_line(frame, body, rows, state, empty);
    if padding > 0 && area.width > 0 {
        for (visible, marker, y) in [
            (rendered.has_above, "↑", area.y),
            (rendered.has_below, "↓", area.bottom() - 1),
        ] {
            if visible {
                frame.render_widget(
                    Paragraph::new(Line::from(marker)),
                    Rect::new(area.x, y, 1, 1),
                );
            }
        }
    }
}
