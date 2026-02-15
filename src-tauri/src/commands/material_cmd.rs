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

use base64::Engine;
use reqwest::header::CONTENT_TYPE;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use tauri::State;
use tracing::warn;
use url::Url;
use uuid::Uuid;

use crate::database::DbConnection;
use crate::models::project_model::{
    Material, MaterialFilter, MaterialUpdate, UploadMaterialRequest,
};
use proxycast_services::material_service::MaterialService;

const IMPORT_MAX_FILE_SIZE: usize = 50 * 1024 * 1024;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportMaterialFromUrlRequest {
    pub project_id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub material_type: String,
    pub url: String,
    pub tags: Option<Vec<String>>,
    pub description: Option<String>,
}

fn sanitize_extension(extension: &str) -> Option<String> {
    let normalized = extension.trim().trim_start_matches('.').to_lowercase();
    if normalized.is_empty() || normalized.len() > 12 {
        return None;
    }
    if normalized.chars().all(|c| c.is_ascii_alphanumeric()) {
        Some(normalized)
    } else {
        None
    }
}

fn extension_from_name(name: &str) -> Option<String> {
    Path::new(name)
        .extension()
        .and_then(|value| value.to_str())
        .and_then(sanitize_extension)
}

fn extension_from_url(raw_url: &str) -> Option<String> {
    let parsed = Url::parse(raw_url).ok()?;
    let filename = parsed.path_segments()?.next_back()?;
    if filename.is_empty() {
        return None;
    }
    let extension = filename.rsplit_once('.')?.1;
    sanitize_extension(extension)
}

fn extension_from_mime(mime_type: &str) -> Option<&'static str> {
    match mime_type {
        "image/jpeg" => Some("jpg"),
        "image/png" => Some("png"),
        "image/gif" => Some("gif"),
        "image/webp" => Some("webp"),
        "image/svg+xml" => Some("svg"),
        "image/bmp" => Some("bmp"),
        "audio/mpeg" | "audio/mp3" => Some("mp3"),
        "audio/wav" => Some("wav"),
        "audio/aac" => Some("aac"),
        "audio/ogg" => Some("ogg"),
        "audio/flac" => Some("flac"),
        "video/mp4" => Some("mp4"),
        "video/webm" => Some("webm"),
        "video/quicktime" => Some("mov"),
        "application/pdf" => Some("pdf"),
        "application/json" => Some("json"),
        "text/plain" => Some("txt"),
        "text/markdown" => Some("md"),
        _ => None,
    }
}

fn default_extension_by_material_type(material_type: &str) -> &'static str {
    match material_type {
        "image" => "png",
        "audio" => "mp3",
        "video" => "mp4",
        "data" => "json",
        "text" => "txt",
        "document" => "txt",
        _ => "png",
    }
}

fn resolve_import_extension(
    request: &ImportMaterialFromUrlRequest,
    mime_type: Option<&str>,
    normalized_material_type: &str,
) -> String {
    extension_from_name(&request.name)
        .or_else(|| extension_from_url(&request.url))
        .or_else(|| {
            mime_type
                .and_then(extension_from_mime)
                .map(|value| value.to_string())
        })
        .unwrap_or_else(|| default_extension_by_material_type(normalized_material_type).to_string())
}

fn normalize_material_name(
    raw_name: &str,
    raw_url: &str,
    material_type: &str,
    extension: &str,
) -> String {
    let trimmed = raw_name.trim();
    if !trimmed.is_empty() {
        return trimmed.to_string();
    }

    if let Some(name_from_url) = Url::parse(raw_url)
        .ok()
        .and_then(|url| {
            url.path_segments()
                .and_then(|mut segments| segments.next_back().map(|value| value.to_string()))
        })
        .map(|value| value.trim().to_string())
        .filter(|name| !name.is_empty())
    {
        return name_from_url;
    }

    let prefix = match material_type {
        "image" => "导入图片",
        "audio" => "导入语音",
        "video" => "导入视频",
        "data" => "导入数据",
        "text" => "导入文本",
        _ => "导入素材",
    };
    format!("{prefix}.{extension}")
}

fn decode_data_url(raw_url: &str) -> Result<(Vec<u8>, Option<String>), String> {
    let (header, payload) = raw_url
        .split_once(',')
        .ok_or_else(|| "data URL 格式不正确".to_string())?;

    if !header.starts_with("data:") {
        return Err("不支持的 URL 协议，仅支持 http(s) 或 data URL".to_string());
    }

    let meta = &header[5..];
    let mut mime_type: Option<String> = None;
    let mut is_base64 = false;

    if !meta.is_empty() {
        let mut segments = meta.split(';');
        if let Some(first) = segments.next() {
            let first_trimmed = first.trim();
            if !first_trimmed.is_empty() {
                mime_type = Some(first_trimmed.to_lowercase());
            }
        }
        is_base64 = segments.any(|segment| segment.eq_ignore_ascii_case("base64"));
    }

    if !is_base64 {
        return Err("暂不支持非 base64 的 data URL".to_string());
    }

    let decoded = base64::engine::general_purpose::STANDARD
        .decode(payload.trim())
        .map_err(|e| format!("data URL 解码失败: {e}"))?;

    if decoded.len() > IMPORT_MAX_FILE_SIZE {
        return Err(format!(
            "文件过大: {} bytes (最大 {} bytes)",
            decoded.len(),
            IMPORT_MAX_FILE_SIZE
        ));
    }

    Ok((decoded, mime_type))
}

