//! 素材服务层
//!
//! 提供素材（Material）的业务逻辑，包括：
//! - 文件上传、存储、删除
//! - 素材列表、获取、更新
//! - 素材内容读取（用于 AI 引用）
//!
//! ## 相关需求
//! - Requirements 7.1: 素材列表显示
//! - Requirements 7.2: 上传素材按钮
//! - Requirements 7.3: 素材创建
//! - Requirements 7.4: 素材搜索和筛选
//! - Requirements 7.5: 素材预览
//! - Requirements 7.6: 素材删除

use std::fs;
use std::path::PathBuf;

use rusqlite::Connection;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::database::dao::material_dao::MaterialDao;
use crate::errors::project_error::MaterialError;
use crate::models::project_model::{
    Material, MaterialFilter, MaterialUpdate, UploadMaterialRequest,
};

// ============================================================================
// 常量定义
// ============================================================================

/// 最大文件大小：50MB
const MAX_FILE_SIZE: u64 = 50 * 1024 * 1024;

/// 支持的文档类型
const SUPPORTED_DOCUMENT_TYPES: &[&str] = &["pdf", "doc", "docx", "txt", "md", "rtf", "odt"];

/// 支持的图片类型
const SUPPORTED_IMAGE_TYPES: &[&str] = &["jpg", "jpeg", "png", "gif", "webp", "svg", "bmp"];

/// 支持的数据类型
const SUPPORTED_DATA_TYPES: &[&str] = &["csv", "json", "xml", "xlsx", "xls"];

// ============================================================================
// 素材服务
// ============================================================================

/// 素材服务
///
/// 封装素材的业务逻辑，包括文件处理和数据库操作。
/// 文件存储在 `~/.proxycast/materials/{project_id}/` 目录下。
pub struct MaterialService;

impl MaterialService {
    // ------------------------------------------------------------------------
    // 上传素材
    // ------------------------------------------------------------------------

    /// 上传素材
    ///
    /// 处理文件上传逻辑：
    /// 1. 验证项目存在
    /// 2. 验证文件类型和大小
    /// 3. 复制文件到存储目录
    /// 4. 创建数据库记录
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `req`: 上传素材请求
    ///
    /// # 返回
    /// - 成功返回创建的素材
    /// - 失败返回 MaterialError
    ///
    /// # 示例
    /// ```ignore
    /// let req = UploadMaterialRequest {
    ///     project_id: "project-1".to_string(),
    ///     name: "参考文档.pdf".to_string(),
    ///     material_type: "document".to_string(),
    ///     file_path: Some("/tmp/upload.pdf".to_string()),
    ///     ..Default::default()
    /// };
    /// let material = MaterialService::upload_material(&conn, req)?;
    /// ```
    pub fn upload_material(
        conn: &Connection,
        req: UploadMaterialRequest,
    ) -> Result<Material, MaterialError> {
        // 验证项目存在
        Self::validate_project_exists(conn, &req.project_id)?;

        // 处理文件上传
        let (stored_path, file_size, mime_type) = if let Some(ref source_path) = req.file_path {
            Self::process_file_upload(&req.project_id, source_path, &req.material_type)?
        } else {
            (None, None, None)
        };

        // 创建修改后的请求（使用存储路径）
        let mut modified_req = req.clone();
        modified_req.file_path = stored_path;

        // 调用 DAO 创建素材（带文件元数据）
        let material =
            MaterialDao::create_with_metadata(conn, &modified_req, file_size, mime_type)?;

        info!(
            material_id = %material.id,
            project_id = %material.project_id,
            name = %material.name,
            "素材上传成功"
        );

        Ok(material)
    }

    // ------------------------------------------------------------------------
    // 获取素材列表
    // ------------------------------------------------------------------------

    /// 获取项目的素材列表
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `project_id`: 项目 ID
    /// - `filter`: 可选的筛选条件
    ///
    /// # 返回
    /// - 成功返回素材列表
    /// - 失败返回 MaterialError
    pub fn list_materials(
        conn: &Connection,
        project_id: &str,
        filter: Option<MaterialFilter>,
    ) -> Result<Vec<Material>, MaterialError> {
        MaterialDao::list(conn, project_id, filter.as_ref())
    }

