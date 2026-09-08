use super::helpers::{
    lifecycle_base_item, merge_lifecycle_metadata, merge_optional_field, update_lifecycle_item,
};
use crate::runtime::{raw_string_field, string_array_field, string_field, StoredSession};
use app_server_protocol::AgentEvent;
use serde_json::{json, Value};
use std::collections::HashMap;

pub(super) fn upsert_command_item(
    stored: &StoredSession,
    event: &AgentEvent,
    items: &mut HashMap<String, Value>,
) {
    let Some(command_id) = command_id(&event.payload).or_else(|| Some(event.event_id.clone()))
    else {
        return;
    };
    let status = match event.event_type.as_str() {
        "command.started" => "in_progress",
        "command.exited" => command_exit_item_status(&event.payload),
        _ => "in_progress",
    };
    let entry = items.entry(command_id.clone()).or_insert_with(|| {
        lifecycle_base_item(stored, event, &command_id, "command_execution", status)
    });
    let Some(object) = entry.as_object_mut() else {
        return;
    };

    update_lifecycle_item(object, event, status);
    merge_optional_field(
        object,
        "command",
        command_string(&event.payload)
            .or_else(|| {
                object
                    .get("command")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .or_else(|| Some(command_id.clone()))
            .map(Value::String),
    );
    merge_optional_field(
        object,
        "cwd",
        string_field(&event.payload, &["cwd", "workingDirectory", "working_dir"])
            .map(Value::String),
    );
    merge_optional_field(
        object,
        "aggregated_output",
        raw_string_field(
            &event.payload,
            &["outputPreview", "output_preview", "output", "summary"],
        )
        .map(Value::String),
    );
    merge_optional_field(
        object,
        "exit_code",
        event
            .payload
            .get("exitCode")
            .or_else(|| event.payload.get("exit_code"))
            .and_then(Value::as_i64)
            .map(|value| json!(value)),
    );
    merge_optional_field(
        object,
        "error",
        raw_string_field(&event.payload, &["error", "message", "reason"]).map(Value::String),
    );
    merge_lifecycle_metadata(object, event);
}

pub(super) fn upsert_patch_item(
    stored: &StoredSession,
    event: &AgentEvent,
    items: &mut HashMap<String, Value>,
) {
    let Some(patch_id) = patch_id(&event.payload).or_else(|| Some(event.event_id.clone())) else {
        return;
    };
    let status = match event.event_type.as_str() {
        "patch.failed" => "failed",
        "patch.declined" => "declined",
        "patch.started" => "in_progress",
        _ => "completed",
    };
    let entry = items
        .entry(patch_id.clone())
        .or_insert_with(|| lifecycle_base_item(stored, event, &patch_id, "patch", status));
    let Some(object) = entry.as_object_mut() else {
        return;
    };

    update_lifecycle_item(object, event, status);
    if let Some(changes) = event.payload.get("changes").and_then(Value::as_array) {
        if !changes.is_empty() {
            object.insert("changes".to_string(), Value::Array(changes.clone()));
        }
    }
    let paths = string_array_field(&event.payload, &["paths", "changedFiles", "changed_files"]);
    if !paths.is_empty() {
        object.insert("summary".to_string(), json!(paths));
        object.insert("paths".to_string(), json!(paths));
        object.insert(
            "text".to_string(),
            json!(format!("Patch changed {}", paths.join(", "))),
        );
    } else if !object.contains_key("text") {
        object.insert(
            "text".to_string(),
            json!(
                raw_string_field(&event.payload, &["stdout", "stderr", "message"])
                    .unwrap_or_else(|| "Patch applied".to_string())
            ),
        );
    }
    merge_optional_field(
        object,
        "success",
        event
            .payload
            .get("success")
            .and_then(Value::as_bool)
            .map(Value::Bool),
    );
    merge_optional_field(
        object,
        "stdout",
        raw_string_field(&event.payload, &["stdout"]).map(Value::String),
    );
    merge_optional_field(
        object,
        "stderr",
        raw_string_field(&event.payload, &["stderr"]).map(Value::String),
    );
    merge_lifecycle_metadata(object, event);
}

fn command_id(payload: &Value) -> Option<String> {
    string_field(
        payload,
        &[
            "commandId",
            "command_id",
            "toolCallId",
            "tool_call_id",
            "id",
        ],
    )
}

fn patch_id(payload: &Value) -> Option<String> {
    string_field(
        payload,
        &["patchId", "patch_id", "toolCallId", "tool_call_id", "id"],
    )
}

fn command_string(payload: &Value) -> Option<String> {
    raw_string_field(
        payload,
        &[
            "canonicalCommand",
            "canonical_command",
            "command",
            "commandSummary",
            "command_summary",
        ],
    )
    .or_else(|| {
        let argv = string_array_field(payload, &["commandArgv", "command_argv"]);
        (!argv.is_empty()).then(|| argv.join(" "))
    })
}

fn command_exit_item_status(payload: &Value) -> &'static str {
    if let Some(status) = payload
        .get("status")
        .and_then(Value::as_str)
        .map(|status| status.to_ascii_lowercase())
    {
        match status.as_str() {
            "canceled" | "cancelled" => return "cancelled",
            "interrupted" => return "interrupted",
            "failed" | "error" => return "failed",
            "completed" | "complete" | "success" | "succeeded" | "passed" => return "completed",
            _ => {}
        }
    }
    let exit_code = payload
        .get("exitCode")
        .or_else(|| payload.get("exit_code"))
        .and_then(Value::as_i64);
    match exit_code {
        Some(0) | None => "completed",
        Some(_) => "failed",
    }
}
