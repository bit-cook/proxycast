//! 发布配置数据访问层
//!
//! 提供发布配置（PublishConfig）的 CRUD 操作，包括：
//! - 创建、获取、列表、更新发布配置
//! - 按平台获取配置
//! - 更新发布统计
//!
//! ## 相关需求
//! - Requirements 9.1: 发布配置列表显示
//! - Requirements 9.4: 凭证加密存储

use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::errors::project_error::PublishConfigError;
use crate::models::project_model::PublishConfig;

// ============================================================================
// 数据访问对象
// ============================================================================

/// 发布配置 DAO
///
/// 提供发布配置的数据库操作方法。
pub struct PublishConfigDao;

impl PublishConfigDao {
    // ------------------------------------------------------------------------
    // 创建发布配置
    // ------------------------------------------------------------------------

    /// 创建新的发布配置
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `project_id`: 项目 ID
    /// - `platform`: 平台名称
    ///
    /// # 返回
    /// - 成功返回创建的发布配置
    /// - 失败返回 PublishConfigError
    ///
    /// # 注意
    /// - 每个项目的每个平台只能有一个配置（UNIQUE 约束）
    pub fn create(
        conn: &Connection,
        project_id: &str,
        platform: &str,
    ) -> Result<PublishConfig, PublishConfigError> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "INSERT INTO publish_configs (
                id, project_id, platform, is_configured, credentials_encrypted,
                last_published_at, publish_count, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                id,
                project_id,
                platform,
                0,                      // is_configured
                Option::<String>::None, // credentials_encrypted
                Option::<i64>::None,    // last_published_at
                0,                      // publish_count
                now,
                now,
            ],
        )?;

        Ok(PublishConfig {
            id,
            project_id: project_id.to_string(),
            platform: platform.to_string(),
            is_configured: false,
            last_published_at: None,
            publish_count: 0,
            created_at: now,
            updated_at: now,
        })
    }

    // ------------------------------------------------------------------------
    // 获取发布配置
    // ------------------------------------------------------------------------

    /// 获取单个发布配置
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `id`: 配置 ID
    ///
    /// # 返回
    /// - 成功返回 Option<PublishConfig>
    /// - 失败返回 PublishConfigError
    pub fn get(conn: &Connection, id: &str) -> Result<Option<PublishConfig>, PublishConfigError> {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, platform, is_configured, credentials_encrypted,
                    last_published_at, publish_count, created_at, updated_at
             FROM publish_configs WHERE id = ?",
        )?;

        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::map_row(row)?))
        } else {
            Ok(None)
        }
    }

    /// 按项目和平台获取发布配置
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `project_id`: 项目 ID
    /// - `platform`: 平台名称
    ///
    /// # 返回
    /// - 成功返回 Option<PublishConfig>
    /// - 失败返回 PublishConfigError
    pub fn get_by_platform(
        conn: &Connection,
        project_id: &str,
        platform: &str,
    ) -> Result<Option<PublishConfig>, PublishConfigError> {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, platform, is_configured, credentials_encrypted,
                    last_published_at, publish_count, created_at, updated_at
             FROM publish_configs WHERE project_id = ? AND platform = ?",
        )?;

        let mut rows = stmt.query(params![project_id, platform])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::map_row(row)?))
        } else {
            Ok(None)
        }
    }

    // ------------------------------------------------------------------------
    // 列表发布配置
    // ------------------------------------------------------------------------

    /// 获取项目的所有发布配置
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `project_id`: 项目 ID
    ///
    /// # 返回
    /// - 成功返回发布配置列表
    /// - 失败返回 PublishConfigError
    pub fn list(
        conn: &Connection,
        project_id: &str,
    ) -> Result<Vec<PublishConfig>, PublishConfigError> {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, platform, is_configured, credentials_encrypted,
                    last_published_at, publish_count, created_at, updated_at
             FROM publish_configs WHERE project_id = ? ORDER BY created_at DESC",
        )?;

        let configs: Vec<PublishConfig> = stmt
            .query_map([project_id], Self::map_row)?
            .filter_map(|r| r.ok())
            .collect();

        Ok(configs)
    }

    // ------------------------------------------------------------------------
    // 更新发布配置
    // ------------------------------------------------------------------------

    /// 更新发布配置
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `id`: 配置 ID
    /// - `is_configured`: 是否已配置
    /// - `credentials_encrypted`: 加密后的凭证（可选）
    ///
    /// # 返回
    /// - 成功返回更新后的发布配置
    /// - 失败返回 PublishConfigError
    pub fn update(
        conn: &Connection,
        id: &str,
        is_configured: bool,
        credentials_encrypted: Option<String>,
    ) -> Result<PublishConfig, PublishConfigError> {
        // 先验证配置存在
        Self::get(conn, id)?.ok_or_else(|| PublishConfigError::NotFound(id.to_string()))?;

        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "UPDATE publish_configs SET
                is_configured = ?1, credentials_encrypted = ?2, updated_at = ?3
             WHERE id = ?4",
            params![is_configured, credentials_encrypted, now, id],
        )?;

        Self::get(conn, id)?.ok_or_else(|| PublishConfigError::NotFound(id.to_string()))
    }

    /// 记录发布操作
    ///
    /// 更新最后发布时间和发布次数。
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `id`: 配置 ID
    ///
    /// # 返回
    /// - 成功返回更新后的发布配置
    /// - 失败返回 PublishConfigError
    pub fn record_publish(
        conn: &Connection,
        id: &str,
    ) -> Result<PublishConfig, PublishConfigError> {
        // 先验证配置存在
        Self::get(conn, id)?.ok_or_else(|| PublishConfigError::NotFound(id.to_string()))?;

        let now = chrono::Utc::now().timestamp();

        conn.execute(
            "UPDATE publish_configs SET
                last_published_at = ?1, publish_count = publish_count + 1, updated_at = ?2
             WHERE id = ?3",
            params![now, now, id],
        )?;

        Self::get(conn, id)?.ok_or_else(|| PublishConfigError::NotFound(id.to_string()))
    }

    // ------------------------------------------------------------------------
    // 删除发布配置
    // ------------------------------------------------------------------------

    /// 删除发布配置
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `id`: 配置 ID
    ///
    /// # 返回
    /// - 成功返回 ()
    /// - 失败返回 PublishConfigError
    pub fn delete(conn: &Connection, id: &str) -> Result<(), PublishConfigError> {
        let rows = conn.execute("DELETE FROM publish_configs WHERE id = ?", [id])?;

        if rows == 0 {
            return Err(PublishConfigError::NotFound(id.to_string()));
        }

        Ok(())
    }

    /// 删除项目的所有发布配置
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `project_id`: 项目 ID
    ///
    /// # 返回
    /// - 成功返回删除的数量
    /// - 失败返回 PublishConfigError
    pub fn delete_by_project(
        conn: &Connection,
        project_id: &str,
    ) -> Result<usize, PublishConfigError> {
        let rows = conn.execute(
            "DELETE FROM publish_configs WHERE project_id = ?",
            [project_id],
        )?;
        Ok(rows)
    }

    // ------------------------------------------------------------------------
    // 批量操作
    // ------------------------------------------------------------------------

    /// 获取项目的发布配置数量
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `project_id`: 项目 ID
    ///
    /// # 返回
    /// - 成功返回配置数量
    /// - 失败返回 PublishConfigError
    pub fn count(conn: &Connection, project_id: &str) -> Result<i64, PublishConfigError> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM publish_configs WHERE project_id = ?",
            [project_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// 获取或创建发布配置
    ///
    /// 如果指定项目和平台的配置已存在，返回现有配置；
    /// 否则创建新配置。
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `project_id`: 项目 ID
    /// - `platform`: 平台名称
    ///
    /// # 返回
    /// - 成功返回发布配置
    /// - 失败返回 PublishConfigError
    pub fn get_or_create(
        conn: &Connection,
        project_id: &str,
        platform: &str,
    ) -> Result<PublishConfig, PublishConfigError> {
        if let Some(config) = Self::get_by_platform(conn, project_id, platform)? {
            return Ok(config);
        }
        Self::create(conn, project_id, platform)
    }

    // ------------------------------------------------------------------------
    // 辅助方法
    // ------------------------------------------------------------------------

    /// 映射数据库行到 PublishConfig 结构体
    fn map_row(row: &rusqlite::Row) -> Result<PublishConfig, rusqlite::Error> {
        Ok(PublishConfig {
            id: row.get(0)?,
            project_id: row.get(1)?,
            platform: row.get(2)?,
            is_configured: row.get::<_, i32>(3)? != 0,
            // credentials_encrypted 不返回给前端，保持安全
            // 这里我们跳过它，但在内部使用时可以获取
            last_published_at: row.get(5)?,
            publish_count: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        })
    }

    /// 获取加密凭证（内部使用）
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `id`: 配置 ID
    ///
    /// # 返回
    /// - 成功返回加密凭证（如果有）
    /// - 失败返回 PublishConfigError
    pub fn get_credentials(
        conn: &Connection,
        id: &str,
    ) -> Result<Option<String>, PublishConfigError> {
        let credentials: Option<String> = conn.query_row(
            "SELECT credentials_encrypted FROM publish_configs WHERE id = ?",
            [id],
            |row| row.get(0),
        )?;
        Ok(credentials)
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::schema::create_tables;

    /// 创建测试数据库连接
    fn setup_test_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        create_tables(&conn).unwrap();
        conn
    }

    /// 创建测试项目
    fn create_test_project(conn: &Connection, id: &str) {
        let now = chrono::Utc::now().timestamp();
        conn.execute(
            "INSERT INTO workspaces (id, name, workspace_type, root_path, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                id,
                "测试项目",
                "persistent",
                format!("/test/{}", id),
                now,
                now
            ],
        )
        .unwrap();
    }

    #[test]
    fn test_create_publish_config() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        let config = PublishConfigDao::create(&conn, "project-1", "xiaohongshu").unwrap();

        assert!(!config.id.is_empty());
        assert_eq!(config.project_id, "project-1");
        assert_eq!(config.platform, "xiaohongshu");
        assert!(!config.is_configured);
        assert!(config.last_published_at.is_none());
        assert_eq!(config.publish_count, 0);
    }

    #[test]
    fn test_create_duplicate_platform() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        // 第一次创建成功
        PublishConfigDao::create(&conn, "project-1", "xiaohongshu").unwrap();

        // 第二次创建同一平台应该失败（UNIQUE 约束）
        let result = PublishConfigDao::create(&conn, "project-1", "xiaohongshu");
        assert!(result.is_err());
    }

    #[test]
    fn test_get_publish_config() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        let created = PublishConfigDao::create(&conn, "project-1", "wechat").unwrap();
        let fetched = PublishConfigDao::get(&conn, &created.id).unwrap();

        assert!(fetched.is_some());
        let fetched = fetched.unwrap();
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.platform, "wechat");
    }

    #[test]
    fn test_get_nonexistent_config() {
        let conn = setup_test_db();
        let result = PublishConfigDao::get(&conn, "nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_get_by_platform() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        PublishConfigDao::create(&conn, "project-1", "xiaohongshu").unwrap();
        PublishConfigDao::create(&conn, "project-1", "wechat").unwrap();

        // 获取小红书配置
        let config = PublishConfigDao::get_by_platform(&conn, "project-1", "xiaohongshu")
            .unwrap()
            .unwrap();
        assert_eq!(config.platform, "xiaohongshu");

        // 获取微信配置
        let config = PublishConfigDao::get_by_platform(&conn, "project-1", "wechat")
            .unwrap()
            .unwrap();
        assert_eq!(config.platform, "wechat");

        // 获取不存在的平台
        let config = PublishConfigDao::get_by_platform(&conn, "project-1", "zhihu").unwrap();
        assert!(config.is_none());
    }

    #[test]
    fn test_list_publish_configs() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");
        create_test_project(&conn, "project-2");

        // 为 project-1 创建两个配置
        PublishConfigDao::create(&conn, "project-1", "xiaohongshu").unwrap();
        PublishConfigDao::create(&conn, "project-1", "wechat").unwrap();

        // 为 project-2 创建一个配置
        PublishConfigDao::create(&conn, "project-2", "zhihu").unwrap();

        // 验证 project-1 有 2 个配置
        let configs = PublishConfigDao::list(&conn, "project-1").unwrap();
        assert_eq!(configs.len(), 2);

        // 验证 project-2 有 1 个配置
        let configs = PublishConfigDao::list(&conn, "project-2").unwrap();
        assert_eq!(configs.len(), 1);
    }

    #[test]
    fn test_update_publish_config() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        let created = PublishConfigDao::create(&conn, "project-1", "xiaohongshu").unwrap();
        assert!(!created.is_configured);

        // 更新配置
        let updated = PublishConfigDao::update(
            &conn,
            &created.id,
            true,
            Some("encrypted_credentials_here".to_string()),
        )
        .unwrap();

        assert!(updated.is_configured);
        assert!(updated.updated_at >= created.updated_at);
    }

    #[test]
    fn test_update_nonexistent_config() {
        let conn = setup_test_db();
        let result = PublishConfigDao::update(&conn, "nonexistent", true, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_record_publish() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        let created = PublishConfigDao::create(&conn, "project-1", "xiaohongshu").unwrap();
        assert_eq!(created.publish_count, 0);
        assert!(created.last_published_at.is_none());

        // 记录第一次发布
        let updated = PublishConfigDao::record_publish(&conn, &created.id).unwrap();
        assert_eq!(updated.publish_count, 1);
        assert!(updated.last_published_at.is_some());

        // 记录第二次发布
        let updated = PublishConfigDao::record_publish(&conn, &created.id).unwrap();
        assert_eq!(updated.publish_count, 2);
    }

    #[test]
    fn test_delete_publish_config() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        let created = PublishConfigDao::create(&conn, "project-1", "xiaohongshu").unwrap();

        // 验证配置存在
        assert!(PublishConfigDao::get(&conn, &created.id).unwrap().is_some());

        // 删除配置
        PublishConfigDao::delete(&conn, &created.id).unwrap();

        // 验证配置已删除
        assert!(PublishConfigDao::get(&conn, &created.id).unwrap().is_none());
    }

    #[test]
    fn test_delete_nonexistent_config() {
        let conn = setup_test_db();
        let result = PublishConfigDao::delete(&conn, "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_by_project() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");
        create_test_project(&conn, "project-2");

        // 为 project-1 创建 2 个配置
        PublishConfigDao::create(&conn, "project-1", "xiaohongshu").unwrap();
        PublishConfigDao::create(&conn, "project-1", "wechat").unwrap();

        // 为 project-2 创建 1 个配置
        PublishConfigDao::create(&conn, "project-2", "zhihu").unwrap();

        // 删除 project-1 的所有配置
        let deleted_count = PublishConfigDao::delete_by_project(&conn, "project-1").unwrap();

        // 验证删除了 2 个配置
        assert_eq!(deleted_count, 2);

        // 验证 project-1 没有配置了
        let configs = PublishConfigDao::list(&conn, "project-1").unwrap();
        assert_eq!(configs.len(), 0);

        // 验证 project-2 的配置未受影响
        let configs = PublishConfigDao::list(&conn, "project-2").unwrap();
        assert_eq!(configs.len(), 1);
    }

    #[test]
    fn test_count_publish_configs() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        // 初始数量为 0
        let count = PublishConfigDao::count(&conn, "project-1").unwrap();
        assert_eq!(count, 0);

        // 创建 3 个配置
        PublishConfigDao::create(&conn, "project-1", "xiaohongshu").unwrap();
        PublishConfigDao::create(&conn, "project-1", "wechat").unwrap();
        PublishConfigDao::create(&conn, "project-1", "zhihu").unwrap();

        // 验证数量为 3
        let count = PublishConfigDao::count(&conn, "project-1").unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_get_or_create() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        // 第一次调用应该创建
        let config1 = PublishConfigDao::get_or_create(&conn, "project-1", "xiaohongshu").unwrap();
        assert_eq!(config1.platform, "xiaohongshu");

        // 第二次调用应该返回现有配置
        let config2 = PublishConfigDao::get_or_create(&conn, "project-1", "xiaohongshu").unwrap();
        assert_eq!(config1.id, config2.id);

        // 验证只有一个配置
        let count = PublishConfigDao::count(&conn, "project-1").unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_get_credentials() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        let created = PublishConfigDao::create(&conn, "project-1", "xiaohongshu").unwrap();

        // 初始没有凭证
        let credentials = PublishConfigDao::get_credentials(&conn, &created.id).unwrap();
        assert!(credentials.is_none());

        // 更新凭证
        PublishConfigDao::update(
            &conn,
            &created.id,
            true,
            Some("encrypted_secret".to_string()),
        )
        .unwrap();

        // 获取凭证
        let credentials = PublishConfigDao::get_credentials(&conn, &created.id).unwrap();
        assert_eq!(credentials, Some("encrypted_secret".to_string()));
    }

    #[test]
    fn test_project_scoped_query_correctness() {
        // Property 2: Project-Scoped Query Correctness
        // 验证按 project_id 筛选的查询只返回属于该项目的配置
        let conn = setup_test_db();
        create_test_project(&conn, "project-a");
        create_test_project(&conn, "project-b");

        // 为两个项目创建配置
        PublishConfigDao::create(&conn, "project-a", "xiaohongshu").unwrap();
        PublishConfigDao::create(&conn, "project-a", "wechat").unwrap();
        PublishConfigDao::create(&conn, "project-a", "zhihu").unwrap();

        PublishConfigDao::create(&conn, "project-b", "weibo").unwrap();
        PublishConfigDao::create(&conn, "project-b", "douyin").unwrap();

        // 查询 project-a 的配置
        let configs_a = PublishConfigDao::list(&conn, "project-a").unwrap();
        assert_eq!(configs_a.len(), 3);
        for c in &configs_a {
            assert_eq!(c.project_id, "project-a");
        }

        // 查询 project-b 的配置
        let configs_b = PublishConfigDao::list(&conn, "project-b").unwrap();
        assert_eq!(configs_b.len(), 2);
        for c in &configs_b {
            assert_eq!(c.project_id, "project-b");
        }
    }

    #[test]
    fn test_platform_uniqueness_per_project() {
        // 验证每个项目的每个平台只能有一个配置
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");
        create_test_project(&conn, "project-2");

        // project-1 创建小红书配置
        PublishConfigDao::create(&conn, "project-1", "xiaohongshu").unwrap();

        // project-2 也可以创建小红书配置（不同项目）
        PublishConfigDao::create(&conn, "project-2", "xiaohongshu").unwrap();

        // project-1 再次创建小红书配置应该失败
        let result = PublishConfigDao::create(&conn, "project-1", "xiaohongshu");
        assert!(result.is_err());
    }
}
