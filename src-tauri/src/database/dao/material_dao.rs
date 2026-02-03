//! 素材数据访问层
//!
//! 提供素材（Material）的 CRUD 操作，包括：
//! - 创建、获取、列表、更新、删除素材
//! - 支持按类型和标签筛选
//!
//! ## 相关需求
//! - Requirements 7.1: 素材列表显示
//! - Requirements 7.3: 素材创建
//! - Requirements 7.4: 素材搜索和筛选
//! - Requirements 7.6: 素材删除

use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::errors::project_error::MaterialError;
use crate::models::project_model::{
    Material, MaterialFilter, MaterialUpdate, UploadMaterialRequest,
};

// ============================================================================
// 数据访问对象
// ============================================================================

/// 素材 DAO
///
/// 提供素材的数据库操作方法。
pub struct MaterialDao;

impl MaterialDao {
    // ------------------------------------------------------------------------
    // 创建素材
    // ------------------------------------------------------------------------

    /// 创建新素材
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `req`: 上传素材请求
    ///
    /// # 返回
    /// - 成功返回创建的素材
    /// - 失败返回 MaterialError
    ///
    /// # 注意
    /// 文件大小和 MIME 类型应由 Service 层计算后传入。
    /// DAO 层只负责数据库操作，不处理文件系统。
    pub fn create(
        conn: &Connection,
        req: &UploadMaterialRequest,
    ) -> Result<Material, MaterialError> {
        Self::create_with_metadata(conn, req, None, None)
    }

    /// 创建新素材（带文件元数据）
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `req`: 上传素材请求
    /// - `file_size`: 文件大小（字节）
    /// - `mime_type`: MIME 类型
    ///
    /// # 返回
    /// - 成功返回创建的素材
    /// - 失败返回 MaterialError
    pub fn create_with_metadata(
        conn: &Connection,
        req: &UploadMaterialRequest,
        file_size: Option<i64>,
        mime_type: Option<String>,
    ) -> Result<Material, MaterialError> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();

        // 序列化 tags JSON 字段
        let tags_json = serde_json::to_string(req.tags.as_ref().unwrap_or(&vec![]))
            .unwrap_or_else(|_| "[]".to_string());

