//! Shared metadata for compact activity previews.
//!
//! Disclosure controls are presentation-only. The canonical transcript keeps the full source
//! text; this enum only describes which retained details are hidden by the compact view.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ActivityDisclosure {
    /// The compact presentation hides details without a stable line count.
    Generic,
    /// Retained output lines hidden by the compact presentation.
    ///
    /// Storage-truncated lines are intentionally excluded from this count. They are represented by
    /// the output renderer's own omission marker instead of pretending they can be expanded.
    OutputLines(usize),
}