    // ------------------------------------------------------------------------
    // 获取单个素材
    // ------------------------------------------------------------------------

    /// 获取单个素材
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `id`: 素材 ID
    ///
    /// # 返回
    /// - 成功返回 Option<Material>
    /// - 失败返回 MaterialError
    pub fn get_material(conn: &Connection, id: &str) -> Result<Option<Material>, MaterialError> {
        MaterialDao::get(conn, id)
    }

    // ------------------------------------------------------------------------
    // 更新素材
    // ------------------------------------------------------------------------

    /// 更新素材元数据
    ///
    /// 注意：只能更新名称、标签和描述，不能更新文件内容。
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `id`: 素材 ID
    /// - `update`: 更新内容
    ///
    /// # 返回
    /// - 成功返回更新后的素材
    /// - 失败返回 MaterialError
    pub fn update_material(
        conn: &Connection,
        id: &str,
        update: MaterialUpdate,
    ) -> Result<Material, MaterialError> {
        let material = MaterialDao::update(conn, id, &update)?;

        debug!(
            material_id = %id,
            "素材元数据更新成功"
        );

        Ok(material)
    }

    // ------------------------------------------------------------------------
    // 删除素材
    // ------------------------------------------------------------------------

    /// 删除素材
    ///
    /// 同时删除数据库记录和文件系统中的文件。
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `id`: 素材 ID
    ///
    /// # 返回
    /// - 成功返回 ()
    /// - 失败返回 MaterialError
    ///
    /// # 注意
    /// 此方法会删除文件系统中的文件，操作不可逆。
    pub fn delete_material(conn: &Connection, id: &str) -> Result<(), MaterialError> {
        // 先从数据库删除，获取文件路径
        let material = MaterialDao::delete(conn, id)?;

        // 删除文件（如果存在）
        if let Some(ref file_path) = material.file_path {
            Self::delete_file(file_path);
        }

        info!(
            material_id = %id,
            "素材删除成功"
        );

        Ok(())
    }

    /// 删除项目的所有素材
    ///
    /// 同时删除数据库记录和文件系统中的所有文件。
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `project_id`: 项目 ID
    ///
    /// # 返回
    /// - 成功返回删除的素材数量
    /// - 失败返回 MaterialError
    pub fn delete_materials_by_project(
        conn: &Connection,
        project_id: &str,
    ) -> Result<usize, MaterialError> {
        // 先从数据库删除，获取所有文件路径
        let materials = MaterialDao::delete_by_project(conn, project_id)?;
        let count = materials.len();

        // 删除所有文件
        for material in &materials {
            if let Some(ref file_path) = material.file_path {
                Self::delete_file(file_path);
            }
        }

        // 尝试删除项目的素材目录
        if let Ok(storage_dir) = Self::get_storage_dir(project_id) {
            if storage_dir.exists() {
                if let Err(e) = fs::remove_dir(&storage_dir) {
                    warn!(
                        project_id = %project_id,
                        error = %e,
                        "删除素材目录失败（可能非空）"
                    );
                }
            }
        }

        info!(
            project_id = %project_id,
            count = count,
            "项目素材批量删除成功"
        );

        Ok(count)
    }

    // ------------------------------------------------------------------------
    // 获取素材内容
    // ------------------------------------------------------------------------

    /// 获取素材内容（用于 AI 引用）
    ///
    /// 根据素材类型读取内容：
    /// - text 类型：直接返回 content 字段
    /// - document 类型：读取文件内容（仅支持文本文件）
    /// - 其他类型：返回描述信息
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `id`: 素材 ID
    ///
    /// # 返回
    /// - 成功返回素材内容字符串
    /// - 失败返回 MaterialError
    ///
    /// # 注意
    /// 对于二进制文件（如图片、PDF），返回文件描述而非内容。
    pub fn get_material_content(conn: &Connection, id: &str) -> Result<String, MaterialError> {
        let material =
            MaterialDao::get(conn, id)?.ok_or_else(|| MaterialError::NotFound(id.to_string()))?;

        let content = Self::extract_content(&material)?;

        debug!(
            material_id = %id,
            content_length = content.len(),
            "素材内容读取成功"
        );

        Ok(content)
    }

