//! 排版模板服务层
//!
//! 提供排版模板（Template）的业务逻辑，包括：
//! - 创建、获取、列表、更新、删除模板
//! - 设置项目默认模板
//!
//! ## 相关需求
//! - Requirements 8.1: 模板列表显示
//! - Requirements 8.2: 创建模板按钮
//! - Requirements 8.3: 模板创建表单
//! - Requirements 8.4: 设置默认模板
//! - Requirements 8.5: 模板预览功能

use rusqlite::Connection;

use crate::database::dao::template_dao::TemplateDao;
use crate::errors::project_error::TemplateError;
use crate::models::project_model::{CreateTemplateRequest, Template, TemplateUpdate};

// ============================================================================
// 排版模板服务
// ============================================================================

/// 排版模板服务
///
/// 封装排版模板的业务逻辑，调用 TemplateDao 进行数据操作。
pub struct TemplateService;

impl TemplateService {
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
    ///
    /// # 示例
    /// ```ignore
    /// let req = CreateTemplateRequest {
    ///     project_id: "project-1".to_string(),
    ///     name: "小红书模板".to_string(),
    ///     platform: "xiaohongshu".to_string(),
    ///     ..Default::default()
    /// };
    /// let template = TemplateService::create_template(&conn, req)?;
    /// ```
    pub fn create_template(
        conn: &Connection,
        req: CreateTemplateRequest,
    ) -> Result<Template, TemplateError> {
        // 验证项目存在
        Self::validate_project_exists(conn, &req.project_id)?;

        // 调用 DAO 创建模板
        TemplateDao::create(conn, &req)
    }

    // ------------------------------------------------------------------------
    // 获取模板列表
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
    pub fn list_templates(
        conn: &Connection,
        project_id: &str,
    ) -> Result<Vec<Template>, TemplateError> {
        TemplateDao::list(conn, project_id)
    }

