//! Interaction state for the full-screen transcript projection.
//!
//! The canonical transcript remains owned by `ConversationProjection`. This module only freezes
//! the currently rendered `HyperlinkLine` projection for the lifetime of a local selection.

mod bookmark;
mod composer_gap;
mod disclosure;
mod follow_control;
mod footer;
mod input;
mod prompt_header;
mod search;
mod selection;

pub(crate) use bookmark::{TranscriptAnchorRange, TranscriptBookmark, TranscriptFrame};
pub(crate) use composer_gap::TranscriptComposerGap;
pub(crate) use disclosure::{TranscriptContent, TranscriptDisclosure};
pub(crate) use follow_control::{TranscriptFollowAction, TranscriptFollowControl};
pub(crate) use footer::{search_status, TranscriptFooter};
pub(crate) use input::TranscriptSelectionAction;
pub(crate) use prompt_header::{PromptHeaderSource, TranscriptPromptHeader};
#[cfg(test)]
pub(crate) use search::find_literal_ranges;
pub(crate) use search::{SearchAction, SearchBoundary, SearchHistoryState, TranscriptSearch};
pub(crate) use selection::TranscriptSelection;
