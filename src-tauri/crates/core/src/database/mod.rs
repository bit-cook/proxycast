pub mod dao;
pub mod migration;
pub mod migration_v2;
pub mod migration_v3;
pub mod migration_v4;
pub mod schema;
pub mod system_providers;

use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub type DbConnection = Arc<Mutex<Connection>>;

/// 获取数据库连接锁（自动处理 poisoned lock）
pub fn lock_db(db: &DbConnection) -> Result<std::sync::MutexGuard<'_, Connection>, String> {
    match db.lock() {
        Ok(guard) => Ok(guard),
        Err(poisoned) => {
            tracing::warn!("[数据库] 检测到数据库锁被污染，尝试恢复: {}", poisoned);
            db.clear_poison();
            Ok(poisoned.into_inner())
        }
    }
}

/// 获取数据库文件路径
pub fn get_db_path() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or_else(|| "无法获取主目录".to_string())?;
    let db_dir = home.join(".proxycast");
    std::fs::create_dir_all(&db_dir).map_err(|e| format!("无法创建数据库目录 {db_dir:?}: {e}"))?;
    Ok(db_dir.join("proxycast.db"))
}

/// 初始化数据库连接
pub fn init_database() -> Result<DbConnection, String> {
    let db_path = get_db_path()?;
    let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;

    // 设置 busy_timeout 为 5 秒，避免 "database is locked" 错误
    conn.busy_timeout(std::time::Duration::from_secs(5))
        .map_err(|e| format!("设置 busy_timeout 失败: {e}"))?;

    // 启用 WAL 模式提升并发性能
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous = NORMAL;
         PRAGMA cache_size = -64000;
         PRAGMA temp_store = MEMORY;",
    )
    .map_err(|e| format!("设置数据库优化参数失败: {e}"))?;

    tracing::info!("[数据库] 已启用 WAL 模式和性能优化参数");

    // 创建表结构
    schema::create_tables(&conn).map_err(|e| e.to_string())?;
    migration::migrate_from_json(&conn)?;

    // 执行 Provider ID 迁移（修复旧 ID 与模型注册表不匹配的问题）
    match migration::migrate_provider_ids(&conn) {
        Ok(count) => {
            if count > 0 {
                tracing::info!("[数据库] 已迁移 {} 个 Provider ID", count);
                // 标记需要刷新模型注册表
                migration::mark_model_registry_refresh_needed(&conn);
            }
        }
        Err(e) => {
            tracing::warn!("[数据库] Provider ID 迁移失败（非致命）: {}", e);
        }
    }

    // 检查是否需要刷新模型注册表（版本升级时）
    migration::check_model_registry_version(&conn);

    // 执行 API Keys 到 Provider Pool 的迁移
    match migration::migrate_api_keys_to_pool(&conn) {
        Ok(count) => {
            if count > 0 {
                tracing::info!("[数据库] 已将 {} 条 API Key 迁移到凭证池", count);
            }
        }
        Err(e) => {
            tracing::warn!("[数据库] API Key 迁移失败（非致命）: {}", e);
        }
    }

    // 清理旧的 API Key 凭证（openai_key, claude_key 类型）
    match migration::cleanup_legacy_api_key_credentials(&conn) {
        Ok(count) => {
            if count > 0 {
                tracing::info!("[数据库] 已清理 {} 条旧 API Key 凭证", count);
            }
        }
        Err(e) => {
            tracing::warn!("[数据库] 旧 API Key 凭证清理失败（非致命）: {}", e);
        }
    }

    // 修复历史 MCP 导入数据（补齐 enabled_proxycast）
    match migration::migrate_mcp_proxycast_enabled(&conn) {
        Ok(count) => {
            if count > 0 {
                tracing::info!("[数据库] 已修复 {} 条 MCP ProxyCast 启用状态", count);
            }
        }
        Err(e) => {
            tracing::warn!("[数据库] MCP ProxyCast 启用状态修复失败（非致命）: {}", e);
        }
    }

    // 执行统一内容系统迁移（创建默认项目，迁移话题）
    // _Requirements: 2.1, 2.2, 2.3, 2.4_
    match migration_v2::migrate_unified_content_system(&conn) {
        Ok(result) => {
            if result.executed {
                if let Some(stats) = result.stats {
                    tracing::info!(
                        "[数据库] 统一内容系统迁移完成: 默认项目={}, 迁移内容数={}",
                        stats.default_project_id,
                        stats.migrated_contents_count
                    );
                }
            }
        }
        Err(e) => {
            tracing::warn!("[数据库] 统一内容系统迁移失败（非致命）: {}", e);
        }
    }

    // 执行 Playwright MCP Server 迁移
    match migration_v3::migrate_playwright_mcp_server(&conn) {
        Ok(result) => {
            if result.executed {
                if let Some(server_id) = result.server_id {
                    tracing::info!(
                        "[数据库] Playwright MCP Server 迁移完成: server_id={}",
                        server_id
                    );
                }
            }
        }
        Err(e) => {
            tracing::warn!("[数据库] Playwright MCP Server 迁移失败（非致命）: {}", e);
        }
    }

    // 修复 [object Promise] 路径污染问题（历史 bug 遗留数据）
    match migration_v4::migrate_fix_promise_paths(&conn) {
        Ok(result) => {
            if result.executed {
                tracing::info!(
                    "[数据库] 路径修复和会话统一完成: workspaces={}, sessions={}, unified={}",
                    result.fixed_workspaces,
                    result.fixed_sessions,
                    result.unified_sessions
                );
            }
        }
        Err(e) => {
            tracing::warn!("[数据库] 路径修复和会话统一失败（非致命）: {}", e);
        }
    }

    Ok(Arc::new(Mutex::new(conn)))
}