    /// 批量获取素材内容（用于项目上下文构建）
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `project_id`: 项目 ID
    ///
    /// # 返回
    /// - 成功返回素材内容列表（包含名称和内容）
    /// - 失败返回 MaterialError
    pub fn get_materials_content(
        conn: &Connection,
        project_id: &str,
    ) -> Result<Vec<(String, String)>, MaterialError> {
        let materials = MaterialDao::list(conn, project_id, None)?;
        let mut contents = Vec::new();

        for material in materials {
            match Self::extract_content(&material) {
                Ok(content) => {
                    contents.push((material.name, content));
                }
                Err(e) => {
                    warn!(
                        material_id = %material.id,
                        error = %e,
                        "读取素材内容失败，跳过"
                    );
                }
            }
        }

        Ok(contents)
    }

    // ------------------------------------------------------------------------
    // 辅助方法 - 文件处理
    // ------------------------------------------------------------------------

    /// 处理文件上传
    ///
    /// 验证文件并复制到存储目录。
    fn process_file_upload(
        project_id: &str,
        source_path: &str,
        material_type: &str,
    ) -> Result<(Option<String>, Option<i64>, Option<String>), MaterialError> {
        let source = PathBuf::from(source_path);

        // 验证源文件存在
        if !source.exists() {
            return Err(MaterialError::FileReadError(format!(
                "文件不存在: {}",
                source_path
            )));
        }

        // 获取文件元数据
        let metadata = fs::metadata(&source)?;
        let file_size = metadata.len();

        // 验证文件大小
        if file_size > MAX_FILE_SIZE {
            return Err(MaterialError::FileTooLarge(file_size, MAX_FILE_SIZE));
        }

        // 获取文件扩展名
        let extension = source
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();

        // 验证文件类型
        Self::validate_file_type(&extension, material_type)?;

        // 推断 MIME 类型
        let mime_type = Self::infer_mime_type(&extension);

        // 生成存储路径
        let storage_dir = Self::get_storage_dir(project_id)?;
        fs::create_dir_all(&storage_dir)?;

        let file_name = format!(
            "{}_{}.{}",
            Uuid::new_v4(),
            source
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("file"),
            extension
        );
        let dest_path = storage_dir.join(&file_name);

        // 复制文件
        fs::copy(&source, &dest_path)?;

        debug!(
            source = %source_path,
            dest = %dest_path.display(),
            size = file_size,
            "文件复制成功"
        );

        Ok((
            Some(dest_path.to_string_lossy().to_string()),
            Some(file_size as i64),
            mime_type,
        ))
    }

    /// 获取存储目录
    fn get_storage_dir(project_id: &str) -> Result<PathBuf, MaterialError> {
        let home = dirs::home_dir()
            .ok_or_else(|| MaterialError::FileReadError("无法获取主目录".to_string()))?;

        Ok(home.join(".proxycast").join("materials").join(project_id))
    }

    /// 删除文件
    fn delete_file(file_path: &str) {
        let path = PathBuf::from(file_path);
        if path.exists() {
            if let Err(e) = fs::remove_file(&path) {
                error!(
                    path = %file_path,
                    error = %e,
                    "删除文件失败"
                );
            } else {
                debug!(path = %file_path, "文件删除成功");
            }
        }
    }

    // ------------------------------------------------------------------------
    // 辅助方法 - 验证
    // ------------------------------------------------------------------------

    /// 验证项目是否存在
    fn validate_project_exists(conn: &Connection, project_id: &str) -> Result<(), MaterialError> {
        let mut stmt = conn
            .prepare("SELECT 1 FROM workspaces WHERE id = ?")
            .map_err(MaterialError::DatabaseError)?;

        let exists = stmt
            .exists([project_id])
            .map_err(MaterialError::DatabaseError)?;

        if !exists {
            return Err(MaterialError::ProjectNotFound(project_id.to_string()));
        }

        Ok(())
    }

