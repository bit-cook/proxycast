//! 统一内容系统数据迁移服务
//!
//! 实现从旧版本到新版本的数据迁移，主要包括：
//! - 创建默认项目
//! - 将现有话题迁移到默认项目
//!
//! _Requirements: 2.1, 2.2, 2.3, 2.4_

use chrono::Utc;
use rusqlite::{params, Connection};
use uuid::Uuid;

/// 迁移设置键名
const MIGRATION_KEY_UNIFIED_CONTENT: &str = "migrated_unified_content_system_v1";

/// 默认项目名称
const DEFAULT_PROJECT_NAME: &str = "默认项目";

/// 默认项目图标
const DEFAULT_PROJECT_ICON: &str = "📁";

/// 执行统一内容系统迁移
///
/// 迁移步骤：
/// 1. 检查是否已迁移
/// 2. 创建默认项目（如果不存在）
/// 3. 将所有 project_id 为 null 的内容迁移到默认项目
/// 4. 标记迁移完成
///
/// _Requirements: 2.1, 2.2, 2.3, 2.4_
pub fn migrate_unified_content_system(conn: &Connection) -> Result<MigrationResult, String> {
    // 检查是否已经迁移过
    if is_migration_completed(conn, MIGRATION_KEY_UNIFIED_CONTENT) {
        tracing::debug!("[迁移] 统一内容系统已迁移过，跳过");
        return Ok(MigrationResult::skipped());
    }

    tracing::info!("[迁移] 开始执行统一内容系统迁移");

    // 开始事务
    conn.execute("BEGIN TRANSACTION", [])
        .map_err(|e| format!("开始事务失败: {e}"))?;

    // 执行迁移
    let result = execute_migration(conn);

    match result {
        Ok(stats) => {
            // 标记迁移完成
            mark_migration_completed(conn, MIGRATION_KEY_UNIFIED_CONTENT)?;

            // 提交事务
            conn.execute("COMMIT", [])
                .map_err(|e| format!("提交事务失败: {e}"))?;

            tracing::info!(
                "[迁移] 统一内容系统迁移完成: 默认项目={}, 迁移内容数={}",
                stats.default_project_id,
                stats.migrated_contents_count
            );

            Ok(MigrationResult::success(stats))
        }
        Err(e) => {
            // 回滚事务
            // _Requirements: 2.4_
            let _ = conn.execute("ROLLBACK", []);
            tracing::error!("[迁移] 统一内容系统迁移失败，已回滚: {}", e);
            Err(e)
        }
    }
}

/// 执行迁移的核心逻辑
fn execute_migration(conn: &Connection) -> Result<MigrationStats, String> {
    // 1. 获取或创建默认项目
    // _Requirements: 2.1_
    let default_project_id = get_or_create_default_project(conn)?;

    // 2. 迁移所有 project_id 为 null 的内容到默认项目
    // _Requirements: 2.2_
    let migrated_count = migrate_null_project_contents(conn, &default_project_id)?;

    // 3. 验证迁移结果
    verify_migration(conn)?;

    Ok(MigrationStats {
        default_project_id,
        migrated_contents_count: migrated_count,
    })
}

/// 获取或创建默认项目
///
/// 如果已存在 is_default=true 的项目，返回其 ID
/// 否则创建新的默认项目
///
/// _Requirements: 2.1_
fn get_or_create_default_project(conn: &Connection) -> Result<String, String> {
    // 检查是否已存在默认项目
    let existing_id: Option<String> = conn
        .query_row(
            "SELECT id FROM workspaces WHERE is_default = 1",
            [],
            |row| row.get(0),
        )
        .ok();

    if let Some(id) = existing_id {
        tracing::info!("[迁移] 找到现有默认项目: {}", id);
        return Ok(id);
    }

    // 创建新的默认项目
    let project_id = Uuid::new_v4().to_string();
    let now = Utc::now().timestamp_millis();

    // 使用应用数据目录作为默认项目的 root_path
    let root_path = get_default_project_path()?;

    conn.execute(
        "INSERT INTO workspaces (
            id, name, workspace_type, root_path, is_default,
            settings_json, icon, color, is_favorite, is_archived,
            tags_json, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        params![
            &project_id,
            DEFAULT_PROJECT_NAME,
            "general",
            &root_path,
            true, // is_default = true
            "{}",
            DEFAULT_PROJECT_ICON,
            Option::<String>::None,
            false,
            false,
            "[]",
            now,
            now,
        ],
    )
    .map_err(|e| format!("创建默认项目失败: {e}"))?;

    tracing::info!("[迁移] 创建默认项目: id={}, path={}", project_id, root_path);

    Ok(project_id)
}

