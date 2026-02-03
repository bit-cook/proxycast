//! 排版模板相关的 Tauri 命令
//!
//! 提供排版模板（Template）管理的前端 API，包括：
//! - 创建、获取、列表、更新、删除模板
//! - 设置项目默认模板
//!
//! ## 相关需求
//! - Requirements 8.1: 模板列表显示
//! - Requirements 8.2: 创建模板按钮
//! - Requirements 8.3: 模板创建表单
//! - Requirements 8.4: 设置默认模板
//! - Requirements 8.5: 模板预览功能

use tauri::State;

use crate::database::DbConnection;
use crate::models::project_model::{CreateTemplateRequest, Template, TemplateUpdate};
use crate::services::template_service::TemplateService;

// ============================================================================
// Tauri 命令
// ============================================================================

/// 创建排版模板
///
/// 在指定项目中创建新的排版模板。
///
/// # 参数
/// - `db`: 数据库连接状态
/// - `req`: 创建模板请求，包含项目 ID、名称、平台、样式规则等信息
///
/// # 返回
/// - 成功返回创建的模板
/// - 失败返回错误信息
///
/// # 示例（前端调用）
/// ```typescript
/// const template = await invoke('create_template', {
///   req: {
///     project_id: 'project-1',
///     name: '小红书模板',
///     platform: 'xiaohongshu',
///     title_style: '吸引眼球',
///     emoji_usage: 'heavy',
///   }
/// });
/// ```
#[tauri::command]
pub async fn create_template(
    db: State<'_, DbConnection>,
    req: CreateTemplateRequest,
) -> Result<Template, String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;
    TemplateService::create_template(&conn, req).map_err(|e| e.to_string())
}

/// 获取项目的模板列表
///
/// 获取指定项目下的所有排版模板。
///
/// # 参数
/// - `db`: 数据库连接状态
/// - `project_id`: 项目 ID
///
/// # 返回
/// - 成功返回模板列表
/// - 失败返回错误信息
///
/// # 示例（前端调用）
/// ```typescript
/// const templates = await invoke('list_templates', {
///   projectId: 'project-1'
/// });
/// ```
#[tauri::command]
pub async fn list_templates(
    db: State<'_, DbConnection>,
    project_id: String,
) -> Result<Vec<Template>, String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;
    TemplateService::list_templates(&conn, &project_id).map_err(|e| e.to_string())
}

/// 获取单个模板
///
/// 根据 ID 获取模板详情。
///
/// # 参数
/// - `db`: 数据库连接状态
/// - `id`: 模板 ID
///
/// # 返回
/// - 成功返回 Option<Template>，不存在时返回 None
/// - 失败返回错误信息
///
/// # 示例（前端调用）
/// ```typescript
/// const template = await invoke('get_template', {
///   id: 'template-1'
/// });
/// ```
#[tauri::command]
pub async fn get_template(
    db: State<'_, DbConnection>,
    id: String,
) -> Result<Option<Template>, String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;
    TemplateService::get_template(&conn, &id).map_err(|e| e.to_string())
}

/// 更新模板
///
/// 更新指定模板的配置信息。
///
/// # 参数
/// - `db`: 数据库连接状态
/// - `id`: 模板 ID
/// - `update`: 更新内容，只包含需要更新的字段
///
/// # 返回
/// - 成功返回更新后的模板
/// - 失败返回错误信息
///
/// # 示例（前端调用）
/// ```typescript
/// const template = await invoke('update_template', {
///   id: 'template-1',
///   update: {
///     name: '新名称',
///     title_style: '新标题风格',
///     emoji_usage: 'moderate',
///   }
/// });
/// ```
#[tauri::command]
pub async fn update_template(
    db: State<'_, DbConnection>,
    id: String,
    update: TemplateUpdate,
) -> Result<Template, String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;
    TemplateService::update_template(&conn, &id, update).map_err(|e| e.to_string())
}

/// 删除模板
///
/// 删除指定的排版模板。
///
/// # 参数
/// - `db`: 数据库连接状态
/// - `id`: 模板 ID
///
/// # 返回
/// - 成功返回 ()
/// - 失败返回错误信息
///
/// # 示例（前端调用）
/// ```typescript
/// await invoke('delete_template', {
///   id: 'template-1'
/// });
/// ```
#[tauri::command]
pub async fn delete_template(db: State<'_, DbConnection>, id: String) -> Result<(), String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;
    TemplateService::delete_template(&conn, &id).map_err(|e| e.to_string())
}

/// 设置项目默认模板
///
/// 将指定模板设为项目的默认模板。
/// 同一项目只能有一个默认模板，设置新默认会自动取消原有默认。
///
/// # 参数
/// - `db`: 数据库连接状态
/// - `project_id`: 项目 ID
/// - `template_id`: 要设为默认的模板 ID
///
/// # 返回
/// - 成功返回 ()
/// - 失败返回错误信息
///
/// # 示例（前端调用）
/// ```typescript
/// await invoke('set_default_template', {
///   projectId: 'project-1',
///   templateId: 'template-1'
/// });
/// ```
#[tauri::command]
pub async fn set_default_template(
    db: State<'_, DbConnection>,
    project_id: String,
    template_id: String,
) -> Result<(), String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;
    TemplateService::set_default_template(&conn, &project_id, &template_id)
        .map_err(|e| e.to_string())
}

/// 获取项目的默认模板
///
/// 获取指定项目的默认排版模板。
///
/// # 参数
/// - `db`: 数据库连接状态
/// - `project_id`: 项目 ID
///
/// # 返回
/// - 成功返回 Option<Template>，没有默认模板时返回 None
/// - 失败返回错误信息
///
/// # 示例（前端调用）
/// ```typescript
/// const defaultTemplate = await invoke('get_default_template', {
///   projectId: 'project-1'
/// });
/// ```
#[tauri::command]
pub async fn get_default_template(
    db: State<'_, DbConnection>,
    project_id: String,
) -> Result<Option<Template>, String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;
    TemplateService::get_default_template(&conn, &project_id).map_err(|e| e.to_string())
}
