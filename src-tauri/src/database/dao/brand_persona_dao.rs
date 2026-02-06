//! 品牌人设扩展数据访问层
//!
//! 提供品牌人设扩展（BrandPersonaExtension）的 CRUD 操作，包括：
//! - 创建、获取、更新、删除品牌人设扩展
//! - 获取完整的品牌人设（基础人设 + 扩展）

use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::errors::project_error::PersonaError;
use crate::models::project_model::{
    BrandPersona, BrandPersonaExtension, BrandPersonaTemplate, BrandTone,
    CreateBrandExtensionRequest, DesignConfig, UpdateBrandExtensionRequest, VisualConfig,
};

use super::persona_dao::PersonaDao;

// ============================================================================
// 数据访问对象
// ============================================================================

/// 品牌人设扩展 DAO
///
/// 提供品牌人设扩展的数据库操作方法。
pub struct BrandPersonaDao;

impl BrandPersonaDao {
    // ------------------------------------------------------------------------
    // 创建品牌人设扩展
    // ------------------------------------------------------------------------

    /// 创建品牌人设扩展
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `req`: 创建请求
    ///
    /// # 返回
    /// - 成功返回创建的扩展
    /// - 失败返回 PersonaError
    pub fn create(
        conn: &Connection,
        req: &CreateBrandExtensionRequest,
    ) -> Result<BrandPersonaExtension, PersonaError> {
        // 验证人设存在
        PersonaDao::get(conn, &req.persona_id)?
            .ok_or_else(|| PersonaError::NotFound(req.persona_id.clone()))?;

        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().timestamp();

        // 序列化 JSON 字段
        let brand_tone = req.brand_tone.clone().unwrap_or_default();
        let design = req.design.clone().unwrap_or_default();
        let visual = req.visual.clone().unwrap_or_default();

        let brand_tone_json =
            serde_json::to_string(&brand_tone).unwrap_or_else(|_| "{}".to_string());
        let design_json = serde_json::to_string(&design).unwrap_or_else(|_| "{}".to_string());
        let visual_json = serde_json::to_string(&visual).unwrap_or_else(|_| "{}".to_string());

        conn.execute(
            "INSERT INTO brand_persona_extensions (
                id, persona_id, brand_tone_json, design_json, visual_json,
                created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                id,
                req.persona_id,
                brand_tone_json,
                design_json,
                visual_json,
                now,
                now,
            ],
        )?;

