//! Compatibility re-exports for the Codex-shaped `render::highlight` owner.
//!
//! New terminal renderers should import from `crate::render::highlight`.

pub(crate) use crate::render::highlight::{
    exceeds_highlight_limits, highlight_code_to_lines, CodeLineHighlighter,
};

#[cfg(test)]
pub(crate) use crate::render::highlight::highlight_code_to_styled_spans;
