//! Canonical working-directory synchronization for startup and session transitions.
//!
//! The App Server owns the persisted thread cwd. This owner only applies the server value to the
//! TUI projection after comparing normalized paths; it does not implement `/cd`, trust prompts,
//! or a second cwd preference store.

use std::path::{Path, PathBuf};

use super::App;

/// Apply a server-reported cwd when it differs from the local TUI cwd.
///
/// Returns `true` when the app cwd changed. Existing paths are compared after canonicalization,
/// while missing paths use lexical normalization through [`crate::session_resume::cwds_differ`].
pub(crate) fn sync_server_cwd(app: &mut App, server_cwd: impl AsRef<Path>) -> bool {
    let server_cwd = server_cwd.as_ref();
    if !crate::session_resume::cwds_differ(&app.cwd, server_cwd) {
        return false;
    }
    app.set_cwd(PathBuf::from(server_cwd));
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_server_cwd_updates_only_for_a_different_normalized_path() {
        let mut app = App::default();
        app.set_cwd(PathBuf::from("/workspace/project/../project"));

        assert!(!sync_server_cwd(&mut app, "/workspace/project"));
        assert_eq!(app.cwd, PathBuf::from("/workspace/project/../project"));

        assert!(sync_server_cwd(&mut app, "/workspace/other"));
        assert_eq!(app.cwd, PathBuf::from("/workspace/other"));
    }
}