        Ok(BrandPersonaExtension {
            persona_id: req.persona_id.clone(),
            brand_tone,
            design,
            visual,
            created_at: now,
            updated_at: now,
        })
    }

    // ------------------------------------------------------------------------
    // 获取品牌人设扩展
    // ------------------------------------------------------------------------

    /// 获取品牌人设扩展
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `persona_id`: 人设 ID
    ///
    /// # 返回
    /// - 成功返回 Option<BrandPersonaExtension>
    /// - 失败返回 PersonaError
    pub fn get(
        conn: &Connection,
        persona_id: &str,
    ) -> Result<Option<BrandPersonaExtension>, PersonaError> {
        let mut stmt = conn.prepare(
            "SELECT persona_id, brand_tone_json, design_json, visual_json, created_at, updated_at
             FROM brand_persona_extensions WHERE persona_id = ?",
        )?;

        let mut rows = stmt.query([persona_id])?;

        if let Some(row) = rows.next()? {
            Ok(Some(Self::map_row(row)?))
        } else {
            Ok(None)
        }
    }

    /// 获取完整的品牌人设（基础人设 + 扩展）
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `persona_id`: 人设 ID
    ///
    /// # 返回
    /// - 成功返回 Option<BrandPersona>
    /// - 失败返回 PersonaError
    pub fn get_brand_persona(
        conn: &Connection,
        persona_id: &str,
    ) -> Result<Option<BrandPersona>, PersonaError> {
        // 获取基础人设
        let base = match PersonaDao::get(conn, persona_id)? {
            Some(p) => p,
            None => return Ok(None),
        };

        // 获取扩展
        let extension = Self::get(conn, persona_id)?;

        Ok(Some(BrandPersona {
            base,
            brand_tone: extension.as_ref().map(|e| e.brand_tone.clone()),
            design: extension.as_ref().map(|e| e.design.clone()),
            visual: extension.as_ref().map(|e| e.visual.clone()),
        }))
    }

    // ------------------------------------------------------------------------
    // 更新品牌人设扩展
    // ------------------------------------------------------------------------

    /// 更新品牌人设扩展
    ///
    /// 如果扩展不存在，则创建新的扩展。
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `persona_id`: 人设 ID
    /// - `update`: 更新内容
    ///
    /// # 返回
    /// - 成功返回更新后的扩展
    /// - 失败返回 PersonaError
    pub fn update(
        conn: &Connection,
        persona_id: &str,
        update: &UpdateBrandExtensionRequest,
    ) -> Result<BrandPersonaExtension, PersonaError> {
        // 验证人设存在
        PersonaDao::get(conn, persona_id)?
            .ok_or_else(|| PersonaError::NotFound(persona_id.to_string()))?;

        // 检查扩展是否存在
        let existing = Self::get(conn, persona_id)?;

        if existing.is_none() {
            // 创建新扩展
            let req = CreateBrandExtensionRequest {
                persona_id: persona_id.to_string(),
                brand_tone: update.brand_tone.clone(),
                design: update.design.clone(),
                visual: update.visual.clone(),
            };
            return Self::create(conn, &req);
        }

        let existing = existing.unwrap();
        let now = chrono::Utc::now().timestamp();

        // 构建更新后的值
        let brand_tone = update.brand_tone.clone().unwrap_or(existing.brand_tone);
        let design = update.design.clone().unwrap_or(existing.design);
        let visual = update.visual.clone().unwrap_or(existing.visual);

        // 序列化 JSON 字段
        let brand_tone_json =
            serde_json::to_string(&brand_tone).unwrap_or_else(|_| "{}".to_string());
        let design_json = serde_json::to_string(&design).unwrap_or_else(|_| "{}".to_string());
        let visual_json = serde_json::to_string(&visual).unwrap_or_else(|_| "{}".to_string());

        conn.execute(
            "UPDATE brand_persona_extensions SET
                brand_tone_json = ?1, design_json = ?2, visual_json = ?3, updated_at = ?4
             WHERE persona_id = ?5",
            params![brand_tone_json, design_json, visual_json, now, persona_id,],
        )?;

        Ok(BrandPersonaExtension {
            persona_id: persona_id.to_string(),
            brand_tone,
            design,
            visual,
            created_at: existing.created_at,
            updated_at: now,
        })
    }

    // ------------------------------------------------------------------------
    // 删除品牌人设扩展
    // ------------------------------------------------------------------------

    /// 删除品牌人设扩展
    ///
    /// # 参数
    /// - `conn`: 数据库连接
    /// - `persona_id`: 人设 ID
    ///
    /// # 返回
    /// - 成功返回 ()
    /// - 失败返回 PersonaError
    pub fn delete(conn: &Connection, persona_id: &str) -> Result<(), PersonaError> {
        conn.execute(
            "DELETE FROM brand_persona_extensions WHERE persona_id = ?",
            [persona_id],
        )?;
        Ok(())
    }

    // ------------------------------------------------------------------------
    // 辅助方法
    // ------------------------------------------------------------------------

    /// 映射数据库行到 BrandPersonaExtension 结构体
    fn map_row(row: &rusqlite::Row) -> Result<BrandPersonaExtension, rusqlite::Error> {
        let brand_tone_json: String = row.get(1)?;
        let design_json: String = row.get(2)?;
        let visual_json: String = row.get(3)?;

        // 解析 JSON 字段
        let brand_tone: BrandTone = serde_json::from_str(&brand_tone_json).unwrap_or_default();
        let design: DesignConfig = serde_json::from_str(&design_json).unwrap_or_default();
        let visual: VisualConfig = serde_json::from_str(&visual_json).unwrap_or_default();

        Ok(BrandPersonaExtension {
            persona_id: row.get(0)?,
            brand_tone,
            design,
            visual,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        })
    }

    // ------------------------------------------------------------------------
    // 品牌人设模板
    // ------------------------------------------------------------------------

    /// 获取预定义的品牌人设模板列表
    pub fn list_templates() -> Vec<BrandPersonaTemplate> {
        vec![
            BrandPersonaTemplate {
                id: "ecommerce-promo".to_string(),
                name: "电商促销".to_string(),
                description: "适合电商促销、限时优惠等场景".to_string(),
                brand_tone: BrandTone {
                    keywords: vec!["实惠".to_string(), "限时".to_string(), "优惠".to_string()],
                    personality: "bold".to_string(),
                    voice_tone: Some("紧迫感、吸引力".to_string()),
                    target_audience: Some("追求性价比的消费者".to_string()),
                },
                design: DesignConfig {
                    primary_style: "bold".to_string(),
                    color_scheme: crate::models::project_model::ColorScheme {
                        primary: "#FF4757".to_string(),
                        secondary: "#FFA502".to_string(),
                        accent: "#FF6348".to_string(),
                        background: "#FFFFFF".to_string(),
                        text: "#2F3542".to_string(),
                        text_secondary: "#57606F".to_string(),
                        gradients: None,
                    },
                    typography: crate::models::project_model::Typography {
                        title_font: "阿里巴巴普惠体".to_string(),
                        title_weight: 700,
                        body_font: "思源黑体".to_string(),
                        body_weight: 400,
                        title_size: 80,
                        body_size: 24,
                        line_height: 1.4,
                        letter_spacing: 0.0,
                    },
                },
                visual: None,
            },
            BrandPersonaTemplate {
                id: "brand-image".to_string(),
                name: "品牌形象".to_string(),
                description: "适合品牌宣传、企业形象展示".to_string(),
                brand_tone: BrandTone {
                    keywords: vec!["专业".to_string(), "可信赖".to_string(), "品质".to_string()],
                    personality: "professional".to_string(),
                    voice_tone: Some("专业但不冷漠".to_string()),
                    target_audience: Some("注重品质的消费者".to_string()),
                },
                design: DesignConfig {
                    primary_style: "modern".to_string(),
                    color_scheme: crate::models::project_model::ColorScheme {
                        primary: "#2196F3".to_string(),
                        secondary: "#90CAF9".to_string(),
                        accent: "#1976D2".to_string(),
                        background: "#FFFFFF".to_string(),
                        text: "#212121".to_string(),
                        text_secondary: "#757575".to_string(),
                        gradients: None,
                    },
                    typography: crate::models::project_model::Typography {
                        title_font: "思源黑体".to_string(),
                        title_weight: 600,
                        body_font: "苹方".to_string(),
                        body_weight: 400,
                        title_size: 64,
                        body_size: 20,
                        line_height: 1.6,
                        letter_spacing: 1.0,
                    },
                },
                visual: None,
            },
            BrandPersonaTemplate {
                id: "social-media".to_string(),
                name: "社交媒体".to_string(),
                description: "适合小红书、抖音等社交平台".to_string(),
                brand_tone: BrandTone {
                    keywords: vec!["年轻".to_string(), "时尚".to_string(), "潮流".to_string()],
                    personality: "playful".to_string(),
                    voice_tone: Some("轻松活泼、有趣".to_string()),
                    target_audience: Some("18-30岁年轻人".to_string()),
                },
                design: DesignConfig {
                    primary_style: "playful".to_string(),
                    color_scheme: crate::models::project_model::ColorScheme {
                        primary: "#FF6B9D".to_string(),
                        secondary: "#FFC0D0".to_string(),
                        accent: "#FF4081".to_string(),
                        background: "#FFFFFF".to_string(),
                        text: "#333333".to_string(),
                        text_secondary: "#666666".to_string(),
                        gradients: None,
                    },
                    typography: crate::models::project_model::Typography {
                        title_font: "站酷快乐体".to_string(),
                        title_weight: 400,
                        body_font: "思源黑体".to_string(),
                        body_weight: 400,
                        title_size: 72,
                        body_size: 22,
                        line_height: 1.5,
                        letter_spacing: 0.0,
                    },
                },
                visual: None,
            },
            BrandPersonaTemplate {
                id: "event-promo".to_string(),
                name: "活动宣传".to_string(),
                description: "适合活动宣传、节日促销".to_string(),
                brand_tone: BrandTone {
                    keywords: vec!["热闹".to_string(), "参与".to_string(), "精彩".to_string()],
                    personality: "bold".to_string(),
                    voice_tone: Some("热情洋溢、感染力强".to_string()),
                    target_audience: Some("活动目标参与者".to_string()),
                },
                design: DesignConfig {
                    primary_style: "bold".to_string(),
                    color_scheme: crate::models::project_model::ColorScheme {
                        primary: "#FF9500".to_string(),
                        secondary: "#FFD166".to_string(),
                        accent: "#EF476F".to_string(),
                        background: "#FFFFFF".to_string(),
                        text: "#2D3436".to_string(),
                        text_secondary: "#636E72".to_string(),
                        gradients: None,
                    },
                    typography: crate::models::project_model::Typography {
                        title_font: "站酷庆科黄油体".to_string(),
                        title_weight: 400,
                        body_font: "思源黑体".to_string(),
                        body_weight: 400,
                        title_size: 80,
                        body_size: 24,
                        line_height: 1.4,
                        letter_spacing: 0.0,
                    },
                },
                visual: None,
            },
        ]
    }
}

