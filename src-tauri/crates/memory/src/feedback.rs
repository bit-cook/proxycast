//! Feedback learning system for memory extraction
//!
//! Records user feedback to improve extraction quality over time

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

// ==================== Types ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum FeedbackAction {
    Approve,
    Reject,
    Modify { changes: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserFeedback {
    pub id: String,
    pub memory_id: String,
    pub action: FeedbackAction,
    pub session_id: String,
    pub created_at: i64,
}

// ==================== Database Operations ====================

/// Record user feedback
pub fn record_feedback(db: &Connection, feedback: &UserFeedback) -> Result<(), String> {
    let action_json = serde_json::to_string(&feedback.action)
        .map_err(|e| format!("JSON serialization failed: {e}"))?;

    let sql = r#"
        INSERT INTO memory_feedback (id, memory_id, action, session_id, created_at)
        VALUES (?1, ?2, ?3, ?4, ?5)
    "#;

    db.execute(
        sql,
        params![
            feedback.id,
            feedback.memory_id,
            action_json,
            feedback.session_id,
            feedback.created_at,
        ],
    )
    .map_err(|e| format!("Insert failed: {e}"))?;

    tracing::info!("[Feedback] Recorded: {:?}", feedback.action);
    Ok(())
}

/// Get recent feedbacks for a session
pub fn get_recent_feedbacks(
    db: &Connection,
    session_id: &str,
    limit: usize,
) -> Result<Vec<UserFeedback>, String> {
    let sql = r#"
        SELECT id, memory_id, action, session_id, created_at
        FROM memory_feedback
        WHERE session_id = ?1
        ORDER BY created_at DESC
        LIMIT ?2
    "#;

    let mut stmt = db
        .prepare(sql)
        .map_err(|e| format!("Prepare failed: {e}"))?;

    let feedbacks = stmt
        .query_map(params![session_id, limit as i64], |row| {
            let id: String = row.get(0)?;
            let memory_id: String = row.get(1)?;
            let action_json: String = row.get(2)?;
            let session_id: String = row.get(3)?;
            let created_at: i64 = row.get(4)?;

            let action: FeedbackAction = serde_json::from_str(&action_json)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

            Ok(UserFeedback {
                id,
                memory_id,
                action,
                session_id,
                created_at,
            })
        })
        .map_err(|e| format!("Query failed: {e}"))?
        .collect::<Result<Vec<_>, rusqlite::Error>>()
        .map_err(|e| format!("Collection failed: {e}"))?;

    Ok(feedbacks)
}

/// Calculate approval rate
pub fn calculate_approval_rate(feedbacks: &[UserFeedback]) -> f32 {
    if feedbacks.is_empty() {
        return 0.5; // Default neutral
    }

    let mut score = 0.0;
    let total = feedbacks.len() as f32;

    for feedback in feedbacks {
        match feedback.action {
            FeedbackAction::Approve => score += 1.0,
            FeedbackAction::Reject => score -= 0.5,
            FeedbackAction::Modify { .. } => score += 0.3,
        }
    }

    (score / total).max(0.0).min(1.0)
}

// ==================== Extraction Parameters ====================

#[derive(Debug, Clone)]
pub struct ExtractionParams {
    pub min_importance: u8,
    pub min_confidence: f32,
}

impl Default for ExtractionParams {
    fn default() -> Self {
        Self {
            min_importance: 5,
            min_confidence: 0.6,
        }
    }
}

/// Adjust extraction parameters based on feedback
pub fn adjust_extraction_params(
    db: &Connection,
    session_id: &str,
) -> Result<ExtractionParams, String> {
    let feedbacks = get_recent_feedbacks(db, session_id, 20)?;
    let approval_rate = calculate_approval_rate(&feedbacks);

    tracing::info!(
        "[Feedback] Approval rate: {:.2}, feedbacks: {}",
        approval_rate,
        feedbacks.len()
    );

    let params = if approval_rate > 0.7 {
        // High approval rate: lower thresholds
        ExtractionParams {
            min_importance: 3,
            min_confidence: 0.4,
        }
    } else if approval_rate < 0.3 {
        // Low approval rate: raise thresholds
        ExtractionParams {
            min_importance: 7,
            min_confidence: 0.8,
        }
    } else {
        // Medium approval rate: default
        ExtractionParams::default()
    };

    Ok(params)
}

// ==================== Helper Functions ====================

/// Generate feedback ID
pub fn generate_feedback_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("feedback_{now}")
}

/// Get current timestamp
pub fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

// ==================== Tests ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_approval_rate() {
        let feedbacks = vec![
            UserFeedback {
                id: "1".to_string(),
                memory_id: "m1".to_string(),
                action: FeedbackAction::Approve,
                session_id: "s1".to_string(),
                created_at: 0,
            },
            UserFeedback {
                id: "2".to_string(),
                memory_id: "m2".to_string(),
                action: FeedbackAction::Approve,
                session_id: "s1".to_string(),
                created_at: 1,
            },
            UserFeedback {
                id: "3".to_string(),
                memory_id: "m3".to_string(),
                action: FeedbackAction::Reject,
                session_id: "s1".to_string(),
                created_at: 2,
            },
        ];

        let rate = calculate_approval_rate(&feedbacks);
        // (1.0 + 1.0 - 0.5) / 3 = 0.5
        assert!((rate - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_calculate_approval_rate_empty() {
        let feedbacks = vec![];
        let rate = calculate_approval_rate(&feedbacks);
        assert_eq!(rate, 0.5);
    }
}
