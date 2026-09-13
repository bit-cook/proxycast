//! Bounded hook lifecycle facts for the canonical transcript projection.
//!
//! Hook execution is owned by tool-runtime/App Server. This module only shapes the
//! already bounded v2 HookRunSummary for TUI display; it never executes hooks or
//! exposes model-facing context entries.

use app_server_protocol::protocol::v2::{HookOutputEntryKind, HookRunStatus, HookRunSummary};

use super::compact_text;

const MAX_OUTPUTS: usize = 4;

pub(crate) fn is_quiet_success(run: &HookRunSummary) -> bool {
    run.status == HookRunStatus::Completed
        && run
            .entries
            .iter()
            .all(|entry| entry.kind == HookOutputEntryKind::Context)
}

pub(crate) fn status_text(status: HookRunStatus) -> Option<&'static str> {
    match status {
        HookRunStatus::Completed => Some("hook completed"),
        HookRunStatus::Failed => Some("hook failed"),
        HookRunStatus::Blocked => Some("hook blocked"),
        HookRunStatus::Stopped => Some("hook stopped"),
        HookRunStatus::Running => None,
    }
}

/// Returns only user-visible hook facts. Context entries are model-facing and must stay hidden.
pub(crate) fn output_details(run: &HookRunSummary) -> Vec<String> {
    run.entries
        .iter()
        .filter(|entry| entry.kind != HookOutputEntryKind::Context)
        .filter_map(|entry| {
            let text = compact_text(&entry.text);
            (!text.trim().is_empty()).then(|| format!("hook output: {text}"))
        })
        .take(MAX_OUTPUTS)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_server_protocol::protocol::v2::{
        HookEventName, HookExecutionMode, HookHandlerType, HookScope, HookSource,
    };
    use std::path::PathBuf;

    fn run(status: HookRunStatus, entries: Vec<(HookOutputEntryKind, &str)>) -> HookRunSummary {
        HookRunSummary {
            id: "hook-1".to_string(),
            event_name: HookEventName::PreToolUse,
            handler_type: HookHandlerType::Command,
            execution_mode: HookExecutionMode::Sync,
            scope: HookScope::Turn,
            source_path: PathBuf::from("/workspace/hooks.json"),
            source: HookSource::Project,
            display_order: 0,
            status,
            status_message: None,
            started_at: 1,
            completed_at: Some(2),
            duration_ms: Some(1),
            entries: entries
                .into_iter()
                .map(
                    |(kind, text)| app_server_protocol::protocol::v2::HookOutputEntry {
                        kind,
                        text: text.to_string(),
                    },
                )
                .collect(),
        }
    }

    #[test]
    fn context_only_success_is_quiet() {
        let run = run(
            HookRunStatus::Completed,
            vec![(HookOutputEntryKind::Context, "private context")],
        );
        assert!(is_quiet_success(&run));
        assert!(output_details(&run).is_empty());
    }

    #[test]
    fn non_context_outputs_are_bounded_and_compact() {
        let run = run(
            HookRunStatus::Failed,
            (0..6)
                .map(|index| {
                    (
                        HookOutputEntryKind::Error,
                        if index == 0 {
                            "first\nsecond"
                        } else {
                            "output"
                        },
                    )
                })
                .collect(),
        );
        assert!(!is_quiet_success(&run));
        assert_eq!(output_details(&run).len(), MAX_OUTPUTS);
        assert_eq!(output_details(&run)[0], "hook output: first");
        assert_eq!(status_text(HookRunStatus::Blocked), Some("hook blocked"));
    }
}