// ============================================================================
// 测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::database::schema::create_tables;
    use crate::models::project_model::CreatePersonaRequest;
    use crate::models::Persona;

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

    /// 创建测试人设
    fn create_test_persona(conn: &Connection, project_id: &str) -> Persona {
        let req = CreatePersonaRequest {
            project_id: project_id.to_string(),
            name: "测试人设".to_string(),
            description: None,
            style: "专业".to_string(),
            tone: None,
            target_audience: None,
            forbidden_words: None,
            preferred_words: None,
            examples: None,
            platforms: None,
        };
        PersonaDao::create(conn, &req).unwrap()
    }

    #[test]
    fn test_create_brand_extension() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");
        let persona = create_test_persona(&conn, "project-1");

        let req = CreateBrandExtensionRequest {
            persona_id: persona.id.clone(),
            brand_tone: Some(BrandTone {
                keywords: vec!["专业".to_string(), "可信赖".to_string()],
                personality: "professional".to_string(),
                voice_tone: Some("专业但不冷漠".to_string()),
                target_audience: Some("技术人员".to_string()),
            }),
            design: None,
            visual: None,
        };

        let extension = BrandPersonaDao::create(&conn, &req).unwrap();

        assert_eq!(extension.persona_id, persona.id);
        assert_eq!(extension.brand_tone.keywords.len(), 2);
        assert_eq!(extension.brand_tone.personality, "professional");
    }

    #[test]
    fn test_get_brand_extension() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");
        let persona = create_test_persona(&conn, "project-1");

        let req = CreateBrandExtensionRequest {
            persona_id: persona.id.clone(),
            brand_tone: Some(BrandTone::default()),
            design: Some(DesignConfig::default()),
            visual: Some(VisualConfig::default()),
        };

        BrandPersonaDao::create(&conn, &req).unwrap();

        let extension = BrandPersonaDao::get(&conn, &persona.id).unwrap();
        assert!(extension.is_some());
        let extension = extension.unwrap();
        assert_eq!(extension.persona_id, persona.id);
    }

    #[test]
    fn test_get_nonexistent_extension() {
        let conn = setup_test_db();
        let result = BrandPersonaDao::get(&conn, "nonexistent").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_get_brand_persona() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");
        let persona = create_test_persona(&conn, "project-1");

        // 创建扩展
        let req = CreateBrandExtensionRequest {
            persona_id: persona.id.clone(),
            brand_tone: Some(BrandTone {
                keywords: vec!["测试".to_string()],
                personality: "friendly".to_string(),
                voice_tone: None,
                target_audience: None,
            }),
            design: None,
            visual: None,
        };
        BrandPersonaDao::create(&conn, &req).unwrap();

        // 获取完整品牌人设
        let brand_persona = BrandPersonaDao::get_brand_persona(&conn, &persona.id).unwrap();
        assert!(brand_persona.is_some());
        let brand_persona = brand_persona.unwrap();

        assert_eq!(brand_persona.base.id, persona.id);
        assert!(brand_persona.brand_tone.is_some());
        assert_eq!(brand_persona.brand_tone.unwrap().personality, "friendly");
    }

    #[test]
    fn test_get_brand_persona_without_extension() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");
        let persona = create_test_persona(&conn, "project-1");

        // 获取没有扩展的品牌人设
        let brand_persona = BrandPersonaDao::get_brand_persona(&conn, &persona.id).unwrap();
        assert!(brand_persona.is_some());
        let brand_persona = brand_persona.unwrap();

        assert_eq!(brand_persona.base.id, persona.id);
        assert!(brand_persona.brand_tone.is_none());
        assert!(brand_persona.design.is_none());
        assert!(brand_persona.visual.is_none());
    }

    #[test]
    fn test_update_brand_extension() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");
        let persona = create_test_persona(&conn, "project-1");

        // 创建扩展
        let req = CreateBrandExtensionRequest {
            persona_id: persona.id.clone(),
            brand_tone: Some(BrandTone {
                keywords: vec!["原始".to_string()],
                personality: "professional".to_string(),
                voice_tone: None,
                target_audience: None,
            }),
            design: None,
            visual: None,
        };
        BrandPersonaDao::create(&conn, &req).unwrap();

        // 更新扩展
        let update = UpdateBrandExtensionRequest {
            brand_tone: Some(BrandTone {
                keywords: vec!["更新".to_string(), "测试".to_string()],
                personality: "friendly".to_string(),
                voice_tone: Some("亲切".to_string()),
                target_audience: None,
            }),
            design: None,
            visual: None,
        };

        let updated = BrandPersonaDao::update(&conn, &persona.id, &update).unwrap();

        assert_eq!(updated.brand_tone.keywords.len(), 2);
        assert_eq!(updated.brand_tone.personality, "friendly");
        assert_eq!(updated.brand_tone.voice_tone, Some("亲切".to_string()));
    }

    #[test]
    fn test_update_creates_extension_if_not_exists() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");
        let persona = create_test_persona(&conn, "project-1");

        // 直接更新（不先创建）
        let update = UpdateBrandExtensionRequest {
            brand_tone: Some(BrandTone {
                keywords: vec!["新建".to_string()],
                personality: "bold".to_string(),
                voice_tone: None,
                target_audience: None,
            }),
            design: None,
            visual: None,
        };

        let result = BrandPersonaDao::update(&conn, &persona.id, &update).unwrap();

        assert_eq!(result.brand_tone.keywords, vec!["新建".to_string()]);
        assert_eq!(result.brand_tone.personality, "bold");
    }

    #[test]
    fn test_delete_brand_extension() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");
        let persona = create_test_persona(&conn, "project-1");

        // 创建扩展
        let req = CreateBrandExtensionRequest {
            persona_id: persona.id.clone(),
            brand_tone: Some(BrandTone::default()),
            design: None,
            visual: None,
        };
        BrandPersonaDao::create(&conn, &req).unwrap();

        // 验证存在
        assert!(BrandPersonaDao::get(&conn, &persona.id).unwrap().is_some());

        // 删除
        BrandPersonaDao::delete(&conn, &persona.id).unwrap();

        // 验证已删除
        assert!(BrandPersonaDao::get(&conn, &persona.id).unwrap().is_none());
    }

    #[test]
    fn test_list_templates() {
        let templates = BrandPersonaDao::list_templates();
        assert_eq!(templates.len(), 4);

        let template_ids: Vec<&str> = templates.iter().map(|t| t.id.as_str()).collect();
        assert!(template_ids.contains(&"ecommerce-promo"));
        assert!(template_ids.contains(&"brand-image"));
        assert!(template_ids.contains(&"social-media"));
        assert!(template_ids.contains(&"event-promo"));
    }

    #[test]
    fn test_cascade_delete() {
        let conn = setup_test_db();
        create_test_project(&conn, "project-1");
        let persona = create_test_persona(&conn, "project-1");

        // 创建扩展
        let req = CreateBrandExtensionRequest {
            persona_id: persona.id.clone(),
            brand_tone: Some(BrandTone::default()),
            design: None,
            visual: None,
        };
        BrandPersonaDao::create(&conn, &req).unwrap();

        // 验证扩展存在
        assert!(BrandPersonaDao::get(&conn, &persona.id).unwrap().is_some());

        // 删除人设
        PersonaDao::delete(&conn, &persona.id).unwrap();

        // 验证扩展也被删除（级联删除）
        assert!(BrandPersonaDao::get(&conn, &persona.id).unwrap().is_none());
    }
}
