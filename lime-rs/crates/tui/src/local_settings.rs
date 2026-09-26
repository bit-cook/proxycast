//! Client-owned settings resolved from the App Server user configuration layer.
//!
//! The TUI never reads config files directly. `config/read` is the only load boundary, and the
//! resolved runtime keymap remains immutable for the lifetime of one TUI process.

use anyhow::{Context, Result};
use lime_core::config::TuiConfig;
use serde_json::Value;

use crate::app_server_session::AppServerSession;
use crate::keymap::RuntimeKeymap;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct LocalSettings {
    pub(crate) keymap: RuntimeKeymap,
}

impl LocalSettings {
    pub(crate) async fn read(session: &AppServerSession) -> Result<Self> {
        let response = session.read_config().await?;
        Self::from_config_value(&response.config)
    }

    fn from_config_value(config: &Value) -> Result<Self> {
        let tui = config.get("tui").cloned().unwrap_or_else(|| Value::Object(Default::default()));
        let tui: TuiConfig = serde_json::from_value(tui)
            .context("App Server config/read returned an invalid `tui` configuration")?;
        let keymap = RuntimeKeymap::from_config(&tui.keymap)
            .map_err(anyhow::Error::msg)
            .context("invalid TUI keymap")?;
        Ok(Self { keymap })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keymap::{GlobalKeymapAction, KeyChordMatcher, KeymapMatch};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use serde_json::json;

    #[test]
    fn config_read_value_resolves_a_runtime_snapshot() {
        let settings = LocalSettings::from_config_value(&json!({
            "tui": {
                "keymap": {
                    "global": {"find_transcript": "ctrl-x f"}
                }
            }
        }))
        .expect("valid local settings");
        let mut matcher = KeyChordMatcher::default();
        assert_eq!(
            settings.keymap.transcript().dispatch_global(
                &mut matcher,
                KeyEvent::new(KeyCode::Char('x'), KeyModifiers::CONTROL),
            ),
            KeymapMatch::Pending
        );
        assert_eq!(
            settings.keymap.transcript().dispatch_global(
                &mut matcher,
                KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE),
            ),
            KeymapMatch::Completed(GlobalKeymapAction::FindTranscript)
        );
    }

    #[test]
    fn config_read_value_rejects_ambiguous_runtime_bindings() {
        let error = LocalSettings::from_config_value(&json!({
            "tui": {
                "keymap": {
                    "global": {
                        "open_transcript": "f6",
                        "find_transcript": "f6"
                    }
                }
            }
        }))
        .expect_err("conflict must fail closed");
        assert!(error.to_string().contains("invalid TUI keymap"));
    }
}
