use rusqlite::{params, Connection};

/// 从旧的 JSON 配置迁移数据到 SQLite
#[allow(dead_code)]
pub fn migrate_from_json(conn: &Connection) -> Result<(), String> {
    // 检查是否已经迁移过
    let migrated: bool = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'migrated_from_json'",
            [],
            |row| row.get::<_, String>(0),
        )
        .map(|v| v == "true")
        .unwrap_or(false);

    if migrated {
        return Ok(());
    }

    // 读取旧配置文件（历史路径）
    let home = dirs::home_dir().ok_or_else(|| "无法获取主目录".to_string())?;
    let config_path = home.join(".proxycast").join("config.json");

    if config_path.exists() {
        // 备份旧配置，避免误覆盖
        let backup_path = config_path.with_file_name("config.json.backup");
        if !backup_path.exists() {
            std::fs::copy(&config_path, &backup_path)
                .map_err(|e| format!("备份旧配置失败: {e}"))?;
        }

        return Err(
            "检测到旧版 config.json（~/.proxycast/config.json），当前版本尚未支持自动迁移。请手动导出/重建配置后再启动。"
                .to_string(),
        );
    }

    // 标记迁移完成
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES ('migrated_from_json', 'true')",
        [],
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/// 将 api_keys 表中的数据迁移到 provider_pool_credentials 表
///
/// 迁移逻辑：
/// 1. 读取 api_keys 表中的所有 API Key
/// 2. 根据 provider_id 查找对应的 api_key_providers 配置
/// 3. 将 API Key 转换为 CredentialData::OpenAIKey 或 CredentialData::ClaudeKey
/// 4. 插入到 provider_pool_credentials 表
pub fn migrate_api_keys_to_pool(conn: &Connection) -> Result<usize, String> {
    // 检查是否已经迁移过
    let migrated: bool = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'migrated_api_keys_to_pool'",
            [],
            |row| row.get::<_, String>(0),
        )
        .map(|v| v == "true")
        .unwrap_or(false);

    if migrated {
        tracing::debug!("[迁移] API Keys 已迁移过，跳过");
        return Ok(0);
    }

    tracing::info!("[迁移] 开始将 api_keys 迁移到 provider_pool_credentials");

    // 查询所有 API Keys 及其对应的 Provider 信息
    let mut stmt = conn
        .prepare(
            "SELECT k.id, k.provider_id, k.api_key_encrypted, k.alias, k.enabled,
                    k.usage_count, k.error_count, k.last_used_at, k.created_at,
                    p.type, p.api_host, p.name as provider_name
             FROM api_keys k
             JOIN api_key_providers p ON k.provider_id = p.id
             ORDER BY k.created_at ASC",
        )
        .map_err(|e| format!("准备查询语句失败: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok(ApiKeyMigrationRow {
                id: row.get(0)?,
                provider_id: row.get(1)?,
                api_key_encrypted: row.get(2)?,
                alias: row.get(3)?,
                enabled: row.get(4)?,
                usage_count: row.get::<_, i64>(5)? as u64,
                error_count: row.get::<_, i64>(6)? as u32,
                last_used_at: row.get(7)?,
                created_at: row.get(8)?,
                provider_type: row.get(9)?,
                api_host: row.get(10)?,
                provider_name: row.get(11)?,
            })
        })
        .map_err(|e| format!("查询 API Keys 失败: {e}"))?;

    let mut migrated_count = 0;
    let now = chrono::Utc::now().timestamp();

    for row_result in rows {
        let row = row_result.map_err(|e| format!("读取行数据失败: {e}"))?;

        // 检查是否已存在相同的凭证（通过 api_key_encrypted 判断）
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM provider_pool_credentials 
                 WHERE credential_data LIKE ?1",
                params![format!("%{}%", row.api_key_encrypted)],
                |r| r.get(0),
            )
            .unwrap_or(false);

        if exists {
            tracing::debug!(
                "[迁移] 跳过已存在的 API Key: {} (provider: {})",
                row.alias.as_deref().unwrap_or(&row.id),
                row.provider_id
            );
            continue;
        }

        // 根据 provider_type 确定 pool_provider_type 和 credential_data
        let (pool_provider_type, credential_data) = match row.provider_type.to_lowercase().as_str()
        {
            "anthropic" => {
                let cred = serde_json::json!({
                    "type": "claude_key",
                    "api_key": row.api_key_encrypted,
                    "base_url": if row.api_host.is_empty() { None } else { Some(&row.api_host) }
                });
                ("claude", cred)
            }
            "openai" | "openai-response" => {
                let cred = serde_json::json!({
                    "type": "openai_key",
                    "api_key": row.api_key_encrypted,
                    "base_url": if row.api_host.is_empty() { None } else { Some(&row.api_host) }
                });
                ("openai", cred)
            }
            "gemini" => {
                let cred = serde_json::json!({
                    "type": "gemini_api_key",
                    "api_key": row.api_key_encrypted,
                    "base_url": if row.api_host.is_empty() { None } else { Some(&row.api_host) },
                    "excluded_models": []
                });
                ("gemini_api_key", cred)
            }
            "vertex" | "vertexai" => {
                let cred = serde_json::json!({
                    "type": "vertex_key",
                    "api_key": row.api_key_encrypted,
                    "base_url": if row.api_host.is_empty() { None } else { Some(&row.api_host) },
                    "model_aliases": {}
                });
                ("vertex", cred)
            }
            // 其他类型默认作为 OpenAI 兼容处理
            _ => {
                let cred = serde_json::json!({
                    "type": "openai_key",
                    "api_key": row.api_key_encrypted,
                    "base_url": if row.api_host.is_empty() { None } else { Some(&row.api_host) }
                });
                ("openai", cred)
            }
        };

        // 生成名称：优先使用 alias，否则使用 provider_name
        let name = row
            .alias
            .clone()
            .or_else(|| Some(format!("{} (迁移)", row.provider_name)));

        // 解析时间
        let created_at_ts = row
            .created_at
            .as_ref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.timestamp())
            .unwrap_or(now);

        let last_used_ts = row
            .last_used_at
            .as_ref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.timestamp());

        // 插入到 provider_pool_credentials
        let uuid = uuid::Uuid::new_v4().to_string();
        let credential_json = credential_data.to_string();

        conn.execute(
            "INSERT INTO provider_pool_credentials
             (uuid, provider_type, credential_data, name, is_healthy, is_disabled,
              check_health, check_model_name, not_supported_models, usage_count, error_count,
              last_used, last_error_time, last_error_message, last_health_check_time,
              last_health_check_model, created_at, updated_at, source, proxy_url)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
            params![
                uuid,
                pool_provider_type,
                credential_json,
                name,
                true,                    // is_healthy
                !row.enabled,            // is_disabled (反转 enabled)
                true,                    // check_health
                Option::<String>::None,  // check_model_name
                "[]",                    // not_supported_models
                row.usage_count as i64,
                row.error_count as i32,
                last_used_ts,
                Option::<i64>::None,     // last_error_time
                Option::<String>::None,  // last_error_message
                Option::<i64>::None,     // last_health_check_time
                Option::<String>::None,  // last_health_check_model
                created_at_ts,
                now,
                "imported",              // source: 标记为导入
                Option::<String>::None,  // proxy_url
            ],
        )
        .map_err(|e| format!("插入凭证失败: {e}"))?;

        tracing::info!(
            "[迁移] 已迁移 API Key: {} -> {} (provider_type: {})",
            row.alias.as_deref().unwrap_or(&row.id),
            uuid,
            pool_provider_type
        );

        migrated_count += 1;
    }

    // 标记迁移完成
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES ('migrated_api_keys_to_pool', 'true')",
        [],
    )
    .map_err(|e| format!("标记迁移完成失败: {e}"))?;

    tracing::info!("[迁移] API Keys 迁移完成，共迁移 {} 条记录", migrated_count);

    Ok(migrated_count)
}