        conn.execute(
            "INSERT INTO materials (
                id, project_id, name, material_type, file_path, file_size,
                mime_type, content, tags_json, description, created_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                id,
                req.project_id,
                req.name,
                req.material_type,
                req.file_path,
                file_size,
                mime_type,
                req.content,
                tags_json,
                req.description,
                now,
            ],
        )?;

        // 返回创建的素材
        Ok(Material {
            id,
            project_id: req.project_id.clone(),
            name: req.name.clone(),
            material_type: req.material_type.clone(),
            file_path: req.file_path.clone(),
            file_size,
            mime_type,
            content: req.content.clone(),
            tags: req.tags.clone().unwrap_or_default(),
            description: req.description.clone(),
            created_at: now,
        })
    }

    // ------------------------------------------------------------------------
    // 获取素材
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
    pub fn get(conn: &Connection, id: &str) -> Result<Option<Material>, MaterialError> {
        let mut stmt = conn.prepare(
            "SELECT id, project_id, name, material_type, file_path, file_size,
                    mime_type, content, tags_json, description, created_at
             FROM materials WHERE id = ?",
        )?;

        let mut rows = stmt.query([id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::map_row(row)?))
        } else {
            Ok(None)
        }
    }

    // ------------------------------------------------------------------------
    // 列表素材
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
    pub fn list(
        conn: &Connection,
        project_id: &str,
        filter: Option<&MaterialFilter>,
    ) -> Result<Vec<Material>, MaterialError> {
        // 构建基础查询
        let mut sql = String::from(
            "SELECT id, project_id, name, material_type, file_path, file_size,
                    mime_type, content, tags_json, description, created_at
             FROM materials WHERE project_id = ?",
        );
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(project_id.to_string())];

        // 应用筛选条件
        if let Some(f) = filter {
            // 按类型筛选
            if let Some(ref material_type) = f.material_type {
                sql.push_str(" AND material_type = ?");
                params_vec.push(Box::new(material_type.clone()));
            }

            // 按搜索关键词筛选（名称或描述）
            if let Some(ref query) = f.search_query {
                sql.push_str(" AND (name LIKE ? OR description LIKE ?)");
                let pattern = format!("%{}%", query);
                params_vec.push(Box::new(pattern.clone()));
                params_vec.push(Box::new(pattern));
            }

            // 按标签筛选（使用 JSON 包含检查）
            if let Some(ref tags) = f.tags {
                for tag in tags {
                    sql.push_str(" AND tags_json LIKE ?");
                    params_vec.push(Box::new(format!("%\"{}%", tag)));
                }
            }
        }

        sql.push_str(" ORDER BY created_at DESC");

        let mut stmt = conn.prepare(&sql)?;

        // 转换参数为引用切片
        let params_refs: Vec<&dyn rusqlite::ToSql> =
            params_vec.iter().map(|p| p.as_ref()).collect();

        let materials: Vec<Material> = stmt
            .query_map(params_refs.as_slice(), |row| Self::map_row(row))?
            .filter_map(|r| r.ok())
            .collect();

        Ok(materials)
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
    pub fn update(
        conn: &Connection,
        id: &str,
        update: &MaterialUpdate,
    ) -> Result<Material, MaterialError> {
        // 先获取现有素材
        let existing =
            Self::get(conn, id)?.ok_or_else(|| MaterialError::NotFound(id.to_string()))?;

        // 构建更新后的值
        let name = update.name.as_ref().unwrap_or(&existing.name);
        let tags = update.tags.clone().unwrap_or(existing.tags);
        let description = update.description.clone().or(existing.description);

        // 序列化 tags JSON 字段
        let tags_json = serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string());

        conn.execute(
            "UPDATE materials SET name = ?1, tags_json = ?2, description = ?3 WHERE id = ?4",
            params![name, tags_json, description, id],
        )?;

        // 返回更新后的素材
        Self::get(conn, id)?.ok_or_else(|| MaterialError::NotFound(id.to_string()))
    }

    // ------------------------------------------------------------------------
    // 删除素材
    // ------------------------------------------------------------------------

    /// 删除素材
    ///
    /// 注意：此方法只删除数据库记录，不删除文件。
    /// 文件删除应由 Service 层处理。
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `id`: 素材 ID
    ///
    /// # 返回
    /// - 成功返回被删除的素材（用于后续文件清理）
    /// - 失败返回 MaterialError
    pub fn delete(conn: &Connection, id: &str) -> Result<Material, MaterialError> {
        // 先获取素材信息（用于返回文件路径）
        let material =
            Self::get(conn, id)?.ok_or_else(|| MaterialError::NotFound(id.to_string()))?;

        let rows = conn.execute("DELETE FROM materials WHERE id = ?", [id])?;

        if rows == 0 {
            return Err(MaterialError::NotFound(id.to_string()));
        }

        Ok(material)
    }

    // ------------------------------------------------------------------------
    // 批量操作
    // ------------------------------------------------------------------------

    /// 获取项目的素材数量
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `project_id`: 项目 ID
    ///
    /// # 返回
    /// - 成功返回素材数量
    /// - 失败返回 MaterialError
    pub fn count(conn: &Connection, project_id: &str) -> Result<i64, MaterialError> {
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM materials WHERE project_id = ?",
            [project_id],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// 删除项目的所有素材
    ///
    /// 注意：此方法只删除数据库记录，不删除文件。
    /// 文件删除应由 Service 层处理。
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `project_id`: 项目 ID
    ///
    /// # 返回
    /// - 成功返回被删除的素材列表（用于后续文件清理）
    /// - 失败返回 MaterialError
    pub fn delete_by_project(
        conn: &Connection,
        project_id: &str,
    ) -> Result<Vec<Material>, MaterialError> {
        // 先获取所有素材
        let materials = Self::list(conn, project_id, None)?;

        // 删除所有素材
        conn.execute("DELETE FROM materials WHERE project_id = ?", [project_id])?;

        Ok(materials)
    }

    // ------------------------------------------------------------------------
    // 辅助方法
    // ------------------------------------------------------------------------

    /// 映射数据库行到 Material 结构体
    fn map_row(row: &rusqlite::Row) -> Result<Material, rusqlite::Error> {
        let tags_json: String = row.get(8)?;

        // 解析 tags JSON 字段
        let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

        Ok(Material {
            id: row.get(0)?,
            project_id: row.get(1)?,
            name: row.get(2)?,
            material_type: row.get(3)?,
            file_path: row.get(4)?,
            file_size: row.get(5)?,
            mime_type: row.get(6)?,
            content: row.get(7)?,
            tags,
            description: row.get(9)?,
            created_at: row.get(10)?,
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
    fn test_create_material() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        let req = UploadMaterialRequest {
            project_id: "project-1".to_string(),
            name: "测试文档.pdf".to_string(),
            material_type: "document".to_string(),
            file_path: None,
            content: Some("这是文档内容".to_string()),
            tags: Some(vec!["参考".to_string(), "重要".to_string()]),
            description: Some("测试描述".to_string()),
        };

        let material = MaterialDao::create(&conn, &req).unwrap();

        assert!(!material.id.is_empty());
        assert_eq!(material.project_id, "project-1");
        assert_eq!(material.name, "测试文档.pdf");
        assert_eq!(material.material_type, "document");
        assert_eq!(material.content, Some("这是文档内容".to_string()));
        assert_eq!(material.tags.len(), 2);
        assert!(material.tags.contains(&"参考".to_string()));
    }

    #[test]
    fn test_create_material_minimal() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        let req = UploadMaterialRequest {
            project_id: "project-1".to_string(),
            name: "简单素材".to_string(),
            material_type: "text".to_string(),
            file_path: None,
            content: None,
            tags: None,
            description: None,
        };

        let material = MaterialDao::create(&conn, &req).unwrap();

        assert!(!material.id.is_empty());
        assert_eq!(material.name, "简单素材");
        assert!(material.tags.is_empty());
        assert!(material.description.is_none());
    }

    #[test]
    fn test_get_material() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        let req = UploadMaterialRequest {
            project_id: "project-1".to_string(),
            name: "测试素材".to_string(),
            material_type: "image".to_string(),
            file_path: None,
            content: None,
            tags: Some(vec!["图片".to_string()]),
            description: Some("一张图片".to_string()),
        };

        let created = MaterialDao::create(&conn, &req).unwrap();
        let fetched = MaterialDao::get(&conn, &created.id).unwrap();

        assert!(fetched.is_some());
        let fetched = fetched.unwrap();
        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.name, "测试素材");
        assert_eq!(fetched.material_type, "image");
    }

    #[test]
    fn test_get_nonexistent_material() {
        let conn = setup_test_db();
        let result = MaterialDao::get(&conn, "nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_list_materials() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");
        create_test_project(&conn, "project-2");

        // 为 project-1 创建三个素材
        for i in 1..=3 {
            let req = UploadMaterialRequest {
                project_id: "project-1".to_string(),
                name: format!("素材{}", i),
                material_type: "document".to_string(),
                file_path: None,
                content: None,
                tags: None,
                description: None,
            };
            MaterialDao::create(&conn, &req).unwrap();
        }

        // 为 project-2 创建一个素材
        let req = UploadMaterialRequest {
            project_id: "project-2".to_string(),
            name: "素材4".to_string(),
            material_type: "image".to_string(),
            file_path: None,
            content: None,
            tags: None,
            description: None,
        };
        MaterialDao::create(&conn, &req).unwrap();

        // 验证 project-1 有 3 个素材
        let materials = MaterialDao::list(&conn, "project-1", None).unwrap();
        assert_eq!(materials.len(), 3);

        // 验证 project-2 有 1 个素材
        let materials = MaterialDao::list(&conn, "project-2", None).unwrap();
        assert_eq!(materials.len(), 1);
    }

    #[test]
    fn test_list_materials_filter_by_type() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        // 创建不同类型的素材
        let types = vec!["document", "image", "document", "text"];
        for (i, t) in types.iter().enumerate() {
            let req = UploadMaterialRequest {
                project_id: "project-1".to_string(),
                name: format!("素材{}", i),
                material_type: t.to_string(),
                file_path: None,
                content: None,
                tags: None,
                description: None,
            };
            MaterialDao::create(&conn, &req).unwrap();
        }

        // 筛选 document 类型
        let filter = MaterialFilter {
            material_type: Some("document".to_string()),
            tags: None,
            search_query: None,
        };
        let materials = MaterialDao::list(&conn, "project-1", Some(&filter)).unwrap();
        assert_eq!(materials.len(), 2);

        // 筛选 image 类型
        let filter = MaterialFilter {
            material_type: Some("image".to_string()),
            tags: None,
            search_query: None,
        };
        let materials = MaterialDao::list(&conn, "project-1", Some(&filter)).unwrap();
        assert_eq!(materials.len(), 1);
    }

    #[test]
    fn test_list_materials_filter_by_search() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        // 创建素材
        let names = vec!["重要文档", "普通文档", "重要图片"];
        for name in names {
            let req = UploadMaterialRequest {
                project_id: "project-1".to_string(),
                name: name.to_string(),
                material_type: "document".to_string(),
                file_path: None,
                content: None,
                tags: None,
                description: None,
            };
            MaterialDao::create(&conn, &req).unwrap();
        }

        // 搜索 "重要"
        let filter = MaterialFilter {
            material_type: None,
            tags: None,
            search_query: Some("重要".to_string()),
        };
        let materials = MaterialDao::list(&conn, "project-1", Some(&filter)).unwrap();
        assert_eq!(materials.len(), 2);
    }

    #[test]
    fn test_list_materials_filter_by_tags() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        // 创建带标签的素材
        let req1 = UploadMaterialRequest {
            project_id: "project-1".to_string(),
            name: "素材1".to_string(),
            material_type: "document".to_string(),
            file_path: None,
            content: None,
            tags: Some(vec!["重要".to_string(), "参考".to_string()]),
            description: None,
        };
        MaterialDao::create(&conn, &req1).unwrap();

        let req2 = UploadMaterialRequest {
            project_id: "project-1".to_string(),
            name: "素材2".to_string(),
            material_type: "document".to_string(),
            file_path: None,
            content: None,
            tags: Some(vec!["普通".to_string()]),
            description: None,
        };
        MaterialDao::create(&conn, &req2).unwrap();

        // 筛选包含 "重要" 标签的素材
        let filter = MaterialFilter {
            material_type: None,
            tags: Some(vec!["重要".to_string()]),
            search_query: None,
        };
        let materials = MaterialDao::list(&conn, "project-1", Some(&filter)).unwrap();
        assert_eq!(materials.len(), 1);
        assert_eq!(materials[0].name, "素材1");
    }

    #[test]
    fn test_update_material() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        let req = UploadMaterialRequest {
            project_id: "project-1".to_string(),
            name: "原始名称".to_string(),
            material_type: "document".to_string(),
            file_path: None,
            content: None,
            tags: Some(vec!["标签1".to_string()]),
            description: Some("原始描述".to_string()),
        };

        let created = MaterialDao::create(&conn, &req).unwrap();

        let update = MaterialUpdate {
            name: Some("更新后名称".to_string()),
            tags: Some(vec!["标签2".to_string(), "标签3".to_string()]),
            description: Some("更新后描述".to_string()),
        };

        let updated = MaterialDao::update(&conn, &created.id, &update).unwrap();

        assert_eq!(updated.name, "更新后名称");
        assert_eq!(updated.tags.len(), 2);
        assert!(updated.tags.contains(&"标签2".to_string()));
        assert_eq!(updated.description, Some("更新后描述".to_string()));
        // 验证其他字段未变
        assert_eq!(updated.material_type, "document");
    }

    #[test]
    fn test_update_material_partial() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        let req = UploadMaterialRequest {
            project_id: "project-1".to_string(),
            name: "原始名称".to_string(),
            material_type: "document".to_string(),
            file_path: None,
            content: None,
            tags: Some(vec!["标签1".to_string()]),
            description: Some("原始描述".to_string()),
        };

        let created = MaterialDao::create(&conn, &req).unwrap();

        // 只更新名称
        let update = MaterialUpdate {
            name: Some("新名称".to_string()),
            tags: None,
            description: None,
        };

        let updated = MaterialDao::update(&conn, &created.id, &update).unwrap();

        assert_eq!(updated.name, "新名称");
        // 其他字段保持不变
        assert_eq!(updated.tags, vec!["标签1".to_string()]);
        assert_eq!(updated.description, Some("原始描述".to_string()));
    }

    #[test]
    fn test_update_nonexistent_material() {
        let conn = setup_test_db();
        let update = MaterialUpdate::default();
        let result = MaterialDao::update(&conn, "nonexistent", &update);
        assert!(result.is_err());
    }

    #[test]
    fn test_delete_material() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        let req = UploadMaterialRequest {
            project_id: "project-1".to_string(),
            name: "待删除素材".to_string(),
            material_type: "document".to_string(),
            file_path: Some("/path/to/file.pdf".to_string()),
            content: None,
            tags: None,
            description: None,
        };

        let created = MaterialDao::create(&conn, &req).unwrap();

        // 验证素材存在
        assert!(MaterialDao::get(&conn, &created.id).unwrap().is_some());

        // 删除素材
        let deleted = MaterialDao::delete(&conn, &created.id).unwrap();

        // 验证返回的素材信息正确
        assert_eq!(deleted.id, created.id);
        assert_eq!(deleted.file_path, Some("/path/to/file.pdf".to_string()));

        // 验证素材已删除
        assert!(MaterialDao::get(&conn, &created.id).unwrap().is_none());
    }

    #[test]
    fn test_delete_nonexistent_material() {
        let conn = setup_test_db();
        let result = MaterialDao::delete(&conn, "nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_count_materials() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");

        // 初始数量为 0
        let count = MaterialDao::count(&conn, "project-1").unwrap();
        assert_eq!(count, 0);

        // 创建 3 个素材
        for i in 1..=3 {
            let req = UploadMaterialRequest {
                project_id: "project-1".to_string(),
                name: format!("素材{}", i),
                material_type: "document".to_string(),
                file_path: None,
                content: None,
                tags: None,
                description: None,
            };
            MaterialDao::create(&conn, &req).unwrap();
        }

        // 验证数量为 3
        let count = MaterialDao::count(&conn, "project-1").unwrap();
        assert_eq!(count, 3);
    }

    #[test]
    fn test_delete_by_project() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");
        create_test_project(&conn, "project-2");

        // 为 project-1 创建 2 个素材
        for i in 1..=2 {
            let req = UploadMaterialRequest {
                project_id: "project-1".to_string(),
                name: format!("素材{}", i),
                material_type: "document".to_string(),
                file_path: Some(format!("/path/to/file{}.pdf", i)),
                content: None,
                tags: None,
                description: None,
            };
            MaterialDao::create(&conn, &req).unwrap();
        }

        // 为 project-2 创建 1 个素材
        let req = UploadMaterialRequest {
            project_id: "project-2".to_string(),
            name: "素材3".to_string(),
            material_type: "document".to_string(),
            file_path: None,
            content: None,
            tags: None,
            description: None,
        };
        MaterialDao::create(&conn, &req).unwrap();

        // 删除 project-1 的所有素材
        let deleted = MaterialDao::delete_by_project(&conn, "project-1").unwrap();

        // 验证返回了 2 个被删除的素材
        assert_eq!(deleted.len(), 2);

        // 验证 project-1 没有素材了
        let materials = MaterialDao::list(&conn, "project-1", None).unwrap();
        assert_eq!(materials.len(), 0);

        // 验证 project-2 的素材未受影响
        let materials = MaterialDao::list(&conn, "project-2", None).unwrap();
        assert_eq!(materials.len(), 1);
    }

    #[test]
    fn test_project_scoped_query_correctness() {
        // Property 2: Project-Scoped Query Correctness
        // 验证按 project_id 筛选的查询只返回属于该项目的素材
        let conn = setup_test_db();
        create_test_project(&conn, "project-a");
        create_test_project(&conn, "project-b");

        // 为两个项目创建素材
        for i in 1..=3 {
            let req = UploadMaterialRequest {
                project_id: "project-a".to_string(),
                name: format!("A素材{}", i),
                material_type: "document".to_string(),
                file_path: None,
                content: None,
                tags: None,
                description: None,
            };
            MaterialDao::create(&conn, &req).unwrap();
        }

        for i in 1..=2 {
            let req = UploadMaterialRequest {
                project_id: "project-b".to_string(),
                name: format!("B素材{}", i),
                material_type: "image".to_string(),
                file_path: None,
                content: None,
                tags: None,
                description: None,
            };
            MaterialDao::create(&conn, &req).unwrap();
        }

        // 查询 project-a 的素材
        let materials_a = MaterialDao::list(&conn, "project-a", None).unwrap();
        assert_eq!(materials_a.len(), 3);
        for m in &materials_a {
            assert_eq!(m.project_id, "project-a");
        }

        // 查询 project-b 的素材
        let materials_b = MaterialDao::list(&conn, "project-b", None).unwrap();
        assert_eq!(materials_b.len(), 2);
        for m in &materials_b {
            assert_eq!(m.project_id, "project-b");
        }
    }
}