    // ------------------------------------------------------------------------
    // 获取单个模板
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
    pub fn get_template(conn: &Connection, id: &str) -> Result<Option<Template>, TemplateError> {
        TemplateDao::get(conn, id)
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
    pub fn update_template(
        conn: &Connection,
        id: &str,
        update: TemplateUpdate,
    ) -> Result<Template, TemplateError> {
        TemplateDao::update(conn, id, &update)
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
    pub fn delete_template(conn: &Connection, id: &str) -> Result<(), TemplateError> {
        TemplateDao::delete(conn, id)
    }

    // ------------------------------------------------------------------------
    // 设置默认模板
    // ------------------------------------------------------------------------

    /// 设置项目的默认模板
    ///
    /// 将指定模板设为默认，同时取消该项目其他模板的默认状态。
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `project_id`: 项目 ID
    /// - `template_id`: 要设为默认的模板 ID
    ///
    /// # 返回
    /// - 成功返回 ()
    /// - 失败返回 TemplateError
    pub fn set_default_template(
        conn: &Connection,
        project_id: &str,
        template_id: &str,
    ) -> Result<(), TemplateError> {
        TemplateDao::set_default(conn, project_id, template_id)
    }

    // ------------------------------------------------------------------------
    // 获取默认模板
    // ------------------------------------------------------------------------

    /// 获取项目的默认模板
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `project_id`: 项目 ID
    ///
    /// # 返回
    /// - 成功返回 Option<Template>
    /// - 失败返回 TemplateError
    pub fn get_default_template(
        conn: &Connection,
        project_id: &str,
    ) -> Result<Option<Template>, TemplateError> {
        TemplateDao::get_default(conn, project_id)
    }

    // ------------------------------------------------------------------------
    // 辅助方法
    // ------------------------------------------------------------------------

    /// 验证项目是否存在
    fn validate_project_exists(conn: &Connection, project_id: &str) -> Result<(), TemplateError> {
        let mut stmt = conn
            .prepare("SELECT 1 FROM workspaces WHERE id = ?")
            .map_err(TemplateError::DatabaseError)?;

        let exists = stmt
            .exists([project_id])
            .map_err(TemplateError::DatabaseError)?;

        if !exists {
            return Err(TemplateError::ProjectNotFound(project_id.to_string()));
        }

        Ok(())
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
            rusqlite::params![
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
    fn test_create_template_success() {
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

        let template = TemplateService::create_template(&conn, req).unwrap();

        assert!(!template.id.is_empty());
        assert_eq!(template.project_id, "project-1");
        assert_eq!(template.name, "小红书模板");
        assert_eq!(template.platform, "xiaohongshu");
        assert_eq!(template.emoji_usage, "heavy");
    }

    #[test]
    fn test_create_template_project_not_found() {
        let conn = setup_test_db();

        let req = CreateTemplateRequest {
            project_id: "nonexistent".to_string(),
            name: "测试模板".to_string(),
            platform: "markdown".to_string(),
            title_style: None,
            paragraph_style: None,
            ending_style: None,
            emoji_usage: None,
            hashtag_rules: None,
            image_rules: None,
        };

        let result = TemplateService::create_template(&conn, req);
        assert!(result.is_err());

        match result.unwrap_err() {
            TemplateError::ProjectNotFound(id) => assert_eq!(id, "nonexistent"),
            _ => panic!("期望 ProjectNotFound 错误"),
        }
    }

    #[test]
    fn test_list_templates() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        // 创建两个模板
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
            TemplateService::create_template(&conn, req).unwrap();
        }

        let templates = TemplateService::list_templates(&conn, "project-1").unwrap();
        assert_eq!(templates.len(), 2);
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

        let created = TemplateService::create_template(&conn, req).unwrap();
        let fetched = TemplateService::get_template(&conn, &created.id).unwrap();

        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().id, created.id);
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

        let created = TemplateService::create_template(&conn, req).unwrap();

        let update = TemplateUpdate {
            name: Some("更新后名称".to_string()),
            title_style: Some("更新后标题风格".to_string()),
            paragraph_style: Some("新段落风格".to_string()),
            ending_style: None,
            emoji_usage: Some("heavy".to_string()),
            hashtag_rules: Some("5个话题".to_string()),
            image_rules: None,
        };

        let updated = TemplateService::update_template(&conn, &created.id, update).unwrap();

        assert_eq!(updated.name, "更新后名称");
        assert_eq!(updated.title_style, Some("更新后标题风格".to_string()));
        assert_eq!(updated.paragraph_style, Some("新段落风格".to_string()));
        assert_eq!(updated.emoji_usage, "heavy");
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

        let created = TemplateService::create_template(&conn, req).unwrap();

        // 验证模板存在
        assert!(TemplateService::get_template(&conn, &created.id)
            .unwrap()
            .is_some());

        // 删除模板
        TemplateService::delete_template(&conn, &created.id).unwrap();

        // 验证模板已删除
        assert!(TemplateService::get_template(&conn, &created.id)
            .unwrap()
            .is_none());
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
        let template1 = TemplateService::create_template(&conn, req1).unwrap();

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
        let template2 = TemplateService::create_template(&conn, req2).unwrap();

        // 设置模板1为默认
        TemplateService::set_default_template(&conn, "project-1", &template1.id).unwrap();

        let default = TemplateService::get_default_template(&conn, "project-1").unwrap();
        assert!(default.is_some());
        assert_eq!(default.unwrap().id, template1.id);

        // 设置模板2为默认，模板1应该不再是默认
        TemplateService::set_default_template(&conn, "project-1", &template2.id).unwrap();

        let default = TemplateService::get_default_template(&conn, "project-1").unwrap();
        assert!(default.is_some());
        assert_eq!(default.unwrap().id, template2.id);

        // 验证只有一个默认模板
        let templates = TemplateService::list_templates(&conn, "project-1").unwrap();
        let default_count = templates.iter().filter(|t| t.is_default).count();
        assert_eq!(default_count, 1);
    }

    #[test]
    fn test_get_default_template_none() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        // 没有设置默认模板时应返回 None
        let default = TemplateService::get_default_template(&conn, "project-1").unwrap();
        assert!(default.is_none());
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

        let template = TemplateService::create_template(&conn, req).unwrap();

        assert!(!template.id.is_empty());
        assert_eq!(template.name, "简单模板");
        assert_eq!(template.platform, "markdown");
        // 默认值
        assert_eq!(template.emoji_usage, "moderate");
        assert!(template.title_style.is_none());
    }
}