/// API Key 迁移行数据
struct ApiKeyMigrationRow {
    id: String,
    provider_id: String,
    api_key_encrypted: String,
    alias: Option<String>,
    enabled: bool,
    usage_count: u64,
    error_count: u32,
    last_used_at: Option<String>,
    created_at: Option<String>,
    provider_type: String,
    api_host: String,
    provider_name: String,
}

/// 迁移旧的 Provider ID 到新的 ID
///
/// 修复 system_providers.rs 中 Provider ID 与模型注册表 JSON 文件名不匹配的问题。
/// 例如：silicon -> siliconflow, gemini -> google 等
pub fn migrate_provider_ids(conn: &Connection) -> Result<usize, String> {
    // 检查是否已经迁移过
    let migrated: bool = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'migrated_provider_ids_v1'",
            [],
            |row| row.get::<_, String>(0),
        )
        .map(|v| v == "true")
        .unwrap_or(false);

    if migrated {
        tracing::debug!("[迁移] Provider ID 已迁移过，跳过");
        return Ok(0);
    }

    tracing::info!("[迁移] 开始迁移旧的 Provider ID");

    // 定义需要迁移的 ID 映射（旧 ID -> 新 ID）
    let id_mappings = [
        ("silicon", "siliconflow"),
        ("gemini", "google"),
        ("zhipu", "zhipuai"),
        ("dashscope", "alibaba"),
        ("moonshot", "moonshotai"),
        ("grok", "xai"),
        ("github", "github-models"),
        ("copilot", "github-copilot"),
        ("vertexai", "google-vertex"),
        ("aws-bedrock", "amazon-bedrock"),
        ("together", "togetherai"),
        ("fireworks", "fireworks-ai"),
        ("mimo", "xiaomi"),
    ];

    let mut migrated_count = 0;

    for (old_id, new_id) in &id_mappings {
        // 检查旧 ID 是否存在
        let old_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM api_key_providers WHERE id = ?1",
                params![old_id],
                |r| r.get(0),
            )
            .unwrap_or(false);

        if !old_exists {
            continue;
        }

        // 检查新 ID 是否存在
        let new_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM api_key_providers WHERE id = ?1",
                params![new_id],
                |r| r.get(0),
            )
            .unwrap_or(false);

        // 检查旧 ID 是否有 API Keys
        let has_keys: bool = conn
            .query_row(
                "SELECT COUNT(*) > 0 FROM api_keys WHERE provider_id = ?1",
                params![old_id],
                |r| r.get(0),
            )
            .unwrap_or(false);

        if has_keys {
            // 如果旧 ID 有 API Keys，需要迁移到新 ID
            if new_exists {
                // 新 ID 已存在，将 API Keys 迁移过去
                conn.execute(
                    "UPDATE api_keys SET provider_id = ?1 WHERE provider_id = ?2",
                    params![new_id, old_id],
                )
                .map_err(|e| format!("迁移 API Keys 失败: {e}"))?;

                tracing::info!("[迁移] 已将 {} 的 API Keys 迁移到 {}", old_id, new_id);
            } else {
                // 新 ID 不存在，直接更新旧 ID
                conn.execute(
                    "UPDATE api_key_providers SET id = ?1 WHERE id = ?2",
                    params![new_id, old_id],
                )
                .map_err(|e| format!("更新 Provider ID 失败: {e}"))?;

                conn.execute(
                    "UPDATE api_keys SET provider_id = ?1 WHERE provider_id = ?2",
                    params![new_id, old_id],
                )
                .map_err(|e| format!("更新 API Keys provider_id 失败: {e}"))?;

                tracing::info!("[迁移] 已将 Provider {} 重命名为 {}", old_id, new_id);
                migrated_count += 1;
                continue;
            }
        }

        // 删除旧的 Provider（无论是否有 API Keys，因为 Keys 已迁移）
        conn.execute(
            "DELETE FROM api_key_providers WHERE id = ?1",
            params![old_id],
        )
        .map_err(|e| format!("删除旧 Provider 失败: {e}"))?;

        tracing::info!("[迁移] 已删除旧 Provider: {}", old_id);
        migrated_count += 1;
    }

    // 标记迁移完成
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES ('migrated_provider_ids_v1', 'true')",
        [],
    )
    .map_err(|e| format!("标记迁移完成失败: {e}"))?;

    if migrated_count > 0 {
        tracing::info!(
            "[迁移] Provider ID 迁移完成，共处理 {} 个 Provider",
            migrated_count
        );
    }

    Ok(migrated_count)
}

