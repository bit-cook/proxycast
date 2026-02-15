//! Gatekeeper mechanism for memory extraction
//!
//! Intelligently determines whether memory extraction is necessary
//! to save API costs and improve efficiency.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

// ==================== Types ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryAnalysisCandidate {
    pub session_id: String,
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub timestamp: i64,
}

// ==================== Configuration ====================

/// Gatekeeper configuration
#[derive(Debug, Clone)]
pub struct GatekeeperConfig {
    /// Maximum number of recent memories to allow skipping
    pub max_recent_memories: usize,

    /// Minimum number of messages required for analysis
    pub min_message_count: usize,

    /// Minimum hours between analyses
    pub min_analysis_interval_hours: i64,

    /// Memory indicator keywords
    pub memory_keywords: Vec<String>,
}

impl Default for GatekeeperConfig {
    fn default() -> Self {
        Self {
            max_recent_memories: 5,
            min_message_count: 3,
            min_analysis_interval_hours: 1,
            memory_keywords: vec![
                "记住".to_string(),
                "记住我".to_string(),
                "我的偏好".to_string(),
                "我喜欢".to_string(),
                "我是".to_string(),
                "我的名字".to_string(),
                "prefer".to_string(),
                "I like".to_string(),
                "I am".to_string(),
                "My name".to_string(),
            ],
        }
    }
}

// ==================== Gatekeeper Functions ====================

/// Check if memory extraction should be performed
pub async fn should_extract_memory(
    db: &Connection,
    candidate: &MemoryAnalysisCandidate,
) -> Result<bool, String> {
    should_extract_memory_with_config(db, candidate, &GatekeeperConfig::default()).await
}

/// Check with custom configuration
pub async fn should_extract_memory_with_config(
    db: &Connection,
    candidate: &MemoryAnalysisCandidate,
    config: &GatekeeperConfig,
) -> Result<bool, String> {
    tracing::info!(
        "[Gatekeeper] Evaluating: session={}, messages={}",
        candidate.session_id,
        candidate.messages.len()
    );

    // 1. Check recent memories (HIGH priority)
    let recent_check =
        check_recent_memories(db, &candidate.session_id, config.max_recent_memories).await?;
    if !recent_check.should_proceed {
        tracing::info!(
            "[Gatekeeper] Blocked: Too many recent memories ({})",
            recent_check.count
        );
        return Ok(false);
    }

    // 2. Check message count (MEDIUM priority)
    let count_check = check_message_count(&candidate.messages, config.min_message_count);
    if !count_check.should_proceed {
        tracing::info!(
            "[Gatekeeper] Blocked: Too few messages ({})",
            count_check.count
        );
        return Ok(false);
    }

    // 3. Check time interval (LOW priority)
    let time_check = check_time_interval(
        db,
        &candidate.session_id,
        config.min_analysis_interval_hours,
    )
    .await?;
    if !time_check.should_proceed {
        tracing::info!(
            "[Gatekeeper] Blocked: Too soon since last analysis ({}h)",
            time_check.hours_since
        );
        return Ok(false);
    }

    // 4. Check keywords (HIGH priority)
    let keyword_check = check_keywords(&candidate.messages, &config.memory_keywords);
    if !keyword_check.should_proceed {
        tracing::info!("[Gatekeeper] Blocked: No memory keywords found");
        return Ok(false);
    }

    tracing::info!("[Gatekeeper] Approved: All checks passed");
    Ok(true)
}

#[derive(Debug)]
struct CheckResult {
    should_proceed: bool,
    count: usize,
    hours_since: i64,
}

// ==================== Check Functions ====================

/// Check 1: Recent memories
/// If too many recent memories, skip analysis
async fn check_recent_memories(
    db: &Connection,
    session_id: &str,
    limit: usize,
) -> Result<CheckResult, String> {
    let sql = "SELECT COUNT(*) FROM unified_memory WHERE session_id = ?1 AND archived = 0";
    let mut stmt = db
        .prepare(sql)
        .map_err(|e| format!("Prepare failed: {e}"))?;

    let count: i64 = stmt
        .query_row(params![session_id], |row| row.get(0))
        .map_err(|e| format!("Query failed: {e}"))?;

    Ok(CheckResult {
        should_proceed: (count as usize) < limit,
        count: count as usize,
        hours_since: 0,
    })
}

/// Check 2: Message count
/// Minimum number of messages required
fn check_message_count(messages: &[ChatMessage], min_count: usize) -> CheckResult {
    let count = messages.len();

    CheckResult {
        should_proceed: count >= min_count,
        count,
        hours_since: 0,
    }
}

/// Check 3: Time interval since last analysis
async fn check_time_interval(
    db: &Connection,
    session_id: &str,
    min_hours: i64,
) -> Result<CheckResult, String> {
    let sql = "SELECT MAX(created_at) FROM unified_memory WHERE session_id = ?1 AND archived = 0";
    let mut stmt = db
        .prepare(sql)
        .map_err(|e| format!("Prepare failed: {e}"))?;

    let max_time: Option<i64> = stmt.query_row(params![session_id], |row| row.get(0)).ok();

    if let Some(last_time) = max_time {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| format!("Time error: {e}"))?
            .as_secs() as i64;

        let hours_since = (now - last_time) / 3600;

        Ok(CheckResult {
            should_proceed: hours_since >= min_hours,
            count: 0,
            hours_since,
        })
    } else {
        Ok(CheckResult {
            should_proceed: true,
            count: 0,
            hours_since: i64::MAX,
        })
    }
}

/// Check 4: Memory indicator keywords
/// Check if messages contain memory-related keywords
fn check_keywords(messages: &[ChatMessage], keywords: &[String]) -> CheckResult {
    let mut found_count = 0;

    for message in messages {
        let content_lower = message.content.to_lowercase();

        for keyword in keywords {
            if content_lower.contains(&keyword.to_lowercase()) {
                found_count += 1;
                break; // Count each message only once
            }
        }
    }

    CheckResult {
        should_proceed: found_count > 0,
        count: found_count,
        hours_since: 0,
    }
}

// ==================== Tests ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_message_count() {
        let messages = vec![
            ChatMessage {
                role: "user".to_string(),
                content: "Hello".to_string(),
                timestamp: 0,
            },
            ChatMessage {
                role: "assistant".to_string(),
                content: "Hi there".to_string(),
                timestamp: 1,
            },
            ChatMessage {
                role: "user".to_string(),
                content: "Remember I like coffee".to_string(),
                timestamp: 2,
            },
        ];

        let result = check_message_count(&messages, 3);
        assert!(result.should_proceed);
        assert_eq!(result.count, 3);
    }

    #[test]
    fn test_check_keywords() {
        let messages = vec![
            ChatMessage {
                role: "user".to_string(),
                content: "Remember I like coffee".to_string(),
                timestamp: 0,
            },
            ChatMessage {
                role: "user".to_string(),
                content: "My preference is tea".to_string(),
                timestamp: 1,
            },
        ];

        let keywords = vec!["记住".to_string(), "喜欢".to_string(), "prefer".to_string()];

        let result = check_keywords(&messages, &keywords);
        assert!(result.should_proceed);
        assert_eq!(result.count, 2);
    }

    #[test]
    fn test_check_keywords_no_match() {
        let messages = vec![ChatMessage {
            role: "user".to_string(),
            content: "Hello world".to_string(),
            timestamp: 0,
        }];

        let keywords = vec!["记住".to_string(), "喜欢".to_string()];

        let result = check_keywords(&messages, &keywords);
        assert!(!result.should_proceed);
        assert_eq!(result.count, 0);
    }
}