    /// 验证文件类型
    fn validate_file_type(extension: &str, material_type: &str) -> Result<(), MaterialError> {
        let is_valid = match material_type {
            "document" => SUPPORTED_DOCUMENT_TYPES.contains(&extension),
            "image" => SUPPORTED_IMAGE_TYPES.contains(&extension),
            "data" => SUPPORTED_DATA_TYPES.contains(&extension),
            "text" => extension == "txt" || extension == "md",
            "link" => true, // 链接类型不需要文件
            _ => false,
        };

        if !is_valid {
            return Err(MaterialError::UnsupportedFileType(format!(
                ".{} (类型: {})",
                extension, material_type
            )));
        }

        Ok(())
    }

    /// 推断 MIME 类型
    fn infer_mime_type(extension: &str) -> Option<String> {
        let mime = match extension {
            // 文档
            "pdf" => "application/pdf",
            "doc" => "application/msword",
            "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            "txt" => "text/plain",
            "md" => "text/markdown",
            "rtf" => "application/rtf",
            "odt" => "application/vnd.oasis.opendocument.text",
            // 图片
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "svg" => "image/svg+xml",
            "bmp" => "image/bmp",
            // 数据
            "csv" => "text/csv",
            "json" => "application/json",
            "xml" => "application/xml",
            "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            "xls" => "application/vnd.ms-excel",
            _ => return None,
        };

        Some(mime.to_string())
    }

    // ------------------------------------------------------------------------
    // 辅助方法 - 内容提取
    // ------------------------------------------------------------------------