/// 清理旧的 API Key 凭证（OpenAIKey 和 ClaudeKey 类型）
///
/// 这些凭证是通过旧的 UI 添加的，现在已经被新的 API Key Provider 系统取代。
/// 此函数会删除 provider_pool_credentials 表中的 openai_key 和 claude_key 类型凭证。
pub fn cleanup_legacy_api_key_credentials(conn: &Connection) -> Result<usize, String> {
    // 检查是否已经清理过
    let cleaned: bool = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'cleaned_legacy_api_key_credentials'",
            [],
            |row| row.get::<_, String>(0),
        )
        .map(|v| v == "true")
        .unwrap_or(false);

    if cleaned {
        tracing::debug!("[清理] 旧 API Key 凭证已清理过，跳过");
        return Ok(0);
    }

    tracing::info!("[清理] 开始清理旧的 API Key 凭证（openai_key, claude_key 类型）");

    // 查询需要清理的凭证数量
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM provider_pool_credentials
             WHERE credential_data LIKE '%\"type\":\"openai_key\"%'
                OR credential_data LIKE '%\"type\":\"claude_key\"%'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    if count == 0 {
        tracing::info!("[清理] 没有需要清理的旧 API Key 凭证");
        // 标记清理完成
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('cleaned_legacy_api_key_credentials', 'true')",
            [],
        )
        .map_err(|e| format!("标记清理完成失败: {e}"))?;
        return Ok(0);
    }

    // 记录将要删除的凭证信息
    let mut stmt = conn
        .prepare(
            "SELECT uuid, name, provider_type, credential_data
             FROM provider_pool_credentials
             WHERE credential_data LIKE '%\"type\":\"openai_key\"%'
                OR credential_data LIKE '%\"type\":\"claude_key\"%'",
        )
        .map_err(|e| format!("准备查询语句失败: {e}"))?;

    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| format!("查询旧凭证失败: {e}"))?;

    for row_result in rows {
        if let Ok((uuid, name, provider_type)) = row_result {
            tracing::info!(
                "[清理] 将删除旧凭证: {} (name: {}, type: {})",
                uuid,
                name.as_deref().unwrap_or("未命名"),
                provider_type
            );
        }
    }

    // 删除旧的 API Key 凭证
    let deleted = conn
        .execute(
            "DELETE FROM provider_pool_credentials
             WHERE credential_data LIKE '%\"type\":\"openai_key\"%'
                OR credential_data LIKE '%\"type\":\"claude_key\"%'",
            [],
        )
        .map_err(|e| format!("删除旧凭证失败: {e}"))?;

    // 标记清理完成
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES ('cleaned_legacy_api_key_credentials', 'true')",
        [],
    )
    .map_err(|e| format!("标记清理完成失败: {e}"))?;

    tracing::info!("[清理] 旧 API Key 凭证清理完成，共删除 {} 条记录", deleted);

    Ok(deleted)
}

