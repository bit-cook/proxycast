//! A2UI 表单数据访问层
//!
//! 提供 A2UI 表单的 CRUD 操作，包括：
//! - 创建、获取、更新、删除表单
//! - 按会话/消息查询表单
//! - 更新表单数据和提交状态

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// 数据模型
// ============================================================================

/// A2UI 表单记录
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct A2UIForm {
    /// 表单 ID
    pub id: String,
    /// 关联的消息 ID
    pub message_id: i64,
    /// 关联的会话 ID
    pub session_id: String,
    /// A2UI 响应 JSON（包含组件定义）
    pub a2ui_response_json: String,
    /// 用户填写的表单数据 JSON
    pub form_data_json: String,
    /// 是否已提交
    pub submitted: bool,
    /// 提交时间
    pub submitted_at: Option<String>,
    /// 创建时间
    pub created_at: i64,
    /// 更新时间
    pub updated_at: i64,
}

/// 创建 A2UI 表单请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateA2UIFormRequest {
    /// 关联的消息 ID
    pub message_id: i64,
    /// 关联的会话 ID
    pub session_id: String,
    /// A2UI 响应 JSON
    pub a2ui_response_json: String,
    /// 初始表单数据（可选）
    pub form_data_json: Option<String>,
}

/// 更新表单数据请求
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateFormDataRequest {
    /// 表单数据 JSON
    pub form_data_json: String,
}

// ============================================================================
// 错误类型
// ============================================================================

/// A2UI 表单错误
#[derive(Debug, thiserror::Error)]
pub enum A2UIFormError {
    #[error("表单不存在: {0}")]
    NotFound(String),

    #[error("数据库错误: {0}")]
    Database(#[from] rusqlite::Error),
}

// ============================================================================
// 数据访问对象
// ============================================================================

/// A2UI 表单 DAO
pub struct A2UIFormDao;

impl A2UIFormDao {
    // ------------------------------------------------------------------------
    // 创建表单
    // ------------------------------------------------------------------------

    /// 创建新的 A2UI 表单记录
    pub fn create(
        conn: &Connection,
        req: &CreateA2UIFormRequest,
    ) -> Result<A2UIForm, A2UIFormError> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();
        let form_data = req
            .form_data_json
            .clone()
            .unwrap_or_else(|| "{}".to_string());

        conn.execute(
            "INSERT INTO a2ui_forms (
                id, message_id, session_id, a2ui_response_json, form_data_json,
                submitted, submitted_at, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                id,
                req.message_id,
                req.session_id,
                req.a2ui_response_json,
                form_data,
                0, // submitted = false
                Option::<String>::None,
                now,
                now,
            ],
        )?;