/// 获取默认项目的存储路径
fn get_default_project_path() -> Result<String, String> {
    let home = dirs::home_dir().ok_or_else(|| "无法获取主目录".to_string())?;
    let path = home.join(".proxycast").join("projects").join("default");

    // 确保目录存在
    std::fs::create_dir_all(&path).map_err(|e| format!("创建默认项目目录失败: {e}"))?;

    path.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "无效的路径".to_string())
}

/// 将所有 project_id 为 null 的内容迁移到默认项目
///
/// _Requirements: 2.2_
fn migrate_null_project_contents(
    conn: &Connection,
    default_project_id: &str,
) -> Result<usize, String> {
    let now = Utc::now().timestamp_millis();

    // 查询需要迁移的内容数量
    let null_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM contents WHERE project_id IS NULL OR project_id = ''",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if null_count == 0 {
        tracing::info!("[迁移] 没有需要迁移的内容");
        return Ok(0);
    }

    tracing::info!("[迁移] 发现 {} 条需要迁移的内容", null_count);

    // 更新所有 project_id 为 null 的内容
    let affected = conn
        .execute(
            "UPDATE contents 
             SET project_id = ?, updated_at = ? 
             WHERE project_id IS NULL OR project_id = ''",
            params![default_project_id, now],
        )
        .map_err(|e| format!("迁移内容失败: {e}"))?;

    tracing::info!("[迁移] 已将 {} 条内容迁移到默认项目", affected);

    Ok(affected)
}

/// 验证迁移结果
///
/// 确保没有 project_id 为 null 的内容
fn verify_migration(conn: &Connection) -> Result<(), String> {
    let null_count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM contents WHERE project_id IS NULL OR project_id = ''",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if null_count > 0 {
        return Err(format!(
            "迁移验证失败: 仍有 {} 条内容的 project_id 为空",
            null_count
        ));
    }

    // 验证默认项目存在
    let default_exists: bool = conn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM workspaces WHERE is_default = 1)",
            [],
            |row| row.get(0),
        )
        .unwrap_or(false);

    if !default_exists {
        return Err("迁移验证失败: 默认项目不存在".to_string());
    }

    tracing::info!("[迁移] 迁移验证通过");
    Ok(())
}

/// 检查迁移是否已完成
fn is_migration_completed(conn: &Connection, key: &str) -> bool {
    conn.query_row(
        "SELECT value FROM settings WHERE key = ?",
        params![key],
        |row| row.get::<_, String>(0),
    )
    .map(|v| v == "true")
    .unwrap_or(false)
}

/// 标记迁移完成
fn mark_migration_completed(conn: &Connection, key: &str) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?, 'true')",
        params![key],
    )
    .map_err(|e| format!("标记迁移完成失败: {e}"))?;
    Ok(())
}

// ============================================================================
// 迁移结果类型
// ============================================================================

/// 迁移统计信息
#[derive(Debug, Clone)]
pub struct MigrationStats {
    /// 默认项目 ID
    pub default_project_id: String,
    /// 迁移的内容数量
    pub migrated_contents_count: usize,
}

/// 迁移结果
#[derive(Debug)]
pub struct MigrationResult {
    /// 是否执行了迁移
    pub executed: bool,
    /// 迁移统计（如果执行了迁移）
    pub stats: Option<MigrationStats>,
}

impl MigrationResult {
    /// 创建跳过的结果
    fn skipped() -> Self {
        Self {
            executed: false,
            stats: None,
        }
    }

