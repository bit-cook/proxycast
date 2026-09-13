//! Bounded display facts for CUA-backed MCP calls.
//!
//! Codex groups adjacent computer calls into a dedicated history cell. Lime's canonical
//! projection currently preserves one `ThreadItem` per MCP call, so this owner only derives the
//! facts that are provable from that item. It deliberately does not decode media or aggregate
//! calls across item boundaries.

use app_server_protocol::protocol::v2::{McpToolCallError, McpToolCallResult};
use serde_json::Value;

use super::compact_text;

const COMPUTER_SERVER: &str = "cua_repl";

pub(crate) fn is_computer_activity(server: &str) -> bool {
    server == COMPUTER_SERVER
}

pub(crate) fn summary(
    arguments: &Value,
    result: Option<&McpToolCallResult>,
    error: Option<&McpToolCallError>,
) -> Vec<String> {
    let title = arguments
        .get("title")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map(compact_text)
        .unwrap_or_else(|| "Computer action".to_string());
    let mut details = vec![format!("computer action: {title}")];

    if let Some(error) = error {
        details.push(format!("computer error: {}", compact_text(&error.message)));
        return details;
    }

    if let Some(result) = result {
        let screenshots = result
            .content
            .iter()
            .filter(|content| content.get("type").and_then(Value::as_str) == Some("image"))
            .count();
        if screenshots > 0 {
            details.push(format!(
                "computer screenshot: {}",
                if screenshots == 1 {
                    "captured".to_string()
                } else {
                    format!("{screenshots} captured")
                }
            ));
        }

        // CUA providers commonly return a diagnostic as a text block. Preserve only its first
        // bounded line; manuals and raw provider payloads do not belong in the transcript.
        if let Some(diagnostic) = result.content.iter().find_map(|content| {
            let text = content.get("text").and_then(Value::as_str)?;
            text.strip_prefix("Script error:")
                .map(str::trim)
                .filter(|text| !text.is_empty())
        }) {
            details.push(format!("computer error: {}", compact_text(diagnostic)));
        }
    }

    details
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_server_protocol::protocol::v2::{McpToolCallError, McpToolCallResult};
    use serde_json::json;

    #[test]
    fn computer_activity_keeps_title_and_screenshot_fact_without_media() {
        let details = summary(
            &json!({"title": "Capture calendar"}),
            Some(&McpToolCallResult {
                content: vec![json!({"type": "image", "data": "secret"})],
                structured_content: None,
                meta: None,
            }),
            None,
        );

        assert_eq!(
            details,
            vec![
                "computer action: Capture calendar",
                "computer screenshot: captured"
            ]
        );
        assert!(details.iter().all(|detail| !detail.contains("secret")));
    }

    #[test]
    fn computer_activity_bounds_provider_diagnostics() {
        let details = summary(
            &json!({}),
            None,
            Some(&McpToolCallError {
                message: "failed\nfull provider manual".to_string(),
            }),
        );

        assert_eq!(details[0], "computer action: Computer action");
        assert_eq!(details[1], "computer error: failed");
    }

    #[test]
    fn computer_activity_bounds_multiline_titles_before_projection() {
        let details = summary(
            &json!({"title": format!("{}\nprovider manual", "a".repeat(200))}),
            None,
            None,
        );

        assert_eq!(details.len(), 1);
        assert_eq!(
            details[0].chars().count(),
            "computer action: ".chars().count() + 163
        );
        assert!(details[0].ends_with("..."));
        assert!(!details[0].contains("provider manual"));
    }
}
