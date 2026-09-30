//! Shared composer geometry for rendering, measurement and cursor placement.
//!
//! Keeping the attachment rows and prompt gutter in one owner prevents the screen splitter and
//! renderer from disagreeing on narrow terminals.  The layout is presentation-only; draft and
//! attachment state remain owned by `ChatComposer`.

use ratatui::layout::Rect;

use super::ChatComposer;

pub(crate) const PROMPT_GUTTER_COLS: u16 = 2;
pub(crate) const COMPOSER_TOP_ROWS: u16 = 1;
pub(crate) const COMPOSER_BOTTOM_ROWS: u16 = 0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ComposerLayout {
    pub(crate) inner: Rect,
    pub(crate) attachments: Rect,
    pub(crate) textarea: Rect,
}

impl ComposerLayout {
    pub(crate) fn for_area(area: Rect, attachment_rows: usize) -> Self {
        let inner = Rect {
            y: area.y.saturating_add(COMPOSER_TOP_ROWS),
            height: area.height.saturating_sub(COMPOSER_TOP_ROWS),
            ..area
        };
        if inner.is_empty() {
            return Self {
                inner,
                ..Self::default()
            };
        }

        let attachment_height = u16::try_from(attachment_rows)
            .unwrap_or(u16::MAX)
            .min(inner.height);
        let attachments = Rect::new(inner.x, inner.y, inner.width, attachment_height);
        let textarea = Rect::new(
            inner.x.saturating_add(PROMPT_GUTTER_COLS.min(inner.width)),
            inner.y.saturating_add(attachment_height),
            inner.width.saturating_sub(PROMPT_GUTTER_COLS),
            inner.height.saturating_sub(attachment_height),
        );
        Self {
            inner,
            attachments,
            textarea,
        }
    }
}

impl ChatComposer {
    pub(crate) fn layout(&self, area: Rect) -> ComposerLayout {
        ComposerLayout::for_area(area, self.pending_image_count())
    }

    /// Measure the complete composer using the same attachment rows and prompt gutter used by
    /// `render_composer`.
    pub(crate) fn desired_height_for_width(&self, width: u16) -> u16 {
        self.desired_height(width.saturating_sub(PROMPT_GUTTER_COLS))
            .saturating_add(u16::try_from(self.pending_image_count()).unwrap_or(u16::MAX))
            .saturating_add(COMPOSER_TOP_ROWS + COMPOSER_BOTTOM_ROWS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::layout::Rect;

    #[test]
    fn layout_reserves_attachment_rows_before_textarea() {
        let layout = ComposerLayout::for_area(Rect::new(3, 4, 20, 8), 2);

        assert_eq!(layout.inner, Rect::new(3, 5, 20, 7));
        assert_eq!(layout.attachments, Rect::new(3, 5, 20, 2));
        assert_eq!(layout.textarea, Rect::new(5, 7, 18, 5));
    }

    #[test]
    fn layout_clamps_attachment_rows_and_prompt_gutter_on_narrow_area() {
        let layout = ComposerLayout::for_area(Rect::new(0, 0, 1, 1), usize::MAX);

        assert!(layout.inner.is_empty());
        assert!(layout.attachments.is_empty());
        assert!(layout.textarea.is_empty());
    }

    #[test]
    fn layout_keeps_textarea_width_zero_when_only_prompt_gutter_fits() {
        let layout = ComposerLayout::for_area(Rect::new(0, 0, 2, 4), 0);

        assert_eq!(layout.textarea.width, 0);
        assert_eq!(layout.textarea.height, 3);
    }
}
