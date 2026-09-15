//! Compatibility re-export for the moved bottom-pane command popup owner.
//!
//! New consumers must import `bottom_pane::command_popup`; this module remains
//! only until all downstream test and integration surfaces have converged.

#[allow(unused_imports)]
pub(crate) use crate::bottom_pane::command_popup::{render, CommandPopup, CommandPopupAction};