    /// 创建成功的结果
    fn success(stats: MigrationStats) -> Self {
        Self {
            executed: true,
            stats: Some(stats),
        }
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 获取默认项目 ID
///
/// 如果默认项目不存在，返回 None
pub fn get_default_project_id(conn: &Connection) -> Option<String> {
    conn.query_row(
        "SELECT id FROM workspaces WHERE is_default = 1",
        [],
        |row| row.get(0),
    )
    .ok()
}

/// 确保默认项目存在
///
/// 如果不存在则创建，返回默认项目 ID
pub fn ensure_default_project(conn: &Connection) -> Result<String, String> {
    get_or_create_default_project(conn)
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    /// 创建测试数据库
    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();

        // 创建 settings 表
        conn.execute(
            "CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
            [],
        )
        .unwrap();

        // 创建 workspaces 表
        conn.execute(
            "CREATE TABLE workspaces (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                workspace_type TEXT NOT NULL DEFAULT 'persistent',
                root_path TEXT NOT NULL UNIQUE,
                is_default INTEGER DEFAULT 0,
                settings_json TEXT DEFAULT '{}',
                icon TEXT,
                color TEXT,
                is_favorite INTEGER DEFAULT 0,
                is_archived INTEGER DEFAULT 0,
                tags_json TEXT DEFAULT '[]',
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        )
        .unwrap();

        // 创建 contents 表
        conn.execute(
            "CREATE TABLE contents (
                id TEXT PRIMARY KEY,
                project_id TEXT,
                title TEXT NOT NULL,
                content_type TEXT NOT NULL DEFAULT 'document',
                status TEXT NOT NULL DEFAULT 'draft',
                sort_order INTEGER NOT NULL DEFAULT 0,
                body TEXT NOT NULL DEFAULT '',
                word_count INTEGER NOT NULL DEFAULT 0,
                metadata_json TEXT,
                session_id TEXT,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
            [],
        )
        .unwrap();

        conn
    }

    #[test]
    fn test_migration_creates_default_project() {
        let conn = setup_test_db();

        // 执行迁移
        let result = migrate_unified_content_system(&conn).unwrap();

        assert!(result.executed);
        assert!(result.stats.is_some());

        // 验证默认项目存在
        let default_exists: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM workspaces WHERE is_default = 1)",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert!(default_exists);
    }

    #[test]
    fn test_migration_migrates_null_project_contents() {
        let conn = setup_test_db();
        let now = Utc::now().timestamp_millis();

        // 插入一些没有 project_id 的内容
        conn.execute(
            "INSERT INTO contents (id, project_id, title, created_at, updated_at)
             VALUES ('c1', NULL, '内容1', ?, ?)",
            params![now, now],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO contents (id, project_id, title, created_at, updated_at)
             VALUES ('c2', '', '内容2', ?, ?)",
            params![now, now],
        )
        .unwrap();

        // 执行迁移
        let result = migrate_unified_content_system(&conn).unwrap();

        assert!(result.executed);
        let stats = result.stats.unwrap();
        assert_eq!(stats.migrated_contents_count, 2);

        // 验证所有内容都有 project_id
        let null_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM contents WHERE project_id IS NULL OR project_id = ''",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(null_count, 0);
    }

    #[test]
    fn test_migration_skips_if_already_done() {
        let conn = setup_test_db();

        // 第一次迁移
        let result1 = migrate_unified_content_system(&conn).unwrap();
        assert!(result1.executed);

        // 第二次迁移应该跳过
        let result2 = migrate_unified_content_system(&conn).unwrap();
        assert!(!result2.executed);
    }

    #[test]
    fn test_migration_uses_existing_default_project() {
        let conn = setup_test_db();
        let now = Utc::now().timestamp_millis();

        // 先创建一个默认项目
        conn.execute(
            "INSERT INTO workspaces (id, name, workspace_type, root_path, is_default, created_at, updated_at)
             VALUES ('existing-default', '已有默认项目', 'general', '/tmp/existing', 1, ?, ?)",
            params![now, now],
        )
        .unwrap();

        // 插入没有 project_id 的内容
        conn.execute(
            "INSERT INTO contents (id, project_id, title, created_at, updated_at)
             VALUES ('c1', NULL, '内容1', ?, ?)",
            params![now, now],
        )
        .unwrap();

        // 执行迁移
        let result = migrate_unified_content_system(&conn).unwrap();

        assert!(result.executed);
        let stats = result.stats.unwrap();
        assert_eq!(stats.default_project_id, "existing-default");

        // 验证内容被迁移到已有的默认项目
        let project_id: String = conn
            .query_row(
                "SELECT project_id FROM contents WHERE id = 'c1'",
                [],
                |row| row.get(0),
            )
            .unwrap();

        assert_eq!(project_id, "existing-default");
    }
}