/// 修复历史 MCP 导入数据：补齐 enabled_proxycast
///
/// 早期版本从 Claude/Codex/Gemini 导入 MCP 时，默认写入 enabled_proxycast=0，
/// 导致 ProxyCast 本身不会使用这些服务器。
///
/// 迁移策略：
/// - 仅处理 enabled_proxycast=0 的记录
/// - 且至少在一个外部应用中启用（enabled_claude/codex/gemini 任一为 1）
/// - 将 enabled_proxycast 设为 1
pub fn migrate_mcp_proxycast_enabled(conn: &Connection) -> Result<usize, String> {
    // 检查是否已经迁移过
    let migrated: bool = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'migrated_mcp_proxycast_enabled'",
            [],
            |row| row.get::<_, String>(0),
        )
        .map(|v| v == "true")
        .unwrap_or(false);

    if migrated {
        tracing::debug!("[迁移] MCP proxycast 启用状态已迁移过，跳过");
        return Ok(0);
    }

    let updated = conn
        .execute(
            "UPDATE mcp_servers
             SET enabled_proxycast = 1
             WHERE enabled_proxycast = 0
               AND (enabled_claude = 1 OR enabled_codex = 1 OR enabled_gemini = 1)",
            [],
        )
        .map_err(|e| format!("修复 MCP enabled_proxycast 失败: {e}"))?;

    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES ('migrated_mcp_proxycast_enabled', 'true')",
        [],
    )
    .map_err(|e| format!("标记 MCP proxycast 迁移完成失败: {e}"))?;

    tracing::info!(
        "[迁移] MCP proxycast 启用状态修复完成，更新 {} 条记录",
        updated
    );

    Ok(updated)
}

