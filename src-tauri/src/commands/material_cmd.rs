//! 素材相关的 Tauri 命令
//!
//! 提供素材（Material）管理的前端 API，包括：
//! - 上传、获取、列表、更新、删除素材
//! - 获取素材内容（用于 AI 引用）
//!
//! ## 相关需求
//! - Requirements 7.1: 素材列表显示
//! - Requirements 7.2: 上传素材按钮
//! - Requirements 7.3: 素材创建
//! - Requirements 7.4: 素材搜索和筛选
//! - Requirements 7.5: 素材预览
//! - Requirements 7.6: 素材删除

use tauri::State;

use crate::database::DbConnection;
use crate::models::project_model::{
    Material, MaterialFilter, MaterialUpdate, UploadMaterialRequest,
};
use crate::services::material_service::MaterialService;

// ============================================================================
// Tauri 命令
// ============================================================================

/// 上传素材
///
/// 在指定项目中上传新的素材。支持文档、图片、文本、数据文件等类型。
///
/// # 参数
/// - `db`: 数据库连接状态
/// - `req`: 上传素材请求，包含项目 ID、名称、类型、文件路径等信息
///
/// # 返回
/// - 成功返回创建的素材
/// - 失败返回错误信息
///
/// # 示例（前端调用）
/// ```typescript
/// const material = await invoke('upload_material', {
///   req: {
///     project_id: 'project-1',
///     name: '参考文档.pdf',
///     type: 'document',
///     file_path: '/tmp/upload.pdf',
///     tags: ['参考', '重要'],
///     description: '项目参考文档',
///   }
/// });
/// ```
#[tauri::command]
pub async fn upload_material(
    db: State<'_, DbConnection>,
    req: UploadMaterialRequest,
) -> Result<Material, String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;
    MaterialService::upload_material(&conn, req).map_err(|e| e.to_string())
}

/// 获取项目的素材列表
///
/// 获取指定项目下的所有素材，支持按类型、标签和关键词筛选。
///
/// # 参数
/// - `db`: 数据库连接状态
/// - `project_id`: 项目 ID
/// - `filter`: 可选的筛选条件
///
/// # 返回
/// - 成功返回素材列表
/// - 失败返回错误信息
///
/// # 示例（前端调用）
/// ```typescript
/// // 获取所有素材
/// const materials = await invoke('list_materials', {
///   projectId: 'project-1'
/// });
///
/// // 按类型筛选
/// const documents = await invoke('list_materials', {
///   projectId: 'project-1',
///   filter: { type: 'document' }
/// });
///
/// // 按标签筛选
/// const important = await invoke('list_materials', {
///   projectId: 'project-1',
///   filter: { tags: ['重要'] }
/// });
/// ```
#[tauri::command]
pub async fn list_materials(
    db: State<'_, DbConnection>,
    project_id: String,
    filter: Option<MaterialFilter>,
) -> Result<Vec<Material>, String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;
    MaterialService::list_materials(&conn, &project_id, filter).map_err(|e| e.to_string())
}

/// 获取单个素材
///
/// 根据 ID 获取素材详情。
///
/// # 参数
/// - `db`: 数据库连接状态
/// - `id`: 素材 ID
///
/// # 返回
/// - 成功返回 Option<Material>，不存在时返回 None
/// - 失败返回错误信息
///
/// # 示例（前端调用）
/// ```typescript
/// const material = await invoke('get_material', {
///   id: 'material-1'
/// });
/// ```
#[tauri::command]
pub async fn get_material(
    db: State<'_, DbConnection>,
    id: String,
) -> Result<Option<Material>, String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;
    MaterialService::get_material(&conn, &id).map_err(|e| e.to_string())
}

