//! 排版模板数据访问层
//!
//! 提供排版模板（Template）的 CRUD 操作，包括：
//! - 创建、获取、列表、更新、删除模板
//! - 设置项目默认模板
//!
//! ## 相关需求
//! - Requirements 8.1: 模板列表显示
//! - Requirements 8.3: 模板创建
//! - Requirements 8.4: 设置默认模板

use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::errors::project_error::TemplateError;
use crate::models::project_model::{CreateTemplateRequest, Template, TemplateUpdate};

// ============================================================================
// 数据访问对象
// ============================================================================

/// 排版模板 DAO
///
/// 提供排版模板的数据库操作方法。
pub struct TemplateDao;

impl TemplateDao {
    // ------------------------------------------------------------------------
    // 创建模板
    // ------------------------------------------------------------------------

    /// 创建新模板
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `req`: 创建模板请求
    ///
    /// # 返回
    /// - 成功返回创建的模板
    /// - 失败返回 TemplateError
    pub fn create(
        conn: &Connection,
        req: &CreateTemplateRequest,
    ) -> Result<Template, TemplateError> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();

        // 使用默认值处理可选字段
        let emoji_usage = req.emoji_usage.as_deref().unwrap_or("moderate");

        conn.execute(
            "INSERT INTO templates (
                id, project_id, name, platform, title_style, paragraph_style,
                ending_style, emoji_usage, hashtag_rules, image_rules,
                is_default, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                id,
                req.project_id,
                req.name,
                req.platform,
                req.title_style,
                req.paragraph_style,
                req.ending_style,
                emoji_usage,
                req.hashtag_rules,
                req.image_rules,
                0, // is_default
                now,
                now,
            ],
        )?;

        // 返回创建的模板
        Ok(Template {
            id,
            project_id: req.project_id.clone(),
            name: req.name.clone(),
            platform: req.platform.clone(),
            title_style: req.title_style.clone(),
            paragraph_style: req.paragraph_style.clone(),
            ending_style: req.ending_style.clone(),
            emoji_usage: emoji_usage.to_string(),
            hashtag_rules: req.hashtag_rules.clone(),
            image_rules: req.image_rules.clone(),
            is_default: false,
            created_at: now,
            updated_at: now,
        })
    }

    // ------------------------------------------------------------------------
    // 获取模板
    // ------------------------------------------------------------------------

    /// 获取单个模板
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `id`: 模板 ID
    ///
    /// # 返回
    /// - 成功返回 Option<Template>
    /// - 失败返回 TemplateError
    pub fn get(conn: &Connection, id: &str) -> Result<Option<Template>, TemplateError> {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, name, platform, title_style, paragraph_style,
                    ending_style, emoji_usage, hashtag_rules, image_rules,
                    is_default, created_at, updated_at
             FROM templates WHERE id = ?",
        )?;

        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::map_row(row)?))
        } else {
            Ok(None)
        }
    }

    // ------------------------------------------------------------------------
    // 列表模板
    // ------------------------------------------------------------------------

    /// 获取项目的模板列表
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `project_id`: 项目 ID
    ///
    /// # 返回
    /// - 成功返回模板列表
    /// - 失败返回 TemplateError
    pub fn list(conn: &Connection, project_id: &str) -> Result<Vec<Template>, TemplateError> {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, name, platform, title_style, paragraph_style,
                    ending_style, emoji_usage, hashtag_rules, image_rules,
                    is_default, created_at, updated_at
             FROM templates WHERE project_id = ? ORDER BY created_at DESC",
        )?;

        let templates: Vec<Template> = stmt
            .query_map([project_id], |row| Self::map_row(row))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(templates)
    }

    // ------------------------------------------------------------------------
    // 更新模板
    // ------------------------------------------------------------------------

    /// 更新模板
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `id`: 模板 ID
    /// - `update`: 更新内容
    ///
    /// # 返回
    /// - 成功返回更新后的模板
    /// - 失败返回 TemplateError
    pub fn update(
        conn: &Connection,
        id: &str,
        update: &TemplateUpdate,
    ) -> Result<Template, TemplateError> {
        // 先获取现有模板
        let existing =
            Self::get(conn, id)?.ok_or_else(|| TemplateError::NotFound(id.to_string()))?;

        let now = chrono::Utc::now().timestamp();

        // 构建更新后的值
        let name = update.name.as_ref().unwrap_or(&existing.name);
        let title_style = update.title_style.clone().or(existing.title_style);
        let paragraph_style = update.paragraph_style.clone().or(existing.paragraph_style);
        let ending_style = update.ending_style.clone().or(existing.ending_style);
        let emoji_usage = update.emoji_usage.as_ref().unwrap_or(&existing.emoji_usage);
        let hashtag_rules = update.hashtag_rules.clone().or(existing.hashtag_rules);
        let image_rules = update.image_rules.clone().or(existing.image_rules);

        conn.execute(
            "UPDATE templates SET
                name = ?1, title_style = ?2, paragraph_style = ?3, ending_style = ?4,
                emoji_usage = ?5, hashtag_rules = ?6, image_rules = ?7, updated_at = ?8
             WHERE id = ?9",
            params![
                name,
                title_style,
                paragraph_style,
                ending_style,
                emoji_usage,
                hashtag_rules,
                image_rules,
                now,
                id,
            ],
        )?;

        // 返回更新后的模板
        Self::get(conn, id)?.ok_or_else(|| TemplateError::NotFound(id.to_string()))
    }

    // ------------------------------------------------------------------------
    // 删除模板
    // ------------------------------------------------------------------------

    /// 删除模板
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `id`: 模板 ID
    ///
    /// # 返回
    /// - 成功返回 ()
    /// - 失败返回 TemplateError
    pub fn delete(conn: &Connection, id: &str) -> Result<(), TemplateError> {
        let rows = conn.execute("DELETE FROM templates WHERE id = ?", [id])?;

        if rows == 0 {
            return Err(TemplateError::NotFound(id.to_string()));
        }

        Ok(())
    }

    // ------------------------------------------------------------------------
    // 设置默认模板
    // ------------------------------------------------------------------------

    /// 设置项目的默认模板
    ///
    /// 将指定模板设为默认，同时取消该项目其他模板的默认状态。
    /// 这确保每个项目最多只有一个默认模板。
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `project_id`: 项目 ID
    /// - `template_id`: 要设为默认的模板 ID
    ///
    /// # 返回
    /// - 成功返回 ()
    /// - 失败返回 TemplateError
    pub fn set_default(
        conn: &Connection,
        project_id: &str,
        template_id: &str,
    ) -> Result<(), TemplateError> {
        // 验证模板存在且属于该项目
        let template = Self::get(conn, template_id)?
            .ok_or_else(|| TemplateError::NotFound(template_id.to_string()))?;

        if template.project_id != project_id {
            return Err(TemplateError::ProjectNotFound(project_id.to_string()));
        }

        let now = chrono::Utc::now().timestamp();

        // 先取消该项目所有模板的默认状态
        conn.execute(
            "UPDATE templates SET is_default = 0, updated_at = ?1 WHERE project_id = ?2",
            params![now, project_id],
        )?;

        // 设置指定模板为默认
        conn.execute(
            "UPDATE templates SET is_default = 1, updated_at = ?1 WHERE id = ?2",
            params![now, template_id],
        )?;

        Ok(())
    }

    /// 获取项目的默认模板
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `project_id`: 项目 ID
    ///
    /// # 返回
    /// - 成功返回 Option<Template>
    /// - 失败返回 TemplateError
    pub fn get_default(
        conn: &Connection,
        project_id: &str,
    ) -> Result<Option<Template>, TemplateError> {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, name, platform, title_style, paragraph_style,
                    ending_style, emoji_usage, hashtag_rules, image_rules,
                    is_default, created_at, updated_at
             FROM templates WHERE project_id = ? AND is_default = 1",
        )?;

        let mut rows = stmt.query([project_id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::map_row(row)?))
        } else {
            Ok(None)
        }
    }

    // ------------------------------------------------------------------------
    // 批量操作
    // ------------------------------------------------------------------------

    /// 获取项目的模板数量
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `project_id`: 项目 ID
    ///
    /// # 返回
    /// - 成功返回模板数量
    /// - 失败返回 TemplateError
    pub fn count(conn: &Connection, project_id: &str) -> Result<i64, TemplateError> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM templates WHERE project_id = ?",
            [project_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// 删除项目的所有模板
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `project_id`: 项目 ID
    ///
    /// # 返回
    /// - 成功返回删除的数量
    /// - 失败返回 TemplateError
    pub fn delete_by_project(conn: &Connection, project_id: &str) -> Result<usize, TemplateError> {
        let rows = conn.execute("DELETE FROM templates WHERE project_id = ?", [project_id])?;
        Ok(rows)
    }

    // ------------------------------------------------------------------------
    // 辅助方法
    // ------------------------------------------------------------------------

    /// 映射数据库行到 Template 结构体
    fn map_row(row: &rusqlite::Row) -> Result<Template, rusqlite::Error> {
        Ok(Template {
            id: row.get(0)?,
            project_id: row.get(1)?,
            name: row.get(2)?,
            platform: row.get(3)?,
            title_style: row.get(4)?,
            paragraph_style: row.get(5)?,
            ending_style: row.get(6)?,
            emoji_usage: row.get(7)?,
            hashtag_rules: row.get(8)?,
            image_rules: row.get(9)?,
            is_default: row.get::<_, i32>(10)? != 0,
            created_at: row.get(11)?,
            updated_at: row.get(12)?,
        })
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
    fn test_create_template() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        let req = CreateTemplateRequest {
            project_id: "project-1".to_string(),
            name: "小红书模板".to_string(),
            platform: "xiaohongshu".to_string(),
            title_style: Some("吸引眼球".to_string()),
            paragraph_style: Some("简短有力".to_string()),
            ending_style: Some("引导互动".to_string()),
            emoji_usage: Some("heavy".to_string()),
            hashtag_rules: Some("3-5个相关话题".to_string()),
            image_rules: Some("配图要精美".to_string()),
        };

        let template = TemplateDao::create(&conn, &req).unwrap();

        assert!(!template.id.is_empty());
        assert_eq!(template.project_id, "project-1");
        assert_eq!(template.name, "小红书模板");
        assert_eq!(template.platform, "xiaohongshu");
        assert_eq!(template.emoji_usage, "heavy");
        assert!(!template.is_default);
    }

    #[test]
    fn test_create_template_minimal() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        let req = CreateTemplateRequest {
            project_id: "project-1".to_string(),
            name: "简单模板".to_string(),
            platform: "markdown".to_string(),
            title_style: None,
            paragraph_style: None,
            ending_style: None,
            emoji_usage: None,
            hashtag_rules: None,
            image_rules: None,
        };

        let template = TemplateDao::create(&conn, &req).unwrap();

        assert!(!template.id.is_empty());
        assert_eq!(template.name, "简单模板");
        assert_eq!(template.platform, "markdown");
        // 默认值
        assert_eq!(template.emoji_usage, "moderate");
        assert!(template.title_style.is_none());
    }

    #[test]
    fn test_get_template() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        let req = CreateTemplateRequest {
            project_id: "project-1".to_string(),
            name: "测试模板".to_string(),
            platform: "wechat".to_string(),
            title_style: Some("正式".to_string()),
            paragraph_style: None,
            ending_style: None,
            emoji_usage: Some("minimal".to_string()),
            hashtag_rules: None,
            image_rules: None,
        };

        let created = TemplateDao::create(&conn, &req).unwrap();
        let fetched = TemplateDao::get(&conn, &created.id).unwrap();

        assert!(fetched.is_some());
        let fetched = fetched.unwrap();
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.name, "测试模板");
        assert_eq!(fetched.platform, "wechat");
        assert_eq!(fetched.emoji_usage, "minimal");
    }

    #[test]
    fn test_get_nonexistent_template() {
        let conn = setup_test_db();
        let result = TemplateDao::get(&conn, "nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_list_templates() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");
        create_test_project(&conn, "project-2");

        // 为 project-1 创建两个模板
        for i in 1..=2 {
            let req = CreateTemplateRequest {
                project_id: "project-1".to_string(),
                name: format!("模板{}", i),
                platform: "xiaohongshu".to_string(),
                title_style: None,
                paragraph_style: None,
                ending_style: None,
                emoji_usage: None,
                hashtag_rules: None,
                image_rules: None,
            };
            TemplateDao::create(&conn, &req).unwrap();
        }

        // 为 project-2 创建一个模板
        let req = CreateTemplateRequest {
            project_id: "project-2".to_string(),
            name: "模板3".to_string(),
            platform: "wechat".to_string(),
            title_style: None,
            paragraph_style: None,
            ending_style: None,
            emoji_usage: None,
            hashtag_rules: None,
            image_rules: None,
        };
        TemplateDao::create(&conn, &req).unwrap();

        // 验证 project-1 有 2 个模板
        let templates = TemplateDao::list(&conn, "project-1").unwrap();
        assert_eq!(templates.len(), 2);

        // 验证 project-2 有 1 个模板
        let templates = TemplateDao::list(&conn, "project-2").unwrap();
        assert_eq!(templates.len(), 1);
    }

    #[test]
    fn test_update_template() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        let req = CreateTemplateRequest {
            project_id: "project-1".to_string(),
            name: "原始名称".to_string(),
            platform: "xiaohongshu".to_string(),
            title_style: Some("原始标题风格".to_string()),
            paragraph_style: None,
            ending_style: None,
            emoji_usage: Some("moderate".to_string()),
            hashtag_rules: None,
            image_rules: None,
        };

        let created = TemplateDao::create(&conn, &req).unwrap();

        let update = TemplateUpdate {
            name: Some("更新后名称".to_string()),
            title_style: Some("更新后标题风格".to_string()),
            paragraph_style: Some("新段落风格".to_string()),
            ending_style: None,
            emoji_usage: Some("heavy".to_string()),
            hashtag_rules: Some("5个话题".to_string()),
            image_rules: None,
        };

        let updated = TemplateDao::update(&conn, &created.id, &update).unwrap();

        assert_eq!(updated.name, "更新后名称");
        assert_eq!(updated.title_style, Some("更新后标题风格".to_string()));
        assert_eq!(updated.paragraph_style, Some("新段落风格".to_string()));
        assert_eq!(updated.emoji_usage, "heavy");
        assert_eq!(updated.hashtag_rules, Some("5个话题".to_string()));
        // 验证平台未变
        assert_eq!(updated.platform, "xiaohongshu");
    }

    #[test]
    fn test_update_template_partial() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        let req = CreateTemplateRequest {
            project_id: "project-1".to_string(),
            name: "原始名称".to_string(),
            platform: "wechat".to_string(),
            title_style: Some("原始标题".to_string()),
            paragraph_style: Some("原始段落".to_string()),
            ending_style: None,
            emoji_usage: Some("moderate".to_string()),
            hashtag_rules: None,
            image_rules: None,
        };

        let created = TemplateDao::create(&conn, &req).unwrap();

        // 只更新名称
        let update = TemplateUpdate {
            name: Some("新名称".to_string()),
            title_style: None,
            paragraph_style: None,
            ending_style: None,
            emoji_usage: None,
            hashtag_rules: None,
            image_rules: None,
        };

        let updated = TemplateDao::update(&conn, &created.id, &update).unwrap();

        assert_eq!(updated.name, "新名称");
        // 其他字段保持不变
        assert_eq!(updated.title_style, Some("原始标题".to_string()));
        assert_eq!(updated.paragraph_style, Some("原始段落".to_string()));
        assert_eq!(updated.emoji_usage, "moderate");
    }

    #[test]
    fn test_update_nonexistent_template() {
        let conn = setup_test_db();
        let update = TemplateUpdate::default();
        let result = TemplateDao::update(&conn, "nonexistent", &update);
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_template() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        let req = CreateTemplateRequest {
            project_id: "project-1".to_string(),
            name: "待删除模板".to_string(),
            platform: "markdown".to_string(),
            title_style: None,
            paragraph_style: None,
            ending_style: None,
            emoji_usage: None,
            hashtag_rules: None,
            image_rules: None,
        };

        let created = TemplateDao::create(&conn, &req).unwrap();

        // 验证模板存在
        assert!(TemplateDao::get(&conn, &created.id).unwrap().is_some());

        // 删除模板
        TemplateDao::delete(&conn, &created.id).unwrap();

        // 验证模板已删除
        assert!(TemplateDao::get(&conn, &created.id).unwrap().is_none());
    }

    #[test]
    fn test_delete_nonexistent_template() {
        let conn = setup_test_db();
        let result = TemplateDao::delete(&conn, "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_set_default_template() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        // 创建两个模板
        let req1 = CreateTemplateRequest {
            project_id: "project-1".to_string(),
            name: "模板1".to_string(),
            platform: "xiaohongshu".to_string(),
            title_style: None,
            paragraph_style: None,
            ending_style: None,
            emoji_usage: None,
            hashtag_rules: None,
            image_rules: None,
        };
        let template1 = TemplateDao::create(&conn, &req1).unwrap();

        let req2 = CreateTemplateRequest {
            project_id: "project-1".to_string(),
            name: "模板2".to_string(),
            platform: "wechat".to_string(),
            title_style: None,
            paragraph_style: None,
            ending_style: None,
            emoji_usage: None,
            hashtag_rules: None,
            image_rules: None,
        };
        let template2 = TemplateDao::create(&conn, &req2).unwrap();

        // 设置模板1为默认
        TemplateDao::set_default(&conn, "project-1", &template1.id).unwrap();

        let t1 = TemplateDao::get(&conn, &template1.id).unwrap().unwrap();
        let t2 = TemplateDao::get(&conn, &template2.id).unwrap().unwrap();
        assert!(t1.is_default);
        assert!(!t2.is_default);

        // 设置模板2为默认，模板1应该不再是默认
        TemplateDao::set_default(&conn, "project-1", &template2.id).unwrap();

        let t1 = TemplateDao::get(&conn, &template1.id).unwrap().unwrap();
        let t2 = TemplateDao::get(&conn, &template2.id).unwrap().unwrap();
        assert!(!t1.is_default);
        assert!(t2.is_default);
    }

    #[test]
    fn test_get_default_template() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        // 初始没有默认模板
        let default = TemplateDao::get_default(&conn, "project-1").unwrap();
        assert!(default.is_none());

        // 创建模板并设为默认
        let req = CreateTemplateRequest {
            project_id: "project-1".to_string(),
            name: "默认模板".to_string(),
            platform: "xiaohongshu".to_string(),
            title_style: None,
            paragraph_style: None,
            ending_style: None,
            emoji_usage: None,
            hashtag_rules: None,
            image_rules: None,
        };
        let template = TemplateDao::create(&conn, &req).unwrap();
        TemplateDao::set_default(&conn, "project-1", &template.id).unwrap();

        // 验证可以获取默认模板
        let default = TemplateDao::get_default(&conn, "project-1").unwrap();
        assert!(default.is_some());
        assert_eq!(default.unwrap().id, template.id);
    }

    #[test]
    fn test_set_default_wrong_project() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");
        create_test_project(&conn, "project-2");

        // 在 project-1 创建模板
        let req = CreateTemplateRequest {
            project_id: "project-1".to_string(),
            name: "模板".to_string(),
            platform: "markdown".to_string(),
            title_style: None,
            paragraph_style: None,
            ending_style: None,
            emoji_usage: None,
            hashtag_rules: None,
            image_rules: None,
        };
        let template = TemplateDao::create(&conn, &req).unwrap();

        // 尝试在 project-2 设置该模板为默认，应该失败
        let result = TemplateDao::set_default(&conn, "project-2", &template.id);
        assert!(result.is_err());
    }

    #[test]
    fn test_count_templates() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        // 初始数量为 0
        let count = TemplateDao::count(&conn, "project-1").unwrap();
        assert_eq!(count, 0);

        // 创建 3 个模板
        for i in 1..=3 {
            let req = CreateTemplateRequest {
                project_id: "project-1".to_string(),
                name: format!("模板{}", i),
                platform: "markdown".to_string(),
                title_style: None,
                paragraph_style: None,
                ending_style: None,
                emoji_usage: None,
                hashtag_rules: None,
                image_rules: None,
            };
            TemplateDao::create(&conn, &req).unwrap();
        }

        // 验证数量为 3
        let count = TemplateDao::count(&conn, "project-1").unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_delete_by_project() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");
        create_test_project(&conn, "project-2");

        // 为 project-1 创建 2 个模板
        for i in 1..=2 {
            let req = CreateTemplateRequest {
                project_id: "project-1".to_string(),
                name: format!("模板{}", i),
                platform: "xiaohongshu".to_string(),
                title_style: None,
                paragraph_style: None,
                ending_style: None,
                emoji_usage: None,
                hashtag_rules: None,
                image_rules: None,
            };
            TemplateDao::create(&conn, &req).unwrap();
        }

        // 为 project-2 创建 1 个模板
        let req = CreateTemplateRequest {
            project_id: "project-2".to_string(),
            name: "模板3".to_string(),
            platform: "wechat".to_string(),
            title_style: None,
            paragraph_style: None,
            ending_style: None,
            emoji_usage: None,
            hashtag_rules: None,
            image_rules: None,
        };
        TemplateDao::create(&conn, &req).unwrap();

        // 删除 project-1 的所有模板
        let deleted_count = TemplateDao::delete_by_project(&conn, "project-1").unwrap();

        // 验证删除了 2 个模板
        assert_eq!(deleted_count, 2);

        // 验证 project-1 没有模板了
        let templates = TemplateDao::list(&conn, "project-1").unwrap();
        assert_eq!(templates.len(), 0);

        // 验证 project-2 的模板未受影响
        let templates = TemplateDao::list(&conn, "project-2").unwrap();
        assert_eq!(templates.len(), 1);
    }

    #[test]
    fn test_project_scoped_query_correctness() {
        // Property 2: Project-Scoped Query Correctness
        // 验证按 project_id 筛选的查询只返回属于该项目的模板
        let conn = setup_test_db();
        create_test_project(&conn, "project-a");
        create_test_project(&conn, "project-b");

        // 为两个项目创建模板
        for i in 1..=3 {
            let req = CreateTemplateRequest {
                project_id: "project-a".to_string(),
                name: format!("A模板{}", i),
                platform: "xiaohongshu".to_string(),
                title_style: None,
                paragraph_style: None,
                ending_style: None,
                emoji_usage: None,
                hashtag_rules: None,
                image_rules: None,
            };
            TemplateDao::create(&conn, &req).unwrap();
        }

        for i in 1..=2 {
            let req = CreateTemplateRequest {
                project_id: "project-b".to_string(),
                name: format!("B模板{}", i),
                platform: "wechat".to_string(),
                title_style: None,
                paragraph_style: None,
                ending_style: None,
                emoji_usage: None,
                hashtag_rules: None,
                image_rules: None,
            };
            TemplateDao::create(&conn, &req).unwrap();
        }

        // 查询 project-a 的模板
        let templates_a = TemplateDao::list(&conn, "project-a").unwrap();
        assert_eq!(templates_a.len(), 3);
        for t in &templates_a {
            assert_eq!(t.project_id, "project-a");
        }

        // 查询 project-b 的模板
        let templates_b = TemplateDao::list(&conn, "project-b").unwrap();
        assert_eq!(templates_b.len(), 2);
        for t in &templates_b {
            assert_eq!(t.project_id, "project-b");
        }
    }

    #[test]
    fn test_default_uniqueness_constraint() {
        // Property 3: Default Uniqueness Constraint
        // 验证每个项目最多只有一个默认模板
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        // 创建三个模板
        let mut template_ids = Vec::new();
        for i in 1..=3 {
            let req = CreateTemplateRequest {
                project_id: "project-1".to_string(),
                name: format!("模板{}", i),
                platform: "markdown".to_string(),
                title_style: None,
                paragraph_style: None,
                ending_style: None,
                emoji_usage: None,
                hashtag_rules: None,
                image_rules: None,
            };
            let template = TemplateDao::create(&conn, &req).unwrap();
            template_ids.push(template.id);
        }

        // 依次设置每个模板为默认，验证只有一个是默认的
        for (i, id) in template_ids.iter().enumerate() {
            TemplateDao::set_default(&conn, "project-1", id).unwrap();

            // 验证只有当前模板是默认的
            let templates = TemplateDao::list(&conn, "project-1").unwrap();
            let default_count = templates.iter().filter(|t| t.is_default).count();
            assert_eq!(
                default_count,
                1,
                "设置第{}个模板为默认后，默认模板数量应为1",
                i + 1
            );

            // 验证当前模板是默认的
            let current = TemplateDao::get(&conn, id).unwrap().unwrap();
            assert!(current.is_default, "当前设置的模板应该是默认的");
        }
    }
}