/// 当前模型注册表版本
/// 每次更新模型数据结构或添加新 Provider 时，增加此版本号
const MODEL_REGISTRY_VERSION: &str = "2026.01.16.1";

/// 标记需要刷新模型注册表
pub fn mark_model_registry_refresh_needed(conn: &Connection) {
    let _ = conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES ('model_registry_refresh_needed', 'true')",
        [],
    );
    tracing::info!("[迁移] 已标记需要刷新模型注册表");
}

/// 检查模型注册表版本，如果版本不匹配则标记需要刷新
pub fn check_model_registry_version(conn: &Connection) {
    let current_version: Option<String> = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'model_registry_version'",
            [],
            |row| row.get(0),
        )
        .ok();

    if current_version.as_deref() != Some(MODEL_REGISTRY_VERSION) {
        tracing::info!(
            "[迁移] 模型注册表版本不匹配: {:?} -> {}，标记需要刷新",
            current_version,
            MODEL_REGISTRY_VERSION
        );
        mark_model_registry_refresh_needed(conn);

        // 更新版本号
        let _ = conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('model_registry_version', ?1)",
            params![MODEL_REGISTRY_VERSION],
        );
    }
}

/// 检查是否需要刷新模型注册表
pub fn is_model_registry_refresh_needed(conn: &Connection) -> bool {
    conn.query_row(
        "SELECT value FROM settings WHERE key = 'model_registry_refresh_needed'",
        [],
        |row| row.get::<_, String>(0),
    )
    .map(|v| v == "true")
    .unwrap_or(false)
}

/// 清除模型注册表刷新标记
pub fn clear_model_registry_refresh_flag(conn: &Connection) {
    let _ = conn.execute(
        "DELETE FROM settings WHERE key = 'model_registry_refresh_needed'",
        [],
    );
}

// ============================================================================
// General Chat 数据迁移到统一表
// ============================================================================