/// 更新素材元数据
///
/// 更新指定素材的元数据信息（名称、标签、描述）。
/// 注意：不能更新文件内容，如需更新文件请删除后重新上传。
///
/// # 参数
/// - `db`: 数据库连接状态
/// - `id`: 素材 ID
/// - `update`: 更新内容，只包含需要更新的字段
///
/// # 返回
/// - 成功返回更新后的素材
/// - 失败返回错误信息
///
/// # 示例（前端调用）
/// ```typescript
/// const material = await invoke('update_material', {
///   id: 'material-1',
///   update: {
///     name: '新名称',
///     tags: ['新标签'],
///     description: '新描述',
///   }
/// });
/// ```
#[tauri::command]
pub async fn update_material(
    db: State<'_, DbConnection>,
    id: String,
    update: MaterialUpdate,
) -> Result<Material, String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;
    MaterialService::update_material(&conn, &id, update).map_err(|e| e.to_string())
}

/// 删除素材
///
/// 删除指定的素材，同时删除数据库记录和文件系统中的文件。
/// 此操作不可逆，请谨慎使用。
///
/// # 参数
/// - `db`: 数据库连接状态
/// - `id`: 素材 ID
///
/// # 返回
/// - 成功返回 ()
/// - 失败返回错误信息
///
/// # 示例（前端调用）
/// ```typescript
/// await invoke('delete_material', {
///   id: 'material-1'
/// });
/// ```
#[tauri::command]
pub async fn delete_material(db: State<'_, DbConnection>, id: String) -> Result<(), String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;
    MaterialService::delete_material(&conn, &id).map_err(|e| e.to_string())
}

/// 获取素材内容
///
/// 获取素材的文本内容，用于 AI 引用。
/// 根据素材类型返回不同的内容：
/// - text 类型：返回 content 字段或读取文件内容
/// - document 类型：对于文本文件返回内容，其他返回描述信息
/// - image/data/link 类型：返回描述信息
///
/// # 参数
/// - `db`: 数据库连接状态
/// - `id`: 素材 ID
///
/// # 返回
/// - 成功返回素材内容字符串
/// - 失败返回错误信息
///
/// # 示例（前端调用）
/// ```typescript
/// const content = await invoke('get_material_content', {
///   id: 'material-1'
/// });
/// ```
#[tauri::command]
pub async fn get_material_content(
    db: State<'_, DbConnection>,
    id: String,
) -> Result<String, String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;
    MaterialService::get_material_content(&conn, &id).map_err(|e| e.to_string())
}

/// 获取项目的素材数量
///
/// 获取指定项目下的素材总数。
///
/// # 参数
/// - `db`: 数据库连接状态
/// - `project_id`: 项目 ID
///
/// # 返回
/// - 成功返回素材数量
/// - 失败返回错误信息
///
/// # 示例（前端调用）
/// ```typescript
/// const count = await invoke('get_material_count', {
///   projectId: 'project-1'
/// });
/// ```
#[tauri::command]
pub async fn get_material_count(
    db: State<'_, DbConnection>,
    project_id: String,
) -> Result<i64, String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;
    crate::database::dao::material_dao::MaterialDao::count(&conn, &project_id)
        .map_err(|e| e.to_string())
}

/// 批量获取素材内容
///
/// 获取项目下所有素材的内容，用于构建项目上下文。
/// 返回素材名称和内容的列表。
///
/// # 参数
/// - `db`: 数据库连接状态
/// - `project_id`: 项目 ID
///
/// # 返回
/// - 成功返回素材内容列表 [(name, content), ...]
/// - 失败返回错误信息
///
/// # 示例（前端调用）
/// ```typescript
/// const contents = await invoke('get_materials_content', {
///   projectId: 'project-1'
/// });
/// // contents: [['文档1', '内容1'], ['文档2', '内容2']]
/// ```
#[tauri::command]
pub async fn get_materials_content(
    db: State<'_, DbConnection>,
    project_id: String,
) -> Result<Vec<(String, String)>, String> {
    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;
    MaterialService::get_materials_content(&conn, &project_id).map_err(|e| e.to_string())
}
