//! 批量任务数据访问对象 (DAO)
//!
//! 提供批量任务和模板的数据库操作

use super::batch::{BatchTask, BatchTaskStatus};
use super::template::TaskTemplate;
use anyhow::{Context, Result};
use proxycast_core::database::DbConnection;
use rusqlite::params;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// 批量任务 DAO
pub struct BatchTaskDao;

impl BatchTaskDao {
    /// 初始化数据库表
    pub fn init_tables(db: &DbConnection) -> Result<()> {
        let conn = db.lock().unwrap();

        // 创建批量任务表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS batch_tasks (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                template_id TEXT NOT NULL,
                status TEXT NOT NULL,
                options_json TEXT NOT NULL,
                tasks_json TEXT NOT NULL,
                results_json TEXT,
                created_at TEXT NOT NULL,
                started_at TEXT,
                completed_at TEXT
            )",
            [],
        )
        .context("创建 batch_tasks 表失败")?;

        // 创建模板表
        conn.execute(
            "CREATE TABLE IF NOT EXISTS batch_templates (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                model TEXT NOT NULL,
                system_prompt TEXT,
                user_message_template TEXT NOT NULL,
                temperature REAL,
                max_tokens INTEGER,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )
        .context("创建 batch_templates 表失败")?;

        // 创建索引
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_batch_tasks_status ON batch_tasks(status)",
            [],
        )?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_batch_tasks_created_at ON batch_tasks(created_at DESC)",
            [],
        )?;

        Ok(())
    }

    /// 保存批量任务
    pub fn save(db: &DbConnection, batch_task: &BatchTask) -> Result<()> {
        let conn = db.lock().unwrap();

        let options_json = serde_json::to_string(&batch_task.options)?;
        let tasks_json = serde_json::to_string(&batch_task.tasks)?;
        let results_json = if batch_task.results.is_empty() {
            None
        } else {
            Some(serde_json::to_string(&batch_task.results)?)
        };

        conn.execute(
            "INSERT OR REPLACE INTO batch_tasks
             (id, name, template_id, status, options_json, tasks_json, results_json,
              created_at, started_at, completed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                batch_task.id.to_string(),
                batch_task.name,
                batch_task.template_id.to_string(),
                serde_json::to_string(&batch_task.status)?,
                options_json,
                tasks_json,
                results_json,
                batch_task.created_at.to_rfc3339(),
                batch_task.started_at.map(|t| t.to_rfc3339()),
                batch_task.completed_at.map(|t| t.to_rfc3339()),
            ],
        )
        .context("保存批量任务失败")?;

        Ok(())
    }

    /// 根据 ID 查询批量任务
    pub fn get_by_id(db: &DbConnection, id: &Uuid) -> Result<Option<BatchTask>> {
        let conn = db.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, name, template_id, status, options_json, tasks_json, results_json,
                    created_at, started_at, completed_at
             FROM batch_tasks WHERE id = ?1",
        )?;

        let result = stmt
            .query_row(params![id.to_string()], |row| {
                let id: String = row.get(0)?;
                let name: String = row.get(1)?;
                let template_id: String = row.get(2)?;
                let status_str: String = row.get(3)?;
                let options_json: String = row.get(4)?;
                let tasks_json: String = row.get(5)?;
                let results_json: Option<String> = row.get(6)?;
                let created_at: String = row.get(7)?;
                let started_at: Option<String> = row.get(8)?;
                let completed_at: Option<String> = row.get(9)?;

                Ok((
                    id,
                    name,
                    template_id,
                    status_str,
                    options_json,
                    tasks_json,
                    results_json,
                    created_at,
                    started_at,
                    completed_at,
                ))
            })
            .optional()?;

        if let Some((
            id,
            name,
            template_id,
            status_str,
            options_json,
            tasks_json,
            results_json,
            created_at,
            started_at,
            completed_at,
        )) = result
        {
            let batch_task = BatchTask {
                id: Uuid::parse_str(&id)?,
                name,
                template_id: Uuid::parse_str(&template_id)?,
                status: serde_json::from_str(&status_str)?,
                options: serde_json::from_str(&options_json)?,
                tasks: serde_json::from_str(&tasks_json)?,
                results: results_json
                    .map(|json| serde_json::from_str(&json))
                    .transpose()?
                    .unwrap_or_default(),
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at)?.into(),
                started_at: started_at
                    .map(|s| chrono::DateTime::parse_from_rfc3339(&s).map(|dt| dt.into()))
                    .transpose()?,
                completed_at: completed_at
                    .map(|s| chrono::DateTime::parse_from_rfc3339(&s).map(|dt| dt.into()))
                    .transpose()?,
            };

            Ok(Some(batch_task))
        } else {
            Ok(None)
        }
    }

    /// 查询所有批量任务
    pub fn list_all(db: &DbConnection, limit: usize) -> Result<Vec<BatchTask>> {
        let conn = db.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, name, template_id, status, options_json, tasks_json, results_json,
                    created_at, started_at, completed_at
             FROM batch_tasks
             ORDER BY created_at DESC
             LIMIT ?1",
        )?;

        let rows = stmt.query_map(params![limit], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let template_id: String = row.get(2)?;
            let status_str: String = row.get(3)?;
            let options_json: String = row.get(4)?;
            let tasks_json: String = row.get(5)?;
            let results_json: Option<String> = row.get(6)?;
            let created_at: String = row.get(7)?;
            let started_at: Option<String> = row.get(8)?;
            let completed_at: Option<String> = row.get(9)?;

            Ok((
                id,
                name,
                template_id,
                status_str,
                options_json,
                tasks_json,
                results_json,
                created_at,
                started_at,
                completed_at,
            ))
        })?;

        let mut batch_tasks = Vec::new();
        for row in rows {
            let (
                id,
                name,
                template_id,
                status_str,
                options_json,
                tasks_json,
                results_json,
                created_at,
                started_at,
                completed_at,
            ) = row?;

            let batch_task = BatchTask {
                id: Uuid::parse_str(&id)?,
                name,
                template_id: Uuid::parse_str(&template_id)?,
                status: serde_json::from_str(&status_str)?,
                options: serde_json::from_str(&options_json)?,
                tasks: serde_json::from_str(&tasks_json)?,
                results: results_json
                    .map(|json| serde_json::from_str(&json))
                    .transpose()?
                    .unwrap_or_default(),
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at)?.into(),
                started_at: started_at
                    .map(|s| chrono::DateTime::parse_from_rfc3339(&s).map(|dt| dt.into()))
                    .transpose()?,
                completed_at: completed_at
                    .map(|s| chrono::DateTime::parse_from_rfc3339(&s).map(|dt| dt.into()))
                    .transpose()?,
            };

            batch_tasks.push(batch_task);
        }

        Ok(batch_tasks)
    }

    /// 删除批量任务
    pub fn delete(db: &DbConnection, id: &Uuid) -> Result<bool> {
        let conn = db.lock().unwrap();

        let affected = conn.execute(
            "DELETE FROM batch_tasks WHERE id = ?1",
            params![id.to_string()],
        )?;

        Ok(affected > 0)
    }

    /// 更新批量任务状态
    pub fn update_status(db: &DbConnection, id: &Uuid, status: BatchTaskStatus) -> Result<()> {
        let conn = db.lock().unwrap();

        conn.execute(
            "UPDATE batch_tasks SET status = ?1 WHERE id = ?2",
            params![serde_json::to_string(&status)?, id.to_string()],
        )?;

        Ok(())
    }
}