/// 执行 General Chat 数据迁移到统一表
///
/// 将 general_chat_sessions/messages 数据迁移到 agent_sessions/messages 表
/// - general_chat_sessions → agent_sessions (mode 前缀为 "general:")
/// - general_chat_messages → agent_messages
pub fn migrate_general_chat_to_unified(conn: &Connection) -> Result<usize, String> {
    // 检查是否已经迁移过
    let migrated: bool = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'migrated_general_chat_to_unified'",
            [],
            |row| row.get::<_, String>(0),
        )
        .map(|v| v == "true")
        .unwrap_or(false);

    if migrated {
        tracing::debug!("[迁移] General Chat 已迁移过，跳过");
        return Ok(0);
    }

    // 检查是否有数据需要迁移
    let general_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM general_chat_sessions", [], |row| {
            row.get(0)
        })
        .unwrap_or(0);

    if general_count == 0 {
        tracing::info!("[迁移] general_chat_sessions 表为空，无需迁移");
        // 标记迁移完成
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES ('migrated_general_chat_to_unified', 'true')",
            [],
        )
        .map_err(|e| format!("标记迁移完成失败: {e}"))?;
        return Ok(0);
    }

    tracing::info!(
        "[迁移] 开始迁移 {} 个 general_chat 会话到统一表",
        general_count
    );

    // 迁移会话
    let migrated_sessions = migrate_general_sessions(conn)?;
    tracing::info!("[迁移] 迁移了 {} 个会话", migrated_sessions);

    // 迁移消息
    let migrated_messages = migrate_general_messages(conn)?;
    tracing::info!("[迁移] 迁移了 {} 条消息", migrated_messages);

    // 标记迁移完成
    conn.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES ('migrated_general_chat_to_unified', 'true')",
        [],
    )
    .map_err(|e| format!("标记迁移完成失败: {e}"))?;

    tracing::info!("[迁移] General Chat 数据迁移完成！");
    Ok(migrated_sessions + migrated_messages)
}

/// 迁移 General Chat 会话数据
fn migrate_general_sessions(conn: &Connection) -> Result<usize, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, created_at, updated_at, metadata 
             FROM general_chat_sessions",
        )
        .map_err(|e| format!("准备查询语句失败: {e}"))?;

    let sessions: Vec<(String, String, i64, i64, Option<String>)> = stmt
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))
        })
        .map_err(|e| format!("查询会话失败: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    let mut count = 0;
    for (id, name, created_at, updated_at, _metadata) in sessions {
        // 检查是否已存在
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM agent_sessions WHERE id = ?",
                params![&id],
                |_| Ok(true),
            )
            .unwrap_or(false);

        if exists {
            tracing::warn!("[迁移] 会话 {} 已存在，跳过", id);
            continue;
        }

        // 转换时间戳格式（general_chat 用毫秒，agent 用 RFC3339）
        let created_str = timestamp_ms_to_rfc3339(created_at);
        let updated_str = timestamp_ms_to_rfc3339(updated_at);

        // 插入到 agent_sessions，model 字段使用 "general:default" 标识模式
        conn.execute(
            "INSERT INTO agent_sessions (id, model, system_prompt, title, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id,
                "general:default", // 标识为 general 模式
                Option::<String>::None,
                name,
                created_str,
                updated_str,
            ],
        )
        .map_err(|e| format!("插入会话失败: {e}"))?;

        count += 1;
    }

    Ok(count)
}

/// 迁移 General Chat 消息数据
fn migrate_general_messages(conn: &Connection) -> Result<usize, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, session_id, role, content, blocks, status, created_at, metadata
             FROM general_chat_messages",
        )
        .map_err(|e| format!("准备查询语句失败: {e}"))?;

    #[allow(clippy::type_complexity)]
    let messages: Vec<(
        String,
        String,
        String,
        String,
        Option<String>,
        String,
        i64,
        Option<String>,
    )> = stmt
        .query_map([], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
            ))
        })
        .map_err(|e| format!("查询消息失败: {e}"))?
        .filter_map(|r| r.ok())
        .collect();

    let mut count = 0;
    for (_id, session_id, role, content, blocks, _status, created_at, _metadata) in messages {
        // 检查会话是否存在于 agent_sessions
        let session_exists: bool = conn
            .query_row(
                "SELECT 1 FROM agent_sessions WHERE id = ?",
                params![&session_id],
                |_| Ok(true),
            )
            .unwrap_or(false);

        if !session_exists {
            tracing::warn!("[迁移] 消息的会话 {} 不存在，跳过", session_id);
            continue;
        }

        // 转换内容格式
        let content_json = convert_general_content_to_json(&content, &blocks);
        let timestamp_str = timestamp_ms_to_rfc3339(created_at);

        // 插入到 agent_messages
        conn.execute(
            "INSERT INTO agent_messages (session_id, role, content_json, timestamp, tool_calls_json, tool_call_id)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                session_id,
                role,
                content_json,
                timestamp_str,
                Option::<String>::None,
                Option::<String>::None,
            ],
        )
        .map_err(|e| format!("插入消息失败: {e}"))?;

        count += 1;
    }

    Ok(count)
}