async fn download_remote_file(raw_url: &str) -> Result<(Vec<u8>, Option<String>), String> {
    let parsed_url = Url::parse(raw_url).map_err(|e| format!("URL 无效: {e}"))?;
    let scheme = parsed_url.scheme().to_lowercase();
    if scheme != "http" && scheme != "https" {
        return Err("仅支持 http(s) 协议".to_string());
    }

    let response = reqwest::get(parsed_url)
        .await
        .map_err(|e| format!("下载失败: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("下载失败: HTTP {}", response.status()));
    }

    if let Some(content_length) = response.content_length() {
        if content_length > IMPORT_MAX_FILE_SIZE as u64 {
            return Err(format!(
                "文件过大: {content_length} bytes (最大 {IMPORT_MAX_FILE_SIZE} bytes)"
            ));
        }
    }

    let mime_type = response
        .headers()
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_lowercase());

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("读取下载内容失败: {e}"))?;

    if bytes.len() > IMPORT_MAX_FILE_SIZE {
        return Err(format!(
            "文件过大: {} bytes (最大 {} bytes)",
            bytes.len(),
            IMPORT_MAX_FILE_SIZE
        ));
    }

    Ok((bytes.to_vec(), mime_type))
}

async fn load_material_bytes(raw_url: &str) -> Result<(Vec<u8>, Option<String>), String> {
    if raw_url.starts_with("data:") {
        decode_data_url(raw_url)
    } else {
        download_remote_file(raw_url).await
    }
}

fn create_temp_file(bytes: &[u8], extension: &str) -> Result<String, String> {
    let file_name = format!("proxycast-material-{}.{}", Uuid::new_v4(), extension);
    let file_path = std::env::temp_dir().join(file_name);
    fs::write(&file_path, bytes).map_err(|e| format!("写入临时文件失败: {e}"))?;
    Ok(file_path.to_string_lossy().to_string())
}

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

#[tauri::command]
pub async fn import_material_from_url(
    db: State<'_, DbConnection>,
    req: ImportMaterialFromUrlRequest,
) -> Result<Material, String> {
    let normalized_material_type = req.material_type.trim().to_lowercase();
    if normalized_material_type.is_empty() {
        return Err("素材类型不能为空".to_string());
    }

    let normalized_url = req.url.trim();
    if normalized_url.is_empty() {
        return Err("URL 不能为空".to_string());
    }

    if normalized_material_type == "link" {
        let upload_req = UploadMaterialRequest {
            project_id: req.project_id,
            name: normalize_material_name(&req.name, normalized_url, "link", "txt"),
            material_type: normalized_material_type,
            file_path: None,
            content: Some(normalized_url.to_string()),
            tags: req.tags,
            description: req.description,
        };

        let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;
        return MaterialService::upload_material(&conn, upload_req).map_err(|e| e.to_string());
    }

    let (bytes, mime_type) = load_material_bytes(normalized_url).await?;
    let extension = resolve_import_extension(
        &req,
        mime_type.as_deref(),
        normalized_material_type.as_str(),
    );
    let temp_file_path = create_temp_file(&bytes, &extension)?;

    let upload_req = UploadMaterialRequest {
        project_id: req.project_id,
        name: normalize_material_name(
            &req.name,
            normalized_url,
            normalized_material_type.as_str(),
            &extension,
        ),
        material_type: normalized_material_type,
        file_path: Some(temp_file_path.clone()),
        content: None,
        tags: req.tags,
        description: req.description,
    };

    let conn = db.lock().map_err(|e| format!("数据库锁定失败: {e}"))?;
    let result = MaterialService::upload_material(&conn, upload_req).map_err(|e| e.to_string());

    if let Err(err) = fs::remove_file(&temp_file_path) {
        warn!(
            path = %temp_file_path,
            error = %err,
            "导入素材后删除临时文件失败"
        );
    }

    result
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_import_request(
        name: &str,
        url: &str,
        material_type: &str,
    ) -> ImportMaterialFromUrlRequest {
        ImportMaterialFromUrlRequest {
            project_id: "project-1".to_string(),
            name: name.to_string(),
            material_type: material_type.to_string(),
            url: url.to_string(),
            tags: None,
            description: None,
        }
    }

    #[test]
    fn test_decode_data_url_success() {
        let raw = "data:text/plain;base64,aGVsbG8=";
        let (bytes, mime_type) = decode_data_url(raw).expect("应成功解析 data URL");
        assert_eq!(bytes, b"hello");
        assert_eq!(mime_type.as_deref(), Some("text/plain"));
    }

    #[test]
    fn test_decode_data_url_non_base64_should_fail() {
        let raw = "data:text/plain,hello";
        let error = decode_data_url(raw).expect_err("非 base64 data URL 应失败");
        assert!(error.contains("非 base64"));
    }

    #[test]
    fn test_resolve_import_extension_priority() {
        let req_with_name = make_import_request("my-file.webp", "https://a.test/b/c.png", "image");
        assert_eq!(
            resolve_import_extension(&req_with_name, Some("image/jpeg"), "image"),
            "webp"
        );

        let req_with_url = make_import_request("", "https://a.test/b/c.jpeg", "image");
        assert_eq!(
            resolve_import_extension(&req_with_url, Some("image/png"), "image"),
            "jpeg"
        );

        let req_with_mime = make_import_request("", "https://a.test/download", "image");
        assert_eq!(
            resolve_import_extension(&req_with_mime, Some("image/png"), "image"),
            "png"
        );
    }

    #[test]
    fn test_normalize_material_name_fallback() {
        let from_name =
            normalize_material_name("  已命名.png  ", "https://a.test/x/y.png", "image", "png");
        assert_eq!(from_name, "已命名.png");

        let from_url = normalize_material_name("", "https://a.test/x/y.png", "image", "png");
        assert_eq!(from_url, "y.png");

        let fallback = normalize_material_name("", "invalid-url", "audio", "mp3");
        assert_eq!(fallback, "导入语音.mp3");
    }
}
