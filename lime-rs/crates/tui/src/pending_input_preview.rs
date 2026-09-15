//! Compatibility re-export for the moved pending-input preview owner.
//!
//! New consumers must import `bottom_pane::pending_input_preview`; this module
//! remains only as a migration boundary for historical integrations.

#[allow(unused_imports)]
pub(crate) use crate::bottom_pane::pending_input_preview::{
    can_restore_submission, desired_height, render,
};