/// 将毫秒时间戳转换为 RFC3339 格式
fn timestamp_ms_to_rfc3339(timestamp_ms: i64) -> String {
    use chrono::{TimeZone, Utc};

    let secs = timestamp_ms / 1000;
    let nsecs = ((timestamp_ms % 1000) * 1_000_000) as u32;

    match Utc.timestamp_opt(secs, nsecs) {
        chrono::LocalResult::Single(dt) => dt.to_rfc3339(),
        _ => Utc::now().to_rfc3339(),
    }
}

/// 将 General Chat 内容转换为 JSON 格式
fn convert_general_content_to_json(content: &str, blocks: &Option<String>) -> String {
    // 如果有 blocks，尝试解析并转换
    if let Some(blocks_str) = blocks {
        if let Ok(blocks_arr) = serde_json::from_str::<Vec<serde_json::Value>>(blocks_str) {
            let converted: Vec<serde_json::Value> = blocks_arr
                .into_iter()
                .map(|block| {
                    if let Some(block_type) = block.get("type").and_then(|t| t.as_str()) {
                        match block_type {
                            "text" => {
                                let text =
                                    block.get("content").and_then(|c| c.as_str()).unwrap_or("");
                                serde_json::json!({"type": "text", "text": text})
                            }
                            "image" => {
                                let url =
                                    block.get("content").and_then(|c| c.as_str()).unwrap_or("");
                                serde_json::json!({"type": "image", "url": url})
                            }
                            _ => serde_json::json!({"type": "text", "text": content}),
                        }
                    } else {
                        serde_json::json!({"type": "text", "text": content})
                    }
                })
                .collect();

            if let Ok(json_str) = serde_json::to_string(&converted) {
                return json_str;
            }
        }
    }

    // 默认：纯文本格式
    serde_json::json!([{"type": "text", "text": content}]).to_string()
}

/// 检查 General Chat 迁移状态
#[allow(dead_code)]
pub fn check_general_chat_migration_status(conn: &Connection) -> GeneralChatMigrationStatus {
    let general_sessions: i64 = conn
        .query_row("SELECT COUNT(*) FROM general_chat_sessions", [], |row| {
            row.get(0)
        })
        .unwrap_or(0);

    let general_messages: i64 = conn
        .query_row("SELECT COUNT(*) FROM general_chat_messages", [], |row| {
            row.get(0)
        })
        .unwrap_or(0);

    let unified_general_sessions: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM agent_sessions WHERE model LIKE 'general:%'",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    GeneralChatMigrationStatus {
        general_sessions_count: general_sessions as usize,
        general_messages_count: general_messages as usize,
        migrated_sessions_count: unified_general_sessions as usize,
        needs_migration: general_sessions > 0 && unified_general_sessions == 0,
    }
}

/// General Chat 迁移状态
#[derive(Debug)]
#[allow(dead_code)]
pub struct GeneralChatMigrationStatus {
    pub general_sessions_count: usize,
    pub general_messages_count: usize,
    pub migrated_sessions_count: usize,
    pub needs_migration: bool,
}
