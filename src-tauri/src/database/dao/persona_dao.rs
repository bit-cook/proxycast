//! 人设数据访问层
//!
//! 提供人设（Persona）的 CRUD 操作，包括：
//! - 创建、获取、列表、更新、删除人设
//! - 设置项目默认人设
//!
//! ## 相关需求
//! - Requirements 6.1: 人设列表显示
//! - Requirements 6.3: 人设创建
//! - Requirements 6.4: 设置默认人设
//! - Requirements 6.6: 人设删除

use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::errors::project_error::PersonaError;
use crate::models::project_model::{CreatePersonaRequest, Persona, PersonaUpdate};

// ============================================================================
// 数据访问对象
// ============================================================================

/// 人设 DAO
///
/// 提供人设的数据库操作方法。
pub struct PersonaDao;

impl PersonaDao {
    // ------------------------------------------------------------------------
    // 创建人设
    // ------------------------------------------------------------------------

    /// 创建新人设
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `req`: 创建人设请求
    ///
    /// # 返回
    /// - 成功返回创建的人设
    /// - 失败返回 PersonaError
    pub fn create(conn: &Connection, req: &CreatePersonaRequest) -> Result<Persona, PersonaError> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();

        // 序列化 JSON 字段
        let forbidden_words_json =
            serde_json::to_string(req.forbidden_words.as_ref().unwrap_or(&vec![]))
                .unwrap_or_else(|_| "[]".to_string());

        let preferred_words_json =
            serde_json::to_string(req.preferred_words.as_ref().unwrap_or(&vec![]))
                .unwrap_or_else(|_| "[]".to_string());

        let platforms_json = serde_json::to_string(req.platforms.as_ref().unwrap_or(&vec![]))
            .unwrap_or_else(|_| "[]".to_string());

