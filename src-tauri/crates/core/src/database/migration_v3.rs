//! MCP 服务器默认数据迁移
//!
//! 添加默认的 Playwright MCP Server 配置到数据库

use chrono::Utc;
use rusqlite::{params, Connection};
use serde_json::json;
use uuid::Uuid;

/// 迁移设置键名
const MIGRATION_KEY_PLAYWRIGHT_SERVER: &str = "migrated_playwright_mcp_server_v1";

/// Playwright MCP Server 默认配置
const PLAYWRIGHT_SERVER_NAME: &str = "playwright";
const PLAYWRIGHT_SERVER_DESCRIPTION: &str = "Playwright 浏览器自动化工具";

/// 迁移结果
pub struct MigrationResult {
    /// 是否执行了迁移
    pub executed: bool,
    /// 创建的服务器 ID
    pub server_id: Option<String>,
}

/// 执行 Playwright MCP Server 迁移
///
/// 迁移步骤：
/// 1. 检查是否已迁移
/// 2. 检查是否已存在 playwright 服务器
/// 3. 如果不存在，创建默认配置
/// 4. 标记迁移完成
pub fn migrate_playwright_mcp_server(conn: &Connection) -> Result<MigrationResult, String> {
    // 检查是否已经迁移过
    if is_migration_completed(conn, MIGRATION_KEY_PLAYWRIGHT_SERVER) {
        tracing::debug!("[迁移] Playwright MCP Server 已迁移过，跳过");
        return Ok(MigrationResult {
            executed: false,
            server_id: None,
        });
    }

    tracing::info!("[迁移] 开始执行 Playwright MCP Server 迁移");

    // 检查是否已存在 playwright 服务器
    if server_exists(conn, PLAYWRIGHT_SERVER_NAME) {
        tracing::info!("[迁移] Playwright MCP Server 已存在，跳过创建并标记迁移完成");
        mark_migration_completed(conn, MIGRATION_KEY_PLAYWRIGHT_SERVER)?;
        return Ok(MigrationResult {
            executed: false,
            server_id: None,
        });
    }

    // 开始事务
    conn.execute("BEGIN TRANSACTION", [])
        .map_err(|e| format!("开始事务失败: {e}"))?;

    // 执行迁移
    let result = execute_playwright_migration(conn);

    match result {
        Ok(server_id) => {
            // 标记迁移完成
            mark_migration_completed(conn, MIGRATION_KEY_PLAYWRIGHT_SERVER)?;

            // 提交事务
            conn.execute("COMMIT", [])
                .map_err(|e| format!("提交事务失败: {e}"))?;

            tracing::info!(
                "[迁移] Playwright MCP Server 迁移完成: server_id={}",
                server_id
            );

            Ok(MigrationResult {
                executed: true,
                server_id: Some(server_id),
            })
        }
        Err(e) => {
            // 回滚事务
            let _ = conn.execute("ROLLBACK", []);
            tracing::error!("[迁移] Playwright MCP Server 迁移失败，已回滚: {}", e);
            Err(e)
        }
    }
}

/// 执行迁移的核心逻辑
fn execute_playwright_migration(conn: &Connection) -> Result<String, String> {
    // 创建 Playwright MCP Server 配置
    let server_id = Uuid::new_v4().to_string();
    let created_at = Utc::now().to_rfc3339();

    // 构建服务器配置
    let server_config = json!({
        "command": "npx",
        "args": ["-y", "@modelcontextprotocol/server-playwright"],
        "env": {
            "HEADLESS": "true"
        }
    });

    // 插入服务器配置
    conn.execute(
        "INSERT INTO mcp_servers (id, name, server_config, description,
                                 enabled_proxycast, enabled_claude, enabled_codex,
                                 enabled_gemini, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            server_id,
            PLAYWRIGHT_SERVER_NAME,
            serde_json::to_string(&server_config).unwrap_or_default(),
            PLAYWRIGHT_SERVER_DESCRIPTION,
            1i32, // enabled_proxycast = true
            0i32, // enabled_claude = false
            0i32, // enabled_codex = false
            0i32, // enabled_gemini = false
            created_at,
        ],
    )
    .map_err(|e| format!("插入 Playwright MCP Server 失败: {e}"))?;

    tracing::info!(
        "[迁移] 已创建 Playwright MCP Server: id={}, name={}",
        server_id,
        PLAYWRIGHT_SERVER_NAME
    );

    Ok(server_id)
}

/// 检查服务器是否已存在
fn server_exists(conn: &Connection, name: &str) -> bool {
    conn.query_row(
        "SELECT COUNT(*) FROM mcp_servers WHERE name = ?1",
        [name],
        |row| row.get::<_, i32>(0),
    )
    .unwrap_or(0)
        > 0
}

/// 检查迁移是否已完成
fn is_migration_completed(conn: &Connection, key: &str) -> bool {
    conn.query_row("SELECT value FROM settings WHERE key = ?1", [key], |row| {
        row.get::<_, String>(0)
    })
    .is_ok()
}

/// 标记迁移已完成
fn mark_migration_completed(conn: &Connection, key: &str) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        [key, "1"],
    )
    .map_err(|e| format!("标记迁移完成失败: {e}"))?;
    Ok(())
}
