use rusqlite::Connection;

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
                .map_err(|e| format!("备份旧配置失败: {}", e))?;
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
