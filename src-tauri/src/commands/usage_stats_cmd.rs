//! 使用统计命令
//!
//! 提供使用统计数据的查询功能

use crate::database::DbConnection;
use crate::services::conversation_statistics_service;
use tauri::State;

// 重新导出服务中的类型
pub use conversation_statistics_service::{DailyUsage, ModelUsage, UsageStatsResponse};

/// 获取使用统计数据
///
/// 根据时间范围（week/month/all）返回统计数据
#[tauri::command]
pub async fn get_usage_stats(
    time_range: String,
    db: State<'_, DbConnection>,
) -> Result<UsageStatsResponse, String> {
    tracing::info!("[使用统计] 获取统计数据，时间范围: {}", time_range);

    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;

    conversation_statistics_service::get_usage_stats_from_db(&time_range, &conn)
}

/// 获取模型使用排行
#[tauri::command]
pub async fn get_model_usage_ranking(
    time_range: String,
    db: State<'_, DbConnection>,
) -> Result<Vec<ModelUsage>, String> {
    tracing::info!("[使用统计] 获取模型使用排行，时间范围: {}", time_range);

    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;

    conversation_statistics_service::get_model_usage_ranking_from_db(&time_range, &conn)
}

/// 获取每日使用趋势
#[tauri::command]
pub async fn get_daily_usage_trends(
    time_range: String,
    db: State<'_, DbConnection>,
) -> Result<Vec<DailyUsage>, String> {
    tracing::info!("[使用统计] 获取每日使用趋势，时间范围: {}", time_range);

    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;

    conversation_statistics_service::get_daily_usage_trends_from_db(&time_range, &conn)
}