        Ok(A2UIForm {
            id,
            message_id: req.message_id,
            session_id: req.session_id.clone(),
            a2ui_response_json: req.a2ui_response_json.clone(),
            form_data_json: form_data,
            submitted: false,
            submitted_at: None,
            created_at: now,
            updated_at: now,
        })
    }

    // ------------------------------------------------------------------------
    // 获取表单
    // ------------------------------------------------------------------------

    /// 根据 ID 获取表单
    pub fn get(conn: &Connection, id: &str) -> Result<Option<A2UIForm>, A2UIFormError> {
        let mut stmt = conn.prepare(
            "SELECT id, message_id, session_id, a2ui_response_json, form_data_json,
                    submitted, submitted_at, created_at, updated_at
             FROM a2ui_forms WHERE id = ?",
        )?;

        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::map_row(row)?))
        } else {
            Ok(None)
        }
    }

    /// 根据消息 ID 获取表单列表
    pub fn get_by_message(
        conn: &Connection,
        message_id: i64,
    ) -> Result<Vec<A2UIForm>, A2UIFormError> {
        let mut stmt = conn.prepare(
            "SELECT id, message_id, session_id, a2ui_response_json, form_data_json,
                    submitted, submitted_at, created_at, updated_at
             FROM a2ui_forms WHERE message_id = ? ORDER BY created_at ASC",
        )?;

        let forms: Vec<A2UIForm> = stmt
            .query_map([message_id], Self::map_row)?
            .filter_map(|r| r.ok())
            .collect();

        Ok(forms)
    }

    /// 根据会话 ID 获取所有表单
    pub fn get_by_session(
        conn: &Connection,
        session_id: &str,
    ) -> Result<Vec<A2UIForm>, A2UIFormError> {
        let mut stmt = conn.prepare(
            "SELECT id, message_id, session_id, a2ui_response_json, form_data_json,
                    submitted, submitted_at, created_at, updated_at
             FROM a2ui_forms WHERE session_id = ? ORDER BY created_at ASC",
        )?;

        let forms: Vec<A2UIForm> = stmt
            .query_map([session_id], Self::map_row)?
            .filter_map(|r| r.ok())
            .collect();

        Ok(forms)
    }

    // ------------------------------------------------------------------------
    // 更新表单
    // ------------------------------------------------------------------------

    /// 更新表单数据（用户填写的内容）
    pub fn update_form_data(
        conn: &Connection,
        id: &str,
        form_data_json: &str,
    ) -> Result<A2UIForm, A2UIFormError> {
        let now = chrono::Utc::now().timestamp();

        let rows = conn.execute(
            "UPDATE a2ui_forms SET form_data_json = ?1, updated_at = ?2 WHERE id = ?3",
            params![form_data_json, now, id],
        )?;

        if rows == 0 {
            return Err(A2UIFormError::NotFound(id.to_string()));
        }

        Self::get(conn, id)?.ok_or_else(|| A2UIFormError::NotFound(id.to_string()))
    }

    /// 提交表单
    pub fn submit(
        conn: &Connection,
        id: &str,
        form_data_json: &str,
    ) -> Result<A2UIForm, A2UIFormError> {
        let now = chrono::Utc::now().timestamp();
        let submitted_at = chrono::Utc::now().to_rfc3339();

        let rows = conn.execute(
            "UPDATE a2ui_forms SET 
                form_data_json = ?1, 
                submitted = 1, 
                submitted_at = ?2, 
                updated_at = ?3 
             WHERE id = ?4",
            params![form_data_json, submitted_at, now, id],
        )?;

        if rows == 0 {
            return Err(A2UIFormError::NotFound(id.to_string()));
        }

        Self::get(conn, id)?.ok_or_else(|| A2UIFormError::NotFound(id.to_string()))
    }

    // ------------------------------------------------------------------------
    // 删除表单
    // ------------------------------------------------------------------------

    /// 删除表单
    pub fn delete(conn: &Connection, id: &str) -> Result<(), A2UIFormError> {
        let rows = conn.execute("DELETE FROM a2ui_forms WHERE id = ?", [id])?;

        if rows == 0 {
            return Err(A2UIFormError::NotFound(id.to_string()));
        }

        Ok(())
    }

    /// 删除会话的所有表单
    pub fn delete_by_session(conn: &Connection, session_id: &str) -> Result<u64, A2UIFormError> {
        let rows = conn.execute("DELETE FROM a2ui_forms WHERE session_id = ?", [session_id])?;

        Ok(rows as u64)
    }

    // ------------------------------------------------------------------------
    // 辅助方法
    // ------------------------------------------------------------------------

    /// 映射数据库行到 A2UIForm 结构体
    fn map_row(row: &rusqlite::Row) -> Result<A2UIForm, rusqlite::Error> {
        Ok(A2UIForm {
            id: row.get(0)?,
            message_id: row.get(1)?,
            session_id: row.get(2)?,
            a2ui_response_json: row.get(3)?,
            form_data_json: row.get(4)?,
            submitted: row.get::<_, i32>(5)? != 0,
            submitted_at: row.get(6)?,
            created_at: row.get(7)?,
            updated_at: row.get(8)?,
        })
    }
}