    /// 提取素材内容
    fn extract_content(material: &Material) -> Result<String, MaterialError> {
        // 优先使用 content 字段
        if let Some(ref content) = material.content {
            if !content.is_empty() {
                return Ok(content.clone());
            }
        }

        // 根据类型处理
        match material.material_type.as_str() {
            "text" => {
                // 文本类型：尝试读取文件
                if let Some(ref file_path) = material.file_path {
                    Self::read_text_file(file_path)
                } else {
                    Ok(format!("[文本素材: {}]", material.name))
                }
            }
            "document" => {
                // 文档类型：尝试读取文本文件，否则返回描述
                if let Some(ref file_path) = material.file_path {
                    let path = PathBuf::from(file_path);
                    let ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|e| e.to_lowercase())
                        .unwrap_or_default();

                    // 只读取纯文本文件
                    if ext == "txt" || ext == "md" {
                        Self::read_text_file(file_path)
                    } else {
                        Ok(Self::format_material_description(material))
                    }
                } else {
                    Ok(Self::format_material_description(material))
                }
            }
            "image" => {
                // 图片类型：返回描述
                Ok(Self::format_material_description(material))
            }
            "data" => {
                // 数据类型：尝试读取 CSV/JSON
                if let Some(ref file_path) = material.file_path {
                    let path = PathBuf::from(file_path);
                    let ext = path
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|e| e.to_lowercase())
                        .unwrap_or_default();

                    if ext == "csv" || ext == "json" {
                        Self::read_text_file(file_path)
                    } else {
                        Ok(Self::format_material_description(material))
                    }
                } else {
                    Ok(Self::format_material_description(material))
                }
            }
            "link" => {
                // 链接类型：返回描述
                Ok(Self::format_material_description(material))
            }
            _ => Ok(Self::format_material_description(material)),
        }
    }

    /// 读取文本文件
    fn read_text_file(file_path: &str) -> Result<String, MaterialError> {
        fs::read_to_string(file_path).map_err(|e| {
            MaterialError::FileReadError(format!("读取文件失败: {} - {}", file_path, e))
        })
    }

    /// 格式化素材描述
    fn format_material_description(material: &Material) -> String {
        let mut desc = format!("[素材: {}]", material.name);

        if let Some(ref description) = material.description {
            desc.push_str(&format!("\n描述: {}", description));
        }

        if !material.tags.is_empty() {
            desc.push_str(&format!("\n标签: {}", material.tags.join(", ")));
        }

        if let Some(size) = material.file_size {
            desc.push_str(&format!("\n大小: {} KB", size / 1024));
        }

        desc
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::schema::create_tables;
    use std::io::Write;
    use tempfile::TempDir;

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

    /// 创建临时测试文件
    fn create_temp_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
        let path = dir.path().join(name);
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn test_upload_material_text_content() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        let req = UploadMaterialRequest {
            project_id: "project-1".to_string(),
            name: "测试文本".to_string(),
            material_type: "text".to_string(),
            file_path: None,
            content: Some("这是测试内容".to_string()),
            tags: Some(vec!["测试".to_string()]),
            description: Some("测试描述".to_string()),
        };

        let material = MaterialService::upload_material(&conn, req).unwrap();

        assert!(!material.id.is_empty());
        assert_eq!(material.project_id, "project-1");
        assert_eq!(material.name, "测试文本");
        assert_eq!(material.content, Some("这是测试内容".to_string()));
    }

    #[test]
    fn test_upload_material_project_not_found() {
        let conn = setup_test_db();

        let req = UploadMaterialRequest {
            project_id: "nonexistent".to_string(),
            name: "测试".to_string(),
            material_type: "text".to_string(),
            file_path: None,
            content: None,
            tags: None,
            description: None,
        };

        let result = MaterialService::upload_material(&conn, req);
        assert!(result.is_err());

        match result.unwrap_err() {
            MaterialError::ProjectNotFound(id) => assert_eq!(id, "nonexistent"),
            _ => panic!("期望 ProjectNotFound 错误"),
        }
    }

    #[test]
    fn test_list_materials() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        // 创建多个素材
        for i in 1..=3 {
            let req = UploadMaterialRequest {
                project_id: "project-1".to_string(),
                name: format!("素材{}", i),
                material_type: "text".to_string(),
                file_path: None,
                content: Some(format!("内容{}", i)),
                tags: None,
                description: None,
            };
            MaterialService::upload_material(&conn, req).unwrap();
        }

        let materials = MaterialService::list_materials(&conn, "project-1", None).unwrap();
        assert_eq!(materials.len(), 3);
    }

    #[test]
    fn test_list_materials_with_filter() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        // 创建不同类型的素材
        let types = vec!["document", "image", "text"];
        for (i, t) in types.iter().enumerate() {
            let req = UploadMaterialRequest {
                project_id: "project-1".to_string(),
                name: format!("素材{}", i),
                material_type: t.to_string(),
                file_path: None,
                content: Some("内容".to_string()),
                tags: None,
                description: None,
            };
            MaterialService::upload_material(&conn, req).unwrap();
        }

        // 筛选 text 类型
        let filter = MaterialFilter {
            material_type: Some("text".to_string()),
            tags: None,
            search_query: None,
        };
        let materials = MaterialService::list_materials(&conn, "project-1", Some(filter)).unwrap();
        assert_eq!(materials.len(), 1);
        assert_eq!(materials[0].material_type, "text");
    }

    #[test]
    fn test_get_material() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        let req = UploadMaterialRequest {
            project_id: "project-1".to_string(),
            name: "测试素材".to_string(),
            material_type: "text".to_string(),
            file_path: None,
            content: Some("内容".to_string()),
            tags: None,
            description: None,
        };

        let created = MaterialService::upload_material(&conn, req).unwrap();
        let fetched = MaterialService::get_material(&conn, &created.id).unwrap();

        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().id, created.id);
    }

    #[test]
    fn test_update_material() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        let req = UploadMaterialRequest {
            project_id: "project-1".to_string(),
            name: "原始名称".to_string(),
            material_type: "text".to_string(),
            file_path: None,
            content: Some("内容".to_string()),
            tags: Some(vec!["标签1".to_string()]),
            description: Some("原始描述".to_string()),
        };

        let created = MaterialService::upload_material(&conn, req).unwrap();

        let update = MaterialUpdate {
            name: Some("更新后名称".to_string()),
            tags: Some(vec!["标签2".to_string()]),
            description: Some("更新后描述".to_string()),
        };

        let updated = MaterialService::update_material(&conn, &created.id, update).unwrap();

        assert_eq!(updated.name, "更新后名称");
        assert_eq!(updated.tags, vec!["标签2".to_string()]);
        assert_eq!(updated.description, Some("更新后描述".to_string()));
    }

    #[test]
    fn test_delete_material() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        let req = UploadMaterialRequest {
            project_id: "project-1".to_string(),
            name: "待删除素材".to_string(),
            material_type: "text".to_string(),
            file_path: None,
            content: Some("内容".to_string()),
            tags: None,
            description: None,
        };

        let created = MaterialService::upload_material(&conn, req).unwrap();

        // 验证素材存在
        assert!(MaterialService::get_material(&conn, &created.id)
            .unwrap()
            .is_some());

        // 删除素材
        MaterialService::delete_material(&conn, &created.id).unwrap();

        // 验证素材已删除
        assert!(MaterialService::get_material(&conn, &created.id)
            .unwrap()
            .is_none());
    }

    #[test]
    fn test_get_material_content_text() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        let req = UploadMaterialRequest {
            project_id: "project-1".to_string(),
            name: "文本素材".to_string(),
            material_type: "text".to_string(),
            file_path: None,
            content: Some("这是文本内容".to_string()),
            tags: None,
            description: None,
        };

        let material = MaterialService::upload_material(&conn, req).unwrap();
        let content = MaterialService::get_material_content(&conn, &material.id).unwrap();

        assert_eq!(content, "这是文本内容");
    }

    #[test]
    fn test_get_material_content_not_found() {
        let conn = setup_test_db();
        let result = MaterialService::get_material_content(&conn, "nonexistent");
        assert!(result.is_err());

        match result.unwrap_err() {
            MaterialError::NotFound(id) => assert_eq!(id, "nonexistent"),
            _ => panic!("期望 NotFound 错误"),
        }
    }

    #[test]
    fn test_get_materials_content() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        // 创建多个素材
        for i in 1..=3 {
            let req = UploadMaterialRequest {
                project_id: "project-1".to_string(),
                name: format!("素材{}", i),
                material_type: "text".to_string(),
                file_path: None,
                content: Some(format!("内容{}", i)),
                tags: None,
                description: None,
            };
            MaterialService::upload_material(&conn, req).unwrap();
        }

        let contents = MaterialService::get_materials_content(&conn, "project-1").unwrap();
        assert_eq!(contents.len(), 3);

        // 验证内容
        for (name, content) in &contents {
            assert!(name.starts_with("素材"));
            assert!(content.starts_with("内容"));
        }
    }

    #[test]
    fn test_validate_file_type() {
        // 文档类型
        assert!(MaterialService::validate_file_type("pdf", "document").is_ok());
        assert!(MaterialService::validate_file_type("docx", "document").is_ok());
        assert!(MaterialService::validate_file_type("exe", "document").is_err());

        // 图片类型
        assert!(MaterialService::validate_file_type("jpg", "image").is_ok());
        assert!(MaterialService::validate_file_type("png", "image").is_ok());
        assert!(MaterialService::validate_file_type("pdf", "image").is_err());

        // 数据类型
        assert!(MaterialService::validate_file_type("csv", "data").is_ok());
        assert!(MaterialService::validate_file_type("json", "data").is_ok());
        assert!(MaterialService::validate_file_type("jpg", "data").is_err());

        // 文本类型
        assert!(MaterialService::validate_file_type("txt", "text").is_ok());
        assert!(MaterialService::validate_file_type("md", "text").is_ok());
        assert!(MaterialService::validate_file_type("pdf", "text").is_err());
    }

    #[test]
    fn test_infer_mime_type() {
        assert_eq!(
            MaterialService::infer_mime_type("pdf"),
            Some("application/pdf".to_string())
        );
        assert_eq!(
            MaterialService::infer_mime_type("jpg"),
            Some("image/jpeg".to_string())
        );
        assert_eq!(
            MaterialService::infer_mime_type("json"),
            Some("application/json".to_string())
        );
        assert_eq!(MaterialService::infer_mime_type("unknown"), None);
    }

    #[test]
    fn test_upload_material_with_file() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        // 创建临时文件
        let temp_dir = TempDir::new().unwrap();
        let file_path = create_temp_file(&temp_dir, "test.txt", "文件内容测试");

        let req = UploadMaterialRequest {
            project_id: "project-1".to_string(),
            name: "测试文件".to_string(),
            material_type: "text".to_string(),
            file_path: Some(file_path.to_string_lossy().to_string()),
            content: None,
            tags: None,
            description: None,
        };

        let material = MaterialService::upload_material(&conn, req).unwrap();

        assert!(!material.id.is_empty());
        assert!(material.file_path.is_some());
        assert!(material.file_size.is_some());
        assert_eq!(material.mime_type, Some("text/plain".to_string()));

        // 验证文件已复制到存储目录
        let stored_path = PathBuf::from(material.file_path.as_ref().unwrap());
        assert!(stored_path.exists());

        // 清理：删除素材（会删除文件）
        MaterialService::delete_material(&conn, &material.id).unwrap();
        assert!(!stored_path.exists());
    }

    #[test]
    fn test_upload_material_file_not_found() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        let req = UploadMaterialRequest {
            project_id: "project-1".to_string(),
            name: "测试".to_string(),
            material_type: "document".to_string(),
            file_path: Some("/nonexistent/file.pdf".to_string()),
            content: None,
            tags: None,
            description: None,
        };

        let result = MaterialService::upload_material(&conn, req);
        assert!(result.is_err());

        match result.unwrap_err() {
            MaterialError::FileReadError(_) => {}
            _ => panic!("期望 FileReadError 错误"),
        }
    }

    #[test]
    fn test_delete_materials_by_project() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");
        create_test_project(&conn, "project-2");

        // 为 project-1 创建素材
        for i in 1..=3 {
            let req = UploadMaterialRequest {
                project_id: "project-1".to_string(),
                name: format!("素材{}", i),
                material_type: "text".to_string(),
                file_path: None,
                content: Some("内容".to_string()),
                tags: None,
                description: None,
            };
            MaterialService::upload_material(&conn, req).unwrap();
        }

        // 为 project-2 创建素材
        let req = UploadMaterialRequest {
            project_id: "project-2".to_string(),
            name: "素材".to_string(),
            material_type: "text".to_string(),
            file_path: None,
            content: Some("内容".to_string()),
            tags: None,
            description: None,
        };
        MaterialService::upload_material(&conn, req).unwrap();

        // 删除 project-1 的所有素材
        let count = MaterialService::delete_materials_by_project(&conn, "project-1").unwrap();
        assert_eq!(count, 3);

        // 验证 project-1 没有素材了
        let materials = MaterialService::list_materials(&conn, "project-1", None).unwrap();
        assert_eq!(materials.len(), 0);

        // 验证 project-2 的素材未受影响
        let materials = MaterialService::list_materials(&conn, "project-2", None).unwrap();
        assert_eq!(materials.len(), 1);
    }

    #[test]
    fn test_format_material_description() {
        let material = Material {
            id: "test-id".to_string(),
            project_id: "project-1".to_string(),
            name: "测试图片.jpg".to_string(),
            material_type: "image".to_string(),
            file_path: Some("/path/to/image.jpg".to_string()),
            file_size: Some(102400), // 100KB
            mime_type: Some("image/jpeg".to_string()),
            content: None,
            tags: vec!["风景".to_string(), "自然".to_string()],
            description: Some("一张美丽的风景图".to_string()),
            created_at: 0,
        };

        let desc = MaterialService::format_material_description(&material);

        assert!(desc.contains("[素材: 测试图片.jpg]"));
        assert!(desc.contains("描述: 一张美丽的风景图"));
        assert!(desc.contains("标签: 风景, 自然"));
        assert!(desc.contains("大小: 100 KB"));
    }
}