/// 模板 DAO
pub struct TemplateDao;

impl TemplateDao {
    /// 保存模板
    pub fn save(db: &DbConnection, template: &TaskTemplate) -> Result<()> {
        let conn = db.lock().unwrap();

        conn.execute(
            "INSERT OR REPLACE INTO batch_templates
             (id, name, description, model, system_prompt, user_message_template,
              temperature, max_tokens, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                template.id.to_string(),
                template.name,
                template.description,
                template.model,
                template.system_prompt,
                template.user_message_template,
                template.temperature,
                template.max_tokens,
                template.created_at.to_rfc3339(),
                template.updated_at.to_rfc3339(),
            ],
        )
        .context("保存模板失败")?;

        Ok(())
    }

    /// 根据 ID 查询模板
    pub fn get_by_id(db: &DbConnection, id: &Uuid) -> Result<Option<TaskTemplate>> {
        let conn = db.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, name, description, model, system_prompt, user_message_template,
                    temperature, max_tokens, created_at, updated_at
             FROM batch_templates WHERE id = ?1",
        )?;

        let result = stmt
            .query_row(params![id.to_string()], |row| {
                Ok(TaskTemplate {
                    id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                    name: row.get(1)?,
                    description: row.get(2)?,
                    model: row.get(3)?,
                    system_prompt: row.get(4)?,
                    user_message_template: row.get(5)?,
                    temperature: row.get(6)?,
                    max_tokens: row.get(7)?,
                    created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                        .unwrap()
                        .into(),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                        .unwrap()
                        .into(),
                })
            })
            .optional()?;