        conn.execute(
            "INSERT INTO personas (
                id, project_id, name, description, style, tone, target_audience,
                forbidden_words_json, preferred_words_json, examples, platforms_json,
                is_default, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)",
            params![
                id,
                req.project_id,
                req.name,
                req.description,
                req.style,
                req.tone,
                req.target_audience,
                forbidden_words_json,
                preferred_words_json,
                req.examples,
                platforms_json,
                0, // is_default
                now,
                now,
            ],
        )?;

        // 返回创建的人设
        Ok(Persona {
            id,
            project_id: req.project_id.clone(),
            name: req.name.clone(),
            description: req.description.clone(),
            style: req.style.clone(),
            tone: req.tone.clone(),
            target_audience: req.target_audience.clone(),
            forbidden_words: req.forbidden_words.clone().unwrap_or_default(),
            preferred_words: req.preferred_words.clone().unwrap_or_default(),
            examples: req.examples.clone(),
            platforms: req.platforms.clone().unwrap_or_default(),
            is_default: false,
            created_at: now,
            updated_at: now,
        })
    }

    // ------------------------------------------------------------------------
    // 获取人设
    // ------------------------------------------------------------------------

    /// 获取单个人设
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `id`: 人设 ID
    ///
    /// # 返回
    /// - 成功返回 Option<Persona>
    /// - 失败返回 PersonaError
    pub fn get(conn: &Connection, id: &str) -> Result<Option<Persona>, PersonaError> {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, name, description, style, tone, target_audience,
                    forbidden_words_json, preferred_words_json, examples, platforms_json,
                    is_default, created_at, updated_at
             FROM personas WHERE id = ?",
        )?;

        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::map_row(row)?))
        } else {
            Ok(None)
        }
    }

    // ------------------------------------------------------------------------
    // 列表人设
    // ------------------------------------------------------------------------

    /// 获取项目的人设列表
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `project_id`: 项目 ID
    ///
    /// # 返回
    /// - 成功返回人设列表
    /// - 失败返回 PersonaError
    pub fn list(conn: &Connection, project_id: &str) -> Result<Vec<Persona>, PersonaError> {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, name, description, style, tone, target_audience,
                    forbidden_words_json, preferred_words_json, examples, platforms_json,
                    is_default, created_at, updated_at
             FROM personas WHERE project_id = ? ORDER BY created_at DESC",
        )?;

        let personas: Vec<Persona> = stmt
            .query_map([project_id], |row| Self::map_row(row))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(personas)
    }

    // ------------------------------------------------------------------------
    // 更新人设
    // ------------------------------------------------------------------------

    /// 更新人设
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `id`: 人设 ID
    /// - `update`: 更新内容
    ///
    /// # 返回
    /// - 成功返回更新后的人设
    /// - 失败返回 PersonaError
    pub fn update(
        conn: &Connection,
        id: &str,
        update: &PersonaUpdate,
    ) -> Result<Persona, PersonaError> {
        // 先获取现有人设
        let existing =
            Self::get(conn, id)?.ok_or_else(|| PersonaError::NotFound(id.to_string()))?;

        let now = chrono::Utc::now().timestamp();

        // 构建更新后的值
        let name = update.name.as_ref().unwrap_or(&existing.name);
        let description = update.description.clone().or(existing.description);
        let style = update.style.as_ref().unwrap_or(&existing.style);
        let tone = update.tone.clone().or(existing.tone);
        let target_audience = update.target_audience.clone().or(existing.target_audience);
        let forbidden_words = update
            .forbidden_words
            .clone()
            .unwrap_or(existing.forbidden_words);
        let preferred_words = update
            .preferred_words
            .clone()
            .unwrap_or(existing.preferred_words);
        let examples = update.examples.clone().or(existing.examples);
        let platforms = update.platforms.clone().unwrap_or(existing.platforms);

        // 序列化 JSON 字段
        let forbidden_words_json =
            serde_json::to_string(&forbidden_words).unwrap_or_else(|_| "[]".to_string());
        let preferred_words_json =
            serde_json::to_string(&preferred_words).unwrap_or_else(|_| "[]".to_string());
        let platforms_json = serde_json::to_string(&platforms).unwrap_or_else(|_| "[]".to_string());

        conn.execute(
            "UPDATE personas SET
                name = ?1, description = ?2, style = ?3, tone = ?4, target_audience = ?5,
                forbidden_words_json = ?6, preferred_words_json = ?7, examples = ?8,
                platforms_json = ?9, updated_at = ?10
             WHERE id = ?11",
            params![
                name,
                description,
                style,
                tone,
                target_audience,
                forbidden_words_json,
                preferred_words_json,
                examples,
                platforms_json,
                now,
                id,
            ],
        )?;

        // 返回更新后的人设
        Self::get(conn, id)?.ok_or_else(|| PersonaError::NotFound(id.to_string()))
    }

    // ------------------------------------------------------------------------
    // 删除人设
    // ------------------------------------------------------------------------

    /// 删除人设
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `id`: 人设 ID
    ///
    /// # 返回
    /// - 成功返回 ()
    /// - 失败返回 PersonaError
    pub fn delete(conn: &Connection, id: &str) -> Result<(), PersonaError> {
        let rows = conn.execute("DELETE FROM personas WHERE id = ?", [id])?;

        if rows == 0 {
            return Err(PersonaError::NotFound(id.to_string()));
        }

        Ok(())
    }

    // ------------------------------------------------------------------------
    // 设置默认人设
    // ------------------------------------------------------------------------

    /// 设置项目的默认人设
    ///
    /// 将指定人设设为默认，同时取消该项目其他人设的默认状态。
    /// 这确保每个项目最多只有一个默认人设。
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `project_id`: 项目 ID
    /// - `persona_id`: 要设为默认的人设 ID
    ///
    /// # 返回
    /// - 成功返回 ()
    /// - 失败返回 PersonaError
    pub fn set_default(
        conn: &Connection,
        project_id: &str,
        persona_id: &str,
    ) -> Result<(), PersonaError> {
        // 验证人设存在且属于该项目
        let persona = Self::get(conn, persona_id)?
            .ok_or_else(|| PersonaError::NotFound(persona_id.to_string()))?;

        if persona.project_id != project_id {
            return Err(PersonaError::ProjectNotFound(project_id.to_string()));
        }

        let now = chrono::Utc::now().timestamp();

        // 先取消该项目所有人设的默认状态
        conn.execute(
            "UPDATE personas SET is_default = 0, updated_at = ?1 WHERE project_id = ?2",
            params![now, project_id],
        )?;

        // 设置指定人设为默认
        conn.execute(
            "UPDATE personas SET is_default = 1, updated_at = ?1 WHERE id = ?2",
            params![now, persona_id],
        )?;

        Ok(())
    }

    /// 获取项目的默认人设
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `project_id`: 项目 ID
    ///
    /// # 返回
    /// - 成功返回 Option<Persona>
    /// - 失败返回 PersonaError
    pub fn get_default(
        conn: &Connection,
        project_id: &str,
    ) -> Result<Option<Persona>, PersonaError> {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, name, description, style, tone, target_audience,
                    forbidden_words_json, preferred_words_json, examples, platforms_json,
                    is_default, created_at, updated_at
             FROM personas WHERE project_id = ? AND is_default = 1",
        )?;

        let mut rows = stmt.query([project_id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::map_row(row)?))
        } else {
            Ok(None)
        }
    }

    // ------------------------------------------------------------------------
    // 辅助方法
    // ------------------------------------------------------------------------

    /// 映射数据库行到 Persona 结构体
    fn map_row(row: &rusqlite::Row) -> Result<Persona, rusqlite::Error> {
        let forbidden_words_json: String = row.get(7)?;
        let preferred_words_json: String = row.get(8)?;
        let platforms_json: String = row.get(10)?;

        // 解析 JSON 字段
        let forbidden_words: Vec<String> =
            serde_json::from_str(&forbidden_words_json).unwrap_or_default();
        let preferred_words: Vec<String> =
            serde_json::from_str(&preferred_words_json).unwrap_or_default();
        let platforms: Vec<String> = serde_json::from_str(&platforms_json).unwrap_or_default();

        Ok(Persona {
            id: row.get(0)?,
            project_id: row.get(1)?,
            name: row.get(2)?,
            description: row.get(3)?,
            style: row.get(4)?,
            tone: row.get(5)?,
            target_audience: row.get(6)?,
            forbidden_words,
            preferred_words,
            examples: row.get(9)?,
            platforms,
            is_default: row.get::<_, i32>(11)? != 0,
            created_at: row.get(12)?,
            updated_at: row.get(13)?,
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
    fn test_create_persona() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        let req = CreatePersonaRequest {
            project_id: "project-1".to_string(),
            name: "专业写手".to_string(),
            description: Some("专业的技术文章写手".to_string()),
            style: "专业".to_string(),
            tone: Some("正式".to_string()),
            target_audience: Some("技术人员".to_string()),
            forbidden_words: Some(vec!["禁词1".to_string()]),
            preferred_words: Some(vec!["偏好词1".to_string()]),
            examples: Some("示例文本".to_string()),
            platforms: Some(vec!["xiaohongshu".to_string()]),
        };

        let persona = PersonaDao::create(&conn, &req).unwrap();

        assert!(!persona.id.is_empty());
        assert_eq!(persona.project_id, "project-1");
        assert_eq!(persona.name, "专业写手");
        assert_eq!(persona.style, "专业");
        assert!(!persona.is_default);
    }

    #[test]
    fn test_get_persona() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        let req = CreatePersonaRequest {
            project_id: "project-1".to_string(),
            name: "测试人设".to_string(),
            description: None,
            style: "轻松".to_string(),
            tone: None,
            target_audience: None,
            forbidden_words: None,
            preferred_words: None,
            examples: None,
            platforms: None,
        };

        let created = PersonaDao::create(&conn, &req).unwrap();
        let fetched = PersonaDao::get(&conn, &created.id).unwrap();

        assert!(fetched.is_some());
        let fetched = fetched.unwrap();
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.name, "测试人设");
    }

    #[test]
    fn test_get_nonexistent_persona() {
        let conn = setup_test_db();
        let result = PersonaDao::get(&conn, "nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_list_personas() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");
        create_test_project(&conn, "project-2");

        // 为 project-1 创建两个人设
        for i in 1..=2 {
            let req = CreatePersonaRequest {
                project_id: "project-1".to_string(),
                name: format!("人设{}", i),
                description: None,
                style: "测试".to_string(),
                tone: None,
                target_audience: None,
                forbidden_words: None,
                preferred_words: None,
                examples: None,
                platforms: None,
            };
            PersonaDao::create(&conn, &req).unwrap();
        }

        // 为 project-2 创建一个人设
        let req = CreatePersonaRequest {
            project_id: "project-2".to_string(),
            name: "人设3".to_string(),
            description: None,
            style: "测试".to_string(),
            tone: None,
            target_audience: None,
            forbidden_words: None,
            preferred_words: None,
            examples: None,
            platforms: None,
        };
        PersonaDao::create(&conn, &req).unwrap();

        // 验证 project-1 有 2 个人设
        let personas = PersonaDao::list(&conn, "project-1").unwrap();
        assert_eq!(personas.len(), 2);

        // 验证 project-2 有 1 个人设
        let personas = PersonaDao::list(&conn, "project-2").unwrap();
        assert_eq!(personas.len(), 1);
    }

    #[test]
    fn test_update_persona() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        let req = CreatePersonaRequest {
            project_id: "project-1".to_string(),
            name: "原始名称".to_string(),
            description: Some("原始描述".to_string()),
            style: "原始风格".to_string(),
            tone: None,
            target_audience: None,
            forbidden_words: None,
            preferred_words: None,
            examples: None,
            platforms: None,
        };

        let created = PersonaDao::create(&conn, &req).unwrap();

        let update = PersonaUpdate {
            name: Some("更新后名称".to_string()),
            description: Some("更新后描述".to_string()),
            style: Some("更新后风格".to_string()),
            tone: Some("活泼".to_string()),
            ..Default::default()
        };

        let updated = PersonaDao::update(&conn, &created.id, &update).unwrap();

        assert_eq!(updated.name, "更新后名称");
        assert_eq!(updated.description, Some("更新后描述".to_string()));
        assert_eq!(updated.style, "更新后风格");
        assert_eq!(updated.tone, Some("活泼".to_string()));
    }

    #[test]
    fn test_update_nonexistent_persona() {
        let conn = setup_test_db();
        let update = PersonaUpdate::default();
        let result = PersonaDao::update(&conn, "nonexistent", &update);
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_persona() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        let req = CreatePersonaRequest {
            project_id: "project-1".to_string(),
            name: "待删除人设".to_string(),
            description: None,
            style: "测试".to_string(),
            tone: None,
            target_audience: None,
            forbidden_words: None,
            preferred_words: None,
            examples: None,
            platforms: None,
        };

        let created = PersonaDao::create(&conn, &req).unwrap();

        // 验证人设存在
        assert!(PersonaDao::get(&conn, &created.id).unwrap().is_some());

        // 删除人设
        PersonaDao::delete(&conn, &created.id).unwrap();

        // 验证人设已删除
        assert!(PersonaDao::get(&conn, &created.id).unwrap().is_none());
    }

    #[test]
    fn test_delete_nonexistent_persona() {
        let conn = setup_test_db();
        let result = PersonaDao::delete(&conn, "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_set_default_persona() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        // 创建两个人设
        let req1 = CreatePersonaRequest {
            project_id: "project-1".to_string(),
            name: "人设1".to_string(),
            description: None,
            style: "测试".to_string(),
            tone: None,
            target_audience: None,
            forbidden_words: None,
            preferred_words: None,
            examples: None,
            platforms: None,
        };
        let persona1 = PersonaDao::create(&conn, &req1).unwrap();

        let req2 = CreatePersonaRequest {
            project_id: "project-1".to_string(),
            name: "人设2".to_string(),
            description: None,
            style: "测试".to_string(),
            tone: None,
            target_audience: None,
            forbidden_words: None,
            preferred_words: None,
            examples: None,
            platforms: None,
        };
        let persona2 = PersonaDao::create(&conn, &req2).unwrap();

        // 设置人设1为默认
        PersonaDao::set_default(&conn, "project-1", &persona1.id).unwrap();

        let p1 = PersonaDao::get(&conn, &persona1.id).unwrap().unwrap();
        let p2 = PersonaDao::get(&conn, &persona2.id).unwrap().unwrap();
        assert!(p1.is_default);
        assert!(!p2.is_default);

        // 设置人设2为默认，人设1应该不再是默认
        PersonaDao::set_default(&conn, "project-1", &persona2.id).unwrap();

        let p1 = PersonaDao::get(&conn, &persona1.id).unwrap().unwrap();
        let p2 = PersonaDao::get(&conn, &persona2.id).unwrap().unwrap();
        assert!(!p1.is_default);
        assert!(p2.is_default);
    }

    #[test]
    fn test_get_default_persona() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        // 初始没有默认人设
        let default = PersonaDao::get_default(&conn, "project-1").unwrap();
        assert!(default.is_none());

        // 创建人设并设为默认
        let req = CreatePersonaRequest {
            project_id: "project-1".to_string(),
            name: "默认人设".to_string(),
            description: None,
            style: "测试".to_string(),
            tone: None,
            target_audience: None,
            forbidden_words: None,
            preferred_words: None,
            examples: None,
            platforms: None,
        };
        let persona = PersonaDao::create(&conn, &req).unwrap();
        PersonaDao::set_default(&conn, "project-1", &persona.id).unwrap();

        // 验证可以获取默认人设
        let default = PersonaDao::get_default(&conn, "project-1").unwrap();
        assert!(default.is_some());
        assert_eq!(default.unwrap().id, persona.id);
    }

    #[test]
    fn test_set_default_wrong_project() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");
        create_test_project(&conn, "project-2");

        // 在 project-1 创建人设
        let req = CreatePersonaRequest {
            project_id: "project-1".to_string(),
            name: "人设".to_string(),
            description: None,
            style: "测试".to_string(),
            tone: None,
            target_audience: None,
            forbidden_words: None,
            preferred_words: None,
            examples: None,
            platforms: None,
        };
        let persona = PersonaDao::create(&conn, &req).unwrap();

        // 尝试在 project-2 设置该人设为默认，应该失败
        let result = PersonaDao::set_default(&conn, "project-2", &persona.id);
        assert!(result.is_err());
    }
}
