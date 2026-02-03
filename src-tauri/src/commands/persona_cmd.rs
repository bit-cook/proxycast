//! 人设相关的 Tauri 命令
//!
//! 提供人设（Persona）管理的前端 API，包括：
//! - 创建、获取、列表、更新、删除人设
//! - 设置项目默认人设
//! - 获取人设模板列表
//!
//! ## 相关需求
//! - Requirements 6.1: 人设列表显示
//! - Requirements 6.2: 创建人设按钮
//! - Requirements 6.3: 人设创建表单
//! - Requirements 6.4: 设置默认人设
//! - Requirements 6.5: 人设模板
//! - Requirements 6.6: 人设删除确认

use tauri::State;

use crate::database::DbConnection;
use crate::models::project_model::{CreatePersonaRequest, Persona, PersonaTemplate, PersonaUpdate};
use crate::services::persona_service::PersonaService;

// ============================================================================
// Tauri 命令
// ============================================================================

/// 创建人设
///
/// 在指定项目中创建新的人设配置。
///
/// # 参数
/// - `db`: 数据库连接状态
/// - `req`: 创建人设请求，包含项目 ID、名称、风格等信息
///
/// # 返回
/// - 成功返回创建的人设
/// - 失败返回错误信息
///
/// # 示例（前端调用）
/// ```typescript
/// const persona = await invoke('create_persona', {
///   req: {
///     project_id: 'project-1',
///     name: '专业写手',
///     style: '专业严谨',
///     tone: '正式',
///   }
/// });
/// ```
#[tauri::command]
pub async fn create_persona(
    db: State<'_, DbConnection>,
    req: CreatePersonaRequest,
) -> Result<Persona, String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;
    PersonaService::create_persona(&conn, req).map_err(|e| e.to_string())
}

/// 获取项目的人设列表
///
/// 获取指定项目下的所有人设配置。
///
/// # 参数
/// - `db`: 数据库连接状态
/// - `project_id`: 项目 ID
///
/// # 返回
/// - 成功返回人设列表
/// - 失败返回错误信息
///
/// # 示例（前端调用）
/// ```typescript
/// const personas = await invoke('list_personas', {
///   projectId: 'project-1'
/// });
/// ```
#[tauri::command]
pub async fn list_personas(
    db: State<'_, DbConnection>,
    project_id: String,
) -> Result<Vec<Persona>, String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;
    PersonaService::list_personas(&conn, &project_id).map_err(|e| e.to_string())
}

/// 获取单个人设
///
/// 根据 ID 获取人设详情。
///
/// # 参数
/// - `db`: 数据库连接状态
/// - `id`: 人设 ID
///
/// # 返回
/// - 成功返回 Option<Persona>，不存在时返回 None
/// - 失败返回错误信息
///
/// # 示例（前端调用）
/// ```typescript
/// const persona = await invoke('get_persona', {
///   id: 'persona-1'
/// });
/// ```
#[tauri::command]
pub async fn get_persona(
    db: State<'_, DbConnection>,
    id: String,
) -> Result<Option<Persona>, String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;
    PersonaService::get_persona(&conn, &id).map_err(|e| e.to_string())
}

/// 更新人设
///
/// 更新指定人设的配置信息。
///
/// # 参数
/// - `db`: 数据库连接状态
/// - `id`: 人设 ID
/// - `update`: 更新内容，只包含需要更新的字段
///
/// # 返回
/// - 成功返回更新后的人设
/// - 失败返回错误信息
///
/// # 示例（前端调用）
/// ```typescript
/// const persona = await invoke('update_persona', {
///   id: 'persona-1',
///   update: {
///     name: '新名称',
///     style: '新风格',
///   }
/// });
/// ```
#[tauri::command]
pub async fn update_persona(
    db: State<'_, DbConnection>,
    id: String,
    update: PersonaUpdate,
) -> Result<Persona, String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;
    PersonaService::update_persona(&conn, &id, update).map_err(|e| e.to_string())
}

/// 删除人设
///
/// 删除指定的人设配置。
///
/// # 参数
/// - `db`: 数据库连接状态
/// - `id`: 人设 ID
///
/// # 返回
/// - 成功返回 ()
/// - 失败返回错误信息
///
/// # 示例（前端调用）
/// ```typescript
/// await invoke('delete_persona', {
///   id: 'persona-1'
/// });
/// ```
#[tauri::command]
pub async fn delete_persona(db: State<'_, DbConnection>, id: String) -> Result<(), String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;
    PersonaService::delete_persona(&conn, &id).map_err(|e| e.to_string())
}

/// 设置项目默认人设
///
/// 将指定人设设为项目的默认人设。
/// 同一项目只能有一个默认人设，设置新默认会自动取消原有默认。
///
/// # 参数
/// - `db`: 数据库连接状态
/// - `project_id`: 项目 ID
/// - `persona_id`: 要设为默认的人设 ID
///
/// # 返回
/// - 成功返回 ()
/// - 失败返回错误信息
///
/// # 示例（前端调用）
/// ```typescript
/// await invoke('set_default_persona', {
///   projectId: 'project-1',
///   personaId: 'persona-1'
/// });
/// ```
#[tauri::command]
pub async fn set_default_persona(
    db: State<'_, DbConnection>,
    project_id: String,
    persona_id: String,
) -> Result<(), String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;
    PersonaService::set_default_persona(&conn, &project_id, &persona_id).map_err(|e| e.to_string())
}

/// 获取人设模板列表
///
/// 获取预定义的人设模板，用于快速创建人设。
/// 模板包含常见的写作风格配置，如专业写手、生活博主等。
///
/// # 返回
/// - 人设模板列表
///
/// # 示例（前端调用）
/// ```typescript
/// const templates = await invoke('list_persona_templates');
/// ```
#[tauri::command]
pub async fn list_persona_templates() -> Result<Vec<PersonaTemplate>, String> {
    Ok(PersonaService::list_persona_templates())
}

/// 获取项目的默认人设
///
/// 获取指定项目的默认人设配置。
///
/// # 参数
/// - `db`: 数据库连接状态
/// - `project_id`: 项目 ID
///
/// # 返回
/// - 成功返回 Option<Persona>，没有默认人设时返回 None
/// - 失败返回错误信息
///
/// # 示例（前端调用）
/// ```typescript
/// const defaultPersona = await invoke('get_default_persona', {
///   projectId: 'project-1'
/// });
/// ```
#[tauri::command]
pub async fn get_default_persona(
    db: State<'_, DbConnection>,
    project_id: String,
) -> Result<Option<Persona>, String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;
    PersonaService::get_default_persona(&conn, &project_id).map_err(|e| e.to_string())
}