        Ok(result)
    }

    /// 查询所有模板
    pub fn list_all(db: &DbConnection) -> Result<Vec<TaskTemplate>> {
        let conn = db.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, name, description, model, system_prompt, user_message_template,
                    temperature, max_tokens, created_at, updated_at
             FROM batch_templates
             ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(TaskTemplate {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap(),
                name: row.get(1)?,
                description: row.get(2)?,
                model: row.get(3)?,
                system_prompt: row.get(4)?,
                user_message_template: row.get(5)?,
                temperature: row.get(6)?,
                max_tokens: row.get(7)?,
                created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
                    .unwrap()
                    .into(),
                updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(9)?)
                    .unwrap()
                    .into(),
            })
        })?;

        let mut templates = Vec::new();
        for row in rows {
            templates.push(row?);
        }

        Ok(templates)
    }

    /// 删除模板
    pub fn delete(db: &DbConnection, id: &Uuid) -> Result<bool> {
        let conn = db.lock().unwrap();

        let affected = conn.execute(
            "DELETE FROM batch_templates WHERE id = ?1",
            params![id.to_string()],
        )?;

        Ok(affected > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use std::collections::HashMap;

    fn setup_test_db() -> DbConnection {
        let conn = Connection::open_in_memory().unwrap();
        let db = Arc::new(Mutex::new(conn));
        BatchTaskDao::init_tables(&db).unwrap();
        db
    }

    #[test]
    fn test_init_tables() {
        let db = setup_test_db();
        // 如果能成功创建,说明表初始化成功
        assert!(true);
    }

    #[test]
    fn test_save_and_get_template() {
        let db = setup_test_db();

        let template = TaskTemplate::new(
            "测试模板".to_string(),
            "gpt-4".to_string(),
            "请处理: {{content}}".to_string(),
        );

        // 保存模板
        TemplateDao::save(&db, &template).unwrap();

        // 查询模板
        let loaded = TemplateDao::get_by_id(&db, &template.id).unwrap();
        assert!(loaded.is_some());

        let loaded = loaded.unwrap();
        assert_eq!(loaded.name, template.name);
        assert_eq!(loaded.model, template.model);
        assert_eq!(loaded.user_message_template, template.user_message_template);
    }

    #[test]
    fn test_save_and_get_batch_task() {
        let db = setup_test_db();

        let template = TaskTemplate::new(
            "测试模板".to_string(),
            "gpt-4".to_string(),
            "请处理: {{content}}".to_string(),
        );

        let tasks = vec![super::super::batch::TaskDefinition {
            id: None,
            variables: {
                let mut map = HashMap::new();
                map.insert("content".to_string(), "测试内容".to_string());
                map
            },
            metadata: HashMap::new(),
        }];

        let batch_task = BatchTask::new(
            "测试批量任务".to_string(),
            template.id,
            tasks,
            super::super::batch::BatchOptions::default(),
        );

        // 保存批量任务
        BatchTaskDao::save(&db, &batch_task).unwrap();

        // 查询批量任务
        let loaded = BatchTaskDao::get_by_id(&db, &batch_task.id).unwrap();
        assert!(loaded.is_some());

        let loaded = loaded.unwrap();
        assert_eq!(loaded.name, batch_task.name);
        assert_eq!(loaded.template_id, batch_task.template_id);
        assert_eq!(loaded.tasks.len(), 1);
    }
}
