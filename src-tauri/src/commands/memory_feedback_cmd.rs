//! Memory feedback commands

use crate::database::DbConnection;
use proxycast_memory::feedback::{
    calculate_approval_rate, current_timestamp, generate_feedback_id, get_recent_feedbacks,
    record_feedback, FeedbackAction, UserFeedback,
};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackRequest {
    pub memory_id: String,
    pub action: FeedbackAction,
    pub session_id: String,
}

#[tauri::command]
pub async fn unified_memory_feedback(
    db: State<'_, DbConnection>,
    request: FeedbackRequest,
) -> Result<(), String> {
    let feedback = UserFeedback {
        id: generate_feedback_id(),
        memory_id: request.memory_id,
        action: request.action,
        session_id: request.session_id,
        created_at: current_timestamp(),
    };

    let conn = db.lock().unwrap();
    record_feedback(&conn, &feedback)?;

    Ok(())
}

#[tauri::command]
pub async fn get_memory_feedback_stats(
    db: State<'_, DbConnection>,
    session_id: String,
) -> Result<FeedbackStats, String> {
    let conn = db.lock().unwrap();
    let feedbacks = get_recent_feedbacks(&conn, &session_id, 50)?;

    let approval_rate = calculate_approval_rate(&feedbacks);
    let total = feedbacks.len();

    let mut approve_count = 0;
    let mut reject_count = 0;
    let mut modify_count = 0;

    for feedback in &feedbacks {
        match feedback.action {
            FeedbackAction::Approve => approve_count += 1,
            FeedbackAction::Reject => reject_count += 1,
            FeedbackAction::Modify { .. } => modify_count += 1,
        }
    }

    Ok(FeedbackStats {
        total,
        approve_count,
        reject_count,
        modify_count,
        approval_rate,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackStats {
    pub total: usize,
    pub approve_count: usize,
    pub reject_count: usize,
    pub modify_count: usize,
    pub approval_rate: f32,
}
