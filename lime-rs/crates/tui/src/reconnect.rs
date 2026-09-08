//! Compatibility export for callers that still import the old top-level path.
//!
//! The implementation owner is `app::reconnect`, matching Codex. New call sites must import from
//! that module; this file only delegates while the public crate surface is migrated.

#[allow(unused_imports)]
pub(crate) use crate::app::reconnect::{reconnect_session, ReconnectedSession};
